use egui::Color32;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentColor {
    Blue,
    Teal,
    Purple,
    Green,
    Orange,
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub mode: ThemeMode,
    pub accent: AccentColor,
    pub bg: Color32,
    pub bg_elevated: Color32,
    pub bg_sunken: Color32,
    pub border: Color32,
    pub border_soft: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub accent_text: Color32,
    /// Corner radius for interactive controls: buttons, inputs, chips. Kept
    /// tight and rectangular, matching the Windows 11 / Fluent "small
    /// control" radius, so controls read the same on every screen and every
    /// platform.
    pub control_radius: f32,
    /// Corner radius for containers: cards, sheets, panels. Kept generous,
    /// matching macOS's softer dialog/sheet rounding. The contrast between
    /// a softly rounded card and crisp, rectangular controls inside it is
    /// the app's cross-platform "neutral" look.
    pub card_radius: f32,
}

impl Theme {
    pub fn light(accent: AccentColor) -> Self {
        Self {
            mode: ThemeMode::Light,
            accent,
            bg: hex(0xf8fafd),
            bg_elevated: hex(0xffffff),
            bg_sunken: hex(0xf0f4f7),
            border: hex(0xdbdee3),
            border_soft: hex(0xe5e8ec),
            text: hex(0x171b1f),
            text_muted: hex(0x646a70),
            accent_text: hex(0xfafcfe),
            control_radius: 4.0,
            card_radius: 16.0,
        }
    }

    #[allow(dead_code)]
    pub fn dark(accent: AccentColor) -> Self {
        Self {
            mode: ThemeMode::Dark,
            accent,
            bg: hex(0x0f141a),
            bg_elevated: hex(0x1a2027),
            bg_sunken: hex(0x080c10),
            border: hex(0x343b44),
            border_soft: hex(0x252c33),
            text: hex(0xe8ebef),
            text_muted: hex(0x858d96),
            accent_text: hex(0x06090d),
            control_radius: 4.0,
            card_radius: 16.0,
        }
    }

    pub fn accent_main(self) -> Color32 {
        match (self.mode, self.accent) {
            (ThemeMode::Light, AccentColor::Blue) => hex(0x0073cf),
            (ThemeMode::Light, AccentColor::Teal) => hex(0x008d90),
            (ThemeMode::Light, AccentColor::Purple) => hex(0x8254c4),
            (ThemeMode::Light, AccentColor::Green) => hex(0x008b35),
            (ThemeMode::Light, AccentColor::Orange) => hex(0xbd4600),
            (ThemeMode::Dark, AccentColor::Blue) => hex(0x55adff),
            (ThemeMode::Dark, AccentColor::Teal) => hex(0x00c4c5),
            (ThemeMode::Dark, AccentColor::Purple) => hex(0xb790f7),
            (ThemeMode::Dark, AccentColor::Green) => hex(0x56c173),
            (ThemeMode::Dark, AccentColor::Orange) => hex(0xf38651),
        }
    }

    pub fn accent_soft(self) -> Color32 {
        match (self.mode, self.accent) {
            (ThemeMode::Light, AccentColor::Blue) => hex(0xdaeeff),
            (ThemeMode::Light, AccentColor::Teal) => hex(0xd1f3f2),
            (ThemeMode::Light, AccentColor::Purple) => hex(0xeee6ff),
            (ThemeMode::Light, AccentColor::Green) => hex(0xdbf2df),
            (ThemeMode::Light, AccentColor::Orange) => hex(0xffe5d8),
            (ThemeMode::Dark, AccentColor::Blue) => hex(0x193550),
            (ThemeMode::Dark, AccentColor::Teal) => hex(0x003c3c),
            (ThemeMode::Dark, AccentColor::Purple) => hex(0x382b4d),
            (ThemeMode::Dark, AccentColor::Green) => hex(0x193b22),
            (ThemeMode::Dark, AccentColor::Orange) => hex(0x4c2817),
        }
    }
}

pub fn signal_color(theme: Theme, rssi: Option<i32>) -> Color32 {
    match findbt_core::SignalStrength::from_rssi(rssi) {
        findbt_core::SignalStrength::Strong => match theme.mode {
            ThemeMode::Light => hex(0x1b9247),
            ThemeMode::Dark => hex(0x53be70),
        },
        findbt_core::SignalStrength::Medium => match theme.mode {
            ThemeMode::Light => hex(0xaf7c00),
            ThemeMode::Dark => hex(0xdaa932),
        },
        findbt_core::SignalStrength::Weak => match theme.mode {
            ThemeMode::Light => hex(0xcb4644),
            ThemeMode::Dark => hex(0xe2726b),
        },
        findbt_core::SignalStrength::Unknown => theme.text_muted,
    }
}

pub fn hex(value: u32) -> Color32 {
    Color32::from_rgb(
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    )
}
