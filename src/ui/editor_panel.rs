use eframe::egui::{self, Align, CornerRadius, Layout, Margin, RichText, Stroke};

use crate::{
    audio::{mixer::channel::ChannelId, studio::input::MAX_TRIM},
    project::{
        AudioHandle, InputDevice, InstrumentHandle, InstrumentKind, Project, Track, TrackSource,
        input_devices,
    },
    ui::{
        synth_panel, theme,
        widgets::{card::card, knob::Knob, value::Unit},
    },
};

/// What the user picked in the output dropdown.
enum OutputChoice {
    Channel(Option<ChannelId>),
    NewChannel,
}

/// The selected track as one panel: a header with the track's name, its type
/// and the output channel, and its controls underneath: the instrument's for
/// an instrument track, the input's and clips' for an audio track. `devices`
/// is the list of recording devices offered for an audio track's input.
/// Returns `true` when asked to add a clip to the track; the file dialog is
/// the app's to open, after drawing.
pub fn show(
    ui: &mut egui::Ui,
    project: &mut Project,
    active: usize,
    devices: &mut Vec<InputDevice>,
) -> bool {
    let Some(track) = project.tracks.get(active) else {
        return false;
    };
    let mut choice = None;
    let mut input = None;
    let mut add_clip = false;

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
                header(ui, track, project, &mut choice);
                ui.separator();
                let layout = Layout::left_to_right(Align::Min).with_main_wrap(true);
                ui.with_layout(layout, |ui| match &track.source {
                    TrackSource::Instrument(instrument) => instrument_controls(ui, instrument),
                    TrackSource::Audio(audio) => {
                        input = audio_controls(ui, audio, devices);
                        add_clip = clip_controls(ui, audio);
                    }
                });
            });
        });

    match choice {
        Some(OutputChoice::Channel(id)) => project.tracks[active].route().set_channel(id),
        Some(OutputChoice::NewChannel) => {
            // name the channel after the track that asked for it
            let name = project.tracks[active].name.clone();
            if let Some(id) = project.add_channel(name) {
                project.tracks[active].route().set_channel(Some(id));
            }
        }
        None => {}
    }

    // opened after drawing, since opening a device can take a moment
    if let Some(picked) = input {
        let device = picked.map(|i| &devices[i]);
        if let Err(err) = project.set_track_input(active, device) {
            let name = device.map_or("no input", |d| d.name.as_str());
            eprintln!("failed to open {name}: {err}");
        }
    }

    add_clip
}

fn header(ui: &mut egui::Ui, track: &Track, project: &Project, choice: &mut Option<OutputChoice>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(&track.name).heading().strong());
        if let Some(kind) = track.kind_label() {
            ui.label(
                RichText::new(kind.to_uppercase())
                    .font(theme::label_font())
                    .color(theme::TEXT_DIM),
            );
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let current = track.route().get_channel();
            let current_name = current
                .and_then(|id| project.channel(id))
                .map_or("No output", |c| c.name.as_str());

            egui::ComboBox::from_id_salt("track_output")
                .selected_text(current_name)
                .width(140.0)
                .show_ui(ui, |ui| {
                    for channel in &project.channels {
                        let id = channel.id();
                        if ui
                            .selectable_label(current == Some(id), &channel.name)
                            .clicked()
                        {
                            *choice = Some(OutputChoice::Channel(Some(id)));
                        }
                    }
                    if ui
                        .selectable_label(current.is_none(), "No output")
                        .clicked()
                    {
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

fn instrument_controls(ui: &mut egui::Ui, instrument: &InstrumentHandle) {
    match &instrument.kind {
        InstrumentKind::Synth(params) => synth_panel::show(ui, params),
        InstrumentKind::Sampler { sample } => {
            card(ui, "Sample", |ui| {
                ui.label(RichText::new(sample).font(theme::value_font()));
            });
        }
    }
}

/// Which input the track listens to, and its trim. Muting is on the track's
/// header.
/// Returns the input picked in the dropdown, if one was: an index in
/// `devices`, or `None` for no input.
fn audio_controls(
    ui: &mut egui::Ui,
    audio: &AudioHandle,
    devices: &mut Vec<InputDevice>,
) -> Option<Option<usize>> {
    let mut picked = None;

    card(ui, "Input", |ui| {
        let current = audio.input.as_ref().map(|i| i.name.as_str());
        let combo = egui::ComboBox::from_id_salt("track_input")
            .selected_text(current.unwrap_or("No input"))
            .width(220.0)
            .show_ui(ui, |ui| {
                if ui.selectable_label(current.is_none(), "No input").clicked() {
                    picked = Some(None);
                }
                if devices.is_empty() {
                    ui.label(RichText::new("No input devices").color(theme::TEXT_DIM));
                }
                for (i, device) in devices.iter().enumerate() {
                    let name = device.input_name();
                    if ui
                        .selectable_label(current == Some(name.as_str()), &name)
                        .clicked()
                    {
                        picked = Some(Some(i));
                    }
                }
            });
        // looked up as the list opens: asking the system every frame is slow
        if combo.response.clicked() {
            *devices = input_devices();
        }
    });

    card(ui, "Level", |ui| {
        ui.add(
            Knob::new(&audio.params.trim, 1.0..=MAX_TRIM, "Trim", Unit::Decibels).logarithmic(true),
        );
    });

    picked
}

/// How many clips the track has, and a button to add one at the playhead.
/// Returns `true` when the button is clicked.
fn clip_controls(ui: &mut egui::Ui, audio: &AudioHandle) -> bool {
    let mut add = false;

    card(ui, "Clips", |ui| {
        let count = match audio.clips.len() {
            1 => "1 clip".to_owned(),
            n => format!("{n} clips"),
        };
        ui.label(RichText::new(count).font(theme::value_font()));
        add = ui
            .button("Add WAV…")
            .on_hover_text("Place a WAV file at the playhead")
            .clicked();
    });

    add
}
