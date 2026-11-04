use crate::audio::note::NoteEvent;

/// Most scheduled notes that can sound at once, across all instruments. Room
/// for them is reserved up front, so the audio thread never allocates; past
/// this a note still plays, but isn't released when the playhead jumps.
const MAX_SOUNDING: usize = 256;

/// A note on or off at a point on the timeline.
#[derive(Clone, Copy, Debug)]
pub struct ScheduledNote {
    pub beat: f64,
    /// Index of the instrument slot in the studio that plays it.
    pub instrument: usize,
    pub event: NoteEvent,
}

/// Everything the studio plays along the transport, in the order it plays.
/// Built on the UI thread; the audio thread only walks it.
pub struct Schedule {
    notes: Vec<ScheduledNote>,
    /// Index of the first note that hasn't played yet.
    next: usize,
    /// Notes this schedule turned on and hasn't turned off yet, as
    /// (instrument, note), so they can be released when the playhead jumps.
    sounding: Vec<(usize, u8)>,
}

impl Schedule {
    /// Sorts `notes` by beat. On the same beat, offs go before ons, so a
    /// note ending where the same note starts again is played again.
    pub fn new(mut notes: Vec<ScheduledNote>) -> Self {
        notes.sort_by(|a, b| {
            a.beat
                .total_cmp(&b.beat)
                .then(is_on(a.event).cmp(&is_on(b.event)))
        });

        Self {
            notes,
            next: 0,
            sounding: Vec::with_capacity(MAX_SOUNDING),
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Moves to `beat`: the next note to play is the first one at or after
    /// it. Notes that were sounding are left alone; release them first.
    pub fn seek(&mut self, beat: f64) {
        self.next = self.notes.partition_point(|n| n.beat < beat);
    }

    pub fn next_beat(&self) -> Option<f64> {
        self.notes.get(self.next).map(|n| n.beat)
    }

    /// Plays the next note: hands it back as (instrument, event) and keeps
    /// track of what is sounding.
    pub fn take_next(&mut self) -> Option<(usize, NoteEvent)> {
        let note = *self.notes.get(self.next)?;
        self.next += 1;

        match note.event {
            NoteEvent::On(n) => {
                if self.sounding.len() < MAX_SOUNDING {
                    self.sounding.push((note.instrument, n));
                }
            }
            NoteEvent::Off(n) => {
                if let Some(i) = self
                    .sounding
                    .iter()
                    .position(|&s| s == (note.instrument, n))
                {
                    self.sounding.swap_remove(i);
                }
            }
        }

        Some((note.instrument, note.event))
    }

    /// A note off for every note that is still sounding, as (instrument,
    /// event). For when the transport stops or the playhead jumps, so no note
    /// is left hanging without its note off.
    pub fn release_all(&mut self) -> impl Iterator<Item = (usize, NoteEvent)> + '_ {
        self.sounding
            .drain(..)
            .map(|(instrument, n)| (instrument, NoteEvent::Off(n)))
    }
}

fn is_on(event: NoteEvent) -> bool {
    matches!(event, NoteEvent::On(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(beat: f64, note: u8) -> ScheduledNote {
        ScheduledNote {
            beat,
            instrument: 0,
            event: NoteEvent::On(note),
        }
    }

    fn off(beat: f64, note: u8) -> ScheduledNote {
        ScheduledNote {
            beat,
            instrument: 0,
            event: NoteEvent::Off(note),
        }
    }

    /// Every event due by `beat`, as `On(n)` / `Off(n)` labels.
    fn due(schedule: &mut Schedule, beat: f64) -> Vec<String> {
        std::iter::from_fn(|| {
            schedule.next_beat().filter(|&b| b <= beat)?;
            schedule.take_next()
        })
        .map(|(_, event)| format!("{event:?}"))
        .collect()
    }

    #[test]
    fn plays_notes_in_beat_order() {
        let mut schedule = Schedule::new(vec![on(1.0, 62), on(0.0, 60), on(0.5, 61)]);

        assert_eq!(due(&mut schedule, 1.0), ["On(60)", "On(61)", "On(62)"]);
    }

    #[test]
    fn a_note_waits_for_its_beat() {
        let mut schedule = Schedule::new(vec![on(1.0, 60)]);

        assert!(due(&mut schedule, 0.99).is_empty());
        assert_eq!(due(&mut schedule, 1.0), ["On(60)"]);
    }

    #[test]
    fn each_note_plays_once() {
        let mut schedule = Schedule::new(vec![on(0.0, 60)]);
        due(&mut schedule, 0.0);

        assert!(due(&mut schedule, 2.0).is_empty());
    }

    #[test]
    fn off_goes_before_on_on_the_same_beat() {
        let mut schedule = Schedule::new(vec![on(1.0, 60), off(1.0, 60)]);

        assert_eq!(due(&mut schedule, 1.0), ["Off(60)", "On(60)"]);
    }

    #[test]
    fn seek_skips_the_notes_before() {
        let mut schedule = Schedule::new(vec![on(0.0, 60), on(1.0, 61), on(2.0, 62)]);
        schedule.seek(1.0);

        assert_eq!(due(&mut schedule, 2.0), ["On(61)", "On(62)"]);
    }

    #[test]
    fn seek_back_plays_them_again() {
        let mut schedule = Schedule::new(vec![on(0.0, 60)]);
        due(&mut schedule, 1.0);
        schedule.seek(0.0);

        assert_eq!(due(&mut schedule, 1.0), ["On(60)"]);
    }

    #[test]
    fn release_all_turns_off_what_is_sounding() {
        let mut schedule = Schedule::new(vec![on(0.0, 60), on(0.0, 64), off(1.0, 60)]);
        due(&mut schedule, 1.0);

        let released: Vec<_> = schedule.release_all().collect();
        assert_eq!(released.len(), 1);
        assert!(matches!(released[0], (0, NoteEvent::Off(64))));
    }

    #[test]
    fn release_all_only_releases_once() {
        let mut schedule = Schedule::new(vec![on(0.0, 60)]);
        due(&mut schedule, 0.0);
        schedule.release_all().for_each(drop);

        assert_eq!(schedule.release_all().count(), 0);
    }

    #[test]
    fn same_note_on_two_instruments_is_tracked_apart() {
        let mut schedule = Schedule::new(vec![
            on(0.0, 60),
            ScheduledNote {
                instrument: 1,
                ..on(0.0, 60)
            },
            off(1.0, 60),
        ]);
        due(&mut schedule, 1.0);

        let released: Vec<_> = schedule.release_all().collect();
        assert!(matches!(released[..], [(1, NoteEvent::Off(60))]));
    }
}
