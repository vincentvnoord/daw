use eframe::egui::{
    self, CornerRadius, CursorIcon, Margin, Popup, RichText, Sense, Stroke, StrokeKind,
};

use crate::{
    audio::studio::studio::MAX_INSTRUMENTS,
    shared::AppParams,
    ui::theme,
};

/// Width of the instrument list next to the editor.
pub const LIST_WIDTH: f32 = 200.0;
/// Height of a card's contents: the name and the details line under it.
const CARD_CONTENT_HEIGHT: f32 = 34.0;

/// What the user did in the instrument list.
pub enum InstrumentAction {
    Select(usize),
    AddSynth,
    AddSample,
}

/// Lists the instruments, with their type and which channel each one plays
/// into. Clicking one selects it: the keyboard plays it and the editor shows
/// it. A last card with a + adds an instrument.
pub fn show(ui: &mut egui::Ui, params: &AppParams, active: usize) -> Option<InstrumentAction> {
    let mut action = None;

    ui.vertical(|ui| {
        ui.set_width(LIST_WIDTH);
        ui.label(
            RichText::new("INSTRUMENTS")
                .font(theme::label_font())
                .color(theme::TEXT_DIM)
                .strong(),
        );
        ui.add_space(2.0);

        for (i, instrument) in params.instruments.iter().enumerate() {
            let is_active = i == active;
            let output = instrument
                .params
                .get_channel()
                .and_then(|id| params.channel(id))
                .map_or("no output", |c| c.name.as_str());

            let click = card(ui, ("instrument", i), is_active, |ui| {
                ui.label(RichText::new(&instrument.name).strong());
                let details = match instrument.kind_label() {
                    Some(kind) => format!("{kind} · → {output}"),
                    None => format!("→ {output}"),
                };
                ui.label(
                    RichText::new(details)
                        .font(theme::value_font())
                        .color(theme::TEXT_DIM),
                );
            });
            if click.clicked() && !is_active {
                action = Some(InstrumentAction::Select(i));
            }
        }

        if params.instruments.len() < MAX_INSTRUMENTS {
            let add = card(ui, "add_instrument", false, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("+").size(20.0).color(theme::TEXT_DIM));
                });
            })
            .on_hover_text("Add instrument");

            let menu_frame = egui::Frame::new()
                .fill(theme::PANEL)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .corner_radius(CornerRadius::same(theme::RADIUS))
                .inner_margin(Margin::same(4));

            Popup::menu(&add)
                .width(add.rect.width())
                .gap(4.0)
                .frame(menu_frame)
                .show(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    if menu_option(ui, "Synth", "Oscillator, filter and envelope") {
                        action = Some(InstrumentAction::AddSynth);
                        ui.close();
                    }
                    if menu_option(ui, "Sample", "Play a WAV file from disk") {
                        action = Some(InstrumentAction::AddSample);
                        ui.close();
                    }
                });
        }
    });

    action
}

/// A full-width option in the add menu: a name with a short description
/// under it. Returns `true` when clicked.
fn menu_option(ui: &mut egui::Ui, title: &str, description: &str) -> bool {
    const PADDING: egui::Vec2 = egui::vec2(10.0, 8.0);
    const LINE_GAP: f32 = 4.0;

    let width = ui.available_width();
    let text_width = width - 2.0 * PADDING.x;
    let layout = |text: RichText, wrap| {
        egui::WidgetText::from(text).into_galley(ui, Some(wrap), text_width, egui::TextStyle::Body)
    };
    let title = layout(RichText::new(title).size(13.0), egui::TextWrapMode::Truncate);
    // the description wraps onto more lines and the row grows with it
    let description = layout(RichText::new(description).size(10.5), egui::TextWrapMode::Wrap);

    let height = 2.0 * PADDING.y + title.size().y + LINE_GAP + description.size().y;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(rect, CornerRadius::same(theme::RADIUS - 2), theme::SURFACE_HI);
        }

        let title_pos = rect.left_top() + PADDING;
        let description_pos = title_pos + egui::vec2(0.0, title.size().y + LINE_GAP);
        painter.galley(title_pos, title, theme::TEXT);
        painter.galley(description_pos, description, theme::TEXT_DIM);
    }

    response.clicked()
}

/// A clickable card in the list. The selected one is lighter; any card gets
/// an outline while the pointer is over it.
fn card(
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
        ui.painter()
            .rect_stroke(rect, radius, Stroke::new(1.0, theme::ACCENT), StrokeKind::Inside);
    }

    response
}
