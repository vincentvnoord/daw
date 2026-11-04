use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

use rtrb::{Consumer, Producer, RingBuffer};

use crate::{
    atomic::AtomicF32,
    audio::{
        mixer::channel::{ChannelId, ChannelRoute},
        source::AudioSource,
        studio::clips::Clips,
        transport::Transport,
    },
};

/// Most an input can be boosted: +24 dB, enough to bring a guitar or mic
/// up to the level of the instruments.
pub const MAX_TRIM: f32 = 15.85;

pub struct InputParams {
    /// Whether the track is silenced: its live input and its clips. The
    /// input is heard through the channel by default; muting is how you stop
    /// it.
    pub muted: AtomicBool,
    /// Gain applied to the input before it reaches its channel, from 1.0
    /// (as recorded) to `MAX_TRIM`.
    pub trim: AtomicF32,
    pub route: ChannelRoute,
    /// Input samples waiting when the last buffer started, for the latency log.
    pub backlog: AtomicU32,
    /// Samples the input ran dry on since it was opened, for the latency log.
    pub underruns: AtomicU32,
}

impl InputParams {
    pub fn new(channel: Option<ChannelId>) -> Self {
        Self {
            muted: AtomicBool::new(false),
            trim: AtomicF32::new(1.0),
            route: ChannelRoute::new(channel),
            backlog: AtomicU32::new(0),
            underruns: AtomicU32::new(0),
        }
    }
}

/// An audio track in the studio: the live input it monitors, if it has one,
/// and its clips, playing into the track's channel.
pub struct InputSlot {
    /// Samples from the recording device, or `None` with no input chosen.
    samples: Option<Consumer<f32>>,
    clips: Clips,
    pub output: Option<usize>,
    /// The trim, or 0 while muted. Read once per buffer.
    gain: f32,
    muted: bool,
    /// Samples the input ran dry on, copied to the params once per buffer.
    underruns: u32,
    /// Whether the input has sent anything yet. Before that, running dry is
    /// the stream still starting up, not an underrun.
    started: bool,
    pub params: Arc<InputParams>,
}

/// How many output buffers' worth of input may wait: one to play from, and
/// one of slack for an input chunk that arrives late. Every sample waiting
/// is one sample of delay, so anything past this is thrown away.
const BACKLOG_BUFFERS: usize = 2;
const INPUT_QUEUE_SIZE: usize = 8192;

/// A queue for a recording device's samples: the device's stream pushes into
/// the producer, the slot reads from the consumer.
pub fn input_queue() -> (Producer<f32>, Consumer<f32>) {
    RingBuffer::new(INPUT_QUEUE_SIZE)
}

impl InputSlot {
    /// A slot with no input yet, routed to `channel`. Returns the slot and
    /// the params for the UI.
    pub fn create(channel: Option<ChannelId>) -> (Self, Arc<InputParams>) {
        let params = Arc::new(InputParams::new(channel));
        let slot = Self {
            samples: None,
            clips: Clips::empty(),
            output: None,
            gain: 0.0,
            muted: false,
            underruns: 0,
            started: false,
            params: Arc::clone(&params),
        };

        (slot, params)
    }

    /// Switches to another input, or to none. Returns the old one's queue, to
    /// be dropped off the audio thread.
    pub fn set_input(&mut self, samples: Option<Consumer<f32>>) -> Option<Consumer<f32>> {
        self.underruns = 0;
        self.started = false;
        std::mem::replace(&mut self.samples, samples)
    }

    /// Replaces the track's clips. Returns the old ones, to be dropped off
    /// the audio thread.
    pub fn set_clips(&mut self, clips: Clips) -> Clips {
        std::mem::replace(&mut self.clips, clips)
    }

    /// Works out where the clips start at the transport's tempo. Once per
    /// buffer, after the transport.
    pub fn prepare_clips(&mut self, transport: &Transport) {
        self.clips.prepare(transport);
    }

    /// The clips at sample `position` of the transport, or silence while
    /// muted. Clips play as they are: the trim is for the live input.
    pub fn clip_sample(&self, position: u64) -> f32 {
        if self.muted {
            0.0
        } else {
            self.clips.sample_at(position)
        }
    }

    /// Picks up the trim and mute, and throws away input that has
    /// piled up past what a buffer of `frames` frames needs.
    pub fn prepare(&mut self, frames: usize) {
        self.muted = self.params.muted.load(Ordering::Relaxed);
        self.gain = if self.muted {
            0.0
        } else {
            self.params.trim.get()
        };

        let mut backlog = 0;
        if let Some(samples) = &mut self.samples {
            let max = frames * BACKLOG_BUFFERS;
            backlog = samples.slots();
            if backlog > max
                && let Ok(chunk) = samples.read_chunk(backlog - max)
            {
                chunk.commit_all();
                backlog = max;
            }
        }

        self.params.backlog.store(backlog as u32, Ordering::Relaxed);
        self.params
            .underruns
            .store(self.underruns, Ordering::Relaxed);
    }
}

impl AudioSource for InputSlot {
    fn next_sample(&mut self) -> f32 {
        let Some(samples) = &mut self.samples else {
            return 0.0;
        };

        // drained even when muted, so unmuting doesn't play old audio
        match samples.pop() {
            Ok(sample) => {
                self.started = true;
                sample * self.gain
            }
            Err(_) => {
                if self.started {
                    self.underruns = self.underruns.wrapping_add(1);
                }
                0.0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A slot with `queued` samples waiting, numbered from 0.
    fn slot_with(queued: usize) -> InputSlot {
        let (mut slot, _) = InputSlot::create(None);
        let (mut producer, consumer) = input_queue();
        for i in 0..queued {
            producer.push(i as f32).unwrap();
        }
        slot.set_input(Some(consumer));
        slot
    }

    #[test]
    fn keeps_at_most_two_buffers_waiting() {
        let mut slot = slot_with(2000);
        slot.prepare(256);

        assert_eq!(slot.params.backlog.load(Ordering::Relaxed), 512);
    }

    #[test]
    fn throws_away_the_oldest_input() {
        let mut slot = slot_with(2000);
        slot.prepare(256);

        assert_eq!(slot.next_sample(), (2000 - 512) as f32);
    }

    #[test]
    fn a_short_backlog_is_left_alone() {
        let mut slot = slot_with(300);
        slot.prepare(256);

        assert_eq!(slot.params.backlog.load(Ordering::Relaxed), 300);
        assert_eq!(slot.next_sample(), 0.0);
    }

    #[test]
    fn running_dry_counts_as_underruns() {
        let mut slot = slot_with(2);
        slot.prepare(256);
        for _ in 0..5 {
            slot.next_sample();
        }
        slot.prepare(256);

        assert_eq!(slot.params.underruns.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn running_dry_before_the_input_starts_is_not_an_underrun() {
        let mut slot = slot_with(0);
        slot.prepare(256);
        for _ in 0..5 {
            slot.next_sample();
        }
        slot.prepare(256);

        assert_eq!(slot.params.underruns.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn no_input_is_silent_and_never_runs_dry() {
        let (mut slot, _) = InputSlot::create(None);
        slot.prepare(256);

        assert_eq!(slot.next_sample(), 0.0);
        slot.prepare(256);
        assert_eq!(slot.params.underruns.load(Ordering::Relaxed), 0);
    }
}
