use crate::audio::voice::Voice;

#[derive(Clone, Copy)]
pub struct Note {
    pub note: u8,
    pub start: f32,
    pub length: f32,
}

pub struct SequencePlayer {
    events: Vec<Note>,
    position: u32,    // in samples
    loop_length: u32, // in samples
    sample_rate: f32,
}

impl SequencePlayer {
    pub fn new(events: Vec<Note>, loop_seconds: f32, sample_rate: f32) -> SequencePlayer {
        SequencePlayer {
            position: 0,
            events,
            loop_length: (loop_seconds * sample_rate) as u32,
            sample_rate,
        }
    }

    pub fn tick(&mut self, voice: &mut Voice) {
        for event in &self.events {
            let start = (event.start * self.sample_rate) as u32;
            let end = ((event.start + event.length) * self.sample_rate) as u32;

            if self.position == start {
                voice.note_on(event.note);
            }

            if self.position == end {
                voice.note_off();
            }
        }

        self.position += 1;
        if self.position >= self.loop_length {
            self.position = 0;
        }
    }
}
