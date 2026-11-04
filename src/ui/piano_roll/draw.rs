//! Drawing the roll: the pitch rows and beat lines behind the notes, the
//! notes, and the keys down the left.

use eframe::egui::{self, Align2, Color32, Painter, Rect, Stroke, pos2, vec2};

use crate::{
    audio::transport::metronome::BEATS_PER_BAR,
    project::Project,
    ui::{
        piano_roll::{
            PianoRoll,
            edit::SNAP,
            grid::{Grid, KEYS_WIDTH, PITCHES},
        },
        theme,
    },
};

impl PianoRoll {
    pub(super) fn notes(&self, painter: &Painter, grid: &Grid, project: &Project) {
        let sequence = &project.sequences[self.sequence];
        for (index, note) in sequence.notes.iter().enumerate() {
            let Some(rect) = grid.note_rect(note, sequence.length) else {
                continue;
            };
            let dragged = self.drag.as_ref().is_some_and(|d| d.index == index);
            painter.rect(
                rect,
                2.0,
                if dragged { theme::ACCENT } else { theme::METER },
                Stroke::new(1.0, theme::BG),
                egui::StrokeKind::Inside,
            );
        }
    }
}

fn is_black(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}

/// A row per pitch: darker for the black keys, a line under every C.
pub(super) fn rows(painter: &Painter, grid: &Grid) {
    for pitch in 0..PITCHES {
        let row = grid.row(pitch);
        if !row.intersects(grid.visible) {
            continue;
        }
        let fill = if is_black(pitch) {
            theme::BG
        } else {
            theme::PANEL
        };
        painter.rect_filled(row, 0.0, fill);
        if pitch % 12 == 0 {
            painter.hline(row.x_range(), row.bottom(), Stroke::new(1.0, theme::BORDER));
        }
    }
}

/// Lines on every sixteenth, stronger on beats and strongest on bars.
pub(super) fn beat_lines(painter: &Painter, grid: &Grid, rect: Rect, beats: f64) {
    let steps = (beats / SNAP).ceil() as u64;
    let per_beat = (1.0 / SNAP) as u64;
    for step in 0..=steps {
        let x = grid.x(step as f64 * SNAP);
        if x < grid.visible.left() || x > grid.visible.right() {
            continue;
        }
        let color = if step % (per_beat * BEATS_PER_BAR) == 0 {
            theme::BORDER
        } else if step % per_beat == 0 {
            theme::SURFACE_HI
        } else {
            theme::SURFACE
        };
        painter.vline(x, rect.y_range(), Stroke::new(1.0, color));
    }
}

/// Shades everything after the end of the sequence, where notes don't play.
pub(super) fn past_end(painter: &Painter, grid: &Grid, rect: Rect, length: f64) {
    let end = grid.x(length);
    painter.rect_filled(
        Rect::from_min_max(pos2(end, rect.top()), rect.max),
        0.0,
        Color32::from_black_alpha(140),
    );
    painter.vline(end, rect.y_range(), Stroke::new(1.0, theme::ACCENT_DIM));
}

/// The keyboard down the left, pinned to the left of the visible part so it
/// stays when the roll scrolls sideways. C's are named, with C3 at 48 like
/// the computer keyboard.
pub(super) fn keys(painter: &Painter, grid: &Grid) {
    let left = grid.visible.left();
    for pitch in 0..PITCHES {
        let row = grid.row(pitch);
        let key = Rect::from_x_y_ranges(left..=left + KEYS_WIDTH, row.y_range());
        if !key.intersects(grid.visible) {
            continue;
        }

        let fill = if is_black(pitch) {
            theme::BG
        } else {
            theme::SURFACE_HI
        };
        painter.rect(
            key,
            0.0,
            fill,
            Stroke::new(1.0, theme::PANEL),
            egui::StrokeKind::Inside,
        );
        if pitch % 12 == 0 {
            painter.text(
                key.right_center() - vec2(4.0, 0.0),
                Align2::RIGHT_CENTER,
                format!("C{}", pitch as i32 / 12 - 1),
                theme::label_font(),
                theme::TEXT_DIM,
            );
        }
    }
}
