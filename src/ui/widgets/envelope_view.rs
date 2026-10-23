use eframe::egui::{self, Color32, CornerRadius, Pos2, Sense, Shape, Stroke, pos2};

use crate::ui::theme;

/// Read-only ADSR curve display.
pub struct EnvelopeView {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl egui::Widget for EnvelopeView {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let size = egui::vec2(ui.available_width(), 80.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if !ui.is_rect_visible(rect) {
            return response;
        }

        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(4), theme::BG);

        let area = rect.shrink(8.0);
        // the sustain stage gets a fixed width so the shape stays readable
        let hold = 0.6;
        let total = self.attack + self.decay + hold + self.release;
        let x = |t: f32| area.left() + area.width() * t / total;
        let y = |level: f32| area.bottom() - area.height() * level;

        let t_a = self.attack;
        let t_d = t_a + self.decay;
        let t_s = t_d + hold;
        let t_r = t_s + self.release;
        let points: Vec<Pos2> = vec![
            pos2(x(0.0), y(0.0)),
            pos2(x(t_a), y(1.0)),
            pos2(x(t_d), y(self.sustain)),
            pos2(x(t_s), y(self.sustain)),
            pos2(x(t_r), y(0.0)),
        ];

        for t in [t_a, t_d, t_s] {
            painter.line_segment(
                [pos2(x(t), area.top()), pos2(x(t), area.bottom())],
                Stroke::new(1.0, theme::SURFACE),
            );
        }

        let fill = Color32::from_rgba_unmultiplied(
            theme::ACCENT.r(),
            theme::ACCENT.g(),
            theme::ACCENT.b(),
            28,
        );
        // fill each segment separately, since the full outline isn't convex
        for pair in points.windows(2) {
            painter.add(Shape::convex_polygon(
                vec![
                    pair[0],
                    pair[1],
                    pos2(pair[1].x, area.bottom()),
                    pos2(pair[0].x, area.bottom()),
                ],
                fill,
                Stroke::NONE,
            ));
        }
        painter.add(Shape::line(points.clone(), Stroke::new(2.0, theme::ACCENT)));
        for p in &points[1..4] {
            painter.circle_filled(*p, 3.0, theme::TEXT);
        }

        response
    }
}
