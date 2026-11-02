use eframe::egui::{self, Popup, RichText, Sense};

use crate::{
    audio::studio::MAX_INSTRUMENTS,
    project::{Project, Track},
    ui::{
        theme,
        timeline::{RULER_HEIGHT, Timeline},
        widgets::list::{add_card, list_card, menu_frame, menu_option, section_label},
    },
};

/// Width of the column with the track headers, left of the lanes.
const HEADER_WIDTH: f32 = 200.0;

/// What the user did in the track list.
pub enum TrackAction {
    Select(usize),
    AddSynth,
    AddSample,
    /// A placement was dropped somewhere new.
    MovePlacement {
        from: usize,
        index: usize,
        to: usize,
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
/// the keyboard plays it and the editor shows its instrument. A last card
/// with a + adds a track.
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

            if project.tracks.len() < MAX_INSTRUMENTS
                && let Some(added) = add_menu(ui)
            {
                action = Some(added);
            }
        });

        if let Some(edited) = timeline.show(ui, project, &row_heights) {
            action = Some(edited);
        }
    });

    action
}

/// The + card under the headers, with its menu of track types.
fn add_menu(ui: &mut egui::Ui) -> Option<TrackAction> {
    let mut action = None;
    let add = add_card(ui, "add_track").on_hover_text("Add track");

    Popup::menu(&add)
        .width(add.rect.width())
        .gap(4.0)
        .frame(menu_frame())
        .show(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            if menu_option(ui, "Synth", "Oscillator, filter and envelope") {
                action = Some(TrackAction::AddSynth);
                ui.close();
            }
            if menu_option(ui, "Sample", "Play a WAV file from disk") {
                action = Some(TrackAction::AddSample);
                ui.close();
            }
        });

    action
}

fn header(
    ui: &mut egui::Ui,
    index: usize,
    track: &Track,
    project: &Project,
    selected: bool,
) -> egui::Response {
    let output = track
        .instrument
        .params
        .route
        .get_channel()
        .and_then(|id| project.channel(id))
        .map_or("no output", |c| c.name.as_str());

    list_card(ui, ("track", index), selected, |ui| {
        ui.label(RichText::new(&track.name).strong());
        let details = match track.kind_label() {
            Some(kind) => format!("{kind} · → {output}"),
            None => format!("→ {output}"),
        };
        ui.label(
            RichText::new(details)
                .font(theme::value_font())
                .color(theme::TEXT_DIM),
        );
    })
}
