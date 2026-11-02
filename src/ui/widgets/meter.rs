use eframe::egui::{self, CornerRadius, Rect, Sense, pos2, vec2};

use crate::ui::theme;

const WIDTH: f32 = 10.0;
const FLOOR_DB: f32 = -60.0;

/// A vertical peak meter on a dB scale. Turns red above 1.0 (0 dB).
pub struct Meter {
    level: f32,
    height: f32,
}

impl Meter {
    pub fn new(level: f32) -> Self {
        Self {
            level,
            height: 160.0,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl egui::Widget for Meter {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        // same outer size and track as a Fader of the same height, so they line up
        let text_h = 14.0;
        let cap_h = 14.0;
        let (rect, response) = ui.allocate_exact_size(
            vec2(WIDTH, self.height + 2.0 * (text_h + 6.0)),
            Sense::hover(),
        );
        let track = Rect::from_center_size(rect.center(), vec2(WIDTH, self.height - cap_h));

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let radius = CornerRadius::same(2);

            painter.rect_filled(track, radius, theme::BG);

            let db = 20.0 * self.level.max(1e-6).log10();
            let t = ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0);
            if t > 0.0 {
                let top = track.bottom() - track.height() * t;
                let filled = Rect::from_min_max(pos2(track.left(), top), track.right_bottom());
                let color = if self.level > 1.0 {
                    theme::CLIP
                } else {
                    theme::METER
                };
                painter.rect_filled(filled, radius, color);
            }
        }

        response
    }
}
