//! What plays when: sequences of notes, placed on tracks. The UI edits this;
//! `flatten` turns it into the schedule the studio plays.

use crate::{
    audio::{note::NoteEvent, schedule::ScheduledNote},
    project::Project,
};

/// A note in a sequence. Times are in beats from the start of the sequence.
#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub start: f64,
    pub length: f64,
    pub pitch: u8,
}

/// A run of notes with no instrument of its own: whichever track it's placed
/// on plays it. One sequence can be placed many times, on many tracks.
pub struct Sequence {
    pub name: String,
    /// In beats. Notes past the end are cut off there.
    pub length: f64,
    pub notes: Vec<Note>,
}

impl Sequence {
    /// The lowest and highest pitch in it, if it has notes.
    pub fn pitch_range(&self) -> Option<(u8, u8)> {
        let pitches = self.notes.iter().map(|n| n.pitch);
        Some((pitches.clone().min()?, pitches.max()?))
    }
}

/// A sequence playing on a track from `start`, in beats.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub start: f64,
    /// Index in `Project::sequences`.
    pub sequence: usize,
}

impl Project {
    /// Adds a sequence to the project. Returns its index in `sequences`.
    pub fn add_sequence(&mut self, sequence: Sequence) -> usize {
        self.sequences.push(sequence);
        self.sequences.len() - 1
    }

    /// Moves placement `index` of track `from` to `start` on track `to`, and
    /// sends the new schedule.
    pub fn move_placement(&mut self, from: usize, index: usize, to: usize, start: f64) {
        if to >= self.tracks.len()
            || self
                .tracks
                .get(from)
                .is_none_or(|t| index >= t.placements.len())
        {
            return;
        }

        let mut placement = self.tracks[from].placements.remove(index);
        placement.start = start.max(0.0);
        self.tracks[to].placements.push(placement);
        self.update_schedule();
    }

    /// Where the timeline ends: the end of the last placement on any track.
    pub fn timeline_length(&self) -> f64 {
        self.tracks
            .iter()
            .flat_map(|t| &t.placements)
            .filter_map(|p| Some(p.start + self.sequences.get(p.sequence)?.length))
            .fold(0.0, f64::max)
    }

    /// Sends the timeline to the studio as its new schedule. Call after
    /// changing placements or sequences.
    pub fn update_schedule(&mut self) -> bool {
        let tracks = self.tracks.iter().map(|t| t.placements.as_slice());
        let notes = flatten(tracks, &self.sequences);
        self.set_schedule(notes)
    }
}

/// Every note on the timeline as a note on and a note off at their beats.
/// `tracks` gives each track's placements, in track order; a track's index
/// is the instrument slot that plays it, since tracks are never removed yet.
pub fn flatten<'a>(
    tracks: impl IntoIterator<Item = &'a [Placement]>,
    sequences: &[Sequence],
) -> Vec<ScheduledNote> {
    let mut scheduled = Vec::new();

    for (instrument, placements) in tracks.into_iter().enumerate() {
        for placement in placements {
            let Some(sequence) = sequences.get(placement.sequence) else {
                continue;
            };

            for note in &sequence.notes {
                let end = (note.start + note.length).min(sequence.length);
                // an empty note would get its off before its on, and hang
                if end <= note.start {
                    continue;
                }

                for (beat, event) in [
                    (note.start, NoteEvent::On(note.pitch)),
                    (end, NoteEvent::Off(note.pitch)),
                ] {
                    scheduled.push(ScheduledNote {
                        beat: placement.start + beat,
                        instrument,
                        event,
                    });
                }
            }
        }
    }

    scheduled
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequence(length: f64, notes: &[(f64, f64, u8)]) -> Sequence {
        Sequence {
            name: "Test".to_owned(),
            length,
            notes: notes
                .iter()
                .map(|&(start, length, pitch)| Note {
                    start,
                    length,
                    pitch,
                })
                .collect(),
        }
    }

    fn placed(start: f64, sequence: usize) -> Placement {
        Placement { start, sequence }
    }

    /// The schedule as (beat, instrument, event) labels, sorted by beat.
    fn labels(mut notes: Vec<ScheduledNote>) -> Vec<String> {
        notes.sort_by(|a, b| a.beat.total_cmp(&b.beat));
        notes
            .iter()
            .map(|n| format!("{} {} {:?}", n.beat, n.instrument, n.event))
            .collect()
    }

    #[test]
    fn note_becomes_an_on_and_an_off() {
        let sequences = [sequence(4.0, &[(1.0, 2.0, 60)])];
        let tracks = [&[placed(0.0, 0)][..]];

        assert_eq!(
            labels(flatten(tracks, &sequences)),
            ["1 0 On(60)", "3 0 Off(60)"]
        );
    }

    #[test]
    fn notes_move_with_their_placement() {
        let sequences = [sequence(4.0, &[(0.0, 1.0, 60)])];
        let tracks = [&[placed(8.0, 0)][..]];

        assert_eq!(
            labels(flatten(tracks, &sequences)),
            ["8 0 On(60)", "9 0 Off(60)"]
        );
    }

    #[test]
    fn one_sequence_plays_at_every_placement() {
        let sequences = [sequence(1.0, &[(0.0, 0.5, 36)])];
        let tracks = [&[placed(0.0, 0), placed(4.0, 0)][..]];

        assert_eq!(
            labels(flatten(tracks, &sequences)),
            ["0 0 On(36)", "0.5 0 Off(36)", "4 0 On(36)", "4.5 0 Off(36)"]
        );
    }

    #[test]
    fn each_track_plays_on_its_own_instrument() {
        let sequences = [sequence(1.0, &[(0.0, 1.0, 60)])];
        let tracks = [&[][..], &[placed(0.0, 0)][..]];

        assert_eq!(
            labels(flatten(tracks, &sequences)),
            ["0 1 On(60)", "1 1 Off(60)"]
        );
    }

    #[test]
    fn notes_are_cut_off_at_the_end_of_the_sequence() {
        let sequences = [sequence(2.0, &[(1.0, 4.0, 60), (3.0, 1.0, 62)])];
        let tracks = [&[placed(0.0, 0)][..]];

        assert_eq!(
            labels(flatten(tracks, &sequences)),
            ["1 0 On(60)", "2 0 Off(60)"]
        );
    }

    #[test]
    fn empty_notes_are_left_out() {
        let sequences = [sequence(4.0, &[(1.0, 0.0, 60)])];
        let tracks = [&[placed(0.0, 0)][..]];

        assert!(flatten(tracks, &sequences).is_empty());
    }

    #[test]
    fn placement_of_a_missing_sequence_plays_nothing() {
        let tracks = [&[placed(0.0, 3)][..]];

        assert!(flatten(tracks, &[]).is_empty());
    }

    #[test]
    fn pitch_range_spans_the_notes() {
        let notes = sequence(4.0, &[(0.0, 1.0, 64), (1.0, 1.0, 48), (2.0, 1.0, 55)]);

        assert_eq!(notes.pitch_range(), Some((48, 64)));
        assert_eq!(sequence(4.0, &[]).pitch_range(), None);
    }
}
