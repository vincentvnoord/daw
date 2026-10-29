use eframe::egui::{self, Align, CornerRadius, Layout, Margin, RichText, Stroke};

use crate::{
    audio::mixer::channel::ChannelId,
    shared::{AppParams, InstrumentHandle, InstrumentKind},
    ui::{synth_panel, theme, widgets::card::card},
};

/// What the user picked in the output dropdown.
enum OutputChoice {
    Channel(Option<ChannelId>),
    NewChannel,
}

/// The selected instrument as one panel: a header with its name, type and
/// output channel, and its controls underneath.
pub fn show(ui: &mut egui::Ui, params: &mut AppParams, active: usize) {
    let Some(instrument) = params.instruments.get(active) else {
        return;
    };
    let mut choice = None;

    egui::Frame::new()
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(CornerRadius::same(theme::RADIUS))
        .inner_margin(Margin::same(theme::CARD_PADDING))
        .show(ui, |ui| {
            // the frame inherits the left-to-right layout it sits in, so stack
            // header, divider and controls explicitly
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                header(ui, instrument, params, &mut choice);
                ui.separator();
                controls(ui, instrument);
            });
        });

    match choice {
        Some(OutputChoice::Channel(id)) => params.instruments[active].params.route.set_channel(id),
        Some(OutputChoice::NewChannel) => {
            // name the channel after the instrument that asked for it
            let name = params.instruments[active].name.clone();
            if let Some(id) = params.add_channel(name) {
                params.instruments[active].params.route.set_channel(Some(id));
            }
        }
        None => {}
    }
}

fn header(
    ui: &mut egui::Ui,
    instrument: &InstrumentHandle,
    params: &AppParams,
    choice: &mut Option<OutputChoice>,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(&instrument.name).heading().strong());
        if let Some(kind) = instrument.kind_label() {
            ui.label(
                RichText::new(kind.to_uppercase())
                    .font(theme::label_font())
                    .color(theme::TEXT_DIM),
            );
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let current = instrument.params.route.get_channel();
            let current_name = current
                .and_then(|id| params.channel(id))
                .map_or("No output", |c| c.name.as_str());

            egui::ComboBox::from_id_salt("instrument_output")
                .selected_text(current_name)
                .width(140.0)
                .show_ui(ui, |ui| {
                    for channel in &params.channels {
                        let id = channel.id();
                        if ui.selectable_label(current == Some(id), &channel.name).clicked() {
                            *choice = Some(OutputChoice::Channel(Some(id)));
                        }
                    }
                    if ui.selectable_label(current.is_none(), "No output").clicked() {
                        *choice = Some(OutputChoice::Channel(None));
                    }
                    ui.separator();
                    if ui.button("+ New channel").clicked() {
                        *choice = Some(OutputChoice::NewChannel);
                    }
                });

            ui.label(
                RichText::new("OUTPUT")
                    .font(theme::label_font())
                    .color(theme::TEXT_DIM),
            );
        });
    });
}

fn controls(ui: &mut egui::Ui, instrument: &InstrumentHandle) {
    let layout = Layout::left_to_right(Align::Min).with_main_wrap(true);
    ui.with_layout(layout, |ui| match &instrument.kind {
        InstrumentKind::Synth(params) => synth_panel::show(ui, params),
        InstrumentKind::Sampler { sample } => {
            card(ui, "Sample", |ui| {
                ui.label(RichText::new(sample).font(theme::value_font()));
            });
        }
    });
}
