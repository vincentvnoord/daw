use eframe::egui::{self, CornerRadius, Popup, RichText, Sense, Stroke, StrokeKind};

use crate::{
    audio::studio::studio::MAX_INSTRUMENTS,
    shared::{AppParams, Track},
    ui::{
        theme,
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
}

/// Lists the tracks, one row each: a header with the name, the instrument
/// type and the channel it plays into, and the lane the timeline will draw
/// into. Clicking a header selects the track: the keyboard plays it and the
/// editor shows its instrument. A last card with a + adds a track.
pub fn show(ui: &mut egui::Ui, params: &AppParams, active: usize) -> Option<TrackAction> {
    let mut action = None;

    section_label(ui, "TRACKS");

    for (i, track) in params.tracks.iter().enumerate() {
        let is_active = i == active;
        let click = ui
            .horizontal(|ui| {
                let click = ui
                    .vertical(|ui| {
                        ui.set_width(HEADER_WIDTH);
                        header(ui, i, track, params, is_active)
                    })
                    .inner;
                lane(ui, click.rect.height());
                click
            })
            .inner;

        if click.clicked() && !is_active {
            action = Some(TrackAction::Select(i));
        }
    }

    if params.tracks.len() < MAX_INSTRUMENTS {
        ui.vertical(|ui| {
            ui.set_width(HEADER_WIDTH);
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
        });
    }

    action
}

fn header(
    ui: &mut egui::Ui,
    index: usize,
    track: &Track,
    params: &AppParams,
    selected: bool,
) -> egui::Response {
    let output = track
        .instrument
        .params
        .route
        .get_channel()
        .and_then(|id| params.channel(id))
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

/// The empty strip right of a header, where the track's placements will go.
fn lane(ui: &mut egui::Ui, height: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(theme::RADIUS),
        theme::PANEL,
        Stroke::new(1.0, theme::BORDER),
        StrokeKind::Inside,
    );
}
