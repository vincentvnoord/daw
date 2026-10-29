use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use crate::{audio::mixer::meter::Meter, helpers::AtomicF32};

/// Gain a new channel starts with.
pub const DEFAULT_GAIN: f32 = 0.5;

/// Identifies a channel for as long as it exists, wherever it sits in the
/// mixer. Instruments route by id, so removing a channel never shifts other
/// instruments onto the wrong one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChannelId(u32);

impl ChannelId {
    /// An id no other channel has.
    fn next() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    pub fn to_raw(self) -> u32 {
        self.0
    }

    pub fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
}

pub struct ChannelParams {
    pub id: ChannelId,
    pub gain: AtomicF32,
    pub meter: Meter,
}

pub struct Channel {
    input: f32,
    params: Arc<ChannelParams>,
    gain: f32,
    peak: f32,
}

impl Channel {
    /// Creates a channel with a fresh id, together with the params the UI keeps.
    pub fn create(gain: f32) -> (Self, Arc<ChannelParams>) {
        let params = Arc::new(ChannelParams {
            id: ChannelId::next(),
            gain: AtomicF32::new(gain),
            meter: Meter::new(),
        });

        (Self::new(Arc::clone(&params)), params)
    }

    pub fn new(params: Arc<ChannelParams>) -> Self {
        let gain = params.gain.get();
        Channel {
            input: 0.0,
            params,
            gain,
            peak: 0.0,
        }
    }

    pub fn id(&self) -> ChannelId {
        self.params.id
    }

    pub fn add_input(&mut self, sample: f32) {
        self.input += sample;
    }

    pub fn begin_buffer(&mut self) {
        self.params.meter.record(self.peak);
        self.peak = 0.0;

        self.gain = self.params.gain.get();
    }

    pub fn process(&mut self) -> f32 {
        let out = self.input * self.gain;
        self.input = 0.0;
        self.peak = self.peak.max(out.abs());

        out
    }
}
