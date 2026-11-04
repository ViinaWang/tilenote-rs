//! Note library: a draggable map canvas of note tiles plus a filterable
//! three-pane list view (categories / note rows / markdown detail).
use crate::{
    hyprland, model::{self, NoteMeta, Paths, Settings},
    ui::{components, palette::{self, Palette}},
};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, RichText, Sense, Stroke, Vec2, pos2};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::{collections::HashMap, time::Instant};

const MAP_CELL: Vec2 = Vec2::new(206.0, 126.0);
const MAP_ORIGIN: Vec2 = Vec2::new(16.0, 16.0);
const MAP_COLS: usize = 4;
const TILE_SIZE: Vec2 = Vec2::new(190.0, 110.0);
const GRID_STEP: f32 = 26.0;

pub struct Library {
    pub view: usize, // 0 = map, 1 = list
    pub search: String,
    pub filter_tag: Option<String>,
    pub selected: Option<String>,
    pending_delete: Option<String>,
    message: Option<String>,
    notes: Vec<NoteMeta>,
    bodies: HashMap<String, String>,
    markdown_cache: CommonMarkCache,
    last_refresh: Instant,
}

impl Library {
    pub fn new() -> Self {
        Self {
            view: 0,
            search: String::new(),
            filter_tag: None,
            selected: None,
            pending_delete: None,
            message: None,
            notes: Vec::new(),
            bodies: HashMap::new(),
            markdown_cache: CommonMarkCache::default(),
            last_refresh: Instant::now()
                - std::time::Duration::from_secs(10), // force first refresh
        }
    }

    fn refresh(&mut self, paths: &Paths) {
        self.notes = model::list_notes(paths);
        self.bodies.clear();
        for note in &self.notes {
            self.bodies.insert(note.id.clone(), model::read_note(paths, &note.id).unwrap_or_default());
        }
        self.last_refresh = Instant::now();
    }

    fn maybe_refresh(&mut self, paths: &Paths) {
        if self.last_refresh.elapsed() > std::time::Duration::from_secs(2) {
            self.refresh(paths);
        }
    }

    fn matches(&self, note: &NoteMeta) -> bool {
        if let Some(tag) = &self.filter_tag {
            if &note.tag != tag { return false; }
        }
        let q = self.search.trim().to_lowercase();
        if q.is_empty() { return true; }
        note.title.to_lowercase().contains(&q)
            || note.tag.to_lowercase().contains(&q)
            || self.bodies.get(&note.id).map(|b| b.to_lowercase().contains(&q)).unwrap_or(false)
    }

    /// First free grid cell for notes that have never been placed.
    fn place_unplaced(&mut self, paths: &Paths) {
        let mut occupied: Vec<(usize, usize)> = self.notes.iter()
            .filter(|n| n.map_x > 0.0 || n.map_y > 0.0)
            .map(|n| cell_of(n.map_x, n.map_y))
            .collect();
        let mut dirty = false;
        for note in self.notes.iter_mut() {
            if note.map_x != 0.0 || note.map_y != 0.0 { continue; }
            let mut cell = (0, 0);
            loop {
                if !occupied.contains(&cell) { break; }
                cell.0 += 1;
                if cell.0 >= MAP_COLS { cell.0 = 0; cell.1 += 1; }
            }
            occupied.push(cell);
            note.map_x = MAP_ORIGIN.x + cell.0 as f32 * MAP_CELL.x;
            note.map_y = MAP_ORIGIN.y + cell.1 as f32 * MAP_CELL.y;
            let _ = note.save(paths);
            dirty = true;
        }
        let _ = dirty;
    }

    fn rearrange_all(&mut self, paths: &Paths) {
        for (i, note) in self.notes.iter_mut().enumerate() {
            let cell = (i % MAP_COLS, i / MAP_COLS);
            note.map_x = MAP_ORIGIN.x + cell.0 as f32 * MAP_CELL.x;
            note.map_y = MAP_ORIGIN.y + cell.1 as f32 * MAP_CELL.y;
            let _ = note.save(paths);
        }
        self.message = Some("已按网格重新排列".to_owned());
    }

    /// Render the library inside the settings window content area.
    pub fn show(&mut self, ui: &mut egui::Ui, paths: &Paths, settings: &Settings) {
        self.maybe_refresh(paths);
        let p = Palette::new(settings);
        if self.view == 0 { self.place_unplaced(paths); }

        components::section_head(ui, p, "LIBRARY", "笔记库",
            "全部笔记的地图与列表。双击地图磁贴或点击“打开磁贴”回到桌面。");

        // Toolbar: view switch + search + category filter + actions.
        ui.horizontal(|ui| {
            components::segmented(ui, p, &["画布", "列表"], &mut self.view);
            ui.add_space(6.0);
            let search = egui::TextEdit::singleline(&mut self.search)
                .hint_text("⌕ 搜索标题 / 正文 / 分类")
                .desired_width(190.0)
                .frame(true);
            ui.add(search);
            ui.add_space(4.0);
            let mut tags: Vec<String> = self.notes.iter().map(|n| n.tag.clone()).collect();
            for c in &settings.categories {
                if !tags.contains(c) { tags.push(c.clone()); }
            }
            if ui.add(filter_chip(p, self.filter_tag.is_none(), &format!("全部 {}", self.notes.len()))).clicked() {
                self.filter_tag = None;
            }
            for tag in tags {
                let count = self.notes.iter().filter(|n| n.tag == tag).count();
                if count == 0 && self.filter_tag.as_deref() != Some(tag.as_str()) { continue; }
                if ui.add(filter_chip(p, self.filter_tag.as_deref() == Some(tag.as_str()),
                    &format!("{} {}", tag, count))).clicked() {
                    self.filter_tag = Some(tag);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let create = egui::Button::new(RichText::new("＋ 新建笔记").color(p.on_accent).size(12.5))
                    .fill(p.accent).corner_radius(CornerRadius::same(8))
                    .min_size(Vec2::new(0.0, 30.0));
                if ui.add(create).clicked() {
                    match model::create_note(paths, "新笔记", settings) {
                        Ok(meta) => {
                            let _ = crate::start_tile(&meta.id);
                            self.refresh(paths);
                            self.message = Some("已新建笔记磁贴".to_owned());
                        }
                        Err(e) => self.message = Some(format!("创建失败：{e:#}")),
                    }
                }
                if self.view == 0 && ui.button(RichText::new("⊞ 网格整理").size(12.0)).clicked() {
                    self.rearrange_all(paths);
                }
            });
        });
        ui.add_space(10.0);

        if let Some(msg) = self.message.clone() {
            ui.horizontal(|ui| {
                components::status_dot(ui, p.accent, 3.5);
                ui.add_space(4.0);
                ui.label(RichText::new(&msg).size(11.5).color(p.dim));
                if ui.small_button("×").clicked() { self.message = None; }
            });
            ui.add_space(6.0);
        }

        if self.notes.is_empty() {
            empty_state(ui, p);
        } else if self.view == 0 {
            self.show_map(ui, p, paths, settings);
        } else {
            self.show_list(ui, p, paths, settings);
        }
        self.confirm_delete(ui.ctx().clone(), p, paths);
    }

    fn show_map(&mut self, ui: &mut egui::Ui, p: Palette, paths: &Paths, settings: &Settings) {
        let visible: Vec<usize> = self.notes.iter().enumerate()
            .filter(|(_, n)| self.matches(n))
            .map(|(i, _)| i)
            .collect();
        let mut canvas = MAP_ORIGIN * 2.0;
        for i in &visible {
            let n = &self.notes[*i];
            canvas.x = canvas.x.max(n.map_x + TILE_SIZE.x + MAP_ORIGIN.x);
            canvas.y = canvas.y.max(n.map_y + TILE_SIZE.y + MAP_ORIGIN.y);
        }
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            ui.set_min_size(canvas);
            let clip = ui.clip_rect();
            // Dot grid over the visible part of the canvas.
            {
                let painter = ui.painter();
                let start_x = (clip.left() / GRID_STEP).floor() * GRID_STEP;
                let start_y = (clip.top() / GRID_STEP).floor() * GRID_STEP;
                let mut x = start_x;
                while x < clip.right() {
                    let mut y = start_y;
                    while y < clip.bottom() {
                        painter.circle_filled(pos2(x, y), 1.0, p.grid_dot());
                        y += GRID_STEP;
                    }
                    x += GRID_STEP;
                }
            }
            for i in visible {
                let id = self.notes[i].id.clone();
                let selected = self.selected.as_deref() == Some(id.as_str());
                let rect = egui::Rect::from_min_size(pos2(self.notes[i].map_x, self.notes[i].map_y), TILE_SIZE);
                let response = ui.allocate_rect(rect, Sense::click_and_drag());
                let painter = ui.painter();
                // Card visuals.
                painter.rect_filled(rect, CornerRadius::same(10), p.panel);
                let border = if selected { p.accent } else if response.hovered() { p.border } else { p.border_soft() };
                painter.rect_stroke(rect, CornerRadius::same(10), Stroke::new(if selected { 1.5 } else { 1.0 }, border), egui::StrokeKind::Inside);
                painter.rect_filled(egui::Rect::from_min_max(rect.min, pos2(rect.min.x + 3.0, rect.max.y)),
                    CornerRadius { nw: 10, ne: 0, sw: 10, se: 0 },
                    palette::accents(settings.theme)[self.notes[i].accent % 6].gamma_multiply(0.85));
                let title = self.notes[i].title.clone();
                painter.text(pos2(rect.min.x + 12.0, rect.min.y + 17.0), Align2::LEFT_CENTER,
                    truncate(&title, 14), FontId::proportional(12.0), p.strong);
                let body = self.bodies.get(&id).cloned().unwrap_or_default();
                let preview = preview_lines(&body, 2, 26);
                for (line, dy) in preview.iter().zip([38.0, 52.0]) {
                    painter.text(pos2(rect.min.x + 12.0, rect.min.y + dy), Align2::LEFT_CENTER,
                        line, FontId::proportional(10.0), p.dim);
                }
                painter.text(pos2(rect.max.x - 10.0, rect.max.y - 12.0), Align2::RIGHT_CENTER,
                    format!("{} · {} 字", self.notes[i].tag, body.chars().filter(|c| !c.is_whitespace()).count()),
                    FontId::monospace(9.0), p.faint);

                if response.dragged() {
                    let delta = response.drag_delta();
                    self.notes[i].map_x = (self.notes[i].map_x + delta.x).max(0.0);
                    self.notes[i].map_y = (self.notes[i].map_y + delta.y).max(0.0);
                }
                if response.drag_stopped() {
                    let _ = self.notes[i].save(paths);
                }
                if response.clicked() {
                    self.selected = Some(id.clone());
                }
                if response.double_clicked() {
                    let _ = crate::start_tile(&id);
                }
                response.on_hover_cursor(egui::CursorIcon::Grab);
            }
        });
    }

    fn show_list(&mut self, ui: &mut egui::Ui, p: Palette, paths: &Paths, settings: &Settings) {
        let selected_id = self.selected.clone();
        egui::SidePanel::left("lib_tags")
            .exact_width(150.0)
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(4, 0)))
            .show_inside(ui, |ui| {
                let total = self.notes.len();
                if ui.add(nav_item(p, self.filter_tag.is_none(), "全部笔记", total, "⊞")).clicked() {
                    self.filter_tag = None;
                }
                let mut tags: Vec<String> = self.notes.iter().map(|n| n.tag.clone()).collect();
                for c in &settings.categories {
                    if !tags.contains(c) { tags.push(c.clone()); }
                }
                for tag in tags {
                    let count = self.notes.iter().filter(|n| n.tag == tag).count();
                    if count == 0 { continue; }
                    if ui.add(nav_item(p, self.filter_tag.as_deref() == Some(tag.as_str()), &tag, count, "▌")).clicked() {
                        self.filter_tag = Some(tag);
                    }
                }
            });
        egui::SidePanel::right("lib_detail")
            .exact_width(330.0)
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(6, 0)))
            .show_inside(ui, |ui| {
                self.detail(ui, p, paths);
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    let width = ui.available_width();
                    for i in 0..self.notes.len() {
                        if !self.matches(&self.notes[i]) { continue; }
                        let note = self.notes[i].clone();
                        let selected = selected_id.as_deref() == Some(note.id.as_str());
                        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 66.0), Sense::click());
                        let painter = ui.painter();
                        let bg = if selected { p.raise } else if response.hovered() { p.hover() } else { Color32::TRANSPARENT };
                        if bg != Color32::TRANSPARENT {
                            painter.rect_filled(rect, CornerRadius::same(8), bg);
                        }
                        if selected {
                            painter.rect_filled(
                                egui::Rect::from_min_max(pos2(rect.left(), rect.top() + 10.0), pos2(rect.left() + 3.0, rect.bottom() - 10.0)),
                                1.5, p.accent);
                        }
                        let dot_c = pos2(rect.left() + 16.0, rect.min.y + 20.0);
                        painter.circle_filled(dot_c, 4.0, palette::accents(settings.theme)[note.accent % 6]);
                        painter.text(pos2(rect.left() + 28.0, rect.min.y + 20.0), Align2::LEFT_CENTER,
                            truncate(&note.title, 18), FontId::proportional(13.0), p.strong);
                        painter.text(pos2(rect.right() - 12.0, rect.min.y + 20.0), Align2::RIGHT_CENTER,
                            model::relative_time(note.updated_at), FontId::monospace(9.5), p.faint);
                        let body = self.bodies.get(&note.id).cloned().unwrap_or_default();
                        for (line, dy) in preview_lines(&body, 2, 30).iter().zip([42.0, 56.0]) {
                            painter.text(pos2(rect.left() + 28.0, rect.min.y + dy), Align2::LEFT_CENTER,
                                line, FontId::proportional(10.5), p.dim);
                        }
                        if response.clicked() { self.selected = Some(note.id.clone()); }
                        if response.double_clicked() { let _ = crate::start_tile(&note.id); }
                    }
                });
            });
    }

    fn detail(&mut self, ui: &mut egui::Ui, p: Palette, paths: &Paths) {
        let Some(id) = self.selected.clone() else {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("选择一篇笔记查看详情").size(12.0).color(p.faint));
            });
            return;
        };
        let Some(note) = self.notes.iter().find(|n| n.id == id).cloned() else { return };
        let body = self.bodies.get(&id).cloned().unwrap_or_default();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(6.0);
            ui.label(RichText::new(truncate(&note.title, 24)).size(16.0).strong().color(p.strong));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                components::chip(ui, p, Some(p.accent), &note.tag);
                ui.label(RichText::new(format!("{} · {} 字 · {}",
                    model::relative_time(note.updated_at),
                    body.chars().filter(|c| !c.is_whitespace()).count(),
                    note.id)).monospace().size(9.5).color(p.faint));
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let open = egui::Button::new(RichText::new("打开磁贴").color(p.on_accent).size(12.0))
                    .fill(p.accent).corner_radius(CornerRadius::same(8)).min_size(Vec2::new(96.0, 28.0));
                if ui.add(open).clicked() {
                    let _ = crate::start_tile(&id);
                }
                if ui.button(RichText::new("删除").size(12.0).color(p.danger)).clicked() {
                    self.pending_delete = Some(id.clone());
                }
            });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);
            CommonMarkViewer::new().show(ui, &mut self.markdown_cache, &body);
        });
    }

    fn confirm_delete(&mut self, ctx: egui::Context, p: Palette, paths: &Paths) {
        let Some(id) = self.pending_delete.clone() else { return };
        egui::Window::new("删除笔记？")
            .collapsible(false).resizable(false).title_bar(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(egui::Frame::default().fill(p.panel)
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(18))
                .stroke(Stroke::new(1.0, p.border_soft())))
            .show(&ctx, |ui| {
                ui.set_width(260.0);
                ui.label(RichText::new("永久删除这篇笔记？").size(14.0).strong().color(p.strong));
                ui.add_space(4.0);
                ui.label(RichText::new("Markdown 正文与元数据都会被移除，且无法恢复。").size(11.0).color(p.dim));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() { self.pending_delete = None; }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let del = egui::Button::new(RichText::new("删除").color(Color32::WHITE).size(12.0))
                            .fill(p.danger).corner_radius(CornerRadius::same(8));
                        if ui.add(del).clicked() {
                            match hyprland::find_tile(&id) {
                                Ok(Some(_)) => self.message = Some("该磁贴仍在运行，请先关闭它".to_owned()),
                                _ => match model::delete_note(paths, &id) {
                                    Ok(()) => {
                                        if self.selected.as_deref() == Some(id.as_str()) { self.selected = None; }
                                        self.message = Some("笔记已删除".to_owned());
                                    }
                                    Err(e) => self.message = Some(format!("删除失败：{e:#}")),
                                },
                            }
                            self.pending_delete = None;
                            self.refresh(paths);
                        }
                    });
                });
            });
    }
}

/// Grid cell index of a map coordinate.
fn cell_of(x: f32, y: f32) -> (usize, usize) {
    let col = (((x - MAP_ORIGIN.x) / MAP_CELL.x).round() as i64).clamp(0, MAP_COLS as i64 - 1) as usize;
    let row = (((y - MAP_ORIGIN.y) / MAP_CELL.y).round() as i64).max(0) as usize;
    (col, row)
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars { out.push('…'); }
    out
}

/// Up to `lines` preview lines of `per_line` chars each, skipping markdown noise.
fn preview_lines(body: &str, lines: usize, per_line: usize) -> Vec<String> {
    body.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#') && !t.starts_with("---")
        })
        .take(lines)
        .map(|l| truncate(l.trim(), per_line))
        .collect()
}

fn filter_chip(p: Palette, active: bool, label: &str) -> egui::Button<'static> {
    let text = RichText::new(label.to_owned()).size(11.0)
        .color(if active { p.accent } else { p.dim });
    egui::Button::new(text)
        .fill(if active { p.accent.gamma_multiply(0.12) } else { Color32::TRANSPARENT })
        .stroke(egui::Stroke::new(1.0, if active { p.accent.gamma_multiply(0.45) } else { p.border_soft() }))
        .corner_radius(CornerRadius::same(6))
}

fn nav_item(p: Palette, active: bool, label: &str, count: usize, glyph: &str) -> egui::Button<'static> {
    let text = format!("{glyph} {label}  {count}");
    egui::Button::new(RichText::new(text).size(12.0).color(if active { p.strong } else { p.dim }))
        .fill(if active { p.accent.gamma_multiply(0.12) } else { Color32::TRANSPARENT })
        .corner_radius(CornerRadius::same(8))
        .min_size(Vec2::new(0.0, 32.0))
}

fn empty_state(ui: &mut egui::Ui, p: Palette) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("⊞").size(34.0).color(p.faint));
        ui.add_space(6.0);
        ui.label(RichText::new("还没有笔记").size(14.0).color(p.strong));
        ui.label(RichText::new("点击右上角「＋ 新建笔记」开始记录灵感。").size(11.5).color(p.dim));
    });
}
