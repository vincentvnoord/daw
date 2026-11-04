//! Live playing: the computer keyboard and the on-screen piano, both playing
//! the selected track's instrument.

mod keymap;

use std::collections::BTreeSet;

use eframe::egui;

use crate::{
    audio::note::NoteEvent,
    project::{Project, Track},
    ui::{
        keyboard::keymap::{NOTE_RANGE, key_to_note_event, note_to_key},
        widgets::piano::piano,
    },
};

/// The notes being held down, so they can be shown on the piano and
/// released when the keyboard moves to another track.
#[derive(Default)]
pub struct Keyboard {
    held_keys: BTreeSet<u8>,
    /// Note currently played by clicking the on-screen piano.
    mouse_note: Option<u8>,
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

    /// The on-screen piano, showing what's held and labelled with the
    /// computer keys. Clicking a key plays it on `track`.
    pub fn show(&mut self, ui: &mut egui::Ui, project: &mut Project, track: usize) {
        let mut held = self.held_keys.clone();
        held.extend(self.mouse_note);

        let note = piano(ui, NOTE_RANGE, &held, |n| {
            note_to_key(n).map(|k| k.name().to_owned())
        });
        self.set_mouse_note(note, project, track);
    }

    /// Releases every held note on `track`. For when the keyboard moves to
    /// another track, otherwise those notes would never get a note off.
    pub fn release_all(&mut self, project: &mut Project, track: usize) {
        for n in self.held_keys.iter().copied().chain(self.mouse_note) {
            send(project, track, NoteEvent::Off(n));
        }
        self.held_keys.clear();
        self.mouse_note = None;
    }

    fn set_mouse_note(&mut self, note: Option<u8>, project: &mut Project, track: usize) {
        if note == self.mouse_note {
            return;
        }
        if let Some(n) = self.mouse_note {
            send(project, track, NoteEvent::Off(n));
        }
        if let Some(n) = note {
            send(project, track, NoteEvent::On(n));
        }
        self.mouse_note = note;
    }
}

/// Sends a note to `track`'s instrument, if it has one.
fn send(project: &mut Project, track: usize, event: NoteEvent) {
    if let Some(instrument) = project.tracks.get_mut(track).and_then(Track::instrument_mut) {
        let _ = instrument.notes.push(event);
    }
}
