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
    source::{AudioSource, MAX_BLOCK_FRAMES, StereoSource},
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
    scratch: Vec<f32>,
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
            scratch: vec![0.0; MAX_BLOCK_FRAMES],
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

    fn play_due_notes(&mut self) {
        if !self.transport.is_playing() {
            return;
        }

        let position = self.transport.position();
        while let Some(beat) = self.schedule.next_beat() {
            if self.transport.sample_at(beat) > position {
                break;
            }
            if let Some((instrument, event)) = self.schedule.take_next() {
                play(&mut self.instruments, instrument, event);
            }
        }
    }

    fn frames_to_next_note(&self) -> Option<usize> {
        if !self.transport.is_playing() {
            return None;
        }

        let beat = self.schedule.next_beat()?;
        Some((self.transport.sample_at(beat) - self.transport.position()) as usize)
    }

    fn render_instruments(&mut self, from: usize, to: usize) {
        let scratch = &mut self.scratch[..to - from];
        for slot in &mut self.instruments {
            slot.source.process(scratch);
            if let Some(ch) = slot.output {
                self.mixer.add_input(ch, from, scratch);
            }
        }
    }

    fn render_inputs(&mut self, frames: usize, start: u64) {
        let scratch = &mut self.scratch[..frames];
        let playing = self.transport.is_playing();
        for slot in &mut self.inputs {
            slot.process(scratch);
            if playing {
                slot.add_clips(scratch, start);
            }
            if let Some(ch) = slot.output {
                self.mixer.add_input(ch, 0, scratch);
            }
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

    fn process(&mut self, out: &mut [Frame]) {
        let start = self.transport.position();
        let frames = out.len();

        // instruments render up to each scheduled note, so the note starts
        // on the sample its beat falls on
        let mut done = 0;
        while done < frames {
            self.play_due_notes();
            let end = self
                .frames_to_next_note()
                .map_or(frames, |n| (done + n).min(frames));
            self.render_instruments(done, end);
            self.transport.advance((end - done) as u64);
            done = end;
        }

        self.render_inputs(frames, start);
        self.mixer.process(out);
        // the click skips the mixer, so it never ends up in the mix
        self.metronome.add_to(out, &self.transport, start);
    }
}

/// Hands a note to the instrument in slot `instrument`, if there is one.
fn play(instruments: &mut [InstrumentSlot], instrument: usize, event: NoteEvent) {
    if let Some(slot) = instruments.get_mut(instrument) {
        slot.source.note(event);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use rtrb::RingBuffer;

    use super::*;
    use crate::{
        atomic::AtomicF32,
        audio::{
            instrument::NoteSource, mixer::meter::Meter, schedule::ScheduledNote,
            studio::instrument::InstrumentParams,
        },
    };

    /// Remembers how many frames it had rendered when each note came in.
    struct Recorder {
        rendered: usize,
        notes: Arc<Mutex<Vec<usize>>>,
    }

    impl AudioSource for Recorder {
        fn process(&mut self, out: &mut [f32]) {
            out.fill(0.0);
            self.rendered += out.len();
        }
    }

    impl NoteSource for Recorder {
        fn note(&mut self, _event: NoteEvent) {
            self.notes.lock().unwrap().push(self.rendered);
        }
    }

    /// A playing studio at 960 Hz and 120 bpm, so a beat is 480 frames,
    /// with one note scheduled at `beat`.
    fn studio_with_note_at(beat: f64) -> (Studio, Arc<Mutex<Vec<usize>>>) {
        let (mut commands, consumer) = RingBuffer::new(8);
        let (garbage, _) = RingBuffer::new(8);
        let transport = Arc::new(TransportParams::new());
        transport.playing.store(true, Ordering::Relaxed);

        let notes = Arc::new(Mutex::new(Vec::new()));
        let recorder = Recorder {
            rendered: 0,
            notes: Arc::clone(&notes),
        };
        let slot = InstrumentSlot::new(Arc::new(InstrumentParams::new(None)), Box::new(recorder));
        let schedule = Schedule::new(vec![ScheduledNote {
            beat,
            instrument: 0,
            event: NoteEvent::On(60),
        }]);
        assert!(commands.push(StudioCommand::AddInstrument(slot)).is_ok());
        assert!(commands.push(StudioCommand::SetSchedule(schedule)).is_ok());

        let studio = Studio::new(
            Arc::new(MixerParams {
                gain: AtomicF32::new(1.0),
                meter: Meter::new(),
            }),
            transport,
            Arc::new(MetronomeParams::new()),
            consumer,
            garbage,
            Arc::new(StudioStats::default()),
            960.0,
        );

        (studio, notes)
    }

    fn render(studio: &mut Studio, frames: usize) {
        let mut out = vec![Frame::default(); frames];
        studio.begin_buffer(frames);
        studio.process(&mut out);
    }

    #[test]
    fn scheduled_note_starts_on_its_frame_in_the_middle_of_a_block() {
        let (mut studio, notes) = studio_with_note_at(0.25);
        render(&mut studio, 256);

        assert_eq!(*notes.lock().unwrap(), [120]);
    }

    #[test]
    fn note_on_a_block_boundary_starts_the_next_block() {
        let (mut studio, notes) = studio_with_note_at(0.5);
        render(&mut studio, 240);
        assert!(notes.lock().unwrap().is_empty());

        render(&mut studio, 240);
        assert_eq!(*notes.lock().unwrap(), [240]);
    }
}
