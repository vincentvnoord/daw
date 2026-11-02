use std::sync::Arc;

pub mod envelope;
pub mod osc;
pub mod voice;

use crate::audio::dsp::filter::LowPassFilter;
use crate::audio::instrument::Instrument;

use crate::atomic::AtomicF32;
use crate::audio::source::AudioSource;
use crate::audio::synth::envelope::{Envelope, EnvelopeSettings};
use crate::audio::synth::osc::{AtomicWaveForm, Oscillator, WaveForm};
use crate::audio::synth::voice::Voice;

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

    pub fn default(sample_rate: f32) -> Self {
        let env_settings = EnvelopeSettings {
            attack: 0.005,
            decay: 0.5,
            sustain: 1.0,
            release: 0.8,
        };
        let waveform = WaveForm::Saw;
        let voices: Vec<Voice> = (0..8)
            .map(|_| {
                Voice::new(
                    Oscillator::new(waveform, 440.0, sample_rate),
                    Envelope::new(env_settings, sample_rate),
                    LowPassFilter::new(200.0, sample_rate),
                )
            })
            .collect();

        let params = Arc::new(SynthParams {
            waveform: AtomicWaveForm::new(waveform),
            cutoff: AtomicF32::new(500.0),
            attack: AtomicF32::new(env_settings.attack),
            decay: AtomicF32::new(env_settings.decay),
            sustain: AtomicF32::new(env_settings.sustain),
            release: AtomicF32::new(env_settings.release),
        });

        Synth::new(voices, params)
    }

    /// The params shared with the UI.
    pub fn params(&self) -> Arc<SynthParams> {
        Arc::clone(&self.params)
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

impl Instrument for Synth {
    fn note_on(&mut self, note: u8) {
        let i = self.find_voice_for(note);
        self.clock += 1;
        self.ages[i] = self.clock;
        self.voices[i].note_on(note);
    }

    fn note_off(&mut self, note: u8) {
        for v in &mut self.voices {
            if v.is_held() && v.note() == note {
                v.note_off();
            }
        }
    }
}
