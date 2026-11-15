use eframe::egui::{self, CornerRadius, CursorIcon, Rect, Response, Sense, vec2};

use crate::{
    project::Project,
    ui::{
        theme,
        tracks::{
            RULER_HEIGHT,
            timeline::{Grid, drag::SNAP, ruler},
        },
    },
};

/// The strip along the top of the ruler that holds the loop.
const STRIP_HEIGHT: f32 = 10.0;
/// How close to an edge of the loop, in pixels, grabs that edge.
const EDGE: f32 = 6.0;
const MIN_LENGTH: f64 = SNAP;

/// What a drag on the loop strip does, picked where it started.
#[derive(Clone, Copy)]
enum LoopDrag {
    Start,
    End,
    /// Beats between the loop's start and where it was grabbed.
    Move {
        grab: f64,
    },
    /// Draws a new loop from `anchor` to the pointer, either way.
    Create {
        anchor: f64,
    },
}

/// The bar numbers, which seek on a click, with the loop strip above them.
#[derive(Default)]
pub struct Ruler {
    drag: Option<LoopDrag>,
}

impl Ruler {
    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, grid: &Grid, width: f32) -> Rect {
        let (rect, _) = ui.allocate_exact_size(vec2(width, RULER_HEIGHT), Sense::hover());
        let (strip, bars) = rect.split_top_bottom_at_y(rect.top() + STRIP_HEIGHT);

        self.loop_strip(ui, project, grid, strip);

        let response = ui.interact(bars, ui.scope_id().with("ruler"), Sense::click());
        ruler(ui.painter(), grid, bars);

        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            project.transport.seek(grid.beat(pos.x));
        }

        rect
    }

    fn loop_strip(&mut self, ui: &egui::Ui, project: &Project, grid: &Grid, strip: Rect) {
        let response = ui.interact(strip, ui.scope_id().with("loop strip"), Sense::drag());
        let snap = !ui.input(|i| i.modifiers.alt);
        let snapped = |beat: f64| {
            let beat = beat.max(0.0);
            if snap {
                (beat / SNAP).round() * SNAP
            } else {
                beat
            }
        };

        let (start, end) = project.transport.loop_area.get();

        if response.drag_started()
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            // the press, not the pointer now: a drag only starts once it has
            // moved a few pixels, which can already be off an edge
            self.drag = Some(zone(grid, start, end, origin.x, snapped));
        }

        if let Some(drag) = self.drag
            && let Some(pos) = response.interact_pointer_pos()
        {
            let beat = snapped(grid.beat(pos.x));
            let (start, end) = match drag {
                LoopDrag::Start => (beat.min(end - MIN_LENGTH).max(0.0), end),
                LoopDrag::End => (start, beat.max(start + MIN_LENGTH)),
                LoopDrag::Move { grab } => {
                    let moved = snapped(grid.beat(pos.x) - grab);
                    (moved, moved + (end - start))
                }
                LoopDrag::Create { anchor } => {
                    let (from, to) = (anchor.min(beat), anchor.max(beat));
                    (from, to.max(from + MIN_LENGTH))
                }
            };
            project.transport.loop_area.set(start, end);
        }

        if response.drag_stopped() {
            self.drag = None;
        }

        set_cursor(ui, &response, self.drag, grid, start, end);
        draw(
            ui,
            grid,
            strip,
            project,
            response.hovered() || self.drag.is_some(),
        );
    }
}

fn zone(grid: &Grid, start: f64, end: f64, x: f32, snapped: impl Fn(f64) -> f64) -> LoopDrag {
    let (left, right) = (grid.x(start), grid.x(end));
    let beat = grid.beat(x);

    if end <= start {
        LoopDrag::Create {
            anchor: snapped(beat),
        }
    } else if (x - left).abs() <= EDGE {
        LoopDrag::Start
    } else if (x - right).abs() <= EDGE {
        LoopDrag::End
    } else if (left..=right).contains(&x) {
        LoopDrag::Move { grab: beat - start }
    } else {
        LoopDrag::Create {
            anchor: snapped(beat),
        }
    }
}

fn set_cursor(
    ui: &egui::Ui,
    response: &Response,
    drag: Option<LoopDrag>,
    grid: &Grid,
    start: f64,
    end: f64,
) {
    let drag = match drag {
        Some(drag) => drag,
        None => match response.hover_pos() {
            Some(pos) => zone(grid, start, end, pos.x, |beat| beat),
            None => return,
        },
    };
    let cursor = match drag {
        LoopDrag::Start | LoopDrag::End => CursorIcon::ResizeHorizontal,
        LoopDrag::Move { .. } if response.dragged() => CursorIcon::Grabbing,
        LoopDrag::Move { .. } => CursorIcon::Grab,
        LoopDrag::Create { .. } => return,
    };
    ui.ctx().set_cursor_icon(cursor);
}

fn draw(ui: &egui::Ui, grid: &Grid, strip: Rect, project: &Project, active: bool) {
    let (start, end) = project.transport.loop_area.get();
    if end <= start {
        return;
    }

    let rect = Rect::from_x_y_ranges(grid.x(start)..=grid.x(end), strip.y_range().shrink(1.0));
    let color = if active {
        theme::PRESSED
    } else {
        theme::ACCENT_DIM
    };
    ui.painter().rect_filled(rect, CornerRadius::same(2), color);
}
