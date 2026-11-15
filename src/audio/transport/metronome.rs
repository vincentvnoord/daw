use std::{
    f32::consts::TAU,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{
    atomic::AtomicF32,
    audio::{frame::Frame, source::MAX_BLOCK_FRAMES, transport::Transport},
};

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
    clicks: Vec<f32>,
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
            clicks: vec![0.0; MAX_BLOCK_FRAMES],
        }
    }

    pub fn begin_buffer(&mut self) {
        self.enabled = self.params.enabled.load(Ordering::Relaxed);
        self.volume = self.params.volume.get();
    }

    /// Renders the clicks for len frames from offset into the block, which start at sample position
    /// of the transport
    pub fn render(&mut self, transport: &Transport, offset: usize, len: usize, position: u64) {
        if !self.enabled {
            return;
        }

        let clicks = &mut self.clicks[offset..offset + len];
        for (position, click) in (position..).zip(clicks) {
            if let Some(beat) = transport.beat_starting_at(position) {
                self.phase = 0.0;
                self.amplitude = 1.0;
                self.frequency = if beat % BEATS_PER_BAR == 0 {
                    DOWNBEAT_FREQUENCY
                } else {
                    BEAT_FREQUENCY
                };
            }

            *click = self.phase.sin() * self.amplitude * self.volume;

            self.phase = (self.phase + TAU * self.frequency / self.sample_rate) % TAU;
            self.amplitude *= self.decay;
        }
    }

    pub fn add_to(&mut self, out: &mut [Frame]) {
        for (frame, click) in out.iter_mut().zip(&mut self.clicks) {
            frame.left += *click;
            frame.right += *click;
            *click = 0.0;
        }
    }
}
