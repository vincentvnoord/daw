use std::ops::RangeInclusive;

use eframe::egui::{
    self, Align2, CornerRadius, CursorIcon, Rect, Sense, Stroke, StrokeKind, pos2, vec2,
};

use crate::helpers::AtomicF32;
use crate::ui::{
    theme,
    widgets::value::{Mapping, Unit},
};

const WIDTH: f32 = 56.0;
const TRACK_W: f32 = 6.0;
const CAP: egui::Vec2 = vec2(28.0, 14.0);

/// A vertical channel fader bound to an atomic parameter.
pub struct Fader<'a> {
    param: &'a AtomicF32,
    mapping: Mapping,
    label: &'a str,
    unit: Unit,
    height: f32,
}

impl<'a> Fader<'a> {
    pub fn new(
        param: &'a AtomicF32,
        range: RangeInclusive<f32>,
        label: &'a str,
        unit: Unit,
    ) -> Self {
        Self {
            param,
            mapping: Mapping::new(range, false),
            label,
            unit,
            height: 160.0,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl egui::Widget for Fader<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let text_h = 14.0;
        let (rect, mut response) = ui.allocate_exact_size(
            vec2(WIDTH, self.height + 2.0 * (text_h + 6.0)),
            Sense::click_and_drag(),
        );
        response = response.on_hover_cursor(CursorIcon::ResizeVertical);

        let track = Rect::from_center_size(rect.center(), vec2(TRACK_W, self.height - CAP.y));

        if let Some(pos) = response.interact_pointer_pos()
            && (response.dragged() || response.clicked())
        {
            let t = 1.0 - (pos.y - track.top()) / track.height();
            self.param.set(self.mapping.from_normalized(t));
            response.mark_changed();
        }
        let t = self.mapping.to_normalized(self.param.get());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let active = response.hovered() || response.dragged();
            let radius = CornerRadius::same(3);

            painter.text(
                pos2(rect.center().x, rect.top()),
                Align2::CENTER_TOP,
                self.label,
                theme::label_font(),
                if active { theme::TEXT } else { theme::TEXT_DIM },
            );

            // tick marks
            for i in 0..=10 {
                let y = track.bottom() - track.height() * i as f32 / 10.0;
                let len = if i % 5 == 0 { 8.0 } else { 4.0 };
                painter.line_segment(
                    [
                        pos2(track.left() - 6.0 - len, y),
                        pos2(track.left() - 6.0, y),
                    ],
                    Stroke::new(1.0, theme::BORDER),
                );
            }

            painter.rect_filled(track, radius, theme::BG);
            let cap_y = track.bottom() - track.height() * t;
            let filled = Rect::from_min_max(pos2(track.left(), cap_y), track.right_bottom());
            painter.rect_filled(filled, radius, theme::ACCENT);

            let cap = Rect::from_center_size(pos2(track.center().x, cap_y), CAP);
            painter.rect_filled(
                cap,
                radius,
                if active {
                    theme::TEXT
                } else {
                    theme::SURFACE_HI
                },
            );
            painter.rect_stroke(
                cap,
                radius,
                Stroke::new(1.0, theme::BORDER),
                StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    pos2(cap.left() + 5.0, cap_y),
                    pos2(cap.right() - 5.0, cap_y),
                ],
                Stroke::new(
                    1.5,
                    if active {
                        theme::ACCENT
                    } else {
                        theme::TEXT_DIM
                    },
                ),
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
