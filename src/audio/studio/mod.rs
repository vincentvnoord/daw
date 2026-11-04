pub mod clips;
pub mod commands;
pub mod input;
pub mod instrument;

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use rtrb::{Consumer, Producer};

pub use commands::{Garbage, StudioCommand};

use crate::audio::{
    frame::Frame,
    mixer::{Mixer, MixerParams},
    sequencer::Sequencer,
    source::{AudioSource, MAX_BLOCK_FRAMES, StereoSource},
    studio::{input::InputSlot, instrument::InstrumentSlot},
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

pub struct Studio {
    instruments: Vec<InstrumentSlot>,
    inputs: Vec<InputSlot>,
    mixer: Mixer,
    commands: Consumer<StudioCommand>,
    garbage: Producer<Garbage>,
    sequencer: Sequencer,
    metronome: Metronome,
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
            sequencer: Sequencer::new(Transport::new(transport_params, sample_rate as f64)),
            metronome: Metronome::new(metronome_params, sample_rate),
            stats,
            scratch: vec![0.0; MAX_BLOCK_FRAMES],
        }
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
        let playing = self.sequencer.transport().is_playing();
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
}

impl StereoSource for Studio {
    fn begin_buffer(&mut self, frames: usize) {
        self.handle_commands();
        self.stats
            .buffer_frames
            .store(frames as u32, Ordering::Relaxed);
        // first, so the clips see this buffer's tempo
        self.sequencer.begin_buffer(&mut self.instruments);

        for slot in &mut self.instruments {
            slot.output = self.mixer.resolve(&slot.params.route);
            slot.source.begin_buffer();
        }

        for slot in &mut self.inputs {
            slot.output = self.mixer.resolve(&slot.params.route);
            slot.prepare(frames);
            slot.prepare_clips(self.sequencer.transport());
        }

        self.mixer.begin_buffer(frames);
        self.metronome.begin_buffer();
    }

    fn process(&mut self, out: &mut [Frame]) {
        let start = self.sequencer.transport().position();
        let frames = out.len();

        // instruments render up to each scheduled note, so the note starts
        // on the sample its beat falls on
        let mut done = 0;
        while done < frames {
            let len = self.sequencer.step(frames - done, &mut self.instruments);
            self.render_instruments(done, done + len);
            done += len;
        }

        self.render_inputs(frames, start);
        self.mixer.process(out);

        // the click skips the mixer, so it never ends up in the mix
        self.metronome
            .add_to(out, self.sequencer.transport(), start);
    }
}

#[cfg(test)]
mod tests {
    use rtrb::RingBuffer;

    use super::*;
    use crate::{
        atomic::AtomicF32,
        audio::{
            mixer::meter::Meter,
            note::NoteEvent,
            schedule::{Schedule, ScheduledNote},
            studio::instrument::recorder::{Notes, Recorder},
        },
    };

    /// A playing studio at 960 Hz and 120 bpm, so a beat is 480 frames,
    /// with one note scheduled at `beat`.
    fn studio_with_note_at(beat: f64) -> (Studio, Notes) {
        let (mut commands, consumer) = RingBuffer::new(8);
        let (garbage, _) = RingBuffer::new(8);
        let transport = Arc::new(TransportParams::new());
        transport.playing.store(true, Ordering::Relaxed);

        let (slot, notes) = Recorder::slot();
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

    /// How many frames the instrument had rendered when each note came in.
    fn frames(notes: &Notes) -> Vec<usize> {
        notes
            .lock()
            .unwrap()
            .iter()
            .map(|&(frames, _)| frames)
            .collect()
    }

    #[test]
    fn scheduled_note_starts_on_its_frame_in_the_middle_of_a_block() {
        let (mut studio, notes) = studio_with_note_at(0.25);
        render(&mut studio, 256);

        assert_eq!(frames(&notes), [120]);
    }

    #[test]
    fn note_on_a_block_boundary_starts_the_next_block() {
        let (mut studio, notes) = studio_with_note_at(0.5);
        render(&mut studio, 240);
        assert!(frames(&notes).is_empty());

        render(&mut studio, 240);
        assert_eq!(frames(&notes), [240]);
    }
}
