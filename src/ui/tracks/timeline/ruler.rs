use eframe::egui::{self, Rect, Sense, vec2};

use crate::{
    project::Project,
    ui::tracks::{
        RULER_HEIGHT,
        timeline::{Grid, ruler},
    },
};

struct Selection {
    start: f64,
    end: f64,
}

#[derive(Default)]
pub struct Ruler {
    selection: Option<Selection>,
}

impl Ruler {
    pub fn show(&mut self, ui: &mut egui::Ui, project: &Project, grid: &Grid, width: f32) -> Rect {
        let (rect, response) = ui.allocate_exact_size(vec2(width, RULER_HEIGHT), Sense::click());

        ruler(ui.painter(), &grid, rect);

        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let beat = grid.beat(pos.x);
            project.transport.seek(beat);
        }

        rect
    }
}
