use crate::{hyprland, markdown, model::{self, NoteMeta, Paths, Settings}, ui::palette::{self, Palette}};
use anyhow::Result;
use eframe::egui::{self, Color32, RichText};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::time::{Duration, Instant};

pub struct TileApp {
    paths: Paths,
    meta: NoteMeta,
    settings: Settings,
    text: String,
    markdown_cache: CommonMarkCache,
    dirty: bool,
    last_change: Instant,
    last_refresh: Instant,
    error: Option<String>,
}

impl TileApp {
    pub fn new(cc: &eframe::CreationContext<'_>, paths: Paths, id: &str) -> Result<Self> {
        let meta = NoteMeta::load(&paths, id)?;
        let text = model::read_note(&paths, id)?;
        let settings = Settings::load(&paths);
        palette::install_cjk_fallback(&cc.egui_ctx);
        palette::apply(&cc.egui_ctx, &settings);
        hyprland::pin_when_ready(id.to_owned(), meta.pinned);
        Ok(Self {
            paths, meta, settings, text,
            markdown_cache: CommonMarkCache::default(),
            dirty: false,
            last_change: Instant::now(),
            last_refresh: Instant::now(),
            error: None,
        })
    }

    fn changed(&mut self) {
        self.dirty = true;
        self.last_change = Instant::now();
    }

    fn persist(&mut self) {
        if !self.dirty { return; }
        match model::write_note(&self.paths, &self.meta.id, &self.text) {
            Ok(()) => { self.dirty = false; self.error = None; }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn save_meta(&mut self) {
        if let Err(e) = self.meta.save(&self.paths) { self.error = Some(e.to_string()); }
    }

    fn periodic_sync(&mut self, ctx: &egui::Context) {
        if self.last_refresh.elapsed() < Duration::from_secs(1) { return; }
        self.last_refresh = Instant::now();
        let next = Settings::load(&self.paths);
        if self.settings != next {
            self.settings = next;
            palette::apply(ctx, &self.settings);
        }
        // Let a separate settings window rename/retag/recolor a live tile.
        if let Ok(next) = NoteMeta::load(&self.paths, &self.meta.id) {
            if next.title != self.meta.title || next.tag != self.meta.tag || next.accent != self.meta.accent {
                self.meta.title = next.title;
                self.meta.tag = next.tag;
                self.meta.accent = next.accent;
            }
        }
    }

    fn switch_preview(&mut self) {
        if !self.meta.preview {
            let normalized = markdown::normalize_task_shorthand(&self.text);
            if normalized != self.text {
                self.text = normalized;
                self.changed();
            }
        }
        self.meta.preview = !self.meta.preview;
        self.save_meta();
    }

    fn header(&mut self, ctx: &egui::Context, p: Palette) {
        egui::TopBottomPanel::top("tile_title_bar")
            .exact_height(50.0)
            .frame(egui::Frame::default().fill(p.raised).inner_margin(egui::Margin::symmetric(12, 7)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let tag = self.meta.tag.clone();
                    let badge = egui::Button::new(RichText::new(tag).color(p.accent).small())
                        .fill(p.accent.gamma_multiply(0.12));
                    ui.menu_button("▤", |ui| {
                        ui.label("磁贴类型");
                        for t in ["灵感", "会议", "待办", "规范", "随笔", "其他"] {
                            if ui.selectable_label(self.meta.tag == t, t).clicked() {
                                self.meta.tag = t.to_owned(); self.save_meta(); ui.close();
                            }
                        }
                    });
                    // A colored label instead of a native title bar: still editable.
                    ui.add(badge).on_hover_text("点击旁边的 ▤ 更改分类");
                    let title = ui.add(egui::TextEdit::singleline(&mut self.meta.title)
                        .desired_width(120.0).frame(false));
                    if title.changed() { self.save_meta(); }
                    let rest = ui.available_width();
                    if rest > 112.0 {
                        let drag = ui.allocate_response(egui::vec2(rest - 110.0, 26.0), egui::Sense::drag());
                        if drag.drag_started() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                        drag.on_hover_text("拖动磁贴");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("×").on_hover_text("关闭磁贴，笔记仍保留").clicked() {
                            self.persist();
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button(if self.meta.preview { "✎" } else { "👁" })
                            .on_hover_text(if self.meta.preview { "返回文本编辑" } else { "Markdown 实时预览" })
                            .clicked() { self.switch_preview(); }
                        if ui.button(if self.meta.pinned { "⌖" } else { "◇" })
                            .on_hover_text(if self.meta.pinned { "取消置顶固定" } else { "固定到所有工作区" })
                            .clicked() {
                            self.meta.pinned = !self.meta.pinned;
                            if let Err(e) = hyprland::set_pin(&self.meta.id, self.meta.pinned) {
                                self.error = Some(e.to_string());
                            }
                            self.save_meta();
                        }
                    });
                });
            });
    }

    fn footer(&mut self, ctx: &egui::Context, p: Palette) {
        egui::TopBottomPanel::bottom("tile_footer").exact_height(33.0)
            .frame(egui::Frame::default().fill(p.panel).inner_margin(egui::Margin::symmetric(12, 7)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let state = if self.error.is_some() { "● 保存错误" }
                        else if self.dirty { "● 保存中" } else { "● 已保存" };
                    let color = if self.error.is_some() { Color32::RED }
                        else if self.dirty { p.accent } else { p.success };
                    ui.colored_label(color, state);
                    ui.colored_label(p.muted, "·");
                    ui.colored_label(p.muted, format!("{} 字", self.text.chars().filter(|c| !c.is_whitespace()).count()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(p.muted, if self.meta.preview { "预览" } else { "编辑" });
                    });
                });
            });
        if let Some(error) = self.error.clone() {
            egui::TopBottomPanel::bottom("tile_error").show(ctx, |ui| {
                ui.colored_label(Color32::RED, format!("无法保存：{error}"));
                if ui.button("重试").clicked() { self.persist(); }
            });
        }
    }
}

impl eframe::App for TileApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.periodic_sync(ctx);
        let mut p = Palette::new(&self.settings);
        p.accent = match self.settings.theme {
            model::Theme::OneDarkPro => palette::DARK_ACCENTS[self.meta.accent % 6],
            model::Theme::GithubLight => palette::LIGHT_ACCENTS[self.meta.accent % 6],
        };
        self.header(ctx, p);
        self.footer(ctx, p);
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(p.bg).inner_margin(egui::Margin::same(16)))
            .show(ctx, |ui| {
                if self.meta.preview {
                    let original = self.text.clone();
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        CommonMarkViewer::new().show_mut(ui, &mut self.markdown_cache, &mut self.text);
                    });
                    if self.text != original { self.changed(); }
                } else {
                    let editor = egui::TextEdit::multiline(&mut self.text)
                        .font(egui::TextStyle::Monospace)
                        .frame(false)
                        .desired_width(f32::INFINITY)
                        .hint_text("在这里记录灵感…\n\n# 标题\n- [ ] 待办事项\n\n点击右上角 👁 预览");
                    let response = ui.add_sized(ui.available_size(), editor);
                    if response.changed() { self.changed(); }
                }
            });
        if self.dirty && self.last_change.elapsed() >= Duration::from_millis(400) {
            self.persist();
        }
        ctx.request_repaint_after(Duration::from_millis(if self.dirty { 120 } else { 900 }));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist();
    }
}
