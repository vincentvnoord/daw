//! The lanes right of the track headers: a bar ruler, each track's
//! placements with their notes drawn small inside, each audio track's clips
//! with their waveform, and the playhead. Placements and clips can be dragged
//! along the timeline and onto other tracks of their kind, and clips onto the
//! empty track at the bottom; placements can be double-clicked to open their
//! sequence.

mod block;
mod drag;
mod grid;
mod lane;

use eframe::egui::{self, CursorIcon, Rect, Sense, vec2};

use crate::{
    audio::transport::metronome::BEATS_PER_BAR,
    project::Project,
    ui::tracks::{
        TrackAction,
        timeline::{
            block::{Content, Kind, Look, block_rect, draw_block, gather},
            drag::Drag,
            grid::{Grid, PIXELS_PER_BEAT},
            lane::{lane, playhead, ruler},
        },
    },
};

/// Height of the bar numbers above the lanes. The track headers start this
/// far down too, so each header lines up with its lane.
pub const RULER_HEIGHT: f32 = 16.0;

/// UI-side state of the timeline: the block being dragged, if any.
#[derive(Default)]
pub struct Timeline {
    drag: Option<Drag>,
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
                    let (ruler_rect, ruler_response) =
                        ui.allocate_exact_size(vec2(width, RULER_HEIGHT), Sense::click());

                    let grid = Grid::new(ruler_rect.left(), ui.clip_rect(), samples_per_beat);
                    ruler(ui.painter(), &grid, ruler_rect);

                    if ruler_response.clicked()
                        && let Some(pos) = ruler_response.interact_pointer_pos()
                    {
                        let beat = grid.beat(pos.x);
                        project.transport.seek(beat);
                    }

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
