use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle,
};

// Palette: black and white. The only colour is red, kept for clipping.
pub const BG: Color32 = Color32::from_gray(10);
pub const PANEL: Color32 = Color32::from_gray(17);
pub const SURFACE: Color32 = Color32::from_gray(24);
pub const SURFACE_HI: Color32 = Color32::from_gray(38);
pub const BORDER: Color32 = Color32::from_gray(44);
pub const TEXT: Color32 = Color32::from_gray(237);
pub const TEXT_DIM: Color32 = Color32::from_gray(138);
pub const ACCENT: Color32 = Color32::from_gray(250);
pub const ACCENT_DIM: Color32 = Color32::from_gray(90);
/// A held piano key; the accent is white, which wouldn't show on a white key.
pub const PRESSED: Color32 = Color32::from_gray(150);
pub const METER: Color32 = Color32::from_gray(212);
pub const CLIP: Color32 = Color32::from_rgb(228, 64, 64);

// Shape
pub const RADIUS: u8 = 6;
pub const CARD_PADDING: i8 = 12;
pub const GAP: f32 = 10.0;

/// Geist Medium, for headings and labels.
const MEDIUM: &str = "medium";

pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);

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
        // text on a selected item sits on the white accent, so it has to be dark
        v.selection.stroke = Stroke::new(1.0, BG);
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
            .insert(TextStyle::Heading, FontId::new(18.0, medium()));
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
    FontId::new(11.0, medium())
}

fn medium() -> FontFamily {
    FontFamily::Name(MEDIUM.into())
}

/// Puts Geist in front of egui's own fonts. Those stay behind it as a
/// fallback for symbols Geist doesn't have.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    for (name, bytes) in [
        (
            "geist",
            &include_bytes!("../../assets/fonts/Geist-Regular.ttf")[..],
        ),
        (
            MEDIUM,
            &include_bytes!("../../assets/fonts/Geist-Medium.ttf")[..],
        ),
        (
            "geist-mono",
            &include_bytes!("../../assets/fonts/GeistMono-Regular.ttf")[..],
        ),
    ] {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }

    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "geist".to_owned());
    let fallback = proportional.clone();

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "geist-mono".to_owned());

    let mut medium_family = vec![MEDIUM.to_owned()];
    medium_family.extend(fallback);
    fonts.families.insert(medium(), medium_family);

    ctx.set_fonts(fonts);
}

pub fn value_font() -> FontId {
    FontId::monospace(11.0)
}
