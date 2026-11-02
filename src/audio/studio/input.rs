use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rtrb::{Consumer, Producer, RingBuffer};

use crate::{
    atomic::AtomicF32,
    audio::{
        mixer::channel::{ChannelId, ChannelRoute},
        source::AudioSource,
    },
};

/// Most an input can be boosted: +24 dB, enough to bring a guitar or mic
/// up to the level of the instruments.
pub const MAX_TRIM: f32 = 15.85;

pub struct InputParams {
    pub enabled: AtomicBool,
    /// Gain applied to the input before it reaches its channel, from 1.0
    /// (as recorded) to `MAX_TRIM`.
    pub trim: AtomicF32,
    pub route: ChannelRoute,
}

impl InputParams {
    pub fn new(channel: Option<ChannelId>) -> Self {
        Self {
            enabled: AtomicBool::new(false),
            trim: AtomicF32::new(1.0),
            route: ChannelRoute::new(channel),
        }
    }
}

pub struct InputSlot {
    samples: Consumer<f32>,
    pub output: Option<usize>,
    /// The trim, or 0 while the input is off. Read once per buffer.
    gain: f32,
    pub params: Arc<InputParams>,
}

const MAX_BACKLOG: usize = 2048;
const INPUT_QUEUE_SIZE: usize = 8192;

impl InputSlot {
    pub fn create(channel: Option<ChannelId>) -> (Self, Producer<f32>, Arc<InputParams>) {
        let (producer, consumer) = RingBuffer::new(INPUT_QUEUE_SIZE);
        let params = Arc::new(InputParams::new(channel));
        let slot = Self {
            samples: consumer,
            output: None,
            gain: 0.0,
            params: Arc::clone(&params),
        };

        (slot, producer, params)
    }
}

impl AudioSource for InputSlot {
    fn begin_buffer(&mut self) {
        self.gain = if self.params.enabled.load(Ordering::Relaxed) {
            self.params.trim.get()
        } else {
            0.0
        };

        let backlog = self.samples.slots();
        if backlog > MAX_BACKLOG
            && let Ok(chunk) = self.samples.read_chunk(backlog - MAX_BACKLOG)
        {
            chunk.commit_all();
        }
    }

    fn next_sample(&mut self) -> f32 {
        // drained even when off, so turning it on doesn't play old audio
        self.samples.pop().unwrap_or(0.0) * self.gain
    }
}
