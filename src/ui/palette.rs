use crate::model::{Settings, Theme};
use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, TextStyle};
use std::{borrow::Cow, fs, path::Path, process::Command, sync::Arc};

/// One Dark accent ramp; the light theme uses the One Light variants.
pub fn accents(theme: Theme) -> [Color32; 6] {
    match theme {
        Theme::GithubLight => [
            Color32::from_rgb(0x40, 0x78, 0xF2), // blue
            Color32::from_rgb(0x50, 0xA1, 0x4F), // green
            Color32::from_rgb(0xA6, 0x26, 0xA4), // purple
            Color32::from_rgb(0x98, 0x68, 0x01), // orange
            Color32::from_rgb(0x0D, 0x84, 0x8C), // cyan
            Color32::from_rgb(0xE4, 0x56, 0x49), // red
        ],
        _ => [
            Color32::from_rgb(0x61, 0xAF, 0xEF), // blue
            Color32::from_rgb(0x98, 0xC3, 0x79), // green
            Color32::from_rgb(0xC6, 0x78, 0xDD), // purple
            Color32::from_rgb(0xD1, 0x9A, 0x66), // orange
            Color32::from_rgb(0x56, 0xB6, 0xC2), // cyan
            Color32::from_rgb(0xE0, 0x6C, 0x75), // red
        ],
    }
}

/// Semantic color tokens of the One Dark design system, resolved per theme.
#[derive(Clone, Copy)]
pub struct Palette {
    /// Desktop / deepest window background.
    pub canvas: Color32,
    /// Title bars, sidebars, card headers.
    pub chrome: Color32,
    /// Cards and window bodies.
    pub panel: Color32,
    /// Hover highlight / raised controls.
    pub raise: Color32,
    /// Inputs, slider tracks, recessed wells.
    pub sunken: Color32,
    /// Strong separators.
    pub border: Color32,
    pub text: Color32,
    pub strong: Color32,
    pub dim: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub success: Color32,
    pub warn: Color32,
    pub danger: Color32,
    pub is_light: bool,
}

impl Palette {
    pub fn new(s: &Settings) -> Self {
        let theme = s.theme;
        let (canvas, chrome, panel, raise, sunken, border, text, strong, dim, faint, on_accent) = match theme {
            Theme::OneDarkPro => (
                hex(0x1B1E24), hex(0x21252B), hex(0x282C34), hex(0x2C313A), hex(0x1E2127),
                hex(0x3A3F4B), hex(0xABB2BF), hex(0xE6E8EC), hex(0x6B7280), hex(0x5C6370), hex(0x1B1E24),
            ),
            Theme::OneDarkDim => (
                hex(0x14161A), hex(0x191C21), hex(0x1E2126), hex(0x24282E), hex(0x16181C),
                hex(0x31353D), hex(0x9BA1AC), hex(0xD7DBE1), hex(0x626873), hex(0x535965), hex(0x14161A),
            ),
            Theme::GithubLight => (
                hex(0xE9EAEC), hex(0xF2F2F3), hex(0xFAFAFA), hex(0xF0F0F2), hex(0xEAEAEC),
                hex(0xD0D2D6), hex(0x4A4D55), hex(0x23252B), hex(0x8B8F98), hex(0xA0A1A7), hex(0xFFFFFF),
            ),
        };
        Self {
            canvas, chrome, panel, raise, sunken, border, text, strong, dim, faint, on_accent,
            accent: accents(theme)[s.accent % 6],
            success: accents(theme)[1],
            warn: if theme == Theme::GithubLight { hex(0x986801) } else { hex(0xE5C07B) },
            danger: accents(theme)[5],
            is_light: theme == Theme::GithubLight,
        }
    }

    /// Translucent hover wash over panels.
    pub fn hover(&self) -> Color32 {
        if self.is_light { Color32::from_black_alpha(10) } else { Color32::from_white_alpha(12) }
    }

    /// Hairline separators (softer than `border`).
    pub fn border_soft(&self) -> Color32 {
        if self.is_light { Color32::from_black_alpha(23) } else { Color32::from_white_alpha(18) }
    }

    /// Dot grid of the library map canvas.
    pub fn grid_dot(&self) -> Color32 {
        if self.is_light { Color32::from_black_alpha(18) } else { Color32::from_white_alpha(14) }
    }

    pub fn shadow_color(&self) -> Color32 {
        if self.is_light { Color32::from_rgba_premultiplied(30, 34, 42, 56) } else { Color32::from_rgba_premultiplied(0, 0, 0, 140) }
    }
}

pub fn hex(value: u32) -> Color32 {
    Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

pub fn apply(ctx: &egui::Context, s: &Settings) {
    let p = Palette::new(s);
    let mut visuals = match s.theme {
        Theme::OneDarkPro | Theme::OneDarkDim => egui::Visuals::dark(),
        Theme::GithubLight => egui::Visuals::light(),
    };
    visuals.override_text_color = Some(p.text);
    visuals.panel_fill = p.canvas;
    visuals.window_fill = p.panel;
    visuals.extreme_bg_color = p.sunken;
    visuals.faint_bg_color = p.raise;
    visuals.widgets.noninteractive.bg_fill = p.chrome;
    visuals.widgets.inactive.bg_fill = p.sunken;
    visuals.widgets.hovered.bg_fill = p.raise;
    visuals.widgets.active.bg_fill = p.border;
    visuals.selection.bg_fill = p.accent.gamma_multiply(0.38);
    visuals.hyperlink_color = p.accent;
    ctx.set_visuals(visuals);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(s.font_size + 8.0)),
        (TextStyle::Body, FontId::proportional(s.font_size)),
        (TextStyle::Monospace, FontId::monospace(s.font_size - 1.0)),
        (TextStyle::Button, FontId::proportional(s.font_size - 1.0)),
        (TextStyle::Small, FontId::proportional((s.font_size - 2.5).max(9.0))),
    ].into();
    ctx.set_style(style);
}

/// egui's bundled fonts lack comprehensive CJK coverage. Load an installed
/// font at runtime; do not distribute or embed third-party font files.
///
/// CJK fonts ship as .ttc collections whose face 0 is usually the Japanese or
/// Korean variant (upstream Noto CJK order: JP, KR, SC, TC, HK), which renders
/// Han characters with the wrong regional glyph shapes. Always resolve the
/// Simplified-Chinese face index explicitly instead of relying on the default.
pub fn install_cjk_fallback(ctx: &egui::Context) {
    let mut candidates: Vec<(String, u32)> = Vec::new();
    if let Ok(spec) = std::env::var("TILENOTE_FONT") {
        // Accepts "/path/font.ttf" or "/path/font.ttc:2".
        let (path, index) = match spec.rsplit_once(':') {
            Some((p, i)) if !i.is_empty() && i.bytes().all(|b| b.is_ascii_digit()) =>
                (p.to_owned(), i.parse().unwrap_or(0)),
            _ => (spec, 0),
        };
        candidates.push((path, index));
    }
    if let Some(found) = fontconfig_sc_face() { candidates.push(found); }
    candidates.extend([
        ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 2),
        ("/usr/share/fonts/TTF/NotoSansCJK-Regular.ttc", 2),
        ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 2),
        ("/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc", 2),
        ("/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc", 2),
        ("/usr/share/fonts/TTF/NotoSansSC-Regular.otf", 0),
        ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", 0),
    ].into_iter().map(|(p, i)| (p.to_owned(), i)));
    let found = candidates.iter().find(|(path, _)| Path::new(path).is_file());
    if let Some((path, index)) = found {
        if let Ok(bytes) = fs::read(path) {
            let mut fonts = FontDefinitions::default();
            fonts.font_data.insert("tilenote-cjk".to_owned(), Arc::new(FontData {
                font: Cow::Owned(bytes),
                index: *index,
                tweak: Default::default(),
            }));
            for family in [FontFamily::Proportional, FontFamily::Monospace] {
                if let Some(names) = fonts.families.get_mut(&family) { names.push("tilenote-cjk".to_owned()); }
            }
            ctx.set_fonts(fonts);
        }
    }
}

/// Ask fontconfig for the system's Simplified-Chinese sans face; it knows the
/// correct face index inside .ttc collections (format: "path|index").
fn fontconfig_sc_face() -> Option<(String, u32)> {
    let out = Command::new("fc-match")
        .args(["--format=%{file}|%{index}", "Noto Sans CJK SC"])
        .output().ok()?;
    if !out.status.success() { return None; }
    let text = String::from_utf8(out.stdout).ok()?;
    let (path, index) = text.trim().rsplit_once('|')?;
    Some((path.to_owned(), index.parse().ok()?))
}
