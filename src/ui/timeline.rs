//! The lanes right of the track headers: a bar ruler, each track's
//! placements with their notes drawn small inside, and the playhead.
//! Placements can be dragged along the timeline and onto other tracks, and
//! double-clicked to open their sequence.

use std::sync::atomic::Ordering;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, Painter, Pos2, Rect, Sense, Stroke,
    StrokeKind, pos2, vec2,
};

use crate::{
    audio::transport::{beats_at, metronome::BEATS_PER_BAR},
    project::{Project, Sequence},
    ui::{theme, track_panel::TrackAction},
};

/// Height of the bar numbers above the lanes. The track headers start this
/// far down too, so each header lines up with its lane.
pub const RULER_HEIGHT: f32 = 16.0;

const PIXELS_PER_BEAT: f32 = 24.0;
/// What a dragged placement snaps to, in beats. Holding Alt turns it off.
const SNAP: f64 = 1.0;
/// Space between a placement and the top and bottom of its lane.
const BLOCK_INSET: f32 = 3.0;
/// Room at the top of a placement for the sequence's name.
const BLOCK_LABEL_HEIGHT: f32 = 14.0;
const NOTE_HEIGHT: f32 = 3.0;

/// UI-side state of the timeline: the placement being dragged, if any.
#[derive(Default)]
pub struct Timeline {
    drag: Option<Drag>,
}

/// A placement on its way somewhere. It only moves in the project when it's
/// dropped; until then the timeline draws where it would land.
struct Drag {
    track: usize,
    /// Index in the track's placements.
    index: usize,
    /// Beats between the placement's start and where it was grabbed, so it
    /// doesn't jump to put its start under the pointer.
    grab: f64,
}

/// How a placement is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Look {
    Normal,
    /// Where a dragged placement was picked up from.
    Ghost,
    /// Where a dragged placement would land.
    Dragged,
}

impl Timeline {
    /// Draws the timeline, scrolling sideways when it's wider than the space.
    /// `row_heights` are the heights of the track headers, top to bottom, so
    /// every lane is as tall as the header next to it.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        row_heights: &[f32],
    ) -> Option<TrackAction> {
        let beats_per_bar = BEATS_PER_BAR as f64;
        // one bar past the end, so there's always empty space to drag into
        let song_beats = ((project.timeline_length() / beats_per_bar).ceil() + 1.0) * beats_per_bar;
        let width = (song_beats as f32 * PIXELS_PER_BEAT).max(ui.available_width());
        let mut action = None;

        egui::ScrollArea::horizontal()
            .id_salt("timeline")
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    let (ruler_rect, _) =
                        ui.allocate_exact_size(vec2(width, RULER_HEIGHT), Sense::hover());
                    let grid = Grid::new(ruler_rect.left(), ui.clip_rect());
                    ruler(ui.painter(), &grid, ruler_rect);

                    let lanes: Vec<Rect> = row_heights
                        .iter()
                        .take(project.tracks.len())
                        .map(|&height| {
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(width, height), Sense::hover());
                            lane(ui.painter(), &grid, rect);
                            rect
                        })
                        .collect();

                    action = self.placements(ui, project, &grid, &lanes);

                    let all = lanes.iter().fold(ruler_rect, |all, lane| all.union(*lane));
                    playhead(ui.painter(), &grid, all, project);
                });
            });

        action
    }

    /// Draws every placement and handles dragging and double-clicking them.
    fn placements(
        &mut self,
        ui: &egui::Ui,
        project: &Project,
        grid: &Grid,
        lanes: &[Rect],
    ) -> Option<TrackAction> {
        let pointer = ui.input(|i| i.pointer.latest_pos());
        let snap = !ui.input(|i| i.modifiers.alt);
        let mut action = None;

        for (track, (lane, placements)) in lanes
            .iter()
            .zip(project.tracks.iter().map(|t| &t.placements))
            .enumerate()
        {
            for (index, placement) in placements.iter().enumerate() {
                let Some(sequence) = project.sequences.get(placement.sequence) else {
                    continue;
                };
                let rect = block_rect(grid, *lane, placement.start, sequence.length);
                let dragged = self
                    .drag
                    .as_ref()
                    .is_some_and(|d| d.track == track && d.index == index);

                let look = if dragged { Look::Ghost } else { Look::Normal };
                block(ui.painter(), grid, rect, sequence, look);

                // keeps its spot while dragged, so egui keeps the drag going
                let response = ui
                    .interact(
                        rect,
                        ui.scope_id().with(("placement", track, index)),
                        Sense::click_and_drag(),
                    )
                    .on_hover_cursor(CursorIcon::Grab);

                if response.double_clicked() {
                    action = Some(TrackAction::OpenSequence {
                        track,
                        sequence: placement.sequence,
                    });
                }

                if response.drag_started()
                    && let Some(pointer) = pointer
                {
                    self.drag = Some(Drag {
                        track,
                        index,
                        grab: grid.beat(pointer.x) - placement.start,
                    });
                }

                if response.drag_stopped()
                    && let Some(drag) = self.drag.take()
                    && let Some(pointer) = pointer
                {
                    let (to, start) = drag.target(pointer, grid, lanes, snap);
                    action = Some(TrackAction::MovePlacement {
                        from: drag.track,
                        index: drag.index,
                        to,
                        start,
                    });
                }
            }
        }

        // drawn last, so it's on top of the lanes it passes over
        if let Some(drag) = &self.drag
            && let Some(pointer) = pointer
            && let Some(sequence) = project.tracks[drag.track]
                .placements
                .get(drag.index)
                .and_then(|p| project.sequences.get(p.sequence))
        {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            let (to, start) = drag.target(pointer, grid, lanes, snap);
            let rect = block_rect(grid, lanes[to], start, sequence.length);
            block(ui.painter(), grid, rect, sequence, Look::Dragged);
        }

        action
    }
}

impl Drag {
    /// The track and start the placement would land on with the pointer at
    /// `pointer`: the lane under it (or its own, off the lanes), snapped.
    fn target(&self, pointer: Pos2, grid: &Grid, lanes: &[Rect], snap: bool) -> (usize, f64) {
        let track = lanes
            .iter()
            .position(|lane| lane.y_range().contains(pointer.y))
            .unwrap_or(self.track);

        let mut start = grid.beat(pointer.x) - self.grab;
        if snap {
            start = (start / SNAP).round() * SNAP;
        }

        (track, start.max(0.0))
    }
}

/// Turns beats into x positions and back, and knows which part is on screen
/// so only that part gets drawn.
struct Grid {
    /// Where beat 0 is.
    origin: f32,
    visible: Rect,
}

impl Grid {
    fn new(origin: f32, visible: Rect) -> Self {
        Self { origin, visible }
    }

    fn x(&self, beat: f64) -> f32 {
        self.origin + beat as f32 * PIXELS_PER_BEAT
    }

    fn beat(&self, x: f32) -> f64 {
        ((x - self.origin) / PIXELS_PER_BEAT) as f64
    }

    /// Every whole beat on screen, with its x.
    fn visible_beats(&self) -> impl Iterator<Item = (u64, f32)> + '_ {
        let first = self.beat(self.visible.left()).floor().max(0.0) as u64;
        (first..)
            .map(|beat| (beat, self.x(beat as f64)))
            .take_while(|&(_, x)| x <= self.visible.right())
    }
}

/// Bar numbers, counted from 1 like the transport display.
fn ruler(painter: &Painter, grid: &Grid, rect: Rect) {
    for (beat, x) in grid.visible_beats() {
        if beat % BEATS_PER_BAR == 0 {
            painter.text(
                pos2(x + 3.0, rect.center().y),
                Align2::LEFT_CENTER,
                (beat / BEATS_PER_BAR + 1).to_string(),
                theme::label_font(),
                theme::TEXT_DIM,
            );
        }
    }
}

/// One track's row with the beat grid, stronger on every bar.
fn lane(painter: &Painter, grid: &Grid, rect: Rect) {
    painter.rect(
        rect,
        CornerRadius::same(theme::RADIUS),
        theme::PANEL,
        Stroke::new(1.0, theme::BORDER),
        StrokeKind::Inside,
    );

    for (beat, x) in grid.visible_beats() {
        if x <= rect.left() || x >= rect.right() {
            continue;
        }
        let color = if beat % BEATS_PER_BAR == 0 {
            theme::BORDER
        } else {
            theme::SURFACE
        };
        painter.vline(x, rect.y_range().shrink(1.0), Stroke::new(1.0, color));
    }
}

/// Where a placement starting at `start` sits in `lane`.
fn block_rect(grid: &Grid, lane: Rect, start: f64, length: f64) -> Rect {
    Rect::from_x_y_ranges(
        grid.x(start)..=grid.x(start + length),
        lane.y_range().shrink(BLOCK_INSET),
    )
}

/// A placement: a block as long as its sequence, named, with the notes drawn
/// inside like a tiny piano roll.
fn block(painter: &Painter, grid: &Grid, rect: Rect, sequence: &Sequence, look: Look) {
    if !rect.intersects(grid.visible) {
        return;
    }

    let (fill, stroke, ink) = match look {
        Look::Normal => (theme::SURFACE_HI, theme::BORDER, theme::TEXT_DIM),
        Look::Ghost => (theme::SURFACE, theme::BORDER, theme::ACCENT_DIM),
        Look::Dragged => (theme::SURFACE_HI, theme::ACCENT, theme::TEXT),
    };
    painter.rect(
        rect,
        CornerRadius::same(4),
        fill,
        Stroke::new(1.0, stroke),
        StrokeKind::Inside,
    );

    // clipped, so a long name or a note can't spill into the next block
    let painter = painter.with_clip_rect(rect.shrink(1.0).intersect(painter.clip_rect()));
    painter.text(
        rect.left_top() + vec2(6.0, 2.0),
        Align2::LEFT_TOP,
        &sequence.name,
        theme::label_font(),
        ink,
    );

    notes(
        &painter,
        Rect::from_min_max(
            pos2(rect.left(), rect.top() + BLOCK_LABEL_HEIGHT),
            rect.max - vec2(0.0, 4.0),
        ),
        sequence,
        ink,
    );
}

/// The sequence's notes squeezed into `area`: high notes at the top, a
/// sequence of one pitch as a row in the middle.
fn notes(painter: &Painter, area: Rect, sequence: &Sequence, color: Color32) {
    let Some((low, high)) = sequence.pitch_range() else {
        return;
    };
    let span = (high - low) as f32;
    let y = |pitch: u8| {
        if span == 0.0 {
            area.center().y - NOTE_HEIGHT / 2.0
        } else {
            area.bottom()
                - NOTE_HEIGHT
                - (pitch - low) as f32 / span * (area.height() - NOTE_HEIGHT)
        }
    };
    let x = |beat: f64| area.left() + beat as f32 * PIXELS_PER_BEAT;

    for note in &sequence.notes {
        let end = (note.start + note.length).min(sequence.length);
        if end <= note.start {
            continue;
        }
        let left = x(note.start);
        let top = y(note.pitch);
        painter.rect_filled(
            Rect::from_min_max(
                pos2(left, top),
                pos2(x(end).max(left + 1.0), top + NOTE_HEIGHT),
            ),
            1.0,
            color,
        );
    }
}

/// A line where the transport is, across the ruler and every lane.
fn playhead(painter: &Painter, grid: &Grid, rect: Rect, project: &Project) {
    let samples = project.transport.position.load(Ordering::Relaxed);
    let bpm = project.transport.bpm.get() as f64;
    let x = grid.x(beats_at(samples, bpm, project.sample_rate as f64));

    painter.vline(x, rect.y_range(), Stroke::new(1.5, theme::ACCENT));
}
