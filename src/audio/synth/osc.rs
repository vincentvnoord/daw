use std::f32::consts::TAU;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WaveForm {
    Sine,
    Saw,
    Square,
}

impl WaveForm {
    pub const ALL: [WaveForm; 3] = [WaveForm::Sine, WaveForm::Saw, WaveForm::Square];

    pub fn name(self) -> &'static str {
        match self {
            WaveForm::Sine => "Sine",
            WaveForm::Saw => "Saw",
            WaveForm::Square => "Square",
        }
    }
}

/// A [`WaveForm`] that can be shared between the UI and audio threads.
pub struct AtomicWaveForm(AtomicU8);

impl AtomicWaveForm {
    pub fn new(value: WaveForm) -> Self {
        Self(AtomicU8::new(value as u8))
    }

    pub fn get(&self) -> WaveForm {
        match self.0.load(Ordering::Relaxed) {
            0 => WaveForm::Sine,
            1 => WaveForm::Saw,
            _ => WaveForm::Square,
        }
    }

    pub fn set(&self, value: WaveForm) {
        self.0.store(value as u8, Ordering::Relaxed);
    }
}

pub struct Oscillator {
    waveform: WaveForm,
    frequency: f32,
    sample_rate: f32,
    phase: f32,
}

impl Oscillator {
    pub fn new(waveform: WaveForm, frequency: f32, sample_rate: f32) -> Oscillator {
        Oscillator {
            waveform,
            frequency,
            sample_rate,
            phase: 0.0,
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        let value = match self.waveform {
            WaveForm::Sine => (self.phase * TAU).sin(),
            WaveForm::Saw => 2.0 * self.phase - 1.0,
            WaveForm::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        };

        self.phase += self.frequency / self.sample_rate;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        value
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }

    pub fn set_waveform(&mut self, waveform: WaveForm) {
        self.waveform = waveform;
    }
}
