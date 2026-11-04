//! Reusable egui widgets matching the One Dark design system:
//! group cards, setting rows, custom toggles, segmented controls, swatches,
//! keycaps, chips and small icon buttons used by the settings window,
//! the note library and the note tiles.
use crate::model::Density;
use crate::ui::palette::Palette;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, RichText, Sense, Stroke, Ui, Vec2,
    pos2,
};

/// Truncate a string to at most `max_chars` characters, appending an ellipsis.
pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars { out.push('…'); }
    out
}

/// Mono kicker label: 3px accent bar + small uppercase tracking text.
pub fn kicker(ui: &mut Ui, p: Palette, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(3.0, 11.0), Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::same(2), p.accent);
        ui.add_space(6.0);
        ui.label(RichText::new(text.to_uppercase()).monospace().size(9.5).color(p.faint));
    });
}

/// Section header: kicker + 22px display title + dim description.
pub fn section_head(ui: &mut Ui, p: Palette, kick: &str, title: &str, desc: &str) {
    kicker(ui, p, kick);
    ui.add_space(4.0);
    ui.label(RichText::new(title).size(21.0).strong().color(p.strong));
    if !desc.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(desc).size(12.0).color(p.dim));
    }
    ui.add_space(16.0);
}

/// Card group with a chrome header (icon tile + title + description) and a
/// body area; mirrors the `Group` component of the design mockup.
pub fn group(ui: &mut Ui, p: Palette, icon: &str, title: &str, desc: &str, body: impl FnOnce(&mut Ui)) {
    egui::Frame::default()
        .fill(p.panel)
        .corner_radius(CornerRadius::same(12))
        .stroke(Stroke::new(1.0, p.border_soft()))
        .inner_margin(0.0)
        .outer_margin(egui::Margin { bottom: 20, ..Default::default() })
        .show(ui, |ui| {
            let width = ui.available_width();
            let header_h = if desc.is_empty() { 40.0 } else { 48.0 };
            let (header, _) = ui.allocate_exact_size(Vec2::new(width, header_h), Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(
                header.shrink(0.5),
                CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 },
                p.chrome,
            );
            painter.hline(header.left() + 1.0..=header.right() - 1.0, header.bottom() - 0.5, Stroke::new(1.0, p.border_soft()));
            let icon_rect = egui::Rect::from_min_size(
                pos2(header.left() + 14.0, header.center().y - 14.0),
                Vec2::splat(28.0),
            );
            painter.rect_filled(icon_rect, CornerRadius::same(8), p.accent.gamma_multiply(0.15));
            painter.text(icon_rect.center(), Align2::CENTER_CENTER, icon,
                FontId::proportional(13.0), p.accent);
            let text_x = icon_rect.right() + 10.0;
            let max_chars = (((header.right() - text_x - 18.0) / 10.5).max(4.0)) as usize;
            painter.text(pos2(text_x, header.min.y + header_h * 0.30), Align2::LEFT_CENTER,
                truncate_chars(title, max_chars), FontId::proportional(13.0), p.strong);
            if !desc.is_empty() {
                painter.text(pos2(text_x, header.min.y + header_h * 0.72), Align2::LEFT_CENTER,
                    truncate_chars(desc, max_chars), FontId::proportional(10.5), p.dim);
            }
            ui.add_space(6.0);
            body(ui);
            ui.add_space(10.0);
        });
}

/// One label/description + right-aligned control row inside a group card.
/// `flash` highlights the row (search jump); density controls row breathing room.
pub fn setting_row(
    ui: &mut Ui,
    p: Palette,
    density: Density,
    label: &str,
    desc: &str,
    flash: bool,
    body: impl FnOnce(&mut Ui) -> bool,
) {
    let pad = match density {
        Density::Compact => 5.0,
        Density::Cozy => 9.0,
        Density::Roomy => 14.0,
    };
    let width = ui.available_width();
    let top_left = ui.cursor().min;
    // Narrow windows: stack the control below the label instead of letting a
    // right-to-left control row collide with the label column.
    let stacked = width < 480.0;
    ui.add_space(pad);
    if stacked {
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            ui.vertical(|ui| {
                ui.set_max_width((width - 36.0).max(80.0));
                ui.label(RichText::new(label).size(13.0).color(p.strong));
                if !desc.is_empty() {
                    ui.label(RichText::new(desc).size(11.0).color(p.dim));
                }
            });
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            body(ui);
        });
    } else {
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            ui.vertical(|ui| {
                ui.set_min_width((width * 0.42).clamp(140.0, 300.0));
                ui.set_max_width((width * 0.50).max(160.0));
                ui.label(RichText::new(label).size(13.0).color(p.strong));
                if !desc.is_empty() {
                    ui.label(RichText::new(desc).size(11.0).color(p.dim));
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(16.0);
                body(ui);
            });
        });
    }
    ui.add_space(pad);
    let rect = egui::Rect::from_min_max(top_left, pos2(ui.cursor().min.x + width, ui.cursor().min.y - 2.0));
    let painter = ui.painter();
    if flash {
        painter.rect_filled(rect, 4.0, p.accent.gamma_multiply(0.10));
        painter.rect_stroke(rect, 4.0, Stroke::new(1.2, p.accent.gamma_multiply(0.55)), egui::StrokeKind::Inside);
    }
    painter.hline(rect.left() + 14.0..=rect.right() - 14.0, rect.bottom() - 1.0, Stroke::new(1.0, p.border_soft()));
}

/// Custom pill toggle switch (42x22) with animated knob; replaces egui's
/// checkbox look on setting rows.
pub fn toggle_switch(ui: &mut Ui, p: Palette, checked: &mut bool) -> egui::Response {
    let desired = ui.available_size();
    let (rect, mut response) = ui.allocate_exact_size(
        Vec2::new(42.0, 22.0).min(desired),
        Sense::click(),
    );
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    let t = ui.ctx().animate_value_with_time(response.id, if *checked { 1.0 } else { 0.0 }, 0.18);
    let painter = ui.painter();
    let track = if t > 0.5 { p.accent } else { p.border };
    painter.rect_filled(rect, CornerRadius::same(11), track);
    let knob_r = 8.0;
    let x = egui::lerp((rect.left() + 3.0 + knob_r)..=(rect.right() - 3.0 - knob_r), t);
    painter.circle_filled(pos2(x, rect.center().y), knob_r, if p.is_light { Color32::WHITE } else { Color32::from_rgb(0xE6, 0xE8, 0xEC) });
    if !*checked && !p.is_light {
        // inset shadow hint on the off-state, like the mockup
        painter.rect_stroke(rect, CornerRadius::same(11), Stroke::new(1.0, Color32::from_black_alpha(90)), egui::StrokeKind::Inside);
    }
    response
}

/// Segmented control in a sunken track with a raised pill on the active item.
pub fn segmented(ui: &mut Ui, p: Palette, options: &[&str], selected: &mut usize) -> bool {
    let mut changed = false;
    egui::Frame::default()
        .fill(p.sunken)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(3))
        .stroke(Stroke::new(1.0, p.border_soft()))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.set_min_height(26.0);
                for (i, opt) in options.iter().enumerate() {
                    let active = *selected == i;
                    let btn = egui::Button::new(
                        RichText::new(*opt).size(12.0).color(if active { p.strong } else { p.dim }),
                    )
                    .fill(if active { p.raise } else { Color32::TRANSPARENT })
                    .corner_radius(CornerRadius::same(6))
                    .min_size(Vec2::new(28.0, 26.0));
                    if ui.add(btn).clicked() && !active {
                        *selected = i;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// Round color swatches with a double selection ring.
pub fn swatches(ui: &mut Ui, p: Palette, colors: &[Color32], selected: &mut usize, size: f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for (i, c) in colors.iter().enumerate() {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(size + 4.0), Sense::click());
            let painter = ui.painter();
            let center = rect.center();
            let r = size / 2.0;
            if *selected == i {
                painter.circle_stroke(center, r + 3.0, Stroke::new(1.5, *c));
                painter.circle_stroke(center, r + 1.2, Stroke::new(1.5, p.panel));
            } else {
                painter.circle_stroke(center, r, Stroke::new(1.0, Color32::from_black_alpha(60)));
            }
            painter.circle_filled(center, if *selected == i { r - 1.0 } else { r }, *c);
            if response.clicked() {
                *selected = i;
                changed = true;
            }
        }
    });
    changed
}

/// Keyboard key chip.
pub fn keycap(ui: &mut Ui, p: Palette, text: &str) {
    egui::Frame::default()
        .fill(p.raise)
        .corner_radius(CornerRadius::same(5))
        .inner_margin(egui::Margin::symmetric(7, 3))
        .stroke(Stroke::new(1.0, p.border))
        .show(ui, |ui| {
            ui.label(RichText::new(text).monospace().size(10.5).color(p.strong));
        });
}

/// Small colored (or neutral) tag chip.
pub fn chip(ui: &mut Ui, p: Palette, color: Option<Color32>, text: &str) {
    let c = color.unwrap_or(p.dim);
    egui::Frame::default()
        .fill(c.gamma_multiply(0.13))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .stroke(Stroke::new(1.0, c.gamma_multiply(0.42)))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(c));
        });
}

/// Tiny solid status dot (save state, theme indicator...).
pub fn status_dot(ui: &mut Ui, color: Color32, radius: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(radius * 2.0 + 2.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), radius, color);
}

/// 22px square glyph tool button for tile title bars.
pub fn tool_button(ui: &mut Ui, p: Palette, glyph: &str, active: bool, tooltip: &str) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
    let painter = ui.painter();
    let bg = if active {
        p.accent.gamma_multiply(0.16)
    } else if response.hovered() {
        p.hover()
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        painter.rect_filled(rect.shrink(1.0), CornerRadius::same(5), bg);
    }
    painter.text(rect.center(), Align2::CENTER_CENTER, glyph,
        FontId::proportional(11.5), if active { p.accent } else { p.dim });
    response.on_hover_text(tooltip)
}

/// Generic radio card (theme / size presets). `width`x`height` allocates the
/// card; the `draw` closure paints custom preview content inside.
pub fn radio_card(
    ui: &mut Ui,
    p: Palette,
    size: Vec2,
    selected: bool,
    draw: impl FnOnce(&mut Ui),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(12), if selected { p.raise } else { p.sunken });
    painter.rect_stroke(
        rect,
        CornerRadius::same(12),
        if selected { Stroke::new(1.5, p.accent) } else { Stroke::new(1.0, p.border_soft()) },
        egui::StrokeKind::Inside,
    );
    ui.allocate_ui_at_rect(rect.shrink(10.0), draw);
    response
}
