use crate::{
    hyprland, markdown,
    model::{self, NoteMeta, Paths, Settings},
    ui::{components, palette::{self, Palette}},
};
use anyhow::Result;
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, RichText, Sense, Stroke, Vec2, pos2};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::time::{Duration, Instant};

/// Extra window size reserved for the soft shadow when rounded windows are on.
pub const SHADOW_MARGIN: f32 = 16.0;
const TITLE_H: f32 = 32.0;
const FOOTER_H: f32 = 26.0;

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
    confirm_pending: bool,
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
            confirm_pending: false,
        })
    }

    fn palette(&self) -> Palette {
        let mut p = Palette::new(&self.settings);
        p.accent = palette::accents(self.settings.theme)[self.meta.accent % 6];
        p
    }

    fn changed(&mut self) {
        self.dirty = true;
        self.last_change = Instant::now();
    }

    fn persist(&mut self) {
        if !self.dirty { return; }
        match model::write_note(&self.paths, &self.meta.id, &self.text) {
            Ok(()) => {
                self.dirty = false;
                self.error = None;
                self.meta.updated_at = model::now_unix();
                self.save_meta();
            }
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
        if let Ok(next) = NoteMeta::load(&self.paths, &self.meta.id) {
            if next.title != self.meta.title || next.tag != self.meta.tag || next.accent != self.meta.accent {
                self.meta.title = next.title;
                self.meta.tag = next.tag;
                self.meta.accent = next.accent;
            }
        }
    }

    fn switch_preview(&mut self) {
        if !self.meta.preview && self.settings.normalize_shorthand {
            let normalized = markdown::normalize_task_shorthand(&self.text);
            if normalized != self.text {
                self.text = normalized;
                self.changed();
            }
        }
        self.meta.preview = !self.meta.preview;
        self.save_meta();
    }

    fn request_close(&mut self, ctx: &egui::Context) {
        if self.settings.confirm_close {
            self.confirm_pending = true;
        } else {
            self.persist();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn toggle_pin(&mut self) {
        self.meta.pinned = !self.meta.pinned;
        if let Err(e) = hyprland::set_pin(&self.meta.id, self.meta.pinned) {
            self.error = Some(e.to_string());
        }
        self.save_meta();
    }

    fn footer_visible(&self) -> bool {
        self.settings.show_tag || self.settings.show_count || self.settings.show_time || self.error.is_some()
    }

    /// Title bar: accent dot, editable title, todo badge, drag zone, tools.
    fn header(&mut self, ui: &mut egui::Ui, p: Palette) {
        ui.horizontal_centered(|ui| {
            ui.set_min_height(TITLE_H);
            ui.add_space(9.0);
            // Accent dot (visual pin marker sits inside it).
            let (dot, _) = ui.allocate_exact_size(Vec2::splat(17.0), Sense::hover());
            let painter = ui.painter();
            painter.circle_stroke(dot.center(), 8.0, Stroke::new(1.0, Color32::from_black_alpha(70)));
            painter.circle_filled(dot.center(), 7.0, p.accent);
            if self.meta.pinned {
                painter.text(dot.center(), Align2::CENTER_CENTER, "⌖",
                    FontId::proportional(9.0), Color32::WHITE);
            }
            ui.add_space(4.0);
            let title = ui.add(
                egui::TextEdit::singleline(&mut self.meta.title)
                    .desired_width(112.0)
                    .frame(false)
                    .hint_text("未命名磁贴"),
            );
            if title.changed() { self.save_meta(); }
            let (done, total) = count_tasks(&self.text);
            if total > 0 {
                ui.add_space(4.0);
                components::chip(ui, p, Some(p.accent), &format!("{done}/{total}"));
            }
            // Drag zone over the remaining title space.
            let rest = ui.available_width();
            if rest > 118.0 {
                let drag = ui.allocate_response(Vec2::new(rest - 114.0, TITLE_H - 6.0), Sense::drag());
                if drag.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                drag.on_hover_cursor(egui::CursorIcon::Grab)
                    .on_hover_text("拖动磁贴");
            }
            // Tools: category / pin / edit-preview / close.
            let categories = self.settings.categories.clone();
            ui.menu_button(RichText::new("▤").size(11.5).color(p.dim), |ui| {
                ui.set_min_width(120.0);
                ui.label(RichText::new("磁贴分类").size(10.5).color(p.faint));
                for tag in &categories {
                    if ui.selectable_label(self.meta.tag == *tag, tag).clicked() {
                        self.meta.tag = tag.clone();
                        self.save_meta();
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text("磁贴分类");
            if components::tool_button(ui, p, if self.meta.pinned { "⌖" } else { "◇" },
                self.meta.pinned, if self.meta.pinned { "取消固定" } else { "固定到所有工作区" }).clicked() {
                self.toggle_pin();
            }
            if components::tool_button(ui, p, if self.meta.preview { "✎" } else { "👁" },
                self.meta.preview, if self.meta.preview { "返回文本编辑" } else { "Markdown 预览" }).clicked() {
                self.switch_preview();
            }
            let (sep, _) = ui.allocate_exact_size(Vec2::new(1.0, 12.0), Sense::hover());
            ui.painter().rect_filled(sep, 0.0, p.border_soft());
            if components::tool_button(ui, p, "×", false, "关闭磁贴，笔记仍保留").clicked() {
                let ctx = ui.ctx().clone();
                self.request_close(&ctx);
            }
        });
    }

    fn content(&mut self, ui: &mut egui::Ui, p: Palette) {
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(13, 10))
            .show(ui, |ui| {
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
                        .hint_text("在此输入…支持 Markdown 子集\n\n# 标题\n- [ ] 待办事项");
                    let response = ui.add_sized(ui.available_size(), editor);
                    if response.changed() { self.changed(); }
                }
            })
            .response
            .on_hover_cursor(egui::CursorIcon::Text);
        let _ = p;
    }

    fn footer(&mut self, ui: &mut egui::Ui, p: Palette) {
        ui.horizontal_centered(|ui| {
            ui.set_min_height(FOOTER_H);
            ui.add_space(10.0);
            if let Some(error) = self.error.clone() {
                components::status_dot(ui, p.danger, 3.5);
                ui.add_space(3.0);
                if ui.link(RichText::new("保存失败，点击重试").size(10.5).color(p.danger))
                    .on_hover_text(error).clicked() {
                    self.persist();
                }
            } else if self.dirty {
                components::status_dot(ui, p.warn, 3.5);
                ui.add_space(3.0);
                ui.label(RichText::new("保存中").size(10.5).color(p.faint));
            } else {
                components::status_dot(ui, p.success, 3.5);
                ui.add_space(3.0);
                ui.label(RichText::new("已保存").size(10.5).color(p.faint));
            }
            if self.settings.show_tag {
                ui.add_space(6.0);
                components::chip(ui, p, Some(p.accent), &self.meta.tag);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0);
                if self.settings.show_time {
                    ui.label(RichText::new(model::relative_time(self.meta.updated_at))
                        .monospace().size(10.0).color(p.faint));
                    ui.add_space(8.0);
                }
                if self.settings.show_count {
                    let chars = self.text.chars().filter(|c| !c.is_whitespace()).count();
                    ui.label(RichText::new(format!("{chars} 字"))
                        .monospace().size(10.0).color(p.faint));
                    ui.add_space(8.0);
                }
                ui.label(RichText::new(if self.meta.preview { "预览" } else { "编辑" })
                    .size(10.0).color(p.faint));
            });
        });
    }

    fn confirm_dialog(&mut self, ctx: &egui::Context, p: Palette) {
        if !self.confirm_pending { return; }
        egui::Window::new("关闭磁贴？")
            .collapsible(false).resizable(false).title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(egui::Frame::default().fill(p.panel)
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(18))
                .stroke(Stroke::new(1.0, p.border_soft())))
            .show(ctx, |ui| {
                ui.set_width(240.0);
                ui.label(RichText::new("关闭这个磁贴？").size(14.0).strong().color(p.strong));
                ui.add_space(4.0);
                ui.label(RichText::new("笔记内容仍会保留在笔记库中。").size(11.0).color(p.dim));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() { self.confirm_pending = false; }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("关闭磁贴").color(p.danger)).clicked() {
                            self.confirm_pending = false;
                            self.persist();
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                });
            });
    }
}

impl eframe::App for TileApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.periodic_sync(ctx);
        let p = self.palette();
        let rounded = self.settings.tile_rounded;
        let margin = if rounded { SHADOW_MARGIN } else { 0.0 };
        let radius: u8 = if rounded { self.settings.corner_radius.max(1) } else { 0 };
        let footer_h = if self.footer_visible() { FOOTER_H } else { 0.0 };

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let body = ui.available_rect_before_wrap().shrink(margin);
                let painter = ui.painter();
                if rounded {
                    // Layered soft shadow around the tile.
                    for (expand, alpha) in [(13.0f32, 16u8), (8.0, 24), (4.0, 34)] {
                        painter.rect_filled(
                            body.expand(expand),
                            CornerRadius::same(radius.saturating_add(expand as u8)),
                            p.shadow_color().gamma_multiply(alpha as f32 / 60.0),
                        );
                    }
                }
                let focused = ctx.input(|i| i.viewport().focused).unwrap_or(true);
                painter.rect_filled(body, CornerRadius::same(radius), p.panel);
                let border = if focused { p.accent.gamma_multiply(0.35) } else { p.border };
                painter.rect_stroke(body, CornerRadius::same(radius),
                    Stroke::new(1.0, border), egui::StrokeKind::Inside);
                // Left accent strip.
                let strip = egui::Rect::from_min_max(
                    pos2(body.left(), body.top() + radius as f32),
                    pos2(body.left() + 3.0, body.bottom() - radius as f32),
                );
                painter.rect_filled(strip, 0.0, p.accent.gamma_multiply(0.55));

                let title_rect = egui::Rect::from_min_max(
                    body.min,
                    pos2(body.right(), body.min.y + TITLE_H),
                );
                let content_rect = egui::Rect::from_min_max(
                    pos2(body.left(), title_rect.bottom()),
                    pos2(body.right(), body.bottom() - footer_h),
                );
                painter.rect_filled(title_rect,
                    CornerRadius { nw: radius, ne: radius, sw: 0, se: 0 }, p.chrome);
                painter.hline(title_rect.left() + 1.0..=title_rect.right() - 1.0,
                    title_rect.bottom() - 0.5, Stroke::new(1.0, p.border_soft()));
                if footer_h > 0.0 {
                    let footer_rect = egui::Rect::from_min_max(
                        pos2(body.left(), body.bottom() - footer_h),
                        body.max,
                    );
                    painter.rect_filled(footer_rect,
                        CornerRadius { nw: 0, ne: 0, sw: radius, se: radius }, p.chrome);
                    painter.hline(footer_rect.left() + 1.0..=footer_rect.right() - 1.0,
                        footer_rect.top() + 0.5, Stroke::new(1.0, p.border_soft()));
                    let footer_inner = egui::Rect::from_min_max(
                        pos2(footer_rect.left() + 3.0, footer_rect.top()),
                        pos2(footer_rect.right() - 3.0, footer_rect.bottom()),
                    );
                    ui.allocate_ui_at_rect(footer_inner, |ui| {
                        self.footer(ui, p);
                    });
                }
                ui.allocate_ui_at_rect(title_rect, |ui| {
                    self.header(ui, p);
                });
                let content_inner = egui::Rect::from_min_max(
                    pos2(content_rect.left() + 3.0, content_rect.top()),
                    pos2(content_rect.right() - 3.0, content_rect.bottom()),
                );
                ui.allocate_ui_at_rect(content_inner, |ui| {
                    self.content(ui, p);
                });
            });

        self.confirm_dialog(ctx, p);
        let autosave = Duration::from_millis(self.settings.autosave_ms.max(120) as u64);
        if self.dirty && self.last_change.elapsed() >= autosave {
            self.persist();
        }
        ctx.request_repaint_after(Duration::from_millis(if self.dirty { 120 } else { 900 }));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist();
    }
}

/// Count GFM task-list items: returns (done, total).
fn count_tasks(text: &str) -> (usize, usize) {
    let mut done = 0;
    let mut total = 0;
    for line in text.lines() {
        let t = line.trim_start_matches([' ', '\t']);
        let is_task = ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "* [X] "]
            .iter().any(|p| t.starts_with(p));
        if is_task {
            total += 1;
            if t.starts_with("- [x") || t.starts_with("- [X") || t.starts_with("* [x") || t.starts_with("* [X") {
                done += 1;
            }
        }
    }
    (done, total)
}
