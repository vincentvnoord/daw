//! The cards in the column left of the lanes: one per track, and the empty
//! track's under them.

use std::sync::atomic::Ordering;

use eframe::egui::{self, Popup, Rect, RichText, UiBuilder, pos2};

use crate::{
    project::{Project, Track},
    ui::{
        theme,
        tracks::TrackAction,
        widgets::{
            icon_button::{Icon, IconButton},
            list::{list_card, menu_frame, menu_option},
        },
    },
};

/// Gap between an audio track's mute button and the card's right edge.
const MUTE_MARGIN: f32 = 8.0;
/// Width the card's text gives up for the mute button.
const MUTE_ROOM: f32 = IconButton::SIZE.x + MUTE_MARGIN;

/// The empty track's header under the others, with its menu of sources.
/// Returns its height and what was picked, if anything.
pub(super) fn empty_header(ui: &mut egui::Ui) -> (f32, Option<TrackAction>) {
    let mut action = None;
    let add = list_card(ui, "empty_track", false, |ui| {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("+").size(20.0).color(theme::TEXT_DIM));
        });
    })
    .on_hover_text("Add track, or drop a clip on its lane");

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
            if menu_option(ui, "Audio", "Monitor and record a live input") {
                action = Some(TrackAction::AddAudio);
                ui.close();
            }
        });

    (add.rect.height(), action)
}

pub(super) fn header(
    ui: &mut egui::Ui,
    index: usize,
    track: &Track,
    project: &Project,
    selected: bool,
) -> egui::Response {
    let output = track
        .route()
        .get_channel()
        .and_then(|id| project.channel(id))
        .map_or("no output", |c| c.name.as_str());

    // an audio track names its input instead of its type, and lights up
    // while you can hear it
    let (details, color) = match (track.audio(), track.kind_label()) {
        (Some(audio), _) => {
            let input = audio.input.as_ref().map_or("no input", |i| i.name.as_str());
            let muted = audio.params.muted.load(Ordering::Relaxed);
            let color = if muted || audio.input.is_none() {
                theme::TEXT_DIM
            } else {
                theme::TEXT
            };
            (format!("{input} → {output}"), color)
        }
        (None, Some(kind)) => (format!("{kind} · → {output}"), theme::TEXT_DIM),
        (None, None) => (format!("→ {output}"), theme::TEXT_DIM),
    };

    let card = list_card(ui, ("track", index), selected, |ui| {
        if track.audio().is_some() {
            // leave room on the right for the mute button
            ui.set_max_width(ui.available_width() - MUTE_ROOM);
        }
        ui.label(RichText::new(&track.name).strong());
        ui.add(
            egui::Label::new(
                RichText::new(details)
                    .font(theme::value_font())
                    .color(color),
            )
            .truncate(),
        );
    });

    // placed over the card after it, so a click on the button mutes
    // instead of selecting the track. drawn in a child ui: `ui.put` would
    // move the cursor up to the button and the next card would overlap
    if let Some(audio) = track.audio() {
        let muted = audio.params.muted.load(Ordering::Relaxed);
        let size = IconButton::SIZE;
        let rect = Rect::from_min_size(
            pos2(
                card.rect.right() - MUTE_MARGIN - size.x,
                card.rect.center().y - size.y / 2.0,
            ),
            size,
        );
        let hover = if muted { "Unmute" } else { "Mute" };
        if ui
            .new_child(UiBuilder::new().max_rect(rect))
            .add(IconButton::new(Icon::Speaker).selected(muted))
            .on_hover_text(hover)
            .clicked()
        {
            audio.params.muted.fetch_xor(true, Ordering::Relaxed);
        }
    }

    card
}
