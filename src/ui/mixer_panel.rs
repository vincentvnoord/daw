use eframe::egui::{self, RichText};

use crate::{
    audio::mixer::meter,
    helpers::AtomicF32,
    shared::AppParams,
    ui::{
        theme,
        widgets::{card::card, fader::Fader, meter::Meter, value::Unit},
    },
};

const STRIP_HEIGHT: f32 = 150.0;
/// How fast a meter falls back after a peak.
const FALL_DB_PER_SEC: f32 = 20.0;

/// UI-side state of the mixer: the displayed meter levels and the clip light.
#[derive(Default)]
pub struct MixerPanel {
    levels: Vec<f32>,
    master_level: f32,
    clipped: bool,
}

impl MixerPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, params: &AppParams) {
        let dt = ui.input(|i| i.stable_dt);
        let fall = 10f32.powf(-FALL_DB_PER_SEC * dt / 20.0);
        self.levels.resize(params.channels.len(), 0.0);

        card(ui, "Mixer", |ui| {
            ui.horizontal(|ui| {
                for ((name, channel), level) in params.channels.iter().zip(&mut self.levels) {
                    *level = next_level(&channel.meter, *level, fall);
                    strip(ui, name, &channel.gain, *level);
                }

                ui.separator();

                self.master_level = next_level(&params.mixer.meter, self.master_level, fall);
                if self.master_level > 1.0 {
                    self.clipped = true;
                }

                ui.vertical(|ui| {
                    strip(ui, "Master", &params.mixer.gain, self.master_level);

                    let clip = egui::Button::new(
                        RichText::new("CLIP")
                            .font(theme::label_font())
                            .color(if self.clipped { theme::TEXT } else { theme::TEXT_DIM }),
                    )
                    .fill(if self.clipped { theme::CLIP } else { theme::SURFACE });
                    if ui.add(clip).on_hover_text("Click to reset").clicked() {
                        self.clipped = false;
                    }
                });
            });
        });
    }
}

/// Jumps up to a new peak instantly, falls back slowly.
fn next_level(meter: &meter::Meter, level: f32, fall: f32) -> f32 {
    meter.take().max(level * fall)
}

fn strip(ui: &mut egui::Ui, name: &str, gain: &AtomicF32, level: f32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.add(Fader::new(gain, 0.0..=1.0, name, Unit::Decibels).height(STRIP_HEIGHT));
        ui.add(Meter::new(level).height(STRIP_HEIGHT));
    });
}
