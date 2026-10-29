use eframe::egui::{self, Align, Layout, RichText};

use crate::{
    audio::mixer::{MAX_CHANNELS, meter},
    helpers::AtomicF32,
    shared::AppParams,
    ui::{
        theme,
        widgets::{fader::Fader, meter::Meter, value::Unit},
    },
};

const STRIP_HEIGHT: f32 = 120.0;
/// Room kept free on the right for the master strip.
const MASTER_WIDTH: f32 = 110.0;
/// Top of every fader: +6 dB. Unity sits halfway up, so there's room to
/// boost a quiet channel as well as cut a loud one.
const MAX_GAIN: f32 = 2.0;
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
    /// Draws the mixer across the full width: one strip per channel, a button
    /// to add a channel, and the master strip on the right.
    pub fn show(&mut self, ui: &mut egui::Ui, params: &mut AppParams) {
        let dt = ui.input(|i| i.stable_dt);
        let fall = 10f32.powf(-FALL_DB_PER_SEC * dt / 20.0);
        self.levels.resize(params.channels.len(), 0.0);

        let mut add_channel = false;
        let mut remove_channel = None;

        ui.horizontal_top(|ui| {
            egui::ScrollArea::horizontal()
                .max_width(ui.available_width() - MASTER_WIDTH)
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        for (i, (channel, level)) in
                            params.channels.iter().zip(&mut self.levels).enumerate()
                        {
                            *level = next_level(&channel.params.meter, *level, fall);
                            if strip(ui, &channel.name, &channel.params.gain, *level, true) {
                                remove_channel = Some((i, channel.id()));
                            }
                        }

                        // as tall as the strips next to it
                        let height = ui.min_rect().height();
                        let add = egui::Button::new(
                            RichText::new("+").size(20.0).color(theme::TEXT_DIM),
                        )
                        .min_size(egui::vec2(44.0, height));
                        let can_add = params.channels.len() < MAX_CHANNELS;
                        if ui
                            .add_enabled(can_add, add)
                            .on_hover_text("Add channel")
                            .clicked()
                        {
                            add_channel = true;
                        }
                    });
                });

            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                self.master_level = next_level(&params.mixer.meter, self.master_level, fall);
                if self.master_level > 1.0 {
                    self.clipped = true;
                }

                ui.vertical(|ui| {
                    strip(ui, "Master", &params.mixer.gain, self.master_level, false);

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

        if add_channel {
            params.add_channel(params.next_channel_name());
        }
        if let Some((index, id)) = remove_channel {
            params.remove_channel(id);
            self.levels.remove(index);
        }
    }
}

/// Jumps up to a new peak instantly, falls back slowly.
fn next_level(meter: &meter::Meter, level: f32, fall: f32) -> f32 {
    meter.take().max(level * fall)
}

/// Draws one channel strip. Returns `true` when its remove button was clicked.
fn strip(ui: &mut egui::Ui, name: &str, gain: &AtomicF32, level: f32, removable: bool) -> bool {
    let mut removed = false;

    theme::card_frame().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.add(Fader::new(gain, 0.0..=MAX_GAIN, name, Unit::Decibels).height(STRIP_HEIGHT));
                ui.add(Meter::new(level).height(STRIP_HEIGHT));
            });

            if removable {
                removed = ui
                    .small_button("×")
                    .on_hover_text("Remove channel")
                    .clicked();
            }
        });
    });

    removed
}
