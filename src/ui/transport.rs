use std::sync::atomic::Ordering;

use eframe::egui::{self, RichText};

use crate::{
    audio::transport::{MAX_BPM, MIN_BPM, beats_at, metronome::BEATS_PER_BAR},
    project::Project,
    ui::{
        theme,
        widgets::{
            icon_button::{Icon, IconButton},
            knob::Knob,
            value::Unit,
        },
    },
};

/// Play and rewind buttons, the tempo, where the playhead is, and the
/// metronome switch. Sized to sit in the header bar.
pub fn show(ui: &mut egui::Ui, project: &Project) {
    let transport = &project.transport;

    if ui
        .add(IconButton::new(Icon::Rewind))
        .on_hover_text("Back to start")
        .clicked()
    {
        transport.seek(0.0);
    }

    let playing = transport.playing.load(Ordering::Relaxed);
    let play = IconButton::new(if playing { Icon::Pause } else { Icon::Play }).selected(playing);
    if ui
        .add(play)
        .on_hover_text(if playing {
            "Pause (space)"
        } else {
            "Play (space)"
        })
        .clicked()
    {
        transport.toggle_playing();
    }

    ui.add_space(theme::GAP);
    position(ui, project);
    ui.add_space(theme::GAP);

    let mut bpm = transport.bpm.get();
    let tempo = egui::DragValue::new(&mut bpm)
        .range(MIN_BPM..=MAX_BPM)
        .speed(0.5)
        .fixed_decimals(1)
        .suffix(" BPM");
    if ui.add(tempo).changed() {
        transport.bpm.set(bpm);
    }

    ui.add_space(theme::GAP);

    let metronome = &project.metronome;
    let enabled = metronome.enabled.load(Ordering::Relaxed);
    let response = ui
        .add(IconButton::new(Icon::Metronome).selected(enabled))
        .on_hover_text("Metronome, right-click for volume");

    if response.clicked() {
        metronome.enabled.fetch_xor(true, Ordering::Relaxed);
    }

    // the volume is an atomic, so it can be changed right here
    response.context_menu(|ui| {
        ui.add(Knob::new(
            &metronome.volume,
            0.0..=1.0,
            "Volume",
            Unit::Percent,
        ));
    });
}

/// The playhead as bar.beat, and as minutes and seconds.
fn position(ui: &mut egui::Ui, project: &Project) {
    let samples = project.transport.position.load(Ordering::Relaxed);
    let bpm = project.transport.bpm.get() as f64;
    let sample_rate = project.sample_rate as f64;

    let beats = beats_at(samples, bpm, sample_rate).floor() as u64;
    let bar = beats / BEATS_PER_BAR + 1;
    let beat = beats % BEATS_PER_BAR + 1;

    let seconds = samples as f64 / sample_rate;
    let minutes = (seconds / 60.0).floor();

    ui.label(
        RichText::new(format!("{bar:>3}.{beat}"))
            .font(egui::FontId::monospace(16.0))
            .color(theme::ACCENT),
    );
    ui.label(
        RichText::new(format!("{minutes:.0}:{:04.1}", seconds - minutes * 60.0))
            .font(theme::value_font())
            .color(theme::TEXT_DIM),
    );
}
