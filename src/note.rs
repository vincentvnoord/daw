#[allow(dead_code)]
pub enum Note {
    C,
    Cs,
    D,
    Ds,
    E,
    F,
    Fs,
    G,
    Gs,
    A,
    As,
    B,
}

pub fn note_to_midi(note: Note, octave: i32) -> u8 {
    ((octave + 1) * 12 + note as i32) as u8
}

pub fn midi_to_freq(note: u8) -> f32 {
    440.0 * 2f32.powf((note as f32 - 69.0) / 12.0)
}

pub fn note_to_freq(note: Note, octave: i32) -> f32 {
    midi_to_freq(note_to_midi(note, octave))
}
