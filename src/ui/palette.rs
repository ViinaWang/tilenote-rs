use crate::model::{Settings, Theme};
use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, TextStyle};
use std::{fs, path::Path, sync::Arc};

pub const DARK_ACCENTS: [Color32; 6] = [
    Color32::from_rgb(97, 175, 239), // One Dark Pro blue
    Color32::from_rgb(198, 120, 221),
    Color32::from_rgb(152, 195, 121),
    Color32::from_rgb(229, 192, 123),
    Color32::from_rgb(224, 108, 117),
    Color32::from_rgb(86, 182, 194),
];
pub const LIGHT_ACCENTS: [Color32; 6] = [
    Color32::from_rgb(9, 105, 218), // GitHub Light blue
    Color32::from_rgb(130, 80, 223),
    Color32::from_rgb(26, 127, 55),
    Color32::from_rgb(154, 103, 0),
    Color32::from_rgb(207, 34, 46),
    Color32::from_rgb(5, 119, 130),
];

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub raised: Color32,
    pub divider: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub success: Color32,
}

impl Palette {
    pub fn new(s: &Settings) -> Self {
        match s.theme {
            Theme::OneDarkPro => Self {
                bg: hex(0x21252B), panel: hex(0x282C34), raised: hex(0x2C313A),
                divider: hex(0x3E4451), text: hex(0xABB2BF), muted: hex(0x828A9A),
                accent: DARK_ACCENTS[s.accent % 6], success: hex(0x98C379),
            },
            Theme::GithubLight => Self {
                bg: hex(0xFFFFFF), panel: hex(0xF6F8FA), raised: hex(0xEAEEF2),
                divider: hex(0xD0D7DE), text: hex(0x1F2328), muted: hex(0x656D76),
                accent: LIGHT_ACCENTS[s.accent % 6], success: hex(0x1A7F37),
            },
        }
    }
}

pub fn hex(value: u32) -> Color32 {
    Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

pub fn apply(ctx: &egui::Context, s: &Settings) {
    let p = Palette::new(s);
    let mut visuals = match s.theme {
        Theme::OneDarkPro => egui::Visuals::dark(),
        Theme::GithubLight => egui::Visuals::light(),
    };
    visuals.override_text_color = Some(p.text);
    visuals.panel_fill = p.bg;
    visuals.window_fill = p.panel;
    visuals.extreme_bg_color = p.panel;
    visuals.faint_bg_color = p.raised;
    visuals.widgets.noninteractive.bg_fill = p.panel;
    visuals.widgets.inactive.bg_fill = p.raised;
    visuals.widgets.hovered.bg_fill = p.divider;
    visuals.widgets.active.bg_fill = p.divider;
    visuals.selection.bg_fill = p.accent.gamma_multiply(0.4);
    visuals.hyperlink_color = p.accent;
    ctx.set_visuals(visuals);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(s.font_size + 9.0)),
        (TextStyle::Body, FontId::proportional(s.font_size)),
        (TextStyle::Monospace, FontId::monospace(s.font_size - 1.0)),
        (TextStyle::Button, FontId::proportional(s.font_size)),
        (TextStyle::Small, FontId::proportional(s.font_size - 2.0)),
    ].into();
    ctx.set_style(style);
}

/// egui's bundled fonts lack comprehensive CJK coverage. Load an installed
/// font at runtime; do not distribute or embed third-party font files.
pub fn install_cjk_fallback(ctx: &egui::Context) {
    let mut paths: Vec<String> = Vec::new();
    if let Ok(font) = std::env::var("TILENOTE_FONT") { paths.push(font); }
    paths.extend([
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/TTF/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/TTF/NotoSansSC-Regular.otf",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    ].into_iter().map(str::to_owned));
    let maybe = paths.iter().find_map(|p| {
        if Path::new(p).is_file() { fs::read(p).ok() } else { None }
    });
    if let Some(bytes) = maybe {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert("tilenote-cjk".to_owned(), Arc::new(FontData::from_owned(bytes)));
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            if let Some(names) = fonts.families.get_mut(&family) { names.push("tilenote-cjk".to_owned()); }
        }
        ctx.set_fonts(fonts);
    }
}

pub fn frame(p: Palette, radius: u8) -> egui::Frame {
    egui::Frame::default()
        .fill(p.panel)
        .corner_radius(egui::CornerRadius::same(radius))
        .inner_margin(egui::Margin::same(12))
}
