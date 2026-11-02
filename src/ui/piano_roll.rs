//! The notes of one sequence, on a grid of pitch against time. Click to add
//! a note, drag a note to move it or its right edge to resize it, right-click
//! to delete it. Every placement of the sequence plays the changes.

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, CursorIcon, Layout, Margin, Pos2, Rect, RichText,
    Sense, Stroke, pos2, vec2,
};

use crate::{
    audio::transport::metronome::BEATS_PER_BAR,
    project::{Note, Project},
    ui::theme,
};

const ROW_HEIGHT: f32 = 10.0;
const PIXELS_PER_BEAT: f32 = 64.0;
const KEYS_WIDTH: f32 = 44.0;
/// Smallest height the roll shrinks to when there's little room.
const MIN_HEIGHT: f32 = 180.0;
/// What notes snap to, in beats: sixteenths. Holding Alt turns it off.
const SNAP: f64 = 0.25;
/// How close to a note's right edge the pointer resizes instead of moves.
const EDGE: f32 = 6.0;
const PITCHES: u8 = 128;
/// Where the roll scrolls to for a sequence without notes: middle C.
const MIDDLE_C: u8 = 60;

/// UI-side state of an open piano roll.
pub struct PianoRoll {
    /// Index in `Project::sequences` of the sequence being edited.
    sequence: usize,
    /// The track it was opened from, named in the header.
    track: usize,
    drag: Option<NoteDrag>,
    /// How long a new note is: as long as the last one drawn or resized.
    new_length: f64,
    /// Set until the first frame has scrolled the notes into view.
    scroll_to_notes: bool,
}

/// A note being moved or resized. The note changes as the pointer moves; the
/// schedule is only sent once it's let go.
struct NoteDrag {
    index: usize,
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
    pub fn new(sequence: usize, track: usize) -> Self {
        Self {
            sequence,
            track,
            drag: None,
            new_length: SNAP,
            scroll_to_notes: true,
        }
    }

    /// Draws the roll in a panel with the sequence's name and a close button.
    /// Returns `false` once it's closed.
    pub fn show(&mut self, ui: &mut egui::Ui, project: &mut Project) -> bool {
        if self.sequence >= project.sequences.len() {
            return false;
        }
        let mut open = true;

        egui::Frame::new()
            .fill(theme::PANEL)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(CornerRadius::same(theme::RADIUS))
            .inner_margin(Margin::same(theme::CARD_PADDING))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    open = self.header(ui, project);
                    ui.separator();
                    self.roll(ui, project);
                });
            });

        open
    }

    /// The sequence's name and the track it was opened from. Returns `false`
    /// when the close button is clicked.
    fn header(&self, ui: &mut egui::Ui, project: &Project) -> bool {
        let mut open = true;

        ui.horizontal(|ui| {
            let sequence = &project.sequences[self.sequence];
            ui.label(RichText::new(&sequence.name).heading().strong());
            if let Some(track) = project.tracks.get(self.track) {
                ui.label(
                    RichText::new(format!("ON {}", track.name.to_uppercase()))
                        .font(theme::label_font())
                        .color(theme::TEXT_DIM),
                );
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    open = false;
                }
                ui.label(
                    RichText::new("Alt: no snap · right-click: delete")
                        .font(theme::label_font())
                        .color(theme::TEXT_DIM),
                );
            });
        });

        open
    }

    /// The grid with the notes, scrolling both ways, and the keys down the
    /// left side, which stay put when it scrolls sideways.
    fn roll(&mut self, ui: &mut egui::Ui, project: &mut Project) {
        let height = ui.available_height().max(MIN_HEIGHT);
        let length = project.sequences[self.sequence].length;
        // a bar past the end, so the end is easy to see
        let beats = length + BEATS_PER_BAR as f64;

        let mut scroll = egui::ScrollArea::both()
            .id_salt("piano_roll")
            .max_height(height)
            .auto_shrink([false, false]);
        if self.scroll_to_notes {
            self.scroll_to_notes = false;
            let center = project.sequences[self.sequence]
                .pitch_range()
                .map_or(MIDDLE_C, |(low, high)| low + (high - low) / 2);
            let y = Grid::row_top(center, 0.0) + ROW_HEIGHT / 2.0 - height / 2.0;
            scroll = scroll.vertical_scroll_offset(y.max(0.0));
        }

        scroll.show(ui, |ui| {
            let size = vec2(
                KEYS_WIDTH + beats as f32 * PIXELS_PER_BEAT,
                PITCHES as f32 * ROW_HEIGHT,
            );
            let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
            let grid = Grid {
                origin: pos2(rect.left() + KEYS_WIDTH, rect.top()),
                right: rect.right(),
                visible: ui.clip_rect(),
            };

            let changed = self.edit(ui, &response, &grid, project);

            let painter = ui.painter();
            rows(painter, &grid);
            beat_lines(painter, &grid, rect, beats);
            past_end(painter, &grid, rect, length);
            self.notes(painter, &grid, project);
            keys(painter, &grid);

            if changed {
                project.update_schedule();
            }
        });
    }

    /// Adds, moves, resizes and deletes notes from what the pointer did.
    /// Returns `true` when the sequence changed in a way that should reach
    /// the studio.
    fn edit(
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

    fn notes(&self, painter: &egui::Painter, grid: &Grid, project: &Project) {
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

/// Maps beats and pitches to positions in the roll and back.
struct Grid {
    /// Where beat 0 of the top pitch is: the top-left corner right of the keys.
    origin: Pos2,
    /// The right edge of the roll.
    right: f32,
    /// The part on screen.
    visible: Rect,
}

impl Grid {
    /// The top of `pitch`'s row, with the roll's top at `top`. High notes are
    /// at the top.
    fn row_top(pitch: u8, top: f32) -> f32 {
        top + (PITCHES - 1 - pitch) as f32 * ROW_HEIGHT
    }

    fn row(&self, pitch: u8) -> Rect {
        let top = Self::row_top(pitch, self.origin.y);
        Rect::from_min_max(pos2(self.origin.x, top), pos2(self.right, top + ROW_HEIGHT))
    }

    fn x(&self, beat: f64) -> f32 {
        self.origin.x + beat as f32 * PIXELS_PER_BEAT
    }

    fn beat(&self, x: f32) -> f64 {
        ((x - self.origin.x) / PIXELS_PER_BEAT) as f64
    }

    fn pitch(&self, y: f32) -> u8 {
        let row = ((y - self.origin.y) / ROW_HEIGHT).floor() as i32;
        (PITCHES as i32 - 1 - row).clamp(0, 127) as u8
    }

    /// Whether `pos` is over the notes rather than the keys, which cover the
    /// left of whatever part is on screen.
    fn on_notes(&self, pos: Pos2) -> bool {
        pos.x >= self.visible.left() + KEYS_WIDTH && pos.x >= self.origin.x
    }

    /// Where `note` is drawn, cut off at the end of the sequence.
    fn note_rect(&self, note: &Note, length: f64) -> Option<Rect> {
        let end = (note.start + note.length).min(length);
        (end > note.start).then(|| {
            let row = self.row(note.pitch);
            Rect::from_x_y_ranges(self.x(note.start)..=self.x(end), row.y_range())
        })
    }

    /// The note under `pos`, and whether `pos` is on its right edge. The
    /// last note wins, since it's drawn on top.
    fn hit(&self, notes: &[Note], length: f64, pos: Pos2) -> Option<(usize, bool)> {
        notes.iter().enumerate().rev().find_map(|(i, note)| {
            let rect = self.note_rect(note, length)?;
            rect.contains(pos)
                .then(|| (i, pos.x >= rect.right() - EDGE.min(rect.width() / 2.0)))
        })
    }
}

fn is_black(pitch: u8) -> bool {
    matches!(pitch % 12, 1 | 3 | 6 | 8 | 10)
}

/// A row per pitch: darker for the black keys, a line under every C.
fn rows(painter: &egui::Painter, grid: &Grid) {
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
fn beat_lines(painter: &egui::Painter, grid: &Grid, rect: Rect, beats: f64) {
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
fn past_end(painter: &egui::Painter, grid: &Grid, rect: Rect, length: f64) {
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
fn keys(painter: &egui::Painter, grid: &Grid) {
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

/// `beat` rounded to the nearest grid line when snapping.
fn round(beat: f64, snap: bool) -> f64 {
    if snap {
        (beat / SNAP).round() * SNAP
    } else {
        beat
    }
}
