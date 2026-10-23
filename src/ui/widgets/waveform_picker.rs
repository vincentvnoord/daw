use std::f32::consts::TAU;

use eframe::egui::{
    self, Align2, CornerRadius, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, pos2, vec2,
};

use crate::audio::osc::{AtomicWaveForm, WaveForm};
use crate::ui::theme;

const ROW: egui::Vec2 = vec2(96.0, 26.0);
const ICON_W: f32 = 26.0;

/// A vertical list of waveforms to choose from, bound to an atomic parameter.
pub struct WaveformPicker<'a> {
    param: &'a AtomicWaveForm,
}

impl<'a> WaveformPicker<'a> {
    pub fn new(param: &'a AtomicWaveForm) -> Self {
        Self { param }
    }
}

impl egui::Widget for WaveformPicker<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let inner = ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let current = self.param.get();
            let mut changed = false;

            for waveform in WaveForm::ALL {
                let (rect, response) = ui.allocate_exact_size(ROW, Sense::click());
                if response.clicked() && waveform != current {
                    self.param.set(waveform);
                    changed = true;
                }

                let selected = self.param.get() == waveform;
                let hovered = response.hovered();
                let painter = ui.painter();
                let radius = CornerRadius::same(4);

                let fill = if selected {
                    theme::ACCENT_DIM
                } else if hovered {
                    theme::SURFACE_HI
                } else {
                    theme::BG
                };
                painter.rect_filled(rect, radius, fill);
                if selected {
                    painter.rect_stroke(
                        rect,
                        radius,
                        Stroke::new(1.0, theme::ACCENT),
                        StrokeKind::Inside,
                    );
                }

                let fg = if selected || hovered {
                    theme::TEXT
                } else {
                    theme::TEXT_DIM
                };
                let icon = Rect::from_min_size(
                    rect.min + vec2(8.0, 7.0),
                    vec2(ICON_W - 8.0, ROW.y - 14.0),
                );
                painter.add(Shape::line(
                    icon_points(waveform, icon),
                    Stroke::new(1.5, fg),
                ));
                painter.text(
                    pos2(rect.left() + ICON_W + 8.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    waveform.name(),
                    theme::label_font(),
                    fg,
                );
            }

            changed
        });

        let mut response = inner.response;
        if inner.inner {
            response.mark_changed();
        }
        response
    }
}

/// One cycle of the waveform, fitted into `rect`.
fn icon_points(waveform: WaveForm, rect: Rect) -> Vec<Pos2> {
    let at = |x: f32, y: f32| {
        pos2(
            rect.left() + x * rect.width(),
            rect.center().y - y * rect.height() / 2.0,
        )
    };
    match waveform {
        WaveForm::Sine => (0..=24)
            .map(|i| {
                let x = i as f32 / 24.0;
                at(x, (x * TAU).sin())
            })
            .collect(),
        WaveForm::Saw => vec![at(0.0, -1.0), at(0.5, 1.0), at(0.5, -1.0), at(1.0, 1.0)],
        WaveForm::Square => vec![at(0.0, 1.0), at(0.5, 1.0), at(0.5, -1.0), at(1.0, -1.0)],
    }
}
