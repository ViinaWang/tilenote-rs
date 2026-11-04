//! Settings window: custom title bar with search, icon sidebar with quick
//! theme switch, footer status bar, and the note library as the landing page.
use crate::{
    model::{self, Density, Paths, Settings, Theme},
    ui::{components, library::Library, palette::{self, Palette}},
};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, RichText, Sense, Stroke, Vec2, pos2};
use std::time::{Duration, Instant};

const SECTIONS: [(&str, &str); 8] = [
    ("笔记库", "LIBRARY"),
    ("通用", "GENERAL"),
    ("外观", "APPEARANCE"),
    ("磁贴", "TILES"),
    ("笔记与编辑", "EDITOR"),
    ("快捷键", "SHORTCUTS"),
    ("同步与备份", "SYNC"),
    ("关于", "ABOUT"),
];
const SECTION_ICONS: [&str; 8] = ["⊞", "☰", "◈", "▦", "▤", "⌘", "☁", "ⓘ"];

const TITLE_H: f32 = 40.0;
const NAV_W: f32 = 212.0;
const FOOTER_H: f32 = 44.0;

pub struct SettingsApp {
    paths: Paths,
    settings: Settings,
    section: usize,
    search: String,
    flash_row: Option<String>,
    flash_until: Instant,
    library: Library,
    last_read: Instant,
}

impl SettingsApp {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: Paths) -> Self {
        let settings = Settings::load(&paths);
        palette::install_cjk_fallback(&cc.egui_ctx);
        palette::apply(&cc.egui_ctx, &settings);
        // Debug aid: open directly on a given section (e.g. TILENOTE_SECTION=5).
        let section = std::env::var("TILENOTE_SECTION").ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|i| *i < SECTIONS.len())
            .unwrap_or(0);
        Self {
            paths, settings,
            section,
            search: String::new(),
            flash_row: None,
            flash_until: Instant::now(),
            library: Library::new(),
            last_read: Instant::now(),
        }
    }

    fn save(&mut self, ctx: &egui::Context) {
        palette::apply(ctx, &self.settings);
        if let Err(e) = self.settings.save(&self.paths) {
            eprintln!("TileNote: saving settings failed: {e:#}");
        }
    }

    fn flash_active(&self, label: &str) -> bool {
        self.flash_row.as_deref() == Some(label) && Instant::now() < self.flash_until
    }

    fn jump_to(&mut self, section: usize, row: &str) {
        self.section = section;
        self.search.clear();
        self.flash_row = Some(row.to_owned());
        self.flash_until = Instant::now() + Duration::from_secs(3);
    }

    // ---- Window chrome -------------------------------------------------

    fn title_bar(&mut self, ctx: &egui::Context, p: Palette) {
        egui::TopBottomPanel::top("settings_title")
            .exact_height(TITLE_H)
            .frame(egui::Frame::default().fill(p.chrome))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.set_min_height(TITLE_H);
                    ui.add_space(12.0);
                    paint_logo(ui, 20.0);
                    ui.add_space(8.0);
                    ui.label(RichText::new("TileNote").size(13.0).strong().color(p.strong));
                    ui.label(RichText::new(SECTIONS[self.section.min(7)].1)
                        .monospace().size(9.5).color(p.faint));
                    // Drag zone and search width adapt to narrow windows.
                    let rest = ui.available_width();
                    let search_w = (rest - 140.0).clamp(96.0, 200.0);
                    let drag_w = (rest - search_w - 118.0).max(16.0);
                    let drag = ui.allocate_response(Vec2::new(drag_w, TITLE_H - 6.0), Sense::drag());
                    if drag.drag_started() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    drag.on_hover_cursor(egui::CursorIcon::Grab);
                    // Search box.
                    let search = egui::TextEdit::singleline(&mut self.search)
                        .hint_text("⌕ 搜索设置项…")
                        .desired_width(search_w);
                    ui.add(search).on_hover_text("按名称搜索设置项并跳转");
                    window_controls(ui, p);
                });
                let rect = ui.max_rect();
                ui.painter().hline(rect.left()..=rect.right(), rect.bottom() - 0.5, Stroke::new(1.0, p.border_soft()));
            });
    }

    fn nav(&mut self, ctx: &egui::Context, p: Palette) {
        egui::SidePanel::left("settings_nav")
            .exact_width(NAV_W)
            .frame(egui::Frame::default().fill(p.chrome).inner_margin(egui::Margin::same(10)))
            .show(ctx, |ui| {
                ui.label(RichText::new("设置分组").monospace().size(9.0).color(p.faint));
                ui.add_space(6.0);
                for (i, (label, _)) in SECTIONS.iter().enumerate() {
                    let active = self.section == i;
                    let is_library = i == 0;
                    let btn = egui::Button::new(
                        RichText::new(format!("{}  {}", SECTION_ICONS[i], label))
                            .size(12.5)
                            .strong()
                            .color(if active { p.strong } else if is_library { p.accent } else { p.dim }),
                    )
                    .fill(if active { p.accent.gamma_multiply(0.13) } else { Color32::TRANSPARENT })
                    .corner_radius(CornerRadius::same(8))
                    .min_size(Vec2::new(ui.available_width(), 34.0));
                    if ui.add(btn).clicked() { self.section = i; }
                    if i == 0 {
                        ui.add_space(4.0);
                        let (line, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
                        ui.painter().rect_filled(line, 0.0, p.border_soft());
                        ui.add_space(4.0);
                    }
                }
                ui.add_space(12.0);
                // Quick theme switch swatches.
                ui.label(RichText::new("快速切换主题").monospace().size(9.0).color(p.faint));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let themes = [
                        (Theme::OneDarkPro, Color32::from_rgb(0x28, 0x2C, 0x34), "One Dark"),
                        (Theme::OneDarkDim, Color32::from_rgb(0x1E, 0x21, 0x26), "One Dark Dim"),
                        (Theme::GithubLight, Color32::from_rgb(0xFA, 0xFA, 0xFA), "One Light"),
                    ];
                    for (theme, color, name) in themes {
                        let active = self.settings.theme == theme;
                        let (rect, response) = ui.allocate_exact_size(Vec2::new(52.0, 22.0), Sense::click());
                        let painter = ui.painter();
                        painter.rect_filled(rect, CornerRadius::same(6), color);
                        painter.rect_stroke(rect, CornerRadius::same(6),
                            if active { Stroke::new(2.0, p.accent) } else { Stroke::new(1.0, p.border) },
                            egui::StrokeKind::Inside);
                        if response.clicked() {
                            self.settings.theme = theme;
                            let ctx = ui.ctx().clone();
                            self.save(&ctx);
                        }
                        response.on_hover_text(name);
                    }
                });
                // Version footer.
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        components::status_dot(ui, p.success, 3.0);
                        ui.label(RichText::new(match self.settings.theme {
                            Theme::OneDarkPro => "One Dark",
                            Theme::OneDarkDim => "One Dark Dim",
                            Theme::GithubLight => "One Light",
                        }).monospace().size(9.5).color(p.faint));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("v0.1.0").monospace().size(9.5).color(p.faint));
                        });
                    });
                });
            });
    }

    fn footer(&mut self, ctx: &egui::Context, p: Palette) {
        egui::TopBottomPanel::bottom("settings_footer")
            .exact_height(FOOTER_H)
            .frame(egui::Frame::default().fill(p.chrome))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.set_min_height(FOOTER_H);
                    ui.add_space(14.0);
                    components::status_dot(ui, p.success, 3.5);
                    ui.add_space(5.0);
                    ui.label(RichText::new("所有更改立即保存到 settings.json").size(10.5).color(p.faint));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        let reset = egui::Button::new(RichText::new("恢复默认设置").size(11.0).color(p.danger))
                            .fill(p.danger.gamma_multiply(0.08))
                            .stroke(Stroke::new(1.0, p.danger.gamma_multiply(0.4)))
                            .corner_radius(CornerRadius::same(8));
                        if ui.add(reset).clicked() {
                            self.settings = Settings::default();
                            let ctx2 = ui.ctx().clone();
                            self.save(&ctx2);
                        }
                    });
                });
            });
    }

    // ---- Sections ------------------------------------------------------

    fn section_general(&mut self, ui: &mut egui::Ui, p: Palette) {
        components::section_head(ui, p, "GENERAL", "通用", "启动行为、自动保存与本地存储。");
        let d = self.settings.density;
        components::group(ui, p, "☰", "启动与窗口行为", "设置窗口与磁贴窗口相互独立，互不影响", |ui| {
            components::setting_row(ui, p, d, "自动保存防抖间隔", "输入停止多久后将磁贴正文写盘", self.flash_active("自动保存间隔"), |ui| {
                let mut ms = self.settings.autosave_ms;
                let changed = ui.add(egui::Slider::new(&mut ms, 120..=2000).step_by(20.0).suffix(" ms")).changed();
                if changed { self.settings.autosave_ms = ms; }
                changed
            });
            components::setting_row(ui, p, d, "关闭磁贴时确认", "点击磁贴 × 时先弹出确认对话框", self.flash_active("关闭磁贴时确认"), |ui| {
                components::toggle_switch(ui, p, &mut self.settings.confirm_close).changed()
            });
        });
        components::group(ui, p, "▤", "本地存储占用", "笔记全部为 Markdown 与 JSON 明文", |ui| {
            let (count, bytes) = storage_usage(&self.paths);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{count} 条笔记")).size(22.0).strong().color(p.strong));
                ui.label(RichText::new(format!("· {:.1} KB 文本", bytes as f32 / 1024.0))
                    .monospace().size(10.5).color(p.faint));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("打开数据目录").clicked() {
                        let _ = std::process::Command::new("xdg-open").arg(&self.paths.notes).spawn();
                    }
                });
            });
        });
    }

    fn section_appearance(&mut self, ui: &mut egui::Ui, p: Palette) {
        components::section_head(ui, p, "APPEARANCE", "外观", "主题、强调色、密度与字号。");
        let d = self.settings.density;
        components::group(ui, p, "◈", "主题", "三种内置主题，磁贴与设置窗口同步切换", |ui| {
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 16.0) / 3.0;
                let themes = [
                    (Theme::OneDarkPro, "One Dark", "默认", "底色 #282c34 · 正文 #abb2bf"),
                    (Theme::OneDarkDim, "One Dark Dim", "夜间", "整体压暗一档 · 适合深色壁纸"),
                    (Theme::GithubLight, "One Light", "浅色", "底色 #fafafa · 正文 #383a42"),
                ];
                for (theme, name, tag, desc) in themes {
                    let selected = self.settings.theme == theme;
                    let response = components::radio_card(ui, p, Vec2::new(width, 118.0), selected, |ui| {
                        let rect = ui.available_rect_before_wrap();
                        let painter = ui.painter();
                        // Mini window mockup: sidebar + two blocks + accent bar.
                        let bg = match theme {
                            Theme::OneDarkPro => palette::hex(0x282C34),
                            Theme::OneDarkDim => palette::hex(0x1E2126),
                            Theme::GithubLight => palette::hex(0xFAFAFA),
                        };
                        let chrome = match theme {
                            Theme::OneDarkPro => palette::hex(0x21252B),
                            Theme::OneDarkDim => palette::hex(0x191C21),
                            Theme::GithubLight => palette::hex(0xF2F2F3),
                        };
                        let mock = egui::Rect::from_min_size(rect.min, Vec2::new(rect.width(), 46.0));
                        painter.rect_filled(mock, CornerRadius::same(4), bg);
                        painter.rect_filled(
                            egui::Rect::from_min_max(mock.min, pos2(mock.min.x + mock.width() * 0.32, mock.bottom())),
                            CornerRadius { nw: 4, ne: 0, sw: 4, se: 0 }, chrome);
                        painter.rect_filled(
                            egui::Rect::from_min_size(pos2(mock.min.x + 6.0, mock.min.y + 8.0), Vec2::new(28.0, 5.0)), 2.0, p.accent);
                        painter.rect_filled(
                            egui::Rect::from_min_size(pos2(mock.min.x + 6.0, mock.min.y + 20.0), Vec2::new(20.0, 4.0)), 2.0, chrome);
                        let fit = ((rect.width() / 10.0) as usize).max(6);
                        painter.text(pos2(rect.min.x, rect.min.y + 56.0), Align2::LEFT_CENTER,
                            components::truncate_chars(name, fit), FontId::proportional(12.5), p.strong);
                        painter.text(pos2(rect.min.x, rect.min.y + 72.0), Align2::LEFT_CENTER,
                            components::truncate_chars(desc, fit), FontId::proportional(9.5), p.dim);
                        ui.allocate_response(rect.size(), Sense::hover());
                    });
                    if response.clicked() {
                        self.settings.theme = theme;
                        let ctx = ui.ctx().clone();
                        self.save(&ctx);
                    }
                    response.on_hover_text(tag);
                }
            });
        });
        components::group(ui, p, "◈", "强调色", "用于高亮、开关、状态与选中态", |ui| {
            components::setting_row(ui, p, d, "全局强调色", "每张磁贴可再单独覆盖", self.flash_active("强调色"), |ui| {
                let colors = palette::accents(self.settings.theme);
                components::swatches(ui, p, &colors, &mut self.settings.accent, 22.0)
            });
            components::setting_row(ui, p, d, "界面密度", "设置页与列表的行距", self.flash_active("界面密度"), |ui| {
                let mut idx = density_index(self.settings.density);
                let changed = components::segmented(ui, p, &["紧凑", "舒适", "宽松"], &mut idx);
                if changed { self.settings.density = density_from(idx); }
                changed
            });
            components::setting_row(ui, p, d, "正文字号", "设置窗口与磁贴的基础字号", self.flash_active("正文字号"), |ui| {
                ui.add(egui::Slider::new(&mut self.settings.font_size, 12.0..=18.0).integer().suffix(" px")).changed()
            });
        });
    }

    fn section_tiles(&mut self, ui: &mut egui::Ui, p: Palette) {
        components::section_head(ui, p, "TILES", "磁贴", "新建磁贴的默认尺寸与底部信息栏。");
        let d = self.settings.density;
        components::group(ui, p, "▦", "默认尺寸", "新磁贴的初始大小，之后可随意拖拽调整", |ui| {
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 16.0) / 3.0;
                let presets = [("S", 240.0, 180.0, "一句话便签"), ("M", 320.0, 240.0, "清单与短笔记"), ("L", 420.0, 320.0, "长笔记 / 会议记录")];
                for (name, w, h, desc) in presets {
                    let selected = (self.settings.tile_width - w).abs() < 1.0 && (self.settings.tile_height - h).abs() < 1.0;
                    let response = components::radio_card(ui, p, Vec2::new(width, 108.0), selected, |ui| {
                        let rect = ui.available_rect_before_wrap();
                        let painter = ui.painter();
                        let scale = 0.22;
                        let size = Vec2::new(w * scale, h * scale);
                        let mock = egui::Rect::from_min_size(
                            pos2(rect.left(), rect.min.y + 4.0 + (54.0 - size.y) / 2.0), size);
                        painter.rect_filled(mock, CornerRadius::same(3), p.sunken);
                        painter.rect_stroke(mock, CornerRadius::same(3), Stroke::new(1.0, p.border), egui::StrokeKind::Inside);
                        painter.rect_filled(
                            egui::Rect::from_min_max(mock.min, pos2(mock.min.x + 2.0, mock.bottom())),
                            CornerRadius { nw: 3, ne: 0, sw: 3, se: 0 }, p.accent);
                        let fit = ((rect.width() / 10.0) as usize).max(6);
                        painter.text(pos2(rect.min.x, rect.min.y + 66.0), Align2::LEFT_CENTER,
                            components::truncate_chars(&format!("{name} · {}×{}", w as u32, h as u32), fit),
                            FontId::proportional(12.5), p.strong);
                        painter.text(pos2(rect.min.x, rect.min.y + 82.0), Align2::LEFT_CENTER,
                            components::truncate_chars(desc, fit), FontId::proportional(9.5), p.dim);
                        ui.allocate_response(rect.size(), Sense::hover());
                    });
                    if response.clicked() {
                        self.settings.tile_width = w;
                        self.settings.tile_height = h;
                        let ctx = ui.ctx().clone();
                        self.save(&ctx);
                    }
                }
            });
            components::setting_row(ui, p, d, "默认磁贴尺寸", "默认尺寸", self.flash_active("默认磁贴尺寸"), |ui| {
                ui.label(RichText::new("宽").size(11.0).color(p.dim));
                let a = ui.add(egui::Slider::new(&mut self.settings.tile_width, 240.0..=720.0).suffix(" px")).changed();
                ui.label(RichText::new("高").size(11.0).color(p.dim));
                let b = ui.add(egui::Slider::new(&mut self.settings.tile_height, 180.0..=720.0).suffix(" px")).changed();
                a || b
            });
        });
        components::group(ui, p, "▦", "磁贴外观", "圆角、阴影与底部信息栏", |ui| {
            components::setting_row(ui, p, d, "磁贴圆角", "圆角窗口需要在合成器支持透明", self.flash_active("磁贴圆角"), |ui| {
                let mut r = self.settings.corner_radius as f32;
                let changed = ui.add(egui::Slider::new(&mut r, 0.0..=20.0).integer().suffix(" px")).changed();
                if changed { self.settings.corner_radius = r as u8; }
                changed
            });
            components::setting_row(ui, p, d, "圆角与阴影窗口", "关闭后磁贴为直角不透明窗口（需重启磁贴）", self.flash_active("圆角与阴影窗口"), |ui| {
                components::toggle_switch(ui, p, &mut self.settings.tile_rounded).changed()
            });
            components::setting_row(ui, p, d, "显示磁贴分类", "底部信息栏左侧的分类色块", self.flash_active("磁贴底部信息"), |ui| {
                components::toggle_switch(ui, p, &mut self.settings.show_tag).changed()
            });
            components::setting_row(ui, p, d, "显示字数统计", "底部信息栏的非空白字符数", self.flash_active("磁贴底部信息"), |ui| {
                components::toggle_switch(ui, p, &mut self.settings.show_count).changed()
            });
            components::setting_row(ui, p, d, "显示保存时间", "底部信息栏的相对时间", self.flash_active("磁贴底部信息"), |ui| {
                components::toggle_switch(ui, p, &mut self.settings.show_time).changed()
            });
        });
    }

    fn section_editor(&mut self, ui: &mut egui::Ui, p: Palette) {
        components::section_head(ui, p, "EDITOR", "笔记与编辑", "默认视图与 Markdown 任务简写。");
        let d = self.settings.density;
        components::group(ui, p, "▤", "视图模式", "新磁贴打开时所处的模式", |ui| {
            components::setting_row(ui, p, d, "默认视图", "编辑或 Markdown 预览", self.flash_active("默认视图"), |ui| {
                let mut idx = if self.settings.default_preview { 1 } else { 0 };
                let changed = components::segmented(ui, p, &["编辑", "预览"], &mut idx);
                if changed { self.settings.default_preview = idx == 1; }
                changed
            });
        });
        components::group(ui, p, "▤", "Markdown 任务简写", "切换预览时自动规范为 GFM 列表", |ui| {
            components::setting_row(ui, p, d, "启用任务简写", "[] 任务 / [x] 完成 → - [ ] 任务 / - [x] 完成", self.flash_active("任务简写规范"), |ui| {
                components::toggle_switch(ui, p, &mut self.settings.normalize_shorthand).changed()
            });
            ui.horizontal(|ui| {
                components::chip(ui, p, Some(p.success), "[] 便签");
                components::chip(ui, p, Some(p.success), "[x] 已完成");
                ui.label(RichText::new("→").size(11.0).color(p.faint));
                components::chip(ui, p, Some(p.accent), "- [ ] 列表");
                components::chip(ui, p, Some(p.accent), "- [x] 列表");
            });
        });
    }

    fn section_shortcuts(&mut self, ui: &mut egui::Ui, p: Palette) {
        components::section_head(ui, p, "SHORTCUTS", "快捷键", "全局快捷键由 Hyprland 提供，TileNote 不抢占系统按键。");
        components::group(ui, p, "⌘", "建议的绑定", "以下键位为示例，请避开你已占用的组合", |ui| {
            for (keys, cmd, desc) in [
                (&["SUPER", "M"][..], "tilenote toggle", "隐藏 / 呼出所有笔记磁贴"),
                (&["SUPER", "SHIFT", "M"][..], "tilenote new", "创建并打开新笔记"),
                (&["SUPER", "ALT", "M"][..], "tilenote settings", "打开设置窗口"),
            ] {
                let row_w = ui.available_width();
                ui.horizontal(|ui| {
                    // Fixed-width label column: prevents the wrapping labels
                    // from eating the whole row and hiding the keycaps.
                    let right_w = (keys.len() as f32 * 54.0 + 96.0).max(150.0);
                    let left_w = (row_w - right_w - 36.0).max(120.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(left_w, 44.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.add(egui::Label::new(
                                RichText::new(desc).size(12.5).color(p.strong)).truncate());
                            ui.add(egui::Label::new(
                                RichText::new(cmd).monospace().size(10.5).color(p.accent)).truncate());
                        },
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(14.0);
                        if ui.button(RichText::new("复制").size(11.0)).clicked() {
                            ui.ctx().copy_text(cmd.to_owned());
                        }
                        for key in keys.iter().rev() {
                            components::keycap(ui, p, key);
                        }
                    });
                });
                ui.add_space(8.0);
                ui.painter().hline(14.0..=ui.available_width() - 14.0, ui.cursor().min.y, Stroke::new(1.0, p.border_soft()));
                ui.add_space(8.0);
            }
            ui.label(RichText::new("绑定语法取决于 Hyprland 版本：examples/ 里有 hyprlang 与 Lua 两种示例。")
                .size(10.5).color(p.faint));
        });
    }

    fn section_sync(&mut self, ui: &mut egui::Ui, p: Palette) {
        components::section_head(ui, p, "SYNC", "同步与备份", "本地优先：可读的 Markdown 与 JSON 文件。");
        components::group(ui, p, "☁", "数据目录", "配置与笔记的保存位置（XDG 标准）", |ui| {
            components::setting_row(ui, p, self.settings.density, "配置文件", "settings.json", self.flash_active("数据目录"), |ui| {
                if ui.button("复制路径").clicked() {
                    ui.ctx().copy_text(self.paths.settings_path().display().to_string());
                }
                false
            });
            components::setting_row(ui, p, self.settings.density, "笔记目录", "notes/*.md + notes/*.json", self.flash_active("数据目录"), |ui| {
                if ui.button("打开目录").clicked() {
                    let _ = std::process::Command::new("xdg-open").arg(&self.paths.notes).spawn();
                }
                false
            });
            ui.label(RichText::new("目前不内置云同步。可用 Syncthing、Git 等工具直接同步上述目录；多设备同时编辑同一磁贴请自行避免冲突。")
                .size(11.0).color(p.dim));
        });
    }

    fn section_about(&mut self, ui: &mut egui::Ui, p: Palette) {
        components::section_head(ui, p, "ABOUT", "关于", "");
        ui.add_space(4.0);
        components::group(ui, p, "⊞", "TileNote", "磁贴式桌面笔记 · 设置窗口 + 独立磁贴小窗", |ui| {
            ui.horizontal(|ui| {
                paint_logo(ui, 44.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("TileNote").size(22.0).strong().color(p.strong));
                    ui.horizontal(|ui| {
                        components::chip(ui, p, Some(p.accent), "v0.1.0");
                        components::chip(ui, p, None, "MIT License");
                    });
                });
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                for name in ["Rust", "egui / eframe", "egui_commonmark", "One Dark", "Hyprland"] {
                    components::chip(ui, p, None, name);
                }
            });
        });
    }
}

impl eframe::App for SettingsApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        // Always opaque: a borderless window must never show the desktop through it.
        let c = visuals.panel_fill;
        [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_read.elapsed() > Duration::from_secs(3) {
            let next = Settings::load(&self.paths);
            if next != self.settings {
                self.settings = next;
                palette::apply(ctx, &self.settings);
            }
            self.last_read = Instant::now();
        }
        let p = Palette::new(&self.settings);
        let before = self.settings.clone();
        self.title_bar(ctx, p);
        self.nav(ctx, p);
        self.footer(ctx, p);
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(p.canvas).inner_margin(egui::Margin::symmetric(24, 18)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    if !self.search.is_empty() {
                        self.search_results(ui, p);
                    } else {
                        match self.section {
                            0 => self.library.show(ui, &self.paths, &self.settings),
                            1 => self.section_general(ui, p),
                            2 => self.section_appearance(ui, p),
                            3 => self.section_tiles(ui, p),
                            4 => self.section_editor(ui, p),
                            5 => self.section_shortcuts(ui, p),
                            6 => self.section_sync(ui, p),
                            _ => self.section_about(ui, p),
                        }
                    }
                });
            });
        if self.settings != before {
            self.save(ctx);
        }
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

impl SettingsApp {
    fn search_results(&mut self, ui: &mut egui::Ui, p: Palette) {
        components::section_head(ui, p, "SEARCH", "搜索结果", &format!("“{}” 匹配的设置项，点击跳转", self.search));
        let query = self.search.to_lowercase();
        let results: Vec<(usize, &str)> = settings_index().into_iter()
            .filter(|(_, label, keys)| {
                label.to_lowercase().contains(&query) || keys.to_lowercase().contains(&query)
            })
            .map(|(s, l, _)| (s, l))
            .collect();
        if results.is_empty() {
            ui.label(RichText::new("没有匹配的设置项。试试「磁贴」「圆角」「快捷键」「密度」。").size(12.0).color(p.dim));
            return;
        }
        for (section, label) in results {
            let (name, kicker) = SECTIONS[section];
            let btn = egui::Button::new(
                RichText::new(format!("{label}   ·   {name} {kicker}")).size(12.5).color(p.strong),
            )
            .fill(p.panel)
            .stroke(Stroke::new(1.0, p.border_soft()))
            .corner_radius(CornerRadius::same(8))
            .min_size(Vec2::new(ui.available_width(), 36.0));
            if ui.add(btn).clicked() {
                self.jump_to(section, label);
            }
            ui.add_space(4.0);
        }
    }
}

/// Searchable settings entries: (section, visible label, keywords).
fn settings_index() -> Vec<(usize, &'static str, &'static str)> {
    vec![
        (1, "自动保存间隔", "保存 防抖 毫秒 autosave 写盘"),
        (1, "关闭磁贴时确认", "关闭 确认 二次 弹窗"),
        (2, "强调色", "颜色 高亮 accent 主题色"),
        (2, "界面密度", "密度 紧凑 舒适 宽松 行距"),
        (2, "正文字号", "字号 字体 大小 文字"),
        (3, "默认磁贴尺寸", "尺寸 大小 s m l 宽 高 新建"),
        (3, "磁贴圆角", "圆角 半径 corner radius"),
        (3, "圆角与阴影窗口", "圆角 阴影 透明 窗口 背景"),
        (3, "显示字数统计", "字数 底栏 footer 统计"),
        (3, "显示保存时间", "时间 底栏 footer 相对"),
        (3, "显示磁贴分类", "分类 标签 底栏 footer"),
        (4, "默认视图", "编辑 预览 视图 打开"),
        (4, "启用任务简写", "简写 任务 待办 normalize 列表"),
        (6, "数据目录", "目录 同步 备份 syncthing git"),
    ]
}

fn density_index(d: Density) -> usize {
    match d { Density::Compact => 0, Density::Cozy => 1, Density::Roomy => 2 }
}
fn density_from(i: usize) -> Density {
    match i { 0 => Density::Compact, 1 => Density::Cozy, _ => Density::Roomy }
}

/// The TileNote logo: a chrome tile containing four accent mini-tiles.
fn paint_logo(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size + 4.0), Sense::hover());
    let painter = ui.painter();
    let r = size * 0.24;
    painter.rect_filled(rect, CornerRadius::same((r as u8).max(3)), palette::hex(0x21252B));
    let inner = rect.shrink(size * 0.18);
    let cell = Vec2::new(inner.width() / 2.0, inner.height() / 2.0);
    let gap = 1.5;
    let colors = [palette::hex(0x61AFEF), palette::hex(0xC678DD), palette::hex(0x98C379), palette::hex(0xE5C07B)];
    for (i, c) in colors.iter().enumerate() {
        let min = pos2(
            inner.min.x + (i % 2) as f32 * (cell.x + gap),
            inner.min.y + (i / 2) as f32 * (cell.y + gap),
        );
        painter.rect_filled(
            egui::Rect::from_min_size(min, Vec2::new(cell.x - gap, cell.y - gap)),
            CornerRadius::same(2),
            *c,
        );
    }
}

fn window_controls(ui: &mut egui::Ui, p: Palette) {
    ui.add_space(6.0);
    let buttons = [("—", "最小化", false), ("□", "最大化 / 还原", false), ("×", "关闭设置窗口", true)];
    for (glyph, tip, danger) in buttons.iter() {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(34.0, TITLE_H - 8.0), Sense::click());
        let painter = ui.painter();
        let hovered = response.hovered();
        let clicked = response.clicked();
        if hovered && *danger {
            painter.rect_filled(rect, CornerRadius::same(6), p.danger);
        } else if hovered {
            painter.rect_filled(rect, CornerRadius::same(6), p.raise);
        }
        painter.text(rect.center(), Align2::CENTER_CENTER, *glyph,
            FontId::proportional(12.0), if hovered && *danger { Color32::WHITE } else { p.dim });
        response.on_hover_text(*tip);
        if clicked {
            match *glyph {
                "—" => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                "□" => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(
                    !ui.input(|i| i.viewport().maximized.unwrap_or(false)),
                )),
                _ => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
    }
}

fn storage_usage(paths: &Paths) -> (usize, u64) {
    let mut count = 0usize;
    let mut bytes = 0u64;
    if let Ok(entries) = std::fs::read_dir(&paths.notes) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                count += 1;
            }
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() { bytes += meta.len(); }
            }
        }
    }
    (count, bytes)
}
