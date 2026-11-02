//! The UI's side of the studio: the project the user works on, holding
//! handles to everything running on the audio thread, and the methods that
//! change it while audio runs. Each kind of thing (channels, tracks,
//! instruments, inputs) has its own file with its handle and its part of
//! `impl Project`.

use std::sync::Arc;

use rtrb::{Consumer, Producer, RingBuffer};

use crate::{
    atomic::AtomicF32,
    audio::{
        mixer::{MixerParams, meter::Meter},
        schedule::{Schedule, ScheduledNote},
        studio::{Garbage, Studio, StudioCommand},
        transport::{TransportParams, metronome::MetronomeParams},
    },
};

mod channels;
pub mod default;
mod inputs;
mod instruments;
mod timeline;
mod tracks;

pub use channels::ChannelHandle;
pub use inputs::{InputDevice, InputHandle, input_devices};
pub use instruments::{InstrumentHandle, InstrumentKind};
pub use timeline::{Note, Placement, Sequence};
pub use tracks::Track;

/// How many changes from the UI can wait for the audio thread. The garbage
/// queue is the same size, so everything a batch of commands removes fits.
const COMMAND_QUEUE_SIZE: usize = 64;

/// Gain of the master fader in a new project.
const MASTER_GAIN: f32 = 0.5;

pub struct Project {
    /// Sample rate of the audio device, needed to build instruments.
    pub sample_rate: f32,
    pub mixer: Arc<MixerParams>,
    pub transport: Arc<TransportParams>,
    pub metronome: Arc<MetronomeParams>,
    /// The mixer channels, in mixer order.
    pub channels: Vec<ChannelHandle>,
    /// The tracks, top to bottom. Each owns one instrument.
    pub tracks: Vec<Track>,
    /// Every sequence in the project, placed or not.
    pub sequences: Vec<Sequence>,
    pub inputs: Vec<InputHandle>,
    /// Sends changes to the studio on the audio thread.
    pub commands: Producer<StudioCommand>,
    /// What the studio sends back to be dropped on this thread.
    pub garbage: Consumer<Garbage>,
}

impl Project {
    /// An empty project, and the studio that plays it. The studio goes to the
    /// audio engine; everything added to the project afterwards reaches it as
    /// a command.
    pub fn new(sample_rate: f32) -> (Self, Studio) {
        let mixer = Arc::new(MixerParams {
            gain: AtomicF32::new(MASTER_GAIN),
            meter: Meter::new(),
        });
        let transport = Arc::new(TransportParams::new());
        let metronome = Arc::new(MetronomeParams::new());
        let (commands, command_queue) = RingBuffer::new(COMMAND_QUEUE_SIZE);
        let (garbage_bin, garbage) = RingBuffer::new(COMMAND_QUEUE_SIZE);

        let studio = Studio::new(
            Arc::clone(&mixer),
            Arc::clone(&transport),
            Arc::clone(&metronome),
            command_queue,
            garbage_bin,
            sample_rate,
        );

        let project = Self {
            sample_rate,
            mixer,
            transport,
            metronome,
            channels: Vec::new(),
            tracks: Vec::new(),
            sequences: Vec::new(),
            inputs: Vec::new(),
            commands,
            garbage,
        };

        (project, studio)
    }

    /// Replaces the notes the studio plays along the transport. Sorting and
    /// allocating happen here, so the studio only swaps it in. Returns
    /// `false` when the command queue is full.
    pub fn set_schedule(&mut self, notes: Vec<ScheduledNote>) -> bool {
        self.commands
            .push(StudioCommand::SetSchedule(Schedule::new(notes)))
            .is_ok()
    }

    /// Drops what the studio is done with. Call once per frame.
    pub fn collect_garbage(&mut self) {
        while self.garbage.pop().is_ok() {}
    }
}

/// `base`, or `base 2`, `base 3`, ... when `taken` says it's in use.
fn unique_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_owned();
    }

    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|name| !taken(name))
        .expect("some number is free")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_name_is_kept() {
        assert_eq!(unique_name("Synth", |_| false), "Synth");
    }

    #[test]
    fn taken_name_gets_the_first_free_number() {
        let taken = ["Synth", "Synth 2"];
        assert_eq!(unique_name("Synth", |n| taken.contains(&n)), "Synth 3");
    }

    #[test]
    fn numbers_fill_gaps() {
        let taken = ["Synth", "Synth 3"];
        assert_eq!(unique_name("Synth", |n| taken.contains(&n)), "Synth 2");
    }
}
