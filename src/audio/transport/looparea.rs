use std::sync::Arc;

use super::sample_at;
use crate::atomic::AtomicF64;

pub struct LoopAreaParams {
    pub loop_start: AtomicF64,
    pub loop_end: AtomicF64,
}

impl LoopAreaParams {
    pub fn new() -> Self {
        Self {
            loop_start: AtomicF64::new(0.0),
            loop_end: AtomicF64::new(0.0),
        }
    }

    pub fn set(&self, start: f64, end: f64) {
        if start > end {
            self.loop_start.set(0.0);
            self.loop_end.set(0.0);
        }

        self.loop_start.set(start);
        self.loop_end.set(end);
    }

    pub fn get(&self) -> (f64, f64) {
        (self.loop_start.get(), self.loop_end.get())
    }
}

pub struct LoopArea {
    params: Arc<LoopAreaParams>,

    /// Loop start in beats
    loop_start_beat: f64,
    /// Loop start in samples/frames
    loop_start: u64,
    /// Loop end in samples/frames
    loop_end: u64,
}

impl LoopArea {
    pub fn new(params: Arc<LoopAreaParams>) -> Self {
        Self {
            params,
            loop_start: 0,
            loop_end: 0,
            loop_start_beat: 0.0,
        }
    }

    pub fn begin_buffer(&mut self, bpm: f64, sample_rate: f64) {
        self.loop_start_beat = self.params.loop_start.get();
        self.loop_start = sample_at(bpm, sample_rate, self.loop_start_beat);
        self.loop_end = sample_at(bpm, sample_rate, self.params.loop_end.get());
    }

    pub fn get_start(&self) -> u64 {
        self.loop_start
    }

    pub fn get_start_beat(&self) -> f64 {
        self.loop_start_beat
    }

    pub fn is_valid(&self) -> bool {
        self.loop_end > self.loop_start
    }

    pub fn is_after_end(&self, position: u64) -> bool {
        position >= self.loop_end
    }

    pub fn frames_to_end(&self, position: u64) -> u64 {
        self.loop_end - position
    }
}
