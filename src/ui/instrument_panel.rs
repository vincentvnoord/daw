use eframe::egui::{self, RichText};

use crate::{
    shared::AppParams,
    ui::{theme, widgets::card::card},
};

/// Lists the instruments and which channel each one plays into. Clicking one
/// makes it the instrument the keyboard plays.
///
/// Returns the clicked instrument's index if it isn't the active one already.
pub fn show(ui: &mut egui::Ui, params: &AppParams, active: usize) -> Option<usize> {
    let mut selected = None;

    card(ui, "Instruments", |ui| {
        for (i, instrument) in params.instruments.iter().enumerate() {
            let output = instrument
                .params
                .get_channel()
                .and_then(|id| params.channel(id))
                .map_or("no output", |c| c.name.as_str());

            ui.horizontal(|ui| {
                if ui.selectable_label(i == active, &instrument.name).clicked() && i != active {
                    selected = Some(i);
                }
                ui.label(
                    RichText::new(format!("→ {output}"))
                        .font(theme::value_font())
                        .color(theme::TEXT_DIM),
                );
            });
        }
    });

    selected
}
