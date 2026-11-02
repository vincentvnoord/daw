//! The lanes right of the track headers: a bar ruler, each track's
//! placements with their notes drawn small inside, each audio track's clips
//! with their waveform, and the playhead. Placements and clips can be dragged
//! along the timeline and onto other tracks of their kind, and clips onto the
//! empty track at the bottom; placements can be double-clicked to open their
//! sequence.

use std::sync::atomic::Ordering;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, Painter, Pos2, Rect, Sense, Stroke,
    StrokeKind, pos2, vec2,
};

use crate::{
    audio::transport::{beats_at, metronome::BEATS_PER_BAR},
    project::{Clip, Project, Sequence, Track},
    ui::{theme, track_panel::TrackAction},
};

/// Height of the bar numbers above the lanes. The track headers start this
/// far down too, so each header lines up with its lane.
pub const RULER_HEIGHT: f32 = 16.0;

const PIXELS_PER_BEAT: f32 = 24.0;
/// What a dragged block snaps to, in beats. Holding Alt turns it off.
const SNAP: f64 = 1.0;
/// Space between a block and the top and bottom of its lane.
const BLOCK_INSET: f32 = 3.0;
/// Room at the top of a block for its name.
const BLOCK_LABEL_HEIGHT: f32 = 14.0;
const NOTE_HEIGHT: f32 = 3.0;

/// UI-side state of the timeline: the block being dragged, if any.
#[derive(Default)]
pub struct Timeline {
    drag: Option<Drag>,
}

/// What a block on a lane is: a placement of a sequence, or a clip. Each
/// can only be dropped on tracks of its own kind.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Placement,
    Clip,
}

impl Kind {
    fn fits(self, track: &Track) -> bool {
        match self {
            Kind::Placement => track.takes_sequences(),
            Kind::Clip => track.audio().is_some(),
        }
    }
}

/// A block on a lane, gathered from the project so placements and clips can
/// be drawn and dragged the same way.
struct Block<'a> {
    kind: Kind,
    track: usize,
    /// Index in the track's placements or clips.
    index: usize,
    /// In beats.
    start: f64,
    length: f64,
    content: Content<'a>,
}

/// What a block shows inside.
enum Content<'a> {
    /// The sequence's notes, and for a placement its index in `sequences`.
    Notes(&'a Sequence, usize),
    Waveform(&'a Clip),
}

impl Content<'_> {
    fn name(&self) -> &str {
        match self {
            Content::Notes(sequence, _) => &sequence.name,
            Content::Waveform(clip) => &clip.name,
        }
    }
}

/// A block on its way somewhere. It only moves in the project when it's
/// dropped; until then the timeline draws where it would land.
struct Drag {
    kind: Kind,
    track: usize,
    index: usize,
    /// Beats between the block's start and where it was grabbed, so it
    /// doesn't jump to put its start under the pointer.
    grab: f64,
}

/// How a block is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Look {
    Normal,
    /// Where a dragged block was picked up from.
    Ghost,
    /// Where a dragged block would land.
    Dragged,
}

impl Timeline {
    /// Draws the timeline, scrolling sideways when it's wider than the space.
    /// `row_heights` are the heights of the track headers, top to bottom, so
    /// every lane is as tall as the header next to it. One past the tracks is
    /// the empty track's, when there's room for it.
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
        let samples_per_beat =
            project.sample_rate as f64 * 60.0 / project.transport.bpm.get() as f64;
        let mut action = None;

        egui::ScrollArea::horizontal()
            .id_salt("timeline")
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    let (ruler_rect, _) =
                        ui.allocate_exact_size(vec2(width, RULER_HEIGHT), Sense::hover());
                    let grid = Grid::new(ruler_rect.left(), ui.clip_rect(), samples_per_beat);
                    ruler(ui.painter(), &grid, ruler_rect);

                    let lanes: Vec<Rect> = row_heights
                        .iter()
                        .enumerate()
                        .map(|(i, &height)| {
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(width, height), Sense::hover());
                            let empty = i >= project.tracks.len();
                            lane(ui.painter(), &grid, rect, empty);
                            rect
                        })
                        .collect();

                    action = self.blocks(ui, project, &grid, &lanes);

                    let all = lanes.iter().fold(ruler_rect, |all, lane| all.union(*lane));
                    playhead(ui.painter(), &grid, all, project);
                });
            });

        action
    }

    /// Draws every placement and clip, and handles dragging and
    /// double-clicking them.
    fn blocks(
        &mut self,
        ui: &egui::Ui,
        project: &Project,
        grid: &Grid,
        lanes: &[Rect],
    ) -> Option<TrackAction> {
        let pointer = ui.input(|i| i.pointer.latest_pos());
        let snap = !ui.input(|i| i.modifiers.alt);
        let blocks = gather(project, grid.samples_per_beat);
        let mut action = None;

        for block in &blocks {
            let Some(lane) = lanes.get(block.track) else {
                continue;
            };
            let rect = block_rect(grid, *lane, block.start, block.length);
            let dragged = self.drag.as_ref().is_some_and(|d| {
                (d.kind, d.track, d.index) == (block.kind, block.track, block.index)
            });

            let look = if dragged { Look::Ghost } else { Look::Normal };
            draw_block(ui.painter(), grid, rect, &block.content, look);

            // keeps its spot while dragged, so egui keeps the drag going
            let id = (block.kind == Kind::Clip, block.track, block.index);
            let response = ui
                .interact(
                    rect,
                    ui.scope_id().with(("block", id)),
                    Sense::click_and_drag(),
                )
                .on_hover_cursor(CursorIcon::Grab);

            if response.double_clicked()
                && let Content::Notes(_, sequence) = block.content
            {
                action = Some(TrackAction::OpenSequence {
                    track: block.track,
                    sequence,
                });
            }

            if response.drag_started()
                && let Some(pointer) = pointer
            {
                self.drag = Some(Drag {
                    kind: block.kind,
                    track: block.track,
                    index: block.index,
                    grab: grid.beat(pointer.x) - block.start,
                });
            }

            if response.drag_stopped()
                && let Some(drag) = self.drag.take()
                && let Some(pointer) = pointer
            {
                let (to, start) = drag.target(pointer, grid, lanes, project, snap);
                let (from, index) = (drag.track, drag.index);
                action = Some(match drag.kind {
                    Kind::Placement => TrackAction::MovePlacement {
                        from,
                        index,
                        to,
                        start,
                    },
                    Kind::Clip => TrackAction::MoveClip {
                        from,
                        index,
                        to: (to < project.tracks.len()).then_some(to),
                        start,
                    },
                });
            }
        }

        // drawn last, so it's on top of the lanes it passes over
        if let Some(drag) = &self.drag
            && let Some(pointer) = pointer
            && let Some(block) = blocks
                .iter()
                .find(|b| (b.kind, b.track, b.index) == (drag.kind, drag.track, drag.index))
        {
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            let (to, start) = drag.target(pointer, grid, lanes, project, snap);
            let rect = block_rect(grid, lanes[to], start, block.length);
            draw_block(ui.painter(), grid, rect, &block.content, Look::Dragged);
        }

        action
    }
}

/// Every placement and clip in the project, as blocks.
fn gather(project: &Project, samples_per_beat: f64) -> Vec<Block<'_>> {
    let mut blocks = Vec::new();

    for (track, t) in project.tracks.iter().enumerate() {
        for (index, placement) in t.placements.iter().enumerate() {
            let Some(sequence) = project.sequences.get(placement.sequence) else {
                continue;
            };
            blocks.push(Block {
                kind: Kind::Placement,
                track,
                index,
                start: placement.start,
                length: sequence.length,
                content: Content::Notes(sequence, placement.sequence),
            });
        }

        for (index, clip) in t.audio().map_or(&[][..], |a| &a.clips).iter().enumerate() {
            blocks.push(Block {
                kind: Kind::Clip,
                track,
                index,
                start: clip.start,
                length: clip.audio.len() as f64 / samples_per_beat,
                content: Content::Waveform(clip),
            });
        }
    }

    blocks
}

impl Drag {
    /// The lane and start the block would land on with the pointer at
    /// `pointer`: the lane under it if its track fits the block's kind (or
    /// its own otherwise), snapped. The empty track's lane, past the tracks,
    /// takes clips.
    fn target(
        &self,
        pointer: Pos2,
        grid: &Grid,
        lanes: &[Rect],
        project: &Project,
        snap: bool,
    ) -> (usize, f64) {
        let fits = |lane: usize| match project.tracks.get(lane) {
            Some(track) => self.kind.fits(track),
            None => self.kind == Kind::Clip,
        };
        let track = lanes
            .iter()
            .enumerate()
            .position(|(i, lane)| fits(i) && lane.y_range().contains(pointer.y))
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
    /// At the current tempo, for clips: their audio is in samples.
    samples_per_beat: f64,
}

impl Grid {
    fn new(origin: f32, visible: Rect, samples_per_beat: f64) -> Self {
        Self {
            origin,
            visible,
            samples_per_beat,
        }
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

/// One track's row with the beat grid, stronger on every bar. The `empty`
/// track's row has no fill, so it reads as space to drop into.
fn lane(painter: &Painter, grid: &Grid, rect: Rect, empty: bool) {
    painter.rect(
        rect,
        CornerRadius::same(theme::RADIUS),
        if empty { theme::BG } else { theme::PANEL },
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

/// Where a block starting at `start` sits in `lane`.
fn block_rect(grid: &Grid, lane: Rect, start: f64, length: f64) -> Rect {
    Rect::from_x_y_ranges(
        grid.x(start)..=grid.x(start + length),
        lane.y_range().shrink(BLOCK_INSET),
    )
}

/// A block as long as what it plays, named, with its notes drawn inside like
/// a tiny piano roll or its waveform.
fn draw_block(painter: &Painter, grid: &Grid, rect: Rect, content: &Content, look: Look) {
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
        content.name(),
        theme::label_font(),
        ink,
    );

    let area = Rect::from_min_max(
        pos2(rect.left(), rect.top() + BLOCK_LABEL_HEIGHT),
        rect.max - vec2(0.0, 4.0),
    );
    match content {
        Content::Notes(sequence, _) => notes(&painter, area, sequence, ink),
        Content::Waveform(clip) => waveform(&painter, grid, area, clip, ink),
    }
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

/// The clip's audio across `area`: one line per pixel, as tall as the
/// loudest sample under it, mirrored around the middle. Only the part on
/// screen is drawn.
fn waveform(painter: &Painter, grid: &Grid, area: Rect, clip: &Clip, color: Color32) {
    let samples_per_pixel = grid.samples_per_beat / PIXELS_PER_BEAT as f64;
    let sample = |x: f32| ((x - area.left()) as f64 * samples_per_pixel).max(0.0) as usize;
    let left = area.left().max(grid.visible.left()).floor();
    let right = area.right().min(grid.visible.right());
    let stroke = Stroke::new(1.0, color);

    let mut x = left;
    while x < right {
        let peak = clip.peak(sample(x), sample(x + 1.0)).min(1.0);
        // at least a pixel, so silence still shows as a line
        let half = (peak * area.height() / 2.0).max(0.5);
        painter.vline(
            x + 0.5,
            (area.center().y - half)..=(area.center().y + half),
            stroke,
        );
        x += 1.0;
    }
}

/// A line where the transport is, across the ruler and every lane.
fn playhead(painter: &Painter, grid: &Grid, rect: Rect, project: &Project) {
    let samples = project.transport.position.load(Ordering::Relaxed);
    let bpm = project.transport.bpm.get() as f64;
    let x = grid.x(beats_at(samples, bpm, project.sample_rate as f64));

    painter.vline(x, rect.y_range(), Stroke::new(1.5, theme::ACCENT));
}
