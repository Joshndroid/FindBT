//! Minimal, dependency-free PDF writer for the FindBT report.
//!
//! Produces the same content as the HTML report ([`crate::report::generate_html`])
//! as an A4-landscape PDF, using only the built-in PDF Type1 fonts so the output
//! needs no embedded font data and generation works fully offline.
//!
//! The writer emits PDF 1.4 with a classic cross-reference table. Text is
//! restricted to printable ASCII (anything else is replaced with `?`), which
//! keeps the WinAnsi encoding assumptions trivially correct.

use std::collections::BTreeSet;

use chrono::Local;

use crate::{
    models::{normalize_address, ScanPhase},
    registry::DeviceRecord,
    report::{device_identity, format_duration, format_rssi, format_timestamp},
    session::CaptureSession,
};

/// A4 landscape, in PDF points.
const PAGE_WIDTH: f32 = 842.0;
const PAGE_HEIGHT: f32 = 595.0;
const MARGIN: f32 = 36.0;
const CONTENT_WIDTH: f32 = PAGE_WIDTH - 2.0 * MARGIN;

const FONT_REGULAR: &str = "F1";
const FONT_BOLD: &str = "F2";

const BODY_SIZE: f32 = 8.5;
const BODY_LEADING: f32 = 11.0;
const META_SIZE: f32 = 9.0;
const NOTE_SIZE: f32 = 8.0;

/// RGB fill/stroke color, each channel 0.0..=1.0.
type Rgb = (f32, f32, f32);

const COLOR_TEXT: Rgb = (0.09, 0.106, 0.122); // #171b1f
const COLOR_MUTED: Rgb = (0.392, 0.416, 0.439); // #646a70
const COLOR_ACCENT: Rgb = (0.0, 0.451, 0.812); // #0073cf
const COLOR_TABLE_HEAD_BG: Rgb = (0.937, 0.955, 0.969);
const COLOR_ZEBRA_BG: Rgb = (0.961, 0.973, 0.98);
const COLOR_RULE: Rgb = (0.78, 0.81, 0.835);

/// Renders the capture session as a standalone PDF document.
pub fn generate_pdf(session: &CaptureSession) -> Vec<u8> {
    assemble_document(&build_pages(session))
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

struct PdfLayout {
    pages: Vec<String>,
    current: String,
    y: f32,
}

impl PdfLayout {
    fn new() -> Self {
        Self {
            pages: Vec::new(),
            current: String::new(),
            y: PAGE_HEIGHT - MARGIN,
        }
    }

    fn finish_page(&mut self) {
        self.stamp_footer();
        let page = std::mem::take(&mut self.current);
        self.pages.push(page);
        self.y = PAGE_HEIGHT - MARGIN;
    }

    /// Starts a new page when fewer than `needed` points remain, unless the
    /// current page is still empty (a new page would not create more room).
    fn ensure_space(&mut self, needed: f32) {
        if self.y - needed < MARGIN && !self.current.is_empty() {
            self.finish_page();
        }
    }

    fn stamp_footer(&mut self) {
        let label = format!("FindBT Bluetooth Report - page {}", self.pages.len() + 1);
        self.text_op(MARGIN, 20.0, FONT_REGULAR, 7.5, COLOR_MUTED, &label);
    }

    fn text_op(&mut self, x: f32, y: f32, font: &str, size: f32, color: Rgb, content: &str) {
        self.current.push_str(&format!(
            "{:.3} {:.3} {:.3} rg BT /{font} {size} Tf 1 0 0 1 {x:.1} {y:.1} Tm ({}) Tj ET\n",
            color.0,
            color.1,
            color.2,
            escape_pdf_text(content)
        ));
    }

    fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Rgb) {
        self.current.push_str(&format!(
            "{:.3} {:.3} {:.3} rg {x:.1} {y:.1} {width:.1} {height:.1} re f\n",
            color.0, color.1, color.2
        ));
    }

    fn rule(&mut self, x1: f32, y: f32, x2: f32, color: Rgb, width: f32) {
        self.current.push_str(&format!(
            "{:.3} {:.3} {:.3} RG {width:.1} w {x1:.1} {y:.1} m {x2:.1} {y:.1} l S\n",
            color.0, color.1, color.2
        ));
    }

    fn heading(&mut self, text: &str, size: f32) {
        self.ensure_space(size + 20.0);
        self.y -= size + 8.0;
        self.text_op(MARGIN, self.y, FONT_BOLD, size, COLOR_ACCENT, text);
        self.y -= 4.0;
        self.rule(MARGIN, self.y, MARGIN + 46.0, COLOR_ACCENT, 1.6);
        self.y -= 4.0;
    }

    fn paragraph(&mut self, text: &str, size: f32) {
        for line in wrap_text(text, CONTENT_WIDTH, size) {
            self.ensure_space(size + 4.0);
            self.y -= size + 2.5;
            self.text_op(MARGIN, self.y, FONT_REGULAR, size, COLOR_TEXT, &line);
        }
    }

    /// Muted explanatory text under a heading or table.
    fn note(&mut self, text: &str) {
        for line in wrap_text(text, CONTENT_WIDTH, NOTE_SIZE) {
            self.ensure_space(NOTE_SIZE + 4.0);
            self.y -= NOTE_SIZE + 2.5;
            self.text_op(MARGIN, self.y, FONT_REGULAR, NOTE_SIZE, COLOR_MUTED, &line);
        }
    }

    fn spacer(&mut self, amount: f32) {
        self.y -= amount;
    }

    fn table(&mut self, headers: &[&str], widths: &[f32], rows: &[Vec<String>]) {
        let header_cells: Vec<String> = headers.iter().map(|h| (*h).to_string()).collect();
        self.table_row(&header_cells, widths, true, false);
        for (index, row) in rows.iter().enumerate() {
            self.table_row(row, widths, false, index % 2 == 1);
        }
        self.spacer(6.0);
    }

    fn table_row(&mut self, cells: &[String], widths: &[f32], header: bool, shaded: bool) {
        let font = if header { FONT_BOLD } else { FONT_REGULAR };
        let color = if header { COLOR_MUTED } else { COLOR_TEXT };
        let wrapped: Vec<Vec<String>> = cells
            .iter()
            .zip(widths)
            .map(|(cell, width)| wrap_text(cell, width - 4.0, BODY_SIZE))
            .collect();
        let line_count = wrapped.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let row_height = line_count as f32 * BODY_LEADING + 3.0;
        self.ensure_space(row_height + 2.0);

        let top = self.y;
        let total_width: f32 = widths.iter().sum();
        if header {
            self.fill_rect(
                MARGIN,
                top - row_height,
                total_width,
                row_height,
                COLOR_TABLE_HEAD_BG,
            );
        } else if shaded {
            self.fill_rect(
                MARGIN,
                top - row_height,
                total_width,
                row_height,
                COLOR_ZEBRA_BG,
            );
        }
        let mut x = MARGIN;
        for (column, lines) in wrapped.iter().enumerate() {
            let mut line_y = top - BODY_LEADING;
            for line in lines {
                self.text_op(x + 2.0, line_y, font, BODY_SIZE, color, line);
                line_y -= BODY_LEADING;
            }
            x += widths[column];
        }
        self.y = top - row_height;
        let rule_width = if header { 1.1 } else { 0.6 };
        self.rule(
            MARGIN,
            self.y + 1.5,
            MARGIN + total_width,
            COLOR_RULE,
            rule_width,
        );
    }

    fn into_pages(mut self) -> Vec<String> {
        if !self.current.is_empty() || self.pages.is_empty() {
            self.stamp_footer();
            let page = std::mem::take(&mut self.current);
            self.pages.push(page);
        }
        self.pages
    }
}

// ---------------------------------------------------------------------------
// Report content
// ---------------------------------------------------------------------------

fn build_pages(session: &CaptureSession) -> Vec<String> {
    let mut doc = PdfLayout::new();
    let generated_at = Local::now();
    let tagged_norm = normalize_address(&session.host.address);
    let tagged_display = if tagged_norm.is_empty() {
        session.host.address.clone()
    } else {
        tagged_norm
    };

    doc.heading("FindBT Bluetooth Report", 16.0);
    doc.spacer(2.0);
    for line in [
        format!("Generated: {}", format_timestamp(generated_at)),
        format!("Scan date: {}", session.metadata.date),
        format!("Scan reference: {}", session.metadata.name),
        format!("Target device reference: {}", session.metadata.section),
        format!("User: {}", session.metadata.user),
        format!("Computer name: {}", session.host.computer_name),
        format!("Tagged local radio: {tagged_display}"),
        format!(
            "Host adapter: {} {}",
            session.host.name, session.host.address
        ),
    ] {
        doc.paragraph(&line, META_SIZE);
    }

    doc.heading("Phase runs", 12.0);
    doc.note("Every start and stop of each scan phase, as recorded during the capture.");
    doc.spacer(4.0);
    let phase_run_widths = [250.0, 135.0, 135.0, 80.0, 170.0];
    let phase_run_rows: Vec<Vec<String>> = session
        .phase_runs
        .iter()
        .map(|run| {
            vec![
                run.phase.report_label().to_string(),
                format_timestamp(run.started_at),
                run.stopped_at.map(format_timestamp).unwrap_or_default(),
                run.duration().map(format_duration).unwrap_or_default(),
                run.stop_reason.clone(),
            ]
        })
        .collect();
    let phase_run_rows = non_empty(phase_run_rows, "No phase runs recorded.", 5);
    doc.table(
        &["Phase", "Started", "Stopped", "Duration", "Stop reason"],
        &phase_run_widths,
        &phase_run_rows,
    );

    doc.heading("Phase summary", 12.0);
    let summary_widths = [250.0, 320.0, 60.0, 70.0, 70.0];
    let summary_rows: Vec<Vec<String>> = ScanPhase::ALL
        .iter()
        .copied()
        .map(|phase| {
            let raw_count = session
                .raw_log
                .iter()
                .filter(|obs| obs.phase == phase)
                .count();
            let unique_addresses = session
                .registry
                .observed_in(phase)
                .map(device_identity)
                .collect::<BTreeSet<_>>()
                .len();
            let newly_seen = session.registry.newly_seen_in(phase).count();
            vec![
                phase.report_label().to_string(),
                phase.operator_instruction().to_string(),
                raw_count.to_string(),
                unique_addresses.to_string(),
                newly_seen.to_string(),
            ]
        })
        .collect();
    doc.table(
        &["Phase", "Purpose", "Raw obs.", "Unique addr.", "Newly seen"],
        &summary_widths,
        &summary_rows,
    );
    doc.note(
        "\"Newly seen\" means the first phase where that device appears in the registry. The \
         report does not compute or highlight a match; it only renders the observed phase data.",
    );

    doc.heading("Device registry", 12.0);
    doc.note("One row per device, tracked across all three phases.");
    doc.spacer(4.0);
    let registry_widths = [
        105.0, 38.0, 90.0, 36.0, 64.0, 60.0, 60.0, 60.0, 92.0, 74.0, 91.0,
    ];
    let registry_rows: Vec<Vec<String>> = session
        .registry
        .devices()
        .map(registry_row)
        .collect::<Vec<_>>();
    let registry_rows = non_empty(registry_rows, "No devices observed.", 11);
    doc.table(
        &[
            "Name",
            "Kind",
            "Address",
            "Paired",
            "First phase",
            "Baseline",
            "Target",
            "Confirmation",
            "Device id",
            "Last seen",
            "Properties",
        ],
        &registry_widths,
        &registry_rows,
    );
    doc.note(
        "Devices matching the tagged local radio address are prefixed with \"(tagged radio)\".",
    );

    doc.heading("Raw audit log", 12.0);
    doc.note("Append-only capture log. Every backend observation is listed, unfiltered.");
    doc.spacer(4.0);
    let raw_widths = [92.0, 46.0, 100.0, 38.0, 90.0, 36.0, 74.0, 120.0, 174.0];
    let raw_rows: Vec<Vec<String>> = session
        .raw_log
        .iter()
        .map(|obs| {
            vec![
                format_timestamp(obs.observed_at),
                format!("Phase {}", obs.phase.number()),
                obs.name.clone(),
                obs.kind.label().to_string(),
                obs.address.clone(),
                yes_no(obs.is_paired),
                format_rssi(obs.rssi),
                obs.device_id.clone(),
                obs.properties_summary.clone(),
            ]
        })
        .collect();
    let raw_rows = non_empty(raw_rows, "No raw observations recorded.", 9);
    doc.table(
        &[
            "Observed",
            "Phase",
            "Name",
            "Kind",
            "Address",
            "Paired",
            "RSSI",
            "Device id",
            "Properties",
        ],
        &raw_widths,
        &raw_rows,
    );

    doc.into_pages()
}

fn registry_row(device: &DeviceRecord) -> Vec<String> {
    let last_seen = device
        .observations
        .values()
        .map(|obs| obs.last_seen)
        .max()
        .map(format_timestamp)
        .unwrap_or_default();
    let properties = device
        .observations
        .values()
        .last()
        .map(|obs| obs.properties_summary.clone())
        .unwrap_or_default();
    let name = if device.is_local_radio {
        format!("(tagged radio) {}", device.name)
    } else {
        device.name.clone()
    };

    vec![
        name,
        device.kind.label().to_string(),
        device.address.clone(),
        yes_no(device.is_paired),
        device
            .first_seen_phase()
            .map(|phase| format!("Phase {}", phase.number()))
            .unwrap_or_default(),
        phase_rssi_cell(device, ScanPhase::Baseline),
        phase_rssi_cell(device, ScanPhase::Target),
        phase_rssi_cell(device, ScanPhase::Verification),
        device.device_id.clone(),
        last_seen,
        properties,
    ]
}

fn phase_rssi_cell(device: &DeviceRecord, phase: ScanPhase) -> String {
    device
        .seen_in(phase)
        .map(|obs| format_rssi(obs.rssi))
        .unwrap_or_else(|| "not seen".to_string())
}

fn yes_no(value: bool) -> String {
    if value { "Yes" } else { "No" }.to_string()
}

/// Replaces an empty row set with a single placeholder row of `columns` cells.
fn non_empty(rows: Vec<Vec<String>>, placeholder: &str, columns: usize) -> Vec<Vec<String>> {
    if rows.is_empty() {
        let mut row = vec![placeholder.to_string()];
        row.resize(columns, String::new());
        vec![row]
    } else {
        rows
    }
}

// ---------------------------------------------------------------------------
// Text handling
// ---------------------------------------------------------------------------

/// Escapes text for a PDF literal string and replaces anything outside
/// printable ASCII with `?` so the WinAnsi assumption always holds.
fn escape_pdf_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            ' '..='~' => out.push(ch),
            _ => out.push('?'),
        }
    }
    out
}

/// Greedy word wrap based on a conservative average glyph width for Helvetica
/// (0.5 em). Words longer than a line are hard-split.
fn wrap_text(text: &str, width: f32, size: f32) -> Vec<String> {
    let max_chars = ((width / (0.5 * size)).floor() as usize).max(4);
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        push_word(&mut lines, &mut current, word, max_chars);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn push_word(lines: &mut Vec<String>, current: &mut String, word: &str, max_chars: usize) {
    let word_len = word.chars().count();
    if !current.is_empty() {
        if current.chars().count() + 1 + word_len <= max_chars {
            current.push(' ');
            current.push_str(word);
            return;
        }
        lines.push(std::mem::take(current));
    }
    if word_len <= max_chars {
        *current = word.to_string();
        return;
    }
    let mut chunk = String::new();
    let mut chunk_len = 0usize;
    for ch in word.chars() {
        if chunk_len == max_chars {
            lines.push(std::mem::take(&mut chunk));
            chunk_len = 0;
        }
        chunk.push(ch);
        chunk_len += 1;
    }
    *current = chunk;
}

// ---------------------------------------------------------------------------
// PDF file assembly
// ---------------------------------------------------------------------------

/// Object numbering: 1 = Catalog, 2 = Pages, 3 = Helvetica, 4 = Helvetica-Bold,
/// then for page k (0-based): 5+2k = content stream, 6+2k = page object.
fn assemble_document(pages: &[String]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets: Vec<usize> = Vec::new();

    let kids = (0..pages.len())
        .map(|k| format!("{} 0 R", 6 + 2 * k))
        .collect::<Vec<_>>()
        .join(" ");

    push_object(&mut out, &mut offsets, "<< /Type /Catalog /Pages 2 0 R >>");
    push_object(
        &mut out,
        &mut offsets,
        &format!("<< /Type /Pages /Kids [ {kids} ] /Count {} >>", pages.len()),
    );
    push_object(
        &mut out,
        &mut offsets,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    push_object(
        &mut out,
        &mut offsets,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>",
    );

    for (k, content) in pages.iter().enumerate() {
        let content_id = 5 + 2 * k;
        push_object(
            &mut out,
            &mut offsets,
            &format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ),
        );
        push_object(
            &mut out,
            &mut offsets,
            &format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] \
                 /Resources << /Font << /{FONT_REGULAR} 3 0 R /{FONT_BOLD} 4 0 R >> >> \
                 /Contents {content_id} 0 R >>"
            ),
        );
    }

    let xref_offset = out.len();
    let mut trailer = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1);
    for offset in &offsets {
        trailer.push_str(&format!("{offset:010} 00000 n \n"));
    }
    trailer.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
        offsets.len() + 1
    ));
    out.extend_from_slice(trailer.as_bytes());
    out
}

fn push_object(out: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &str) {
    let id = offsets.len() + 1;
    offsets.push(out.len());
    out.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
}

#[cfg(test)]
mod tests {
    use chrono::{Local, NaiveDate};

    use super::*;
    use crate::{CaseMetadata, DeviceKind, HostAdapterInfo, RawObservation};

    fn fixture_session() -> CaptureSession {
        let mut session = CaptureSession::new(
            CaseMetadata {
                date: NaiveDate::from_ymd_opt(2026, 7, 3).unwrap(),
                name: "Alice & Bob (Case 12)".to_string(),
                section: "Lab \\ Section <A>".to_string(),
                user: "Operator One".to_string(),
            },
            HostAdapterInfo {
                name: "Host Radio".to_string(),
                address: "AA:BB:CC:DD:EE:FF".to_string(),
                computer_name: "LAB-PC-01".to_string(),
            },
        );
        session.record(RawObservation {
            device_id: "dev-target".to_string(),
            phase: ScanPhase::Target,
            name: "Target (Device)".to_string(),
            address: "11:22:33:44:55:66".to_string(),
            kind: DeviceKind::Ble,
            is_paired: false,
            rssi: Some(-42),
            properties_summary: "role=candidate".to_string(),
            observed_at: Local::now(),
        });
        session
    }

    fn as_text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).to_string()
    }

    #[test]
    fn pdf_has_valid_header_trailer_and_content() {
        let pdf = generate_pdf(&fixture_session());
        let text = as_text(&pdf);

        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.trim_end().ends_with("%%EOF"));
        assert!(text.contains("(FindBT Bluetooth Report)"));
        assert!(text.contains("Target \\(Device\\)"));
        assert!(text.contains("11:22:33:44:55:66"));
        assert!(text.contains("/Type /Catalog"));
        assert!(text.contains("/BaseFont /Helvetica"));
        // Table header band and page footer are present.
        assert!(text.contains("re f"));
        assert!(text.contains("(FindBT Bluetooth Report - page 1)"));
    }

    #[test]
    fn pdf_startxref_points_at_xref_table() {
        let pdf = generate_pdf(&fixture_session());
        let text = as_text(&pdf);

        let startxref = text
            .rfind("startxref\n")
            .map(|index| index + "startxref\n".len())
            .expect("startxref keyword");
        let offset: usize = text[startxref..]
            .lines()
            .next()
            .expect("offset line")
            .trim()
            .parse()
            .expect("numeric xref offset");
        assert_eq!(&pdf[offset..offset + 4], b"xref");
    }

    #[test]
    fn pdf_object_offsets_match_xref_entries() {
        let pdf = generate_pdf(&fixture_session());
        let text = as_text(&pdf);

        let xref_start = text.rfind("\nxref\n").expect("xref table") + 1;
        let entries: Vec<usize> = text[xref_start..]
            .lines()
            .skip(2) // "xref" and "0 N" lines
            .take_while(|line| line.ends_with("n ") || line.ends_with("f "))
            .skip(1) // free-object entry
            .map(|line| line[..10].parse::<usize>().expect("offset"))
            .collect();

        assert!(!entries.is_empty());
        for (index, offset) in entries.iter().enumerate() {
            let expected = format!("{} 0 obj", index + 1);
            assert_eq!(
                &pdf[*offset..*offset + expected.len()],
                expected.as_bytes(),
                "object {} offset mismatch",
                index + 1
            );
        }
    }

    #[test]
    fn wrap_text_hard_splits_long_words() {
        let lines = wrap_text("Averylongunbrokenidentifierthatkeepsgoing", 40.0, 8.0);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.chars().count() <= 10));
    }

    #[test]
    fn escape_handles_delimiters_and_non_ascii() {
        assert_eq!(escape_pdf_text("a(b)c\\d"), "a\\(b\\)c\\\\d");
        assert_eq!(escape_pdf_text("café"), "caf?");
    }
}
