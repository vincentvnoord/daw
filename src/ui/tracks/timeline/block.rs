//! The placements and clips on the lanes, gathered as blocks so both kinds
//! can be drawn and dragged the same way.

use eframe::egui::{Align2, Color32, CornerRadius, Painter, Rect, Stroke, StrokeKind, pos2, vec2};

use crate::{
    project::{Clip, Project, Sequence, Track},
    ui::{
        theme,
        tracks::timeline::grid::{Grid, PIXELS_PER_BEAT},
    },
};

/// Space between a block and the top and bottom of its lane.
const BLOCK_INSET: f32 = 3.0;
/// Room at the top of a block for its name.
const BLOCK_LABEL_HEIGHT: f32 = 14.0;
const NOTE_HEIGHT: f32 = 3.0;

/// What a block on a lane is: a placement of a sequence, or a clip. Each
/// can only be dropped on tracks of its own kind.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Kind {
    Placement,
    Clip,
}

impl Kind {
    pub(super) fn fits(self, track: &Track) -> bool {
        match self {
            Kind::Placement => track.takes_sequences(),
            Kind::Clip => track.audio().is_some(),
        }
    }
}

/// A block on a lane, gathered from the project so placements and clips can
/// be drawn and dragged the same way.
pub(super) struct Block<'a> {
    pub(super) kind: Kind,
    pub(super) track: usize,
    /// Index in the track's placements or clips.
    pub(super) index: usize,
    /// In beats.
    pub(super) start: f64,
    pub(super) length: f64,
    pub(super) content: Content<'a>,
}

/// What a block shows inside.
pub(super) enum Content<'a> {
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

/// How a block is drawn.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Look {
    Normal,
    /// Where a dragged block was picked up from.
    Ghost,
    /// Where a dragged block would land.
    Dragged,
}

/// Every placement and clip in the project, as blocks.
pub(super) fn gather(project: &Project, samples_per_beat: f64) -> Vec<Block<'_>> {
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

/// Where a block starting at `start` sits in `lane`.
pub(super) fn block_rect(grid: &Grid, lane: Rect, start: f64, length: f64) -> Rect {
    Rect::from_x_y_ranges(
        grid.x(start)..=grid.x(start + length),
        lane.y_range().shrink(BLOCK_INSET),
    )
}

/// A block as long as what it plays, named, with its notes drawn inside like
/// a tiny piano roll or its waveform.
pub(super) fn draw_block(
    painter: &Painter,
    grid: &Grid,
    rect: Rect,
    content: &Content,
    look: Look,
) {
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
