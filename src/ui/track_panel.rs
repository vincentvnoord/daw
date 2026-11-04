use std::sync::atomic::Ordering;

use eframe::egui::{self, Popup, Rect, RichText, Sense, UiBuilder, pos2};

use crate::{
    audio::studio::MAX_INSTRUMENTS,
    project::{Project, Track},
    ui::{
        theme,
        timeline::{RULER_HEIGHT, Timeline},
        widgets::{
            icon_button::{Icon, IconButton},
            list::{list_card, menu_frame, menu_option, section_label},
        },
    },
};

/// Width of the column with the track headers, left of the lanes.
const HEADER_WIDTH: f32 = 200.0;
/// Gap between an audio track's mute button and the card's right edge.
const MUTE_MARGIN: f32 = 8.0;
/// Width the card's text gives up for the mute button.
const MUTE_ROOM: f32 = IconButton::SIZE.x + MUTE_MARGIN;

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

/// The empty track's header under the others, with its menu of sources.
/// Returns its height and what was picked, if anything.
fn empty_header(ui: &mut egui::Ui) -> (f32, Option<TrackAction>) {
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

fn header(
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
