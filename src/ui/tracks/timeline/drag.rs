use eframe::egui::{Pos2, Rect};

use crate::{
    project::Project,
    ui::tracks::timeline::{block::Kind, grid::Grid},
};

/// What a dragged block snaps to, in beats. Holding Alt turns it off.
const SNAP: f64 = 1.0;

/// A block on its way somewhere. It only moves in the project when it's
/// dropped; until then the timeline draws where it would land.
pub(super) struct Drag {
    pub(super) kind: Kind,
    pub(super) track: usize,
    pub(super) index: usize,
    /// Beats between the block's start and where it was grabbed, so it
    /// doesn't jump to put its start under the pointer.
    pub(super) grab: f64,
}

impl Drag {
    /// The lane and start the block would land on with the pointer at
    /// `pointer`: the lane under it if its track fits the block's kind (or
    /// its own otherwise), snapped. The empty track's lane, past the tracks,
    /// takes clips.
    pub(super) fn target(
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
