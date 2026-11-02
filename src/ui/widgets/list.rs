//! The pieces of the track list: clickable cards, the add menu
//! under the + card, and the section title above each list.

use eframe::egui::{self, CornerRadius, CursorIcon, Margin, RichText, Sense, Stroke, StrokeKind};

use crate::ui::theme;

/// Height of a card's contents: the name and the details line under it.
const CARD_CONTENT_HEIGHT: f32 = 34.0;

/// The small caps title above a list.
pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme::label_font())
            .color(theme::TEXT_DIM)
            .strong(),
    );
    ui.add_space(2.0);
}

/// A clickable card in the list. The selected one is lighter; any card gets
/// an outline while the pointer is over it.
pub fn list_card(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    selected: bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let radius = CornerRadius::same(theme::RADIUS);
    let rect = egui::Frame::new()
        .fill(if selected {
            theme::SURFACE_HI
        } else {
            theme::SURFACE
        })
        .corner_radius(radius)
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(CARD_CONTENT_HEIGHT);
            add_contents(ui);
        })
        .response
        .rect;

    let response = ui
        .interact(rect, ui.scope_id().with(id), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);

    // drawn on top afterwards, so hovering doesn't shift the layout
    if response.hovered() {
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, theme::ACCENT),
            StrokeKind::Inside,
        );
    }

    response
}

/// The last card in a list, with a + that adds something.
pub fn add_card(ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug) -> egui::Response {
    list_card(ui, id, false, |ui| {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("+").size(20.0).color(theme::TEXT_DIM));
        });
    })
}

/// Frame for the menu that drops down from an add card.
pub fn menu_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(theme::RADIUS))
        .inner_margin(Margin::same(4))
}

/// A full-width option in the add menu: a name with a short description
/// under it, if it has one. Returns `true` when clicked.
pub fn menu_option(ui: &mut egui::Ui, title: &str, description: &str) -> bool {
    const PADDING: egui::Vec2 = egui::vec2(10.0, 8.0);
    const LINE_GAP: f32 = 4.0;

    let width = ui.available_width();
    let text_width = width - 2.0 * PADDING.x;
    let layout = |text: RichText, wrap| {
        egui::WidgetText::from(text).into_galley(ui, Some(wrap), text_width, egui::TextStyle::Body)
    };
    let title = layout(
        RichText::new(title).size(13.0),
        egui::TextWrapMode::Truncate,
    );
    // the description wraps onto more lines and the row grows with it
    let description = layout(
        RichText::new(description).size(10.5),
        egui::TextWrapMode::Wrap,
    );

    let description_height = if description.is_empty() {
        0.0
    } else {
        LINE_GAP + description.size().y
    };
    let height = 2.0 * PADDING.y + title.size().y + description_height;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(
                rect,
                CornerRadius::same(theme::RADIUS - 2),
                theme::SURFACE_HI,
            );
        }

        let title_pos = rect.left_top() + PADDING;
        let description_pos = title_pos + egui::vec2(0.0, title.size().y + LINE_GAP);
        painter.galley(title_pos, title, theme::TEXT);
        painter.galley(description_pos, description, theme::TEXT_DIM);
    }

    response.clicked()
}
