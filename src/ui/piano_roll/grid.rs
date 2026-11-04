use eframe::egui::{Pos2, Rect, pos2};

use crate::project::Note;

pub(super) const ROW_HEIGHT: f32 = 10.0;
pub(super) const PIXELS_PER_BEAT: f32 = 64.0;
pub(super) const KEYS_WIDTH: f32 = 44.0;
pub(super) const PITCHES: u8 = 128;
/// How close to a note's right edge the pointer resizes instead of moves.
const EDGE: f32 = 6.0;

/// Maps beats and pitches to positions in the roll and back.
pub(super) struct Grid {
    /// Where beat 0 of the top pitch is: the top-left corner right of the keys.
    pub(super) origin: Pos2,
    /// The right edge of the roll.
    pub(super) right: f32,
    /// The part on screen.
    pub(super) visible: Rect,
}

impl Grid {
    /// The top of `pitch`'s row, with the roll's top at `top`. High notes are
    /// at the top.
    pub(super) fn row_top(pitch: u8, top: f32) -> f32 {
        top + (PITCHES - 1 - pitch) as f32 * ROW_HEIGHT
    }

    pub(super) fn row(&self, pitch: u8) -> Rect {
        let top = Self::row_top(pitch, self.origin.y);
        Rect::from_min_max(pos2(self.origin.x, top), pos2(self.right, top + ROW_HEIGHT))
    }

    pub(super) fn x(&self, beat: f64) -> f32 {
        self.origin.x + beat as f32 * PIXELS_PER_BEAT
    }

    pub(super) fn beat(&self, x: f32) -> f64 {
        ((x - self.origin.x) / PIXELS_PER_BEAT) as f64
    }

    pub(super) fn pitch(&self, y: f32) -> u8 {
        let row = ((y - self.origin.y) / ROW_HEIGHT).floor() as i32;
        (PITCHES as i32 - 1 - row).clamp(0, 127) as u8
    }

    /// Whether `pos` is over the notes rather than the keys, which cover the
    /// left of whatever part is on screen.
    pub(super) fn on_notes(&self, pos: Pos2) -> bool {
        pos.x >= self.visible.left() + KEYS_WIDTH && pos.x >= self.origin.x
    }

    /// Where `note` is drawn, cut off at the end of the sequence.
    pub(super) fn note_rect(&self, note: &Note, length: f64) -> Option<Rect> {
        let end = (note.start + note.length).min(length);
        (end > note.start).then(|| {
            let row = self.row(note.pitch);
            Rect::from_x_y_ranges(self.x(note.start)..=self.x(end), row.y_range())
        })
    }

    /// The note under `pos`, and whether `pos` is on its right edge. The
    /// last note wins, since it's drawn on top.
    pub(super) fn hit(&self, notes: &[Note], length: f64, pos: Pos2) -> Option<(usize, bool)> {
        notes.iter().enumerate().rev().find_map(|(i, note)| {
            let rect = self.note_rect(note, length)?;
            rect.contains(pos)
                .then(|| (i, pos.x >= rect.right() - EDGE.min(rect.width() / 2.0)))
        })
    }
}
