use std::f32::consts::PI;
use std::ops::RangeInclusive;

use eframe::egui::{self, Align2, CursorIcon, Sense, Shape, Stroke, Vec2, pos2, vec2};

use crate::helpers::AtomicF32;
use crate::ui::{
    theme,
    widgets::value::{Mapping, Unit},
};

const SIZE: f32 = 46.0;
const START_ANGLE: f32 = 0.75 * PI;
const SWEEP: f32 = 1.5 * PI;
/// Normalized change per pixel of vertical drag.
const DRAG_SPEED: f32 = 0.005;

/// A rotary knob bound to an atomic parameter. Drag vertically to change,
/// hold shift for fine adjustment.
pub struct Knob<'a> {
    param: &'a AtomicF32,
    range: RangeInclusive<f32>,
    log: bool,
    bipolar: bool,
    label: &'a str,
    unit: Unit,
}

impl<'a> Knob<'a> {
    pub fn new(
        param: &'a AtomicF32,
        range: RangeInclusive<f32>,
        label: &'a str,
        unit: Unit,
    ) -> Self {
        Self {
            param,
            range,
            log: false,
            bipolar: false,
            label,
            unit,
        }
    }

    pub fn logarithmic(mut self, log: bool) -> Self {
        self.log = log;
        self
    }

    /// Fills the arc out from the top instead of from the start, for
    /// parameters like pan where the middle is the neutral position.
    pub fn bipolar(mut self, bipolar: bool) -> Self {
        self.bipolar = bipolar;
        self
    }
}

impl egui::Widget for Knob<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let label_h = 14.0;
        let desired = vec2(SIZE + 20.0, label_h + SIZE + label_h + 4.0);
        let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click_and_drag());
        response = response.on_hover_cursor(CursorIcon::ResizeVertical);

        let mapping = Mapping::new(self.range, self.log);
        let mut t = mapping.to_normalized(self.param.get());
        if response.dragged() {
            let fine = ui.input(|i| i.modifiers.shift);
            let speed = if fine { DRAG_SPEED * 0.2 } else { DRAG_SPEED };
            let new_t = (t - response.drag_delta().y * speed).clamp(0.0, 1.0);
            if new_t != t {
                t = new_t;
                self.param.set(mapping.from_normalized(t));
                response.mark_changed();
            }
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let active = response.hovered() || response.dragged();

            painter.text(
                pos2(rect.center().x, rect.top()),
                Align2::CENTER_TOP,
                self.label,
                theme::label_font(),
                if active { theme::TEXT } else { theme::TEXT_DIM },
            );

            let center = pos2(rect.center().x, rect.top() + label_h + SIZE / 2.0);
            let radius = SIZE / 2.0;
            let track_r = radius - 3.0;

            painter.add(arc(
                center,
                track_r,
                START_ANGLE,
                SWEEP,
                Stroke::new(4.0, theme::BG),
            ));
            let (fill_start, fill_sweep) = if self.bipolar {
                (START_ANGLE + SWEEP * 0.5, SWEEP * (t - 0.5))
            } else {
                (START_ANGLE, SWEEP * t)
            };
            painter.add(arc(
                center,
                track_r,
                fill_start,
                fill_sweep,
                Stroke::new(4.0, theme::ACCENT),
            ));

            let body = if active {
                theme::SURFACE_HI
            } else {
                theme::PANEL
            };
            painter.circle(center, radius - 8.0, body, Stroke::new(1.0, theme::BORDER));

            let angle = START_ANGLE + SWEEP * t;
            let dir = Vec2::angled(angle);
            painter.line_segment(
                [
                    center + dir * (radius - 16.0),
                    center + dir * (radius - 9.0),
                ],
                Stroke::new(2.0, theme::TEXT),
            );

            painter.text(
                pos2(rect.center().x, rect.bottom()),
                Align2::CENTER_BOTTOM,
                self.unit.format(self.param.get()),
                theme::value_font(),
                if active { theme::ACCENT } else { theme::TEXT },
            );
        }

        response
    }
}

fn arc(center: egui::Pos2, radius: f32, start: f32, sweep: f32, stroke: Stroke) -> Shape {
    let segments = ((sweep.abs() / SWEEP) * 48.0).ceil().max(1.0) as usize;
    let points = (0..=segments)
        .map(|i| center + Vec2::angled(start + sweep * i as f32 / segments as f32) * radius)
        .collect();
    Shape::line(points, stroke)
}
