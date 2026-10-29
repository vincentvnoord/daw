use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, pos2, vec2,
};

use crate::ui::theme;

const WHITE: Color32 = Color32::from_rgb(222, 222, 228);
const BLACK: Color32 = Color32::from_rgb(28, 28, 32);

/// On-screen piano that highlights held notes.
///
/// Returns the note under the pointer while the primary button is held.
pub fn piano(
    ui: &mut egui::Ui,
    range: RangeInclusive<u8>,
    held: &BTreeSet<u8>,
    label: impl Fn(u8) -> Option<String>,
) -> Option<u8> {
    let whites: Vec<u8> = range.clone().filter(|n| !is_black(*n)).collect();
    let size = vec2(ui.available_width(), 96.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());

    let white_w = rect.width() / whites.len() as f32;
    let black_size = vec2(white_w * 0.6, rect.height() * 0.6);

    let white_rect = |i: usize| {
        Rect::from_min_size(
            pos2(rect.left() + i as f32 * white_w, rect.top()),
            vec2(white_w, rect.height()),
        )
    };
    // black keys sit on the boundary after the white key below them
    let blacks: Vec<(u8, Rect)> = whites
        .iter()
        .enumerate()
        .filter(|(_, n)| range.contains(&(*n + 1)) && is_black(*n + 1))
        .map(|(i, n)| {
            let x = white_rect(i).right();
            (
                n + 1,
                Rect::from_min_size(pos2(x - black_size.x / 2.0, rect.top()), black_size),
            )
        })
        .collect();

    let pressed = response
        .is_pointer_button_down_on()
        .then(|| ui.input(|i| i.pointer.interact_pos()))
        .flatten()
        .filter(|p| rect.contains(*p))
        .and_then(|p| {
            blacks
                .iter()
                .find(|(_, r)| r.contains(p))
                .map(|(n, _)| *n)
                .or_else(|| {
                    whites
                        .get(((p.x - rect.left()) / white_w) as usize)
                        .copied()
                })
        });

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius {
            nw: 0,
            ne: 0,
            sw: 3,
            se: 3,
        };
        let font = FontId::monospace(10.0);

        for (i, &note) in whites.iter().enumerate() {
            let r = white_rect(i).shrink2(vec2(1.0, 0.0));
            let fill = if held.contains(&note) {
                theme::PRESSED
            } else {
                WHITE
            };
            painter.rect_filled(r, radius, fill);
            if let Some(text) = label(note) {
                painter.text(
                    r.center_bottom() - vec2(0.0, 6.0),
                    Align2::CENTER_BOTTOM,
                    text,
                    font.clone(),
                    BLACK,
                );
            }
        }

        for &(note, r) in &blacks {
            let fill = if held.contains(&note) {
                theme::PRESSED
            } else {
                BLACK
            };
            painter.rect_filled(r, radius, fill);
            painter.rect_stroke(r, radius, Stroke::new(1.0, theme::BG), StrokeKind::Inside);
            if let Some(text) = label(note) {
                painter.text(
                    r.center_bottom() - vec2(0.0, 5.0),
                    Align2::CENTER_BOTTOM,
                    text,
                    font.clone(),
                    theme::TEXT_DIM,
                );
            }
        }
    }

    pressed
}

fn is_black(note: u8) -> bool {
    matches!(note % 12, 1 | 3 | 6 | 8 | 10)
}
