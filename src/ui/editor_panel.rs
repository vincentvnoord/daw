use eframe::egui::{self, RichText};

use crate::{
    shared::{InstrumentHandle, InstrumentKind},
    ui::{synth_panel, theme, widgets::card::card},
};

/// Shows the editor for the selected instrument.
pub fn show(ui: &mut egui::Ui, instrument: &InstrumentHandle) {
    match &instrument.kind {
        InstrumentKind::Synth(params) => synth_panel::show(ui, params),
        InstrumentKind::Sampler { sample } => {
            card(ui, &instrument.name, |ui| {
                ui.label(
                    RichText::new("Sample")
                        .font(theme::label_font())
                        .color(theme::TEXT_DIM),
                );
                ui.label(RichText::new(sample).font(theme::value_font()));
            });
        }
    }
}
