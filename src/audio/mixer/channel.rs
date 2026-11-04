use std::{
    f32::consts::PI,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use crate::{
    atomic::AtomicF32,
    audio::{frame::Frame, mixer::meter::Meter, source::MAX_BLOCK_FRAMES},
};

/// Gain and pan a new channel starts with.
pub const DEFAULT_GAIN: f32 = 0.5;
pub const DEFAULT_PAN: f32 = 0.0;

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
    pub pan: AtomicF32,
}

pub struct Channel {
    input: Vec<f32>,
    params: Arc<ChannelParams>,
    gain: f32,
    pan_l: f32,
    pan_r: f32,
    peak: f32,
}

impl Channel {
    /// Creates a channel with a fresh id, together with the params the UI keeps.
    pub fn create(gain: f32) -> (Self, Arc<ChannelParams>) {
        let params = Arc::new(ChannelParams {
            id: ChannelId::next(),
            gain: AtomicF32::new(gain),
            meter: Meter::new(),
            pan: AtomicF32::new(DEFAULT_PAN),
        });

        (Self::new(Arc::clone(&params)), params)
    }

    pub fn new(params: Arc<ChannelParams>) -> Self {
        let gain = params.gain.get();
        let (pan_l, pan_r) = Self::pan_gains(params.pan.get());
        Channel {
            input: vec![0.0; MAX_BLOCK_FRAMES],
            params,
            gain,
            peak: 0.0,
            pan_l,
            pan_r,
        }
    }

    pub fn id(&self) -> ChannelId {
        self.params.id
    }

    /// Adds `samples` into the input, starting `offset` frames into the block.
    pub fn add_input(&mut self, offset: usize, samples: &[f32]) {
        for (input, sample) in self.input[offset..].iter_mut().zip(samples) {
            *input += sample;
        }
    }

    pub fn begin_buffer(&mut self) {
        self.params.meter.record(self.peak);
        self.peak = 0.0;

        self.gain = self.params.gain.get();

        let pan = self.params.pan.get();
        let (pan_l, pan_r) = Self::pan_gains(pan);
        self.pan_l = pan_l;
        self.pan_r = pan_r;
    }

    /// Adds the input, gained and panned, into `out`, and clears it.
    pub fn add_to(&mut self, out: &mut [Frame]) {
        for (frame, input) in out.iter_mut().zip(&mut self.input) {
            let mono = *input * self.gain;
            let left = mono * self.pan_l;
            let right = mono * self.pan_r;

            frame.left += left;
            frame.right += right;
            self.peak = self.peak.max(left.abs().max(right.abs()));
            *input = 0.0;
        }
    }

    fn pan_gains(pan: f32) -> (f32, f32) {
        let out = (pan + 1.0) * PI / 4.0;
        (out.cos(), out.sin())
    }
}

const NO_CHANNEL: u32 = u32::MAX;

/// Which channel something plays into, by id. The UI changes it while audio
/// runs; the studio resolves it to a mixer index once per buffer.
pub struct ChannelRoute(AtomicU32);

impl ChannelRoute {
    pub fn new(channel: Option<ChannelId>) -> Self {
        Self(AtomicU32::new(
            channel.map_or(NO_CHANNEL, ChannelId::to_raw),
        ))
    }

    pub fn set_channel(&self, channel: Option<ChannelId>) {
        self.0.store(
            channel.map_or(NO_CHANNEL, ChannelId::to_raw),
            Ordering::Relaxed,
        );
    }

    pub fn get_channel(&self) -> Option<ChannelId> {
        match self.0.load(Ordering::Relaxed) {
            NO_CHANNEL => None,
            raw => Some(ChannelId::from_raw(raw)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_1_SQRT_2;

    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn centre_is_minus_3db_and_extremes_are_one_sided() {
        let (l, r) = Channel::pan_gains(0.0);
        assert!(close(l, FRAC_1_SQRT_2) && close(r, FRAC_1_SQRT_2));

        let (l, r) = Channel::pan_gains(-1.0);
        assert!(close(l, 1.0) && close(r, 0.0));

        let (l, r) = Channel::pan_gains(1.0);
        assert!(close(l, 0.0) && close(r, 1.0));
    }

    #[test]
    fn power_stays_constant_across_the_sweep() {
        for pan in [-1.0, -0.6, -0.25, 0.0, 0.3, 0.75, 1.0] {
            let (l, r) = Channel::pan_gains(pan);
            assert!(close(l * l + r * r, 1.0), "pan {pan}: {}", l * l + r * r);
        }
    }
}
