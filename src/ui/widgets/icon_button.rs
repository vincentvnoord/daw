use eframe::egui::{self, Color32, Pos2, Rect, Sense, Shape, Stroke, pos2, vec2};

use crate::ui::theme;

const SIZE: egui::Vec2 = vec2(30.0, 24.0);
/// Height of the drawn symbol inside the button.
const ICON: f32 = 10.0;

#[derive(Clone, Copy)]
pub enum Icon {
    Play,
    Pause,
    /// Back to the start: a bar with a triangle pointing at it.
    Rewind,
    /// A trapezoid body with a pendulum leaning to the right.
    Metronome,
}

/// A button showing an icon drawn with shapes. `selected` fills it, like a
/// pressed toggle.
pub struct IconButton {
    icon: Icon,
    selected: bool,
}

impl IconButton {
    pub fn new(icon: Icon) -> Self {
        Self {
            icon,
            selected: false,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl egui::Widget for IconButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(SIZE, Sense::click());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact_selectable(&response, self.selected);
            let painter = ui.painter();
            painter.rect(
                rect,
                theme::RADIUS,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
                egui::StrokeKind::Inside,
            );

            let color = if self.selected {
                theme::BG
            } else {
                visuals.fg_stroke.color
            };
            let icon = Rect::from_center_size(rect.center(), vec2(ICON, ICON));
            for shape in shapes(self.icon, icon, color) {
                painter.add(shape);
            }
        }

        response
    }
}

fn shapes(icon: Icon, r: Rect, color: Color32) -> Vec<Shape> {
    match icon {
        Icon::Play => vec![triangle_right(r, color)],
        Icon::Pause => {
            let bar = r.width() * 0.3;
            vec![
                Shape::rect_filled(
                    Rect::from_min_max(r.min, pos2(r.left() + bar, r.bottom())),
                    1.0,
                    color,
                ),
                Shape::rect_filled(
                    Rect::from_min_max(pos2(r.right() - bar, r.top()), r.max),
                    1.0,
                    color,
                ),
            ]
        }
        Icon::Rewind => {
            let bar = r.width() * 0.18;
            let triangle = Rect::from_min_max(pos2(r.left() + bar + 1.0, r.top()), r.max);
            vec![
                Shape::rect_filled(
                    Rect::from_min_max(r.min, pos2(r.left() + bar, r.bottom())),
                    1.0,
                    color,
                ),
                triangle_left(triangle, color),
            ]
        }
        Icon::Metronome => {
            // outlined, so the pendulum stays visible where it crosses the body
            let stroke = Stroke::new(1.5, color);
            let inset = r.width() * 0.3;
            let body = vec![
                pos2(r.left() + inset, r.top()),
                pos2(r.right() - inset, r.top()),
                r.right_bottom(),
                r.left_bottom(),
            ];
            vec![
                Shape::closed_line(body, stroke),
                Shape::line_segment(
                    [r.center_bottom(), pos2(r.right(), r.top() + r.height() * 0.15)],
                    stroke,
                ),
            ]
        }
    }
}

fn triangle_right(r: Rect, color: Color32) -> Shape {
    triangle([r.left_top(), r.right_center(), r.left_bottom()], color)
}

fn triangle_left(r: Rect, color: Color32) -> Shape {
    triangle([r.right_top(), r.right_bottom(), r.left_center()], color)
}

fn triangle(points: [Pos2; 3], color: Color32) -> Shape {
    Shape::convex_polygon(points.to_vec(), color, Stroke::NONE)
}
