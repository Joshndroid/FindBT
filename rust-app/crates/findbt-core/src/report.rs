use std::collections::BTreeSet;

use chrono::{DateTime, Local};

use crate::{
    models::{normalize_address, DeviceKind, ScanPhase, SignalStrength},
    registry::{DeviceRecord, RawObservation},
    session::CaptureSession,
};

/// Stylesheet for the standalone HTML report. Kept as a plain constant so the
/// HTML template below needs no brace escaping. Everything is self-contained:
/// no external fonts, scripts, or images.
const STYLE: &str = r#"    :root {
      --accent: #0073cf; --accent-deep: #00509b; --accent-soft: #daeeff;
      --ink: #171b1f; --muted: #646a70; --border: #dbdee3; --border-soft: #e5e8ec;
      --bg: #f2f5f8; --card: #ffffff;
      --strong: #1b9247; --medium: #af7c00; --weak: #cb4644;
    }
    * { box-sizing: border-box; }
    body {
      font-family: -apple-system, "Segoe UI", Roboto, Arial, sans-serif;
      margin: 0; background: var(--bg); color: var(--ink);
    }
    .page { max-width: 1180px; margin: 0 auto; padding: 32px 24px 64px; }
    header.report {
      background: linear-gradient(135deg, var(--accent-deep), var(--accent));
      color: #fff; border-radius: 12px; padding: 26px 30px;
    }
    header.report h1 { margin: 0 0 4px; font-size: 24px; letter-spacing: 0.2px; }
    header.report .sub { opacity: 0.85; font-size: 13px; margin-bottom: 18px; }
    .meta-grid {
      display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 10px 14px; margin: 0;
    }
    .meta-grid div { background: rgba(255,255,255,0.12); border-radius: 8px; padding: 8px 12px; }
    .meta-grid dt { font-size: 10px; text-transform: uppercase; letter-spacing: 0.8px; opacity: 0.75; }
    .meta-grid dd { margin: 2px 0 0; font-size: 14px; font-weight: 600; word-break: break-word; }
    section {
      background: var(--card); border: 1px solid var(--border-soft); border-radius: 12px;
      padding: 20px 24px; margin-top: 20px; box-shadow: 0 1px 3px rgba(23,27,31,0.06);
    }
    h2 { margin: 0 0 4px; font-size: 17px; }
    h2::before { content: ""; display: inline-block; width: 5px; height: 15px;
      background: var(--accent); border-radius: 3px; margin-right: 9px; }
    .section-note { color: var(--muted); font-size: 12.5px; margin: 0 0 14px; }
    .table-wrap { overflow-x: auto; }
    table { border-collapse: collapse; width: 100%; font-size: 13px; }
    th, td { padding: 9px 10px; text-align: left; vertical-align: top;
      border-bottom: 1px solid var(--border-soft); }
    thead th { font-size: 11px; text-transform: uppercase; letter-spacing: 0.6px;
      color: var(--muted); background: #f6f9fb; border-bottom: 2px solid var(--border);
      white-space: nowrap; }
    tbody tr:nth-child(even) { background: #fafcfd; }
    tr.local td { background: #fff6d8; }
    .mono { font-family: ui-monospace, Consolas, "SFMono-Regular", Menlo, monospace;
      font-size: 12px; }
    .muted { color: var(--muted); }
    .small { font-size: 12px; }
    .count { font-variant-numeric: tabular-nums; }
    .pill { display: inline-block; padding: 2px 9px; border-radius: 999px;
      font-size: 11px; font-weight: 600; white-space: nowrap; }
    .pill.kind-ble { background: var(--accent-soft); color: var(--accent-deep); }
    .pill.kind-classic { background: #eee6ff; color: #5a3b8f; }
    .pill.kind-unknown { background: #eceff2; color: var(--muted); }
    .pill.paired { background: #dbf2df; color: #135c2f; }
    .pill.tag { background: #ffe9a8; color: #6b5200; }
    .sig-strong { color: var(--strong); font-weight: 600; }
    .sig-medium { color: var(--medium); font-weight: 600; }
    .sig-weak { color: var(--weak); font-weight: 600; }
    .sig-unknown { color: var(--muted); }
    @media print {
      body { background: #fff; }
      .page { padding: 0; max-width: none; }
      section { box-shadow: none; break-inside: avoid; }
    }
"#;

pub fn generate_html(session: &CaptureSession) -> String {
    let generated_at = Local::now();

    let tagged_local_radio = if normalize_address(&session.host.address).is_empty() {
        session.host.address.clone()
    } else {
        normalize_address(&session.host.address)
    };

    let meta_items = [
        ("Scan date", session.metadata.date.to_string()),
        ("Scan reference", session.metadata.name.clone()),
        (
            "Target device reference",
            session.metadata.section.clone(),
        ),
        ("User", session.metadata.user.clone()),
        ("Computer name", session.host.computer_name.clone()),
        ("Tagged local radio", tagged_local_radio),
        (
            "Host adapter",
            format!("{} {}", session.host.name, session.host.address),
        ),
    ]
    .iter()
    .map(|(label, value)| {
        format!(
            "        <div><dt>{}</dt><dd>{}</dd></div>",
            html_encode(label),
            html_encode(value)
        )
    })
    .collect::<Vec<_>>()
    .join("\n");

    let phase_rows = session
        .phase_runs
        .iter()
        .map(|run| {
            let stopped = run.stopped_at.map(format_timestamp).unwrap_or_default();
            let duration = run.duration().map(format_duration).unwrap_or_default();
            format!(
                "          <tr><td>{}</td><td>{}</td><td>{}</td><td class=\"count\">{}</td><td>{}</td></tr>",
                html_encode(run.phase.report_label()),
                html_encode(&format_timestamp(run.started_at)),
                html_encode(&stopped),
                html_encode(&duration),
                html_encode(&run.stop_reason)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let phase_rows = if phase_rows.is_empty() {
        "          <tr><td colspan=\"5\" class=\"muted\">No phase runs recorded.</td></tr>"
            .to_string()
    } else {
        phase_rows
    };

    let summary_rows = ScanPhase::ALL
        .iter()
        .copied()
        .map(|phase| phase_summary_row(session, phase))
        .collect::<Vec<_>>()
        .join("\n");

    let device_rows = session
        .registry
        .devices()
        .map(device_row)
        .collect::<Vec<_>>()
        .join("\n");
    let device_rows = if device_rows.is_empty() {
        "          <tr><td colspan=\"12\" class=\"muted\">No devices observed.</td></tr>"
            .to_string()
    } else {
        device_rows
    };

    let tagged_norm = normalize_address(&session.host.address);
    let raw_rows = session
        .raw_log
        .iter()
        .map(|obs| raw_row(obs, &tagged_norm))
        .collect::<Vec<_>>()
        .join("\n");
    let raw_rows = if raw_rows.is_empty() {
        "          <tr><td colspan=\"10\" class=\"muted\">No raw observations recorded.</td></tr>"
            .to_string()
    } else {
        raw_rows
    };

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>FindBT Bluetooth Report</title>
  <style>
{style}  </style>
</head>
<body>
  <div class="page">
    <header class="report">
      <h1>FindBT Bluetooth Report</h1>
      <div class="sub">Generated {generated}</div>
      <dl class="meta-grid">
{meta_items}
      </dl>
    </header>

    <section>
      <h2>Phase runs</h2>
      <p class="section-note">Every start and stop of each scan phase, as recorded during the capture.</p>
      <div class="table-wrap">
        <table>
          <thead><tr><th>Phase</th><th>Started</th><th>Stopped</th><th>Duration</th><th>Stop reason</th></tr></thead>
          <tbody>
{phase_rows}
          </tbody>
        </table>
      </div>
    </section>

    <section>
      <h2>Phase summary</h2>
      <div class="table-wrap">
        <table>
          <thead><tr><th>Phase</th><th>Purpose</th><th>Raw observations</th><th>Unique addresses</th><th>Newly seen</th></tr></thead>
          <tbody>
{summary_rows}
          </tbody>
        </table>
      </div>
      <p class="section-note">"Newly seen" means the first phase where that device appears in the registry. The report does not compute or highlight a match; it only renders the observed phase data.</p>
    </section>

    <section>
      <h2>Device registry</h2>
      <p class="section-note">One row per device, tracked across all three phases.</p>
      <div class="table-wrap">
        <table>
          <thead><tr><th>Local Radio</th><th>Name</th><th>Kind</th><th>Address</th><th>Paired</th><th>First phase</th><th>Baseline RSSI</th><th>Target RSSI</th><th>Verification RSSI</th><th>Windows/device id</th><th>Last seen</th><th>Properties</th></tr></thead>
          <tbody>
{device_rows}
          </tbody>
        </table>
      </div>
    </section>

    <section>
      <h2>Raw audit log</h2>
      <p class="section-note">Append-only capture log. Every backend observation is listed, unfiltered.</p>
      <div class="table-wrap">
        <table>
          <thead><tr><th>Observed</th><th>Phase</th><th>Local Radio</th><th>Name</th><th>Kind</th><th>Address</th><th>Paired</th><th>RSSI</th><th>Device id</th><th>Properties</th></tr></thead>
          <tbody>
{raw_rows}
          </tbody>
        </table>
      </div>
    </section>
  </div>
</body>
</html>
"#,
        style = STYLE,
        generated = html_encode(&format_timestamp(generated_at)),
        meta_items = meta_items,
        phase_rows = phase_rows,
        summary_rows = summary_rows,
        device_rows = device_rows,
        raw_rows = raw_rows,
    )
}

fn phase_summary_row(session: &CaptureSession, phase: ScanPhase) -> String {
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

    format!(
        "          <tr><td>{}</td><td class=\"muted small\">{}</td><td class=\"count\">{}</td><td class=\"count\">{}</td><td class=\"count\"><strong>{}</strong></td></tr>",
        html_encode(phase.report_label()),
        html_encode(phase.operator_instruction()),
        raw_count,
        unique_addresses,
        newly_seen
    )
}

fn device_row(device: &DeviceRecord) -> String {
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
        .map(|obs| obs.properties_summary.as_str())
        .unwrap_or_default();

    format!(
        "          <tr{local_class}><td>{local}</td><td><strong>{name}</strong></td><td>{kind}</td><td class=\"mono\">{address}</td><td>{paired}</td><td>{first_phase}</td><td>{baseline}</td><td>{target}</td><td>{verification}</td><td class=\"mono\">{device_id}</td><td>{last_seen}</td><td class=\"muted small\">{properties}</td></tr>",
        local_class = if device.is_local_radio { " class=\"local\"" } else { "" },
        local = if device.is_local_radio {
            "<span class=\"pill tag\">Tagged radio</span>"
        } else {
            ""
        },
        name = html_encode(&device.name),
        kind = kind_pill(device.kind),
        address = html_encode(&device.address),
        paired = paired_html(device.is_paired),
        first_phase = html_encode(device.first_seen_phase().map(ScanPhase::report_label).unwrap_or("")),
        baseline = phase_rssi_html(device, ScanPhase::Baseline),
        target = phase_rssi_html(device, ScanPhase::Target),
        verification = phase_rssi_html(device, ScanPhase::Verification),
        device_id = html_encode(&device.device_id),
        last_seen = html_encode(&last_seen),
        properties = html_encode(properties),
    )
}

fn raw_row(obs: &RawObservation, tagged_norm: &str) -> String {
    let is_tagged = !tagged_norm.is_empty() && normalize_address(&obs.address) == tagged_norm;
    format!(
        "          <tr><td>{observed}</td><td>{phase}</td><td>{local}</td><td>{name}</td><td>{kind}</td><td class=\"mono\">{address}</td><td>{paired}</td><td>{rssi}</td><td class=\"mono\">{device_id}</td><td class=\"muted small\">{properties}</td></tr>",
        observed = html_encode(&format_timestamp(obs.observed_at)),
        phase = html_encode(obs.phase.report_label()),
        local = if is_tagged {
            "<span class=\"pill tag\">Tagged radio</span>"
        } else {
            ""
        },
        name = html_encode(&obs.name),
        kind = kind_pill(obs.kind),
        address = html_encode(&obs.address),
        paired = paired_html(obs.is_paired),
        rssi = rssi_html(obs.rssi),
        device_id = html_encode(&obs.device_id),
        properties = html_encode(&obs.properties_summary),
    )
}

fn kind_pill(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Ble => "<span class=\"pill kind-ble\">BLE</span>",
        DeviceKind::Classic => "<span class=\"pill kind-classic\">Classic</span>",
        DeviceKind::Unknown => "<span class=\"pill kind-unknown\">Unknown</span>",
    }
}

fn paired_html(paired: bool) -> &'static str {
    if paired {
        "<span class=\"pill paired\">Paired</span>"
    } else {
        "<span class=\"muted\">No</span>"
    }
}

fn rssi_html(rssi: Option<i32>) -> String {
    match rssi {
        Some(value) => {
            let strength = SignalStrength::from_rssi(Some(value));
            format!(
                "<span class=\"sig-{}\">{} dBm ({})</span>",
                signal_class(strength),
                value,
                signal_label(strength)
            )
        }
        None => "<span class=\"muted\">unknown</span>".to_string(),
    }
}

fn phase_rssi_html(device: &DeviceRecord, phase: ScanPhase) -> String {
    match device.seen_in(phase) {
        Some(obs) => rssi_html(obs.rssi),
        None => "<span class=\"muted\">not seen</span>".to_string(),
    }
}

pub(crate) fn format_rssi(rssi: Option<i32>) -> String {
    match rssi {
        Some(value) => format!(
            "{value} dBm ({})",
            signal_label(SignalStrength::from_rssi(Some(value)))
        ),
        None => "unknown".to_string(),
    }
}

fn signal_label(strength: SignalStrength) -> &'static str {
    match strength {
        SignalStrength::Strong => "strong",
        SignalStrength::Medium => "medium",
        SignalStrength::Weak => "weak",
        SignalStrength::Unknown => "unknown",
    }
}

fn signal_class(strength: SignalStrength) -> &'static str {
    match strength {
        SignalStrength::Strong => "strong",
        SignalStrength::Medium => "medium",
        SignalStrength::Weak => "weak",
        SignalStrength::Unknown => "unknown",
    }
}

pub(crate) fn device_identity(device: &DeviceRecord) -> String {
    let normalized = normalize_address(&device.address);
    if normalized.is_empty() {
        format!("{:?}:{}", device.kind, device.device_id)
    } else {
        normalized
    }
}

pub(crate) fn format_timestamp(value: DateTime<Local>) -> String {
    value.format("%Y-%m-%d %H:%M:%S %:z").to_string()
}

pub(crate) fn format_duration(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds().max(0);
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn html_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => encoded.push_str("&amp;"),
            '<' => encoded.push_str("&lt;"),
            '>' => encoded.push_str("&gt;"),
            '"' => encoded.push_str("&quot;"),
            '\'' => encoded.push_str("&#39;"),
            _ => encoded.push(ch),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use chrono::{Local, NaiveDate};

    use super::*;
    use crate::{CaseMetadata, DeviceKind, HostAdapterInfo};

    #[test]
    fn report_contains_core_sections_and_escaped_fixture_values() {
        let mut session = CaptureSession::new(
            CaseMetadata {
                date: NaiveDate::from_ymd_opt(2026, 7, 3).unwrap(),
                name: "Alice & Bob".to_string(),
                section: "Lab <A>".to_string(),
                user: "Operator <One>".to_string(),
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
            name: "Target <Device>".to_string(),
            address: "11:22:33:44:55:66".to_string(),
            kind: DeviceKind::Ble,
            is_paired: false,
            rssi: Some(-42),
            properties_summary: "role=\"candidate\"".to_string(),
            observed_at: Local::now(),
        });

        let html = generate_html(&session);

        assert!(html.contains("Phase summary"));
        assert!(html.contains("Device registry"));
        assert!(html.contains("Raw audit log"));
        assert!(html.contains("Alice &amp; Bob"));
        assert!(html.contains("Lab &lt;A&gt;"));
        assert!(html.contains("Target &lt;Device&gt;"));
        assert!(html.contains("role=&quot;candidate&quot;"));
        assert!(html.contains("AA:BB:CC:DD:EE:FF"));
        assert!(html.contains("11:22:33:44:55:66"));
        assert!(html.contains("sig-strong"));
        assert!(html.contains("pill kind-ble"));
    }
}
