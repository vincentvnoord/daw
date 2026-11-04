//! The controls for an audio track: its input and trim, and its clips.

use eframe::egui::{self, RichText};

use crate::{
    audio::studio::input::MAX_TRIM,
    project::{AudioHandle, InputDevice, input_devices},
    ui::{
        theme,
        widgets::{card::card, knob::Knob, value::Unit},
    },
};

/// Which input the track listens to, and its trim. Muting is on the track's
/// header.
/// Returns the input picked in the dropdown, if one was: an index in
/// `devices`, or `None` for no input.
pub(super) fn audio_controls(
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
pub(super) fn clip_controls(ui: &mut egui::Ui, audio: &AudioHandle) -> bool {
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
