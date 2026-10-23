use eframe::egui::{self, Color32, CornerRadius, FontId, Margin, Stroke, TextStyle};

// Palette
pub const BG: Color32 = Color32::from_rgb(16, 16, 19);
pub const PANEL: Color32 = Color32::from_rgb(23, 23, 27);
pub const SURFACE: Color32 = Color32::from_rgb(31, 31, 37);
pub const SURFACE_HI: Color32 = Color32::from_rgb(44, 44, 52);
pub const BORDER: Color32 = Color32::from_rgb(52, 52, 60);
pub const TEXT: Color32 = Color32::from_rgb(226, 226, 232);
pub const TEXT_DIM: Color32 = Color32::from_rgb(134, 134, 146);
pub const ACCENT: Color32 = Color32::from_rgb(232, 98, 52);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(120, 56, 34);

// Shape
pub const RADIUS: u8 = 6;
pub const CARD_PADDING: i8 = 12;
pub const GAP: f32 = 10.0;

pub fn apply(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        let v = &mut style.visuals;
        *v = egui::Visuals::dark();
        v.panel_fill = BG;
        v.window_fill = PANEL;
        v.window_stroke = Stroke::new(1.0, BORDER);
        v.window_corner_radius = CornerRadius::same(RADIUS);
        v.extreme_bg_color = BG;
        v.faint_bg_color = SURFACE;
        v.override_text_color = Some(TEXT);
        v.selection.bg_fill = ACCENT;
        v.selection.stroke = Stroke::new(1.0, TEXT);
        v.slider_trailing_fill = true;

        let w = &mut v.widgets;
        for (state, fill) in [
            (&mut w.noninteractive, PANEL),
            (&mut w.inactive, SURFACE),
            (&mut w.hovered, SURFACE_HI),
            (&mut w.active, SURFACE_HI),
            (&mut w.open, SURFACE_HI),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.corner_radius = CornerRadius::same(RADIUS);
        }
        w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        w.hovered.bg_stroke = Stroke::new(1.0, ACCENT_DIM);
        w.active.bg_stroke = Stroke::new(1.0, ACCENT);

        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);

        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(18.0));
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(13.0));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(10.5));
    });
}

/// Frame for a top-level panel (header bar, keyboard dock, ...).
pub fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .inner_margin(Margin::symmetric(16, 10))
        .stroke(Stroke::new(1.0, BORDER))
}

/// Frame for a card that groups related controls.
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(RADIUS))
        .inner_margin(Margin::same(CARD_PADDING))
}

pub fn label_font() -> FontId {
    FontId::proportional(11.0)
}

pub fn value_font() -> FontId {
    FontId::monospace(11.0)
}
