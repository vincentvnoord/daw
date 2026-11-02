use std::sync::atomic::Ordering;

use eframe::egui::{self, RichText};

use crate::{
    audio::transport::{MAX_BPM, MIN_BPM, beats_at, metronome::BEATS_PER_BAR},
    shared::AppParams,
    ui::{
        theme,
        widgets::icon_button::{Icon, IconButton},
    },
};

const VOLUME_WIDTH: f32 = 60.0;

/// Play and rewind buttons, the tempo, where the playhead is, and the
/// metronome switch. Sized to sit in the header bar.
pub fn show(ui: &mut egui::Ui, params: &AppParams) {
    let transport = &params.transport;

    if ui
        .add(IconButton::new(Icon::Rewind))
        .on_hover_text("Back to start")
        .clicked()
    {
        transport.rewind.store(true, Ordering::Relaxed);
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
    position(ui, params);
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

    let metronome = &params.metronome;
    let mut enabled = metronome.enabled.load(Ordering::Relaxed);
    if ui
        .toggle_value(&mut enabled, "Click")
        .on_hover_text("Metronome")
        .changed()
    {
        metronome.enabled.store(enabled, Ordering::Relaxed);
    }

    let mut volume = metronome.volume.get();
    ui.spacing_mut().slider_width = VOLUME_WIDTH;
    let slider = egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false);
    if ui
        .add_enabled(enabled, slider)
        .on_hover_text("Click volume")
        .changed()
    {
        metronome.volume.set(volume);
    }
}

/// The playhead as bar.beat, and as minutes and seconds.
fn position(ui: &mut egui::Ui, params: &AppParams) {
    let samples = params.transport.position.load(Ordering::Relaxed);
    let bpm = params.transport.bpm.get() as f64;
    let sample_rate = params.sample_rate as f64;

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
