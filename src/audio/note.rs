/// Frequency in Hz of a MIDI note number, with A4 (69) at 440 Hz.
pub fn midi_to_freq(note: u8) -> f32 {
    440.0 * 2f32.powf((note as f32 - 69.0) / 12.0)
}

/// A key pressed or released, by MIDI note number.
#[derive(Clone, Copy, Debug)]
pub enum NoteEvent {
    On(u8),
    Off(u8),
}
