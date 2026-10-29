use eframe::egui::{self, Popup, RichText};

use crate::{
    audio::studio::studio::MAX_INSTRUMENTS,
    shared::AppParams,
    ui::{
        theme,
        widgets::list::{add_card, list_card, menu_frame, menu_option, section_label},
    },
};

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

    section_label(ui, "INSTRUMENTS");

    for (i, instrument) in params.instruments.iter().enumerate() {
        let is_active = i == active;
        let output = instrument
            .params
            .route
            .get_channel()
            .and_then(|id| params.channel(id))
            .map_or("no output", |c| c.name.as_str());

        let click = list_card(ui, ("instrument", i), is_active, |ui| {
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
        let add = add_card(ui, "add_instrument").on_hover_text("Add instrument");

        Popup::menu(&add)
            .width(add.rect.width())
            .gap(4.0)
            .frame(menu_frame())
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

    action
}
