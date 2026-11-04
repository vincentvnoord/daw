//! The notes of one sequence, on a grid of pitch against time. Click to add
//! a note, drag a note to move it or its right edge to resize it, right-click
//! to delete it. Every placement of the sequence plays the changes.

mod draw;
mod edit;
mod grid;

use eframe::egui::{
    self, Align, CornerRadius, Layout, Margin, RichText, Sense, Stroke, pos2, vec2,
};

use crate::{
    audio::transport::metronome::BEATS_PER_BAR,
    project::Project,
    ui::{
        piano_roll::{
            draw::{beat_lines, keys, past_end, rows},
            edit::{NoteDrag, SNAP},
            grid::{Grid, KEYS_WIDTH, PITCHES, PIXELS_PER_BEAT, ROW_HEIGHT},
        },
        theme,
    },
};

/// Smallest height the roll shrinks to when there's little room.
const MIN_HEIGHT: f32 = 180.0;
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
}
