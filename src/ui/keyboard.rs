use eframe::egui;

use crate::events::NoteEvent;

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

pub fn key_to_note(key: egui::Key) -> Option<u8> {
    use egui::Key::*;
    let note = match key {
        // lower octave: bottom row = white keys, home row = black keys
        Z => 48, // C3
        S => 49, // C#3
        X => 50, // D3
        D => 51, // D#3
        C => 52, // E3
        V => 53, // F3
        G => 54, // F#3
        B => 55, // G3
        H => 56, // G#3
        N => 57, // A3
        J => 58, // A#3
        M => 59, // B3

        // upper octave: letter row = white keys, number row = black keys
        Q => 60,    // C4 (middle C)
        Num2 => 61, // C#4
        W => 62,    // D4
        Num3 => 63, // D#4
        E => 64,    // E4
        R => 65,    // F4
        Num5 => 66, // F#4
        T => 67,    // G4
        Num6 => 68, // G#4
        Y => 69,    // A4 (440 Hz)
        Num7 => 70, // A#4
        U => 71,    // B4
        I => 72,    // C5
        Num9 => 73, // C#5
        O => 74,    // D5
        Num0 => 75, // D#5
        P => 76,    // E5

        _ => return None,
    };
    Some(note)
}
