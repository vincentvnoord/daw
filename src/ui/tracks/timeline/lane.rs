//! What's drawn behind and over the blocks: the bar ruler, each track's
//! lane with its beat grid, and the playhead.

use std::sync::atomic::Ordering;

use eframe::egui::{Align2, CornerRadius, Painter, Rect, Stroke, StrokeKind, pos2};

use crate::{
    audio::transport::{beats_at, metronome::BEATS_PER_BAR},
    project::Project,
    ui::{theme, tracks::timeline::grid::Grid},
};

/// Bar numbers, counted from 1 like the transport display.
pub(super) fn ruler(painter: &Painter, grid: &Grid, rect: Rect) {
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
pub(super) fn lane(painter: &Painter, grid: &Grid, rect: Rect, empty: bool) {
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

/// A line where the transport is, across the ruler and every lane.
pub(super) fn playhead(painter: &Painter, grid: &Grid, rect: Rect, project: &Project) {
    let samples = project.transport.position.load(Ordering::Relaxed);
    let bpm = project.transport.bpm.get() as f64;
    let x = grid.x(beats_at(samples, bpm, project.sample_rate as f64));

    painter.vline(x, rect.y_range(), Stroke::new(1.5, theme::ACCENT));
}
