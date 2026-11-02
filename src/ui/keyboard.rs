use std::ops::RangeInclusive;

use eframe::egui::{self, Key};

use crate::audio::note::NoteEvent;

/// Notes reachable from the computer keyboard.
pub const NOTE_RANGE: RangeInclusive<u8> = 48..=76;

const KEYMAP: [(Key, u8); 29] = [
    // lower octave: bottom row = white keys, home row = black keys
    (Key::Z, 48), // C3
    (Key::S, 49), // C#3
    (Key::X, 50), // D3
    (Key::D, 51), // D#3
    (Key::C, 52), // E3
    (Key::V, 53), // F3
    (Key::G, 54), // F#3
    (Key::B, 55), // G3
    (Key::H, 56), // G#3
    (Key::N, 57), // A3
    (Key::J, 58), // A#3
    (Key::M, 59), // B3
    // upper octave: letter row = white keys, number row = black keys
    (Key::Q, 60),    // C4 (middle C)
    (Key::Num2, 61), // C#4
    (Key::W, 62),    // D4
    (Key::Num3, 63), // D#4
    (Key::E, 64),    // E4
    (Key::R, 65),    // F4
    (Key::Num5, 66), // F#4
    (Key::T, 67),    // G4
    (Key::Num6, 68), // G#4
    (Key::Y, 69),    // A4 (440 Hz)
    (Key::Num7, 70), // A#4
    (Key::U, 71),    // B4
    (Key::I, 72),    // C5
    (Key::Num9, 73), // C#5
    (Key::O, 74),    // D5
    (Key::Num0, 75), // D#5
    (Key::P, 76),    // E5
];

pub fn key_to_note_event(event: &egui::Event) -> Option<NoteEvent> {
    let egui::Event::Key {
        key,
        pressed,
        repeat,
        ..
    } = event
    else {
        return None;
    };
    if *repeat {
        return None;
    }
    let note = key_to_note(*key)?;
    Some(if *pressed {
        NoteEvent::On(note)
    } else {
        NoteEvent::Off(note)
    })
}

pub fn key_to_note(key: Key) -> Option<u8> {
    KEYMAP.iter().find(|(k, _)| *k == key).map(|(_, n)| *n)
}

pub fn note_to_key(note: u8) -> Option<Key> {
    KEYMAP.iter().find(|(_, n)| *n == note).map(|(k, _)| *k)
}
