mod header;
pub mod timeline;

use eframe::egui::{self, Sense};

use crate::{
    audio::studio::MAX_INSTRUMENTS,
    project::Project,
    ui::{
        tracks::{
            header::{empty_header, header},
            timeline::{RULER_HEIGHT, Timeline},
        },
        widgets::list::section_label,
    },
};

/// Width of the column with the track headers, left of the lanes.
const HEADER_WIDTH: f32 = 200.0;

/// What the user did in the track list.
pub enum TrackAction {
    Select(usize),
    AddSynth,
    AddSample,
    AddAudio,
    /// A placement was dropped somewhere new.
    MovePlacement {
        from: usize,
        index: usize,
        to: usize,
        start: f64,
    },
    /// A clip was dropped somewhere new.
    MoveClip {
        from: usize,
        index: usize,
        /// The track it was dropped on, or `None` for the empty track at the
        /// bottom, which becomes a new audio track.
        to: Option<usize>,
        start: f64,
    },
    /// A placement was double-clicked: open its sequence in the piano roll.
    OpenSequence {
        track: usize,
        sequence: usize,
    },
}

/// The tracks with the timeline: a column of headers with the name, the
/// instrument type and the channel each track plays into, and next to it the
/// lanes with what each track plays. Clicking a header selects the track:
/// the keyboard plays it and the editor shows its instrument. Below the last
/// track is an empty one, which isn't in the project: clicking its header
/// picks a source for it, and dropping a clip on its lane makes it an audio
/// track.
pub fn show(
    ui: &mut egui::Ui,
    project: &Project,
    active: usize,
    timeline: &mut Timeline,
) -> Option<TrackAction> {
    let mut action = None;

    section_label(ui, "TRACKS");

    ui.horizontal_top(|ui| {
        let mut row_heights = Vec::with_capacity(project.tracks.len());

        ui.vertical(|ui| {
            ui.set_width(HEADER_WIDTH);
            // level with the ruler, so each header sits next to its lane
            ui.allocate_exact_size(egui::vec2(HEADER_WIDTH, RULER_HEIGHT), Sense::hover());

            for (i, track) in project.tracks.iter().enumerate() {
                let is_active = i == active;
                let click = header(ui, i, track, project, is_active);
                row_heights.push(click.rect.height());

                if click.clicked() && !is_active {
                    action = Some(TrackAction::Select(i));
                }
            }

            // its height goes in too, so the timeline draws a lane next to it
            if project.tracks.len() < MAX_INSTRUMENTS {
                let (height, added) = empty_header(ui);
                row_heights.push(height);
                if let Some(added) = added {
                    action = Some(added);
                }
            }
        });

        if let Some(edited) = timeline.show(ui, project, &row_heights) {
            action = Some(edited);
        }
    });

    action
}
