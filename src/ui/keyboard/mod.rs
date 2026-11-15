//! Live playing: the computer keyboard plays the selected track's instrument.

mod keymap;

use std::collections::BTreeSet;

use eframe::egui;

use crate::{
    audio::note::NoteEvent,
    project::{Project, Track},
    ui::keyboard::keymap::key_to_note_event,
};

/// The notes being held down, so they can be released when the keyboard
/// moves to another track.
#[derive(Default)]
pub struct Keyboard {
    held_keys: BTreeSet<u8>,
}

impl Keyboard {
    /// Plays the keys pressed and released this frame on `track`.
    pub fn handle_keys(&mut self, ui: &egui::Ui, project: &mut Project, track: usize) {
        let events: Vec<NoteEvent> =
            ui.input(|i| i.events.iter().filter_map(key_to_note_event).collect());

        for event in events {
            match event {
                NoteEvent::On(n) => {
                    self.held_keys.insert(n);
                }
                NoteEvent::Off(n) => {
                    self.held_keys.remove(&n);
                }
            }

            send(project, track, event);
        }
    }

    /// Releases every held note on `track`. For when the keyboard moves to
    /// another track, otherwise those notes would never get a note off.
    pub fn release_all(&mut self, project: &mut Project, track: usize) {
        for n in std::mem::take(&mut self.held_keys) {
            send(project, track, NoteEvent::Off(n));
        }
    }
}

/// Sends a note to `track`'s instrument, if it has one.
fn send(project: &mut Project, track: usize, event: NoteEvent) {
    if let Some(instrument) = project
        .tracks
        .get_mut(track)
        .and_then(Track::instrument_mut)
    {
        let _ = instrument.notes.push(event);
    }
}
