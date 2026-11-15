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

#[derive(Clone, Copy)]
struct Range {
    start: u64,
    end: u64,
    start_beat: f64,
}

pub struct LoopArea {
    params: Arc<LoopAreaParams>,
    range: Option<Range>,
}

impl LoopArea {
    pub fn new(params: Arc<LoopAreaParams>) -> Self {
        Self {
            params,
            range: None,
        }
    }

    pub fn begin_buffer(&mut self, bpm: f64, sample_rate: f64) {
        let (start_beat, end_beat) = self.params.get();
        let start = sample_at(bpm, sample_rate, start_beat);
        let end = sample_at(bpm, sample_rate, end_beat);
        self.range = (end > start).then_some(Range {
            start,
            end,
            start_beat,
        });
    }

    pub fn wrap(&self, position: u64) -> Option<(u64, f64)> {
        let range = self.range?;
        (position >= range.end).then_some((range.start, range.start_beat))
    }

    pub fn frames_left(&self, position: u64) -> Option<u64> {
        let range = self.range?;
        Some(range.end.saturating_sub(position))
    }
}
