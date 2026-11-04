use crate::{hyprland, model::{self, Paths, Settings, Theme}, ui::palette::{self, Palette}};
use eframe::egui::{self, Color32, RichText};
use std::time::{Duration, Instant};

pub struct SettingsApp {
    paths: Paths,
    settings: Settings,
    section: usize,
    search: String,
    pending_delete: Option<String>,
    message: Option<String>,
    last_read: Instant,
}

const SECTIONS: [&str; 5] = ["通用 / 笔记", "外观", "同步与备份", "快捷键", "关于"];

impl SettingsApp {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: Paths) -> Self {
        let settings = Settings::load(&paths);
        palette::install_cjk_fallback(&cc.egui_ctx);
        palette::apply(&cc.egui_ctx, &settings);
        Self {
            paths, settings, section: 1, search: String::new(), pending_delete: None,
            message: None, last_read: Instant::now(),
        }
    }

    fn save(&mut self, ctx: &egui::Context) {
        palette::apply(ctx, &self.settings);
        if let Err(e) = self.settings.save(&self.paths) {
            self.message = Some(format!("保存配置失败：{e:#}"));
        }
    }

    fn topbar(&self, ctx: &egui::Context, p: Palette) {
        egui::TopBottomPanel::top("settings_top")
            .exact_height(47.0)
            .frame(egui::Frame::default().fill(p.raised).inner_margin(egui::Margin::symmetric(16, 10)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(p.accent, "■");
                    ui.label(RichText::new("设置").strong());
                    ui.colored_label(p.muted, "TileNote — Preferences");
                });
            });
    }

    fn nav(&mut self, ctx: &egui::Context, p: Palette) {
        egui::SidePanel::left("settings_nav").exact_width(236.0)
            .resizable(false)
            .frame(egui::Frame::default().fill(p.panel).inner_margin(egui::Margin::same(12)))
            .show(ctx, |ui| {
                ui.add_space(3.0);
                ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("⌕ 搜索设置").desired_width(f32::INFINITY));
                ui.add_space(16.0);
                for (index, label) in SECTIONS.iter().enumerate() {
                    if !self.search.is_empty() && !label.contains(self.search.trim()) { continue; }
                    let text = RichText::new(*label)
                        .color(if self.section == index { p.text } else { p.muted });
                    let button = egui::Button::new(text)
                        .fill(if self.section == index { p.raised } else { p.panel })
                        .min_size(egui::vec2(210.0, 36.0));
                    if ui.add(button).clicked() { self.section = index; }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(10.0);
                    palette::frame(p, self.settings.corner_radius).show(ui, |ui| {
                        ui.colored_label(p.muted, "CONFIG");
                        ui.add_space(5.0);
                        ui.small(self.paths.settings_path().display().to_string());
                    });
                });
            });
    }

    fn section_heading(ui: &mut egui::Ui, title: &str, marker: &str, p: Palette) {
        ui.horizontal(|ui| {
            ui.heading(RichText::new(title).color(p.text));
            ui.colored_label(p.muted, marker);
        });
        ui.add_space(16.0);
    }

    fn setting_row(ui: &mut egui::Ui, label: &str, detail: &str, p: Palette, body: impl FnOnce(&mut egui::Ui)) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(label).size(15.0));
                if !detail.is_empty() { ui.colored_label(p.muted, detail); }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| body(ui));
        });
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);
    }

    fn theme_choice(ui: &mut egui::Ui, label: &str, s: &Settings, theme: Theme, p: Palette) -> bool {
        let selected = s.theme == theme;
        let background = if selected { p.raised } else { p.bg };
        let (rect, response) = ui.allocate_exact_size(egui::vec2(158.0, 100.0), egui::Sense::click());
        let stroke = egui::Stroke::new(if selected { 1.7 } else { 1.0 }, if selected { p.accent } else { p.divider });
        ui.painter().rect(rect, egui::CornerRadius::same(8), background, stroke, egui::StrokeKind::Inside);
        let dark = theme == Theme::OneDarkPro;
        let mini = egui::Rect::from_min_size(rect.min + egui::vec2(12.0, 12.0), egui::vec2(134.0, 48.0));
        ui.painter().rect_filled(mini, 4.0, if dark { palette::hex(0x282C34) } else { Color32::WHITE });
        let mini_line = if dark { palette::hex(0xABB2BF) } else { palette::hex(0x57606A) };
        let mini_accent = if dark { palette::hex(0x61AFEF) } else { palette::hex(0x0969DA) };
        ui.painter().rect_filled(egui::Rect::from_min_size(mini.min + egui::vec2(10.0, 15.0), egui::vec2(43.0, 9.0)), 1.0, mini_accent);
        ui.painter().rect_filled(egui::Rect::from_min_size(mini.min + egui::vec2(59.0, 15.0), egui::vec2(27.0, 9.0)), 1.0, mini_line.gamma_multiply(0.48));
        ui.painter().text(rect.min + egui::vec2(12.0, 75.0), egui::Align2::LEFT_CENTER,
            label, egui::FontId::monospace(12.0), p.text);
        response.clicked()
    }

    fn appearance(&mut self, ui: &mut egui::Ui, p: Palette) -> bool {
        Self::section_heading(ui, "外观", "PANE 02 / 05", p);
        let mut changed = false;
        ui.colored_label(p.accent, "主题  ──────────────────────────────────────────  THEME");
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            if Self::theme_choice(ui, "One Dark Pro", &self.settings, Theme::OneDarkPro, p) {
                self.settings.theme = Theme::OneDarkPro; changed = true;
            }
            if Self::theme_choice(ui, "GitHub Light", &self.settings, Theme::GithubLight, p) {
                self.settings.theme = Theme::GithubLight; changed = true;
            }
        });
        ui.add_space(28.0);
        ui.colored_label(p.accent, "强调色  ──────────────────────────────────────  ACCENT");
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("全局强调色");
                ui.colored_label(p.muted, "用于高亮、开关、状态条与选中态");
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let colors = match self.settings.theme {
                    Theme::OneDarkPro => palette::DARK_ACCENTS,
                    Theme::GithubLight => palette::LIGHT_ACCENTS,
                };
                // right_to_left layout: reverse iter order to keep blue first visually.
                for (i, color) in colors.iter().enumerate().rev() {
                    let size = if i == self.settings.accent { egui::vec2(33.0, 33.0) } else { egui::vec2(27.0, 27.0) };
                    if ui.add(egui::Button::new(" ").fill(*color).min_size(size))
                        .on_hover_text(format!("颜色 {}", i + 1)).clicked() {
                        self.settings.accent = i; changed = true;
                    }
                }
            });
        });
        ui.add_space(28.0);
        ui.colored_label(p.accent, "排版  ────────────────────────────────────────  TYPE");
        ui.add_space(10.0);
        Self::setting_row(ui, "界面字号", "12–18px", p, |ui| {
            changed |= ui.add(egui::Slider::new(&mut self.settings.font_size, 12.0..=18.0)
                .integer().show_value(true)).changed();
        });
        Self::setting_row(ui, "磁贴圆角", "0–12px", p, |ui| {
            changed |= ui.add(egui::Slider::new(&mut self.settings.corner_radius, 0..=12)).changed();
        });
        Self::setting_row(ui, "窗口阴影", "可能由 Hyprland 合成器覆盖", p, |ui| {
            changed |= ui.checkbox(&mut self.settings.shadow, "启用").changed();
        });
        ui.add_space(18.0);
        ui.colored_label(p.muted, "主题为应用内固定选择，不读取系统主题。");
        changed
    }

    fn general(&mut self, ui: &mut egui::Ui, p: Palette) -> bool {
        Self::section_heading(ui, "通用与笔记", "PANE 01 / 05", p);
        let mut changed = false;
        ui.colored_label(p.accent, "行为  ────────────────────────────────────────  GENERAL");
        ui.add_space(12.0);
        Self::setting_row(ui, "默认固定磁贴", "在 Hyprland 的所有普通工作区保持可见", p, |ui| {
            changed |= ui.checkbox(&mut self.settings.auto_pin, "新磁贴置顶").changed();
        });
        Self::setting_row(ui, "默认预览模式", "新磁贴也可以选择直接进入文本编辑", p, |ui| {
            changed |= ui.checkbox(&mut self.settings.default_preview, "Markdown 预览").changed();
        });
        Self::setting_row(ui, "磁贴默认宽度", "可以在窗口管理器中随时调整大小", p, |ui| {
            changed |= ui.add(egui::Slider::new(&mut self.settings.tile_width, 320.0..=700.0).suffix("px")).changed();
        });
        Self::setting_row(ui, "磁贴默认高度", "", p, |ui| {
            changed |= ui.add(egui::Slider::new(&mut self.settings.tile_height, 220.0..=650.0).suffix("px")).changed();
        });
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.colored_label(p.accent, "笔记磁贴  ──────────────────────  NOTES");
            if ui.button("＋ 新建笔记").clicked() {
                match model::create_note(&self.paths, "新笔记", &self.settings) {
                    Ok(meta) => match crate::start_tile(&meta.id) {
                        Ok(()) => self.message = Some("已新建笔记磁贴".to_owned()),
                        Err(e) => self.message = Some(format!("笔记已创建，但启动失败：{e:#}")),
                    },
                    Err(e) => self.message = Some(format!("创建笔记失败：{e:#}")),
                }
            }
        });
        ui.add_space(12.0);
        let notes = model::list_notes(&self.paths);
        if notes.is_empty() {
            ui.colored_label(p.muted, "尚未创建笔记。点击上方按钮开始记录灵感。");
        }
        for meta in notes {
            palette::frame(p, self.settings.corner_radius).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(p.accent, "▌");
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&meta.title).strong());
                        ui.colored_label(p.muted, format!("{}  ·  {}", meta.tag, meta.id));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("删除").clicked() { self.pending_delete = Some(meta.id.clone()); }
                        if ui.button("打开").clicked() {
                            match hyprland::focus_tile(&meta.id, meta.pinned) {
                                Ok(true) => (),
                                _ => if let Err(e) = crate::start_tile(&meta.id) {
                                    self.message = Some(format!("打开磁贴失败：{e:#}"));
                                },
                            }
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }
        changed
    }

    fn backup(&self, ui: &mut egui::Ui, p: Palette) {
        Self::section_heading(ui, "备份与同步", "PANE 03 / 05", p);
        ui.label("笔记全部使用可读的 Markdown 文件保存，无需数据库。");
        ui.add_space(15.0);
        ui.colored_label(p.accent, "数据目录");
        ui.monospace(self.paths.notes.display().to_string());
        if ui.button("复制数据目录路径").clicked() { ui.ctx().copy_text(self.paths.notes.display().to_string()); }
        ui.add_space(16.0);
        ui.colored_label(p.accent, "配置目录");
        ui.monospace(self.paths.settings_path().display().to_string());
        if ui.button("复制配置文件路径").clicked() { ui.ctx().copy_text(self.paths.settings_path().display().to_string()); }
        ui.add_space(16.0);
        ui.colored_label(p.muted, "目前不内置云同步。可以通过 Syncthing、Git 等工具同步上述文件目录，避免多个设备同时编辑同一磁贴。");
    }

    fn hotkeys(&self, ui: &mut egui::Ui, p: Palette) {
        Self::section_heading(ui, "快捷键", "PANE 04 / 05", p);
        ui.label("全局快捷键由 Hyprland 负责，TileNote 不抢占系统按键。以下键位仅为示例，请避开你已占用的组合。");
        ui.add_space(15.0);
        for (key, cmd, desc) in [
            ("SUPER + M", "tilenote toggle", "隐藏 / 呼出所有笔记磁贴"),
            ("SUPER + SHIFT + M", "tilenote new", "创建并打开新笔记"),
            ("SUPER + ALT + M", "tilenote settings", "打开设置"),
        ] {
            palette::frame(p, self.settings.corner_radius).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.monospace(key);
                    ui.colored_label(p.muted, desc);
                });
                ui.horizontal(|ui| {
                    ui.colored_label(p.accent, cmd);
                    if ui.small_button("复制").clicked() { ui.ctx().copy_text(cmd.to_owned()); }
                });
            });
            ui.add_space(5.0);
        }
        ui.colored_label(p.muted, "具体绑定语法取决于 Hyprland 版本。请参考 examples 目录里的配置示例（含 hyprlang 与 Lua 两种）。");
    }

    fn about(&self, ui: &mut egui::Ui, p: Palette) {
        Self::section_heading(ui, "关于 TileNote", "PANE 05 / 05", p);
        ui.heading("TileNote 0.1.0");
        ui.label("基于 Rust、egui / eframe 的轻量级 Markdown 桌面磁贴。");
        ui.add_space(15.0);
        ui.colored_label(p.muted, "专为 Linux Wayland + Hyprland 工作流设计；笔记数据完全本地保存。");
        ui.label("源码协议：MIT");
    }

    fn confirm_delete(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.pending_delete.clone() {
            egui::Window::new("删除笔记？").collapsible(false).resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.label("这会永久删除对应的 Markdown 正文和元数据。请先关闭正在编辑的磁贴。");
                    ui.horizontal(|ui| {
                        if ui.button("取消").clicked() { self.pending_delete = None; }
                        if ui.button("确认删除").clicked() {
                            match hyprland::find_tile(&id) {
                                Ok(Some(_)) => self.message = Some("该磁贴仍在运行，请先关闭磁贴后再删除。".to_owned()),
                                _ => match model::delete_note(&self.paths, &id) {
                                    Ok(()) => self.message = Some("笔记已删除".to_owned()),
                                    Err(e) => self.message = Some(format!("删除失败：{e:#}")),
                                },
                            }
                            self.pending_delete = None;
                        }
                    });
                });
        }
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_read.elapsed() > Duration::from_secs(3) {
            // External settings changes (CLI or other settings windows).
            let next = Settings::load(&self.paths);
            if next != self.settings {
                self.settings = next;
                palette::apply(ctx, &self.settings);
            }
            self.last_read = Instant::now();
        }
        let p = Palette::new(&self.settings);
        self.topbar(ctx, p);
        self.nav(ctx, p);
        let mut changed = false;
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(p.bg).inner_margin(egui::Margin::same(24)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    changed = match self.section {
                        0 => self.general(ui, p),
                        1 => self.appearance(ui, p),
                        2 => { self.backup(ui, p); false }
                        3 => { self.hotkeys(ui, p); false }
                        _ => { self.about(ui, p); false }
                    };
                });
            });
        if changed { self.save(ctx); }
        if let Some(msg) = self.message.clone() {
            egui::TopBottomPanel::bottom("settings_message")
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(&msg);
                        if ui.small_button("×").clicked() { self.message = None; }
                    });
                });
        }
        self.confirm_delete(ctx);
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}
