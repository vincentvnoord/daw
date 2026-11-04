//! The playhead and the notes that play along it.

use crate::audio::{
    note::NoteEvent, schedule::Schedule, studio::instrument::InstrumentSlot, transport::Transport,
};

/// Plays the schedule along the transport, onto the studio's instruments.
/// The transport lives here for as long as the studio does; the schedule is
/// swapped out whenever the UI sends a new one.
pub struct Sequencer {
    transport: Transport,
    schedule: Schedule,
}

impl Sequencer {
    pub fn new(transport: Transport) -> Self {
        Self {
            transport,
            schedule: Schedule::empty(),
        }
    }

    /// The playhead, for whatever else plays along it: clips, the metronome.
    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    /// Picks up what the UI changed on the transport. When playing stopped or
    /// the playhead jumped, releases the notes left sounding, and after a
    /// jump finds the schedule's place again.
    pub fn begin_buffer(&mut self, instruments: &mut [InstrumentSlot]) {
        self.transport.begin_buffer();

        if self.transport.jumped() || self.transport.stopped() {
            self.release(instruments);
        }
        if self.transport.jumped() {
            self.schedule.seek(self.transport.position_in_beats());
        }
    }

    /// Plays `schedule` from the playhead on. Returns the old one, to be
    /// dropped off the audio thread.
    pub fn set_schedule(
        &mut self,
        mut schedule: Schedule,
        instruments: &mut [InstrumentSlot],
    ) -> Schedule {
        // the new schedule would never send the old one's note offs
        self.release(instruments);
        schedule.seek(self.transport.position_in_beats());
        std::mem::replace(&mut self.schedule, schedule)
    }

    /// Plays the notes due at the playhead, then moves the playhead on to the
    /// next note, or by `max` frames if that comes first. Returns how far it
    /// moved: the frames to render before anything else is due.
    ///
    /// The playhead moves before those frames are rendered. That's fine for
    /// instruments, which never look at the transport; whatever does reads
    /// the position from before the first step.
    pub fn step(&mut self, max: usize, instruments: &mut [InstrumentSlot]) -> usize {
        self.play_due(instruments);
        let frames = self.frames_to_next_note().map_or(max, |n| n.min(max));
        self.transport.advance(frames as u64);
        frames
    }

    fn play_due(&mut self, instruments: &mut [InstrumentSlot]) {
        if !self.transport.is_playing() {
            return;
        }

        let position = self.transport.position();
        while let Some(beat) = self.schedule.next_beat() {
            if self.transport.sample_at(beat) > position {
                break;
            }
            if let Some((instrument, event)) = self.schedule.take_next() {
                play(instruments, instrument, event);
            }
        }
    }

    /// Frames from the playhead to the next note, or `None` when stopped or
    /// with no notes left. After `play_due` it's at least 1.
    fn frames_to_next_note(&self) -> Option<usize> {
        if !self.transport.is_playing() {
            return None;
        }

        let beat = self.schedule.next_beat()?;
        Some((self.transport.sample_at(beat) - self.transport.position()) as usize)
    }

    /// Sends a note off for every scheduled note that is still sounding.
    fn release(&mut self, instruments: &mut [InstrumentSlot]) {
        for (instrument, event) in self.schedule.release_all() {
            play(instruments, instrument, event);
        }
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
    use std::sync::{Arc, atomic::Ordering};

    use super::*;
    use crate::audio::{
        schedule::ScheduledNote,
        studio::instrument::recorder::{Notes, Recorder},
        transport::TransportParams,
    };

    fn on(beat: f64, note: u8) -> ScheduledNote {
        ScheduledNote {
            beat,
            instrument: 0,
            event: NoteEvent::On(note),
        }
    }

    /// A playing sequencer at 960 Hz and 120 bpm, so a beat is 480 frames,
    /// with one instrument that writes down the notes it gets.
    struct Fixture {
        sequencer: Sequencer,
        instruments: Vec<InstrumentSlot>,
        notes: Notes,
        params: Arc<TransportParams>,
    }

    impl Fixture {
        fn new(schedule: Vec<ScheduledNote>) -> Self {
            let params = Arc::new(TransportParams::new());
            params.playing.store(true, Ordering::Relaxed);

            let (slot, notes) = Recorder::slot();
            let mut instruments = vec![slot];
            let mut sequencer = Sequencer::new(Transport::new(Arc::clone(&params), 960.0));
            sequencer.begin_buffer(&mut instruments);
            sequencer.set_schedule(Schedule::new(schedule), &mut instruments);

            Self {
                sequencer,
                instruments,
                notes,
                params,
            }
        }

        fn step(&mut self, max: usize) -> usize {
            self.sequencer.step(max, &mut self.instruments)
        }

        /// Every note played so far, as `On(n)` / `Off(n)` labels.
        fn played(&self) -> Vec<String> {
            let notes = self.notes.lock().unwrap();
            notes
                .iter()
                .map(|(_, event)| format!("{event:?}"))
                .collect()
        }
    }

    #[test]
    fn plays_the_notes_due_at_the_playhead() {
        let mut fixture = Fixture::new(vec![on(0.0, 60), on(1.0, 62)]);
        fixture.step(100);

        assert_eq!(fixture.played(), ["On(60)"]);
    }

    #[test]
    fn a_step_ends_where_the_next_note_starts() {
        let mut fixture = Fixture::new(vec![on(1.0, 60)]);

        assert_eq!(fixture.step(1000), 480);
        assert!(fixture.played().is_empty());

        fixture.step(1000);
        assert_eq!(fixture.played(), ["On(60)"]);
    }

    #[test]
    fn with_no_note_ahead_a_step_takes_all_it_is_given() {
        let mut fixture = Fixture::new(Vec::new());

        assert_eq!(fixture.step(256), 256);
    }

    #[test]
    fn stopping_releases_the_notes_that_are_sounding() {
        let mut fixture = Fixture::new(vec![on(0.0, 60)]);
        fixture.step(1);

        fixture.params.playing.store(false, Ordering::Relaxed);
        fixture.sequencer.begin_buffer(&mut fixture.instruments);

        assert_eq!(fixture.played(), ["On(60)", "Off(60)"]);
    }

    #[test]
    fn a_new_schedule_starts_from_the_playhead() {
        let mut fixture = Fixture::new(Vec::new());
        fixture.step(480);

        let schedule = Schedule::new(vec![on(0.5, 60), on(1.0, 62)]);
        fixture
            .sequencer
            .set_schedule(schedule, &mut fixture.instruments);
        fixture.step(1);

        assert_eq!(fixture.played(), ["On(62)"]);
    }
}
