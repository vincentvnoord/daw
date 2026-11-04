//! What the pointer does to the notes: add, move, resize and delete.

use eframe::egui::{self, CursorIcon, Pos2};

use crate::{
    project::{Note, Project},
    ui::piano_roll::{PianoRoll, grid::Grid},
};

/// What notes snap to, in beats: sixteenths. Holding Alt turns it off.
pub(super) const SNAP: f64 = 0.25;

/// A note being moved or resized. The note changes as the pointer moves; the
/// schedule is only sent once it's let go.
pub(super) struct NoteDrag {
    pub(super) index: usize,
    mode: DragMode,
}

enum DragMode {
    /// `beat` and `pitch` are how far from the note's start and pitch it was
    /// grabbed, so it doesn't jump to put its corner under the pointer.
    Move {
        beat: f64,
        pitch: i32,
    },
    Resize,
}

impl PianoRoll {
    /// Adds, moves, resizes and deletes notes from what the pointer did.
    /// Returns `true` when the sequence changed in a way that should reach
    /// the studio.
    pub(super) fn edit(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        grid: &Grid,
        project: &mut Project,
    ) -> bool {
        let snap = !ui.input(|i| i.modifiers.alt);
        let sequence = &mut project.sequences[self.sequence];
        let mut changed = false;

        let pointer = ui.input(|i| i.pointer.latest_pos());
        if self.drag.is_none()
            && let Some(pointer) = pointer.filter(|&p| grid.on_notes(p))
        {
            match grid.hit(&sequence.notes, sequence.length, pointer) {
                Some((_, true)) => ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal),
                Some((_, false)) => ui.ctx().set_cursor_icon(CursorIcon::Grab),
                None => {}
            }
        }

        if response.secondary_clicked()
            && let Some(pointer) = response
                .interact_pointer_pos()
                .filter(|&p| grid.on_notes(p))
            && let Some((index, _)) = grid.hit(&sequence.notes, sequence.length, pointer)
        {
            sequence.notes.remove(index);
            changed = true;
        }

        if response.clicked()
            && let Some(pointer) = response
                .interact_pointer_pos()
                .filter(|&p| grid.on_notes(p))
            && grid
                .hit(&sequence.notes, sequence.length, pointer)
                .is_none()
            && let Some(note) = self.new_note(grid, pointer, sequence.length, snap)
        {
            sequence.notes.push(note);
            changed = true;
        }

        if response.drag_started()
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
            && grid.on_notes(origin)
        {
            self.drag = match grid.hit(&sequence.notes, sequence.length, origin) {
                Some((index, true)) => Some(NoteDrag {
                    index,
                    mode: DragMode::Resize,
                }),
                Some((index, false)) => {
                    let note = sequence.notes[index];
                    Some(NoteDrag {
                        index,
                        mode: DragMode::Move {
                            beat: grid.beat(origin.x) - note.start,
                            pitch: grid.pitch(origin.y) as i32 - note.pitch as i32,
                        },
                    })
                }
                // dragging from an empty spot draws a new note and moves it
                None => self
                    .new_note(grid, origin, sequence.length, snap)
                    .map(|note| {
                        sequence.notes.push(note);
                        NoteDrag {
                            index: sequence.notes.len() - 1,
                            mode: DragMode::Move {
                                beat: grid.beat(origin.x) - note.start,
                                pitch: 0,
                            },
                        }
                    }),
            };
        }

        if let Some(drag) = &self.drag
            && let Some(pointer) = pointer
            && let Some(note) = sequence.notes.get_mut(drag.index)
        {
            match drag.mode {
                DragMode::Move { beat, pitch } => {
                    let start = round(grid.beat(pointer.x) - beat, snap);
                    note.start = start.clamp(0.0, (sequence.length - note.length).max(0.0));
                    note.pitch = (grid.pitch(pointer.y) as i32 - pitch).clamp(0, 127) as u8;
                    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                }
                DragMode::Resize => {
                    let shortest = if snap { SNAP } else { SNAP / 4.0 };
                    let end = round(grid.beat(pointer.x), snap).min(sequence.length);
                    note.length = (end - note.start).max(shortest);
                    self.new_length = note.length;
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                }
            }
        }

        if response.drag_stopped() && self.drag.take().is_some() {
            changed = true;
        }

        changed
    }

    /// A note under `pointer`, starting on the grid line before it, as long
    /// as the last note drawn. `None` past the end of the sequence.
    fn new_note(&self, grid: &Grid, pointer: Pos2, length: f64, snap: bool) -> Option<Note> {
        let beat = grid.beat(pointer.x).max(0.0);
        let start = if snap {
            (beat / SNAP).floor() * SNAP
        } else {
            beat
        };
        (start < length).then(|| Note {
            start,
            length: self.new_length.min(length - start),
            pitch: grid.pitch(pointer.y),
        })
    }
}

/// `beat` rounded to the nearest grid line when snapping.
fn round(beat: f64, snap: bool) -> f64 {
    if snap {
        (beat / SNAP).round() * SNAP
    } else {
        beat
    }
}
