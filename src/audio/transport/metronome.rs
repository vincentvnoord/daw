use std::{
    f32::consts::TAU,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{audio::transport::Transport, helpers::AtomicF32};

pub const BEATS_PER_BAR: u64 = 4;
pub const DEFAULT_VOLUME: f32 = 0.3;

const BEAT_FREQUENCY: f32 = 1000.0;
const DOWNBEAT_FREQUENCY: f32 = 1500.0;
/// How long a click takes to fade to silence (-60 dB).
const CLICK_SECONDS: f32 = 0.05;

pub struct MetronomeParams {
    pub enabled: AtomicBool,
    pub volume: AtomicF32,
}

impl MetronomeParams {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            volume: AtomicF32::new(DEFAULT_VOLUME),
        }
    }
}

/// Clicks on every beat of the transport, higher on the first beat of a bar.
pub struct Metronome {
    params: Arc<MetronomeParams>,
    enabled: bool,
    volume: f32,
    phase: f32,
    frequency: f32,
    amplitude: f32,
    /// What the amplitude is multiplied by each sample to fade the click out.
    decay: f32,
    sample_rate: f32,
}

impl Metronome {
    pub fn new(params: Arc<MetronomeParams>, sample_rate: f32) -> Self {
        Self {
            params,
            enabled: false,
            volume: 0.0,
            phase: 0.0,
            frequency: BEAT_FREQUENCY,
            amplitude: 0.0,
            decay: 0.001_f32.powf(1.0 / (CLICK_SECONDS * sample_rate)),
            sample_rate,
        }
    }

    pub fn begin_buffer(&mut self) {
        self.enabled = self.params.enabled.load(Ordering::Relaxed);
        self.volume = self.params.volume.get();
    }

    /// The click's sample for the transport's current position. Call it
    /// before the transport advances, so a beat's first sample isn't missed.
    pub fn next_sample(&mut self, transport: &Transport) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if let Some(beat) = transport.beat_starting_now() {
            self.phase = 0.0;
            self.amplitude = 1.0;
            self.frequency = if beat % BEATS_PER_BAR == 0 {
                DOWNBEAT_FREQUENCY
            } else {
                BEAT_FREQUENCY
            };
        }

        let sample = self.phase.sin() * self.amplitude * self.volume;

        self.phase = (self.phase + TAU * self.frequency / self.sample_rate) % TAU;
        self.amplitude *= self.decay;

        sample
    }
}
