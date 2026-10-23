use std::sync::Arc;

use crate::audio::{osc::AtomicWaveForm, source::AudioSource, voice::Voice};
use crate::events::NoteEvent;
use crate::helpers::AtomicF32;

pub struct SynthParams {
    pub waveform: AtomicWaveForm,
    pub cutoff: AtomicF32,
    pub attack: AtomicF32,
    pub decay: AtomicF32,
    pub sustain: AtomicF32,
    pub release: AtomicF32,
}

pub struct Synth {
    params: Arc<SynthParams>,
    voices: Vec<Voice>,
    ages: Vec<u64>,
    clock: u64,
}

impl Synth {
    pub fn new(voices: Vec<Voice>, params: Arc<SynthParams>) -> Self {
        let n = voices.len();
        Self {
            clock: 0,
            ages: vec![0; n],
            voices,
            params,
        }
    }

    pub fn handle(&mut self, event: NoteEvent) {
        match event {
            NoteEvent::On(note) => {
                let i = self.find_voice_for(note);
                self.clock += 1;
                self.ages[i] = self.clock;
                self.voices[i].note_on(note);
            }
            NoteEvent::Off(note) => {
                for v in &mut self.voices {
                    if v.is_held() && v.note() == note {
                        v.note_off();
                    }
                }
            }
        }
    }

    fn find_voice_for(&self, note: u8) -> usize {
        let mut first_idle = None;
        let mut oldest = 0;

        for (i, v) in self.voices.iter().enumerate() {
            if !v.is_idle() && v.note() == note {
                return i;
            }

            if first_idle.is_none() && v.is_idle() {
                first_idle = Some(i);
            }

            if self.ages[i] < self.ages[oldest] {
                oldest = i;
            }
        }

        first_idle.unwrap_or(oldest)
    }
}

impl AudioSource for Synth {
    fn begin_buffer(&mut self) {
        let p = &self.params;
        let waveform = p.waveform.get();
        let cutoff = p.cutoff.get();
        let attack = p.attack.get();
        let decay = p.decay.get();
        let sustain = p.sustain.get();
        let release = p.release.get();

        for v in &mut self.voices {
            v.set_waveform(waveform);
            v.set_cutoff(cutoff);
            v.set_adsr(attack, decay, sustain, release);
        }
    }

    fn next_sample(&mut self) -> f32 {
        self.voices.iter_mut().map(|v| v.next_sample()).sum()
    }
}
