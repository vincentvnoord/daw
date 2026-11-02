pub mod clips;
pub mod input;
pub mod instrument;

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use rtrb::{Consumer, Producer};

use crate::audio::{
    frame::Frame,
    mixer::{
        Mixer, MixerParams,
        channel::{Channel, ChannelId},
    },
    note::NoteEvent,
    schedule::Schedule,
    source::{AudioSource, StereoSource},
    studio::{clips::Clips, input::InputSlot, instrument::InstrumentSlot},
    transport::{
        Transport, TransportParams,
        metronome::{Metronome, MetronomeParams},
    },
};

/// Most instruments and inputs the studio can hold. Space for all of them is
/// reserved up front, so adding one while audio runs never allocates on the
/// audio thread.
pub const MAX_INSTRUMENTS: usize = 64;
pub const MAX_INPUTS: usize = 32;

/// What the studio reports back about itself, for the latency log.
#[derive(Default)]
pub struct StudioStats {
    /// Frames in the last buffer the output asked for.
    pub buffer_frames: AtomicU32,
}

/// Changes the UI asks the studio to make while the audio is running.
pub enum StudioCommand {
    AddChannel(Channel),
    RemoveChannel(ChannelId),
    AddInstrument(InstrumentSlot),
    AddInput(InputSlot),
    /// Gives input slot `slot` another recording device to read from, or none.
    SetInput {
        slot: usize,
        samples: Option<Consumer<f32>>,
    },
    /// Replaces the clips input slot `slot` plays.
    SetClips {
        slot: usize,
        clips: Clips,
    },
    /// Replaces what plays along the transport.
    SetSchedule(Schedule),
}

/// Things the studio no longer uses. They go back to the UI thread to be
/// dropped there, because freeing memory on the audio thread can glitch.
#[allow(dead_code)] // only held so it gets dropped on the UI thread
pub enum Garbage {
    Channel(Channel),
    Instrument(InstrumentSlot),
    Input(InputSlot),
    InputQueue(Consumer<f32>),
    Clips(Clips),
    Schedule(Schedule),
}

pub struct Studio {
    instruments: Vec<InstrumentSlot>,
    inputs: Vec<InputSlot>,
    mixer: Mixer,
    commands: Consumer<StudioCommand>,
    garbage: Producer<Garbage>,
    transport: Transport,
    metronome: Metronome,
    schedule: Schedule,
    stats: Arc<StudioStats>,
}

impl Studio {
    pub fn new(
        mixer_params: Arc<MixerParams>,
        transport_params: Arc<TransportParams>,
        metronome_params: Arc<MetronomeParams>,
        commands: Consumer<StudioCommand>,
        garbage: Producer<Garbage>,
        stats: Arc<StudioStats>,
        sample_rate: f32,
    ) -> Self {
        Self {
            instruments: Vec::with_capacity(MAX_INSTRUMENTS),
            inputs: Vec::with_capacity(MAX_INPUTS),
            mixer: Mixer::new(mixer_params),
            commands,
            garbage,
            transport: Transport::new(transport_params, sample_rate as f64),
            metronome: Metronome::new(metronome_params, sample_rate),
            schedule: Schedule::empty(),
            stats,
        }
    }

    fn handle_commands(&mut self) {
        while let Ok(command) = self.commands.pop() {
            match command {
                StudioCommand::AddChannel(channel) => {
                    if let Err(channel) = self.mixer.add(channel) {
                        self.discard(Garbage::Channel(channel));
                    }
                }
                StudioCommand::RemoveChannel(id) => {
                    if let Some(channel) = self.mixer.remove(id) {
                        self.discard(Garbage::Channel(channel));
                    }
                }
                StudioCommand::AddInstrument(slot) => {
                    if self.instruments.len() < MAX_INSTRUMENTS {
                        self.instruments.push(slot);
                    } else {
                        self.discard(Garbage::Instrument(slot));
                    }
                }
                StudioCommand::AddInput(slot) => {
                    if self.inputs.len() < MAX_INPUTS {
                        self.inputs.push(slot);
                    } else {
                        self.discard(Garbage::Input(slot));
                    }
                }
                StudioCommand::SetInput { slot, samples } => {
                    // with no such slot the new queue goes back unused
                    let unused = match self.inputs.get_mut(slot) {
                        Some(input) => input.set_input(samples),
                        None => samples,
                    };
                    if let Some(queue) = unused {
                        self.discard(Garbage::InputQueue(queue));
                    }
                }
                StudioCommand::SetClips { slot, clips } => {
                    let old = match self.inputs.get_mut(slot) {
                        Some(input) => input.set_clips(clips),
                        None => clips,
                    };
                    self.discard(Garbage::Clips(old));
                }
                StudioCommand::SetSchedule(mut schedule) => {
                    // the new schedule would never send the old one's note offs
                    self.release_scheduled();
                    schedule.seek(self.transport.position_in_beats());
                    let old = std::mem::replace(&mut self.schedule, schedule);
                    self.discard(Garbage::Schedule(old));
                }
            }
        }
    }

    /// Sends a note off for every scheduled note that is still sounding.
    fn release_scheduled(&mut self) {
        for (instrument, event) in self.schedule.release_all() {
            play(&mut self.instruments, instrument, event);
        }
    }

    fn discard(&mut self, garbage: Garbage) {
        // if the UI hasn't emptied the queue it gets dropped here after all,
        // which is better than keeping it forever
        let _ = self.garbage.push(garbage);
    }
}

impl StereoSource for Studio {
    fn begin_buffer(&mut self, frames: usize) {
        self.handle_commands();
        self.stats
            .buffer_frames
            .store(frames as u32, Ordering::Relaxed);
        // first, so the clips see this buffer's tempo
        self.transport.begin_buffer();

        for slot in &mut self.instruments {
            slot.output = self.mixer.resolve(&slot.params.route);
            slot.source.begin_buffer();
        }

        for slot in &mut self.inputs {
            slot.output = self.mixer.resolve(&slot.params.route);
            slot.prepare(frames);
            slot.prepare_clips(&self.transport);
        }

        self.mixer.begin_buffer(frames);
        self.metronome.begin_buffer();

        if self.transport.jumped() || self.transport.stopped() {
            self.release_scheduled();
        }
        if self.transport.jumped() {
            self.schedule.seek(self.transport.position_in_beats());
        }
    }

    fn next_frame(&mut self) -> Frame {
        // before the instruments make their sample, so each note starts on
        // the sample its beat falls on
        if self.transport.is_playing() {
            let beat = self.transport.position_in_beats();
            while let Some((instrument, event)) = self.schedule.next_due(beat) {
                play(&mut self.instruments, instrument, event);
            }
        }

        for slot in &mut self.instruments {
            let sample = slot.source.next_sample();
            if let Some(ch) = slot.output {
                self.mixer.add_input(ch, sample);
            }
        }

        let playhead = self
            .transport
            .is_playing()
            .then(|| self.transport.position());
        for slot in &mut self.inputs {
            let mut sample = slot.next_sample();
            if let Some(position) = playhead {
                sample += slot.clip_sample(position);
            }
            if let Some(ch) = slot.output {
                self.mixer.add_input(ch, sample);
            }
        }

        let click = self.metronome.next_sample(&self.transport);
        self.transport.advance(1);

        // the click skips the mixer, so it never ends up in the mix
        self.mixer.next_frame()
            + Frame {
                left: click,
                right: click,
            }
    }
}

/// Hands a note to the instrument in slot `instrument`, if there is one.
fn play(instruments: &mut [InstrumentSlot], instrument: usize, event: NoteEvent) {
    if let Some(slot) = instruments.get_mut(instrument) {
        slot.source.note(event);
    }
}
