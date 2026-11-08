//! Small pieces of interface the panel is made of.

use egui::{Align, Color32, CornerRadius, Layout, Response, Sense, Stroke, Ui};

/// A group of related controls on a slightly raised surface.
pub fn card(ui: &mut Ui, content: impl FnOnce(&mut Ui)) {
    let visuals = ui.visuals();
    let frame = egui::Frame::new()
        .fill(visuals.faint_bg_color)
        .corner_radius(visuals.widgets.inactive.corner_radius)
        .inner_margin(14);
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        content(ui);
    });
}

/// A card's first line: its title on the left and whatever `right` draws on the right.
pub fn header(ui: &mut Ui, title: &str, right: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.strong(title);
        ui.with_layout(Layout::right_to_left(Align::Center), right);
    });
}

/// An on/off switch that slides, in the accent color when on.
pub fn switch(ui: &mut Ui, on: &mut bool) -> Response {
    let size = egui::vec2(38.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, ""));
    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let track = ui
            .visuals()
            .widgets
            .inactive
            .bg_fill
            .lerp_to_gamma(ui.visuals().selection.bg_fill, how_on);
        let radius = rect.height() / 2.0;
        ui.painter().rect_filled(rect, CornerRadius::same(radius as u8), track);
        let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let knob = if ui.is_enabled() {
            Color32::WHITE
        } else {
            visuals.fg_stroke.color.gamma_multiply(0.6)
        };
        ui.painter()
            .circle_filled(egui::pos2(knob_x, rect.center().y), radius - 3.0, knob);
    }
    response
}

/// A round color button; `selected` rings it.
pub fn swatch(ui: &mut Ui, color: Color32, selected: bool) -> Response {
    let diameter = 28.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(diameter, diameter), Sense::click());
    if ui.is_rect_visible(rect) {
        let enabled = ui.is_enabled();
        let fill = if enabled { color } else { color.gamma_multiply(0.35) };
        let grow = if response.hovered() && enabled { 1.5 } else { 0.0 };
        ui.painter()
            .circle_filled(rect.center(), diameter / 2.0 - 3.0 + grow, fill);
        if selected && enabled {
            let ink = ui.visuals().strong_text_color();
            ui.painter()
                .circle_stroke(rect.center(), diameter / 2.0 - 0.5, Stroke::new(2.0_f32, ink));
        }
    }
    response
}

/// A small filled dot, for a status.
pub fn dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}
