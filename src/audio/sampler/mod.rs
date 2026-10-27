use std::sync::Arc;

pub mod sample;

use crate::audio::{instrument::Instrument, source::AudioSource};

pub struct Sampler {
    voices: Vec<SampleVoice>,
    ages: Vec<u64>,
    clock: u64,
}

impl Sampler {
    pub fn new(sample: Arc<Vec<f32>>, voice_count: usize) -> Self {
        let voices = (0..voice_count)
            .map(|_| SampleVoice::new(Arc::clone(&sample)))
            .collect();

        Self {
            clock: 0,
            ages: vec![0; voice_count],
            voices,
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

impl Instrument for Sampler {
    fn note_on(&mut self, note: u8) {
        let i = self.find_voice_for(note);
        let voice = &mut self.voices[i];
        self.clock += 1;
        self.ages[i] = self.clock;
        voice.play(note);
    }

    fn note_off(&mut self, _note: u8) {}
}

impl AudioSource for Sampler {
    fn next_sample(&mut self) -> f32 {
        self.voices.iter_mut().map(|v| v.next_sample()).sum()
    }
}

pub struct SampleVoice {
    sample: Arc<Vec<f32>>,
    position: f64,
    note: u8,
}

impl SampleVoice {
    pub fn new(sample: Arc<Vec<f32>>) -> Self {
        let position = sample.len() as f64;

        Self {
            sample,
            position,
            note: 0,
        }
    }

    pub fn play(&mut self, note: u8) {
        self.note = note;
        self.position = 0.0;
    }

    pub fn is_idle(&self) -> bool {
        self.position as usize >= self.sample.len()
    }

    pub fn note(&self) -> u8 {
        self.note
    }
}

impl AudioSource for SampleVoice {
    fn next_sample(&mut self) -> f32 {
        if self.is_idle() {
            return 0.0;
        }

        let i = self.position as usize;
        let frac = (self.position - i as f64) as f32;

        let current = self.sample[i];
        let next = self.sample.get(i + 1).copied().unwrap_or(0.0);
        let out = current + (next - current) * frac;

        self.position += 1.0;

        out
    }
}
