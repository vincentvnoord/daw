//! An audio track's clips as the studio plays them along the transport.

use std::sync::Arc;

use crate::audio::transport::Transport;

/// Audio placed on a track at a beat. It plays at its own speed whatever
/// the tempo, so only where it starts depends on the tempo, not how long it
/// lasts.
pub struct ScheduledClip {
    /// In beats.
    pub start: f64,
    /// Mono, at the studio's sample rate.
    pub samples: Arc<Vec<f32>>,
}

/// Every clip on one track. Built on the UI thread; the audio thread only
/// reads it.
pub struct Clips {
    clips: Vec<ScheduledClip>,
    /// The sample of the transport each clip starts on at the current tempo,
    /// worked out once per buffer.
    starts: Vec<u64>,
}

impl Clips {
    pub fn new(clips: Vec<ScheduledClip>) -> Self {
        // reserved here, so `prepare` only writes into it
        let starts = vec![0; clips.len()];
        Self { clips, starts }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Works out where each clip starts at the transport's tempo. Call once
    /// per buffer, after the transport has picked up the tempo.
    pub fn prepare(&mut self, transport: &Transport) {
        for (start, clip) in self.starts.iter_mut().zip(&self.clips) {
            *start = transport.sample_at(clip.start);
        }
    }

    /// Adds the clips' audio from sample `position` of the transport into
    /// `out`. Clips that overlap add up.
    pub fn add_to(&self, out: &mut [f32], position: u64) {
        let end = position + out.len() as u64;

        for (&start, clip) in self.starts.iter().zip(&self.clips) {
            let from = start.max(position);
            let to = (start + clip.samples.len() as u64).min(end);
            if from >= to {
                continue;
            }

            let samples = &clip.samples[(from - start) as usize..(to - start) as usize];
            let out = &mut out[(from - position) as usize..(to - position) as usize];
            for (o, s) in out.iter_mut().zip(samples) {
                *o += s;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::*;
    use crate::audio::transport::TransportParams;

    fn transport(bpm: f32) -> Transport {
        let params = TransportParams::new();
        params.bpm.set(bpm);
        params.playing.store(true, Ordering::Relaxed);
        let mut transport = Transport::new(Arc::new(params), 48000.0);
        transport.begin_buffer();
        transport
    }

    /// A clip at `start` whose samples count up from 1.
    fn clip(start: f64, length: usize) -> ScheduledClip {
        ScheduledClip {
            start,
            samples: Arc::new((1..=length).map(|i| i as f32).collect()),
        }
    }

    fn prepared(clips: Vec<ScheduledClip>, bpm: f32) -> Clips {
        let mut clips = Clips::new(clips);
        clips.prepare(&transport(bpm));
        clips
    }

    fn render(clips: &Clips, position: u64, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames];
        clips.add_to(&mut out, position);
        out
    }

    #[test]
    fn clip_starts_on_the_sample_its_beat_falls_on() {
        // at 120 bpm beat 1 is sample 24000
        let clips = prepared(vec![clip(1.0, 4)], 120.0);

        assert_eq!(render(&clips, 23999, 3), [0.0, 1.0, 2.0]);
    }

    #[test]
    fn clip_between_samples_starts_on_the_sample_after() {
        // at 130 bpm beat 1 is at sample 22153.8
        let clips = prepared(vec![clip(1.0, 4)], 130.0);

        assert_eq!(render(&clips, 22153, 2), [0.0, 1.0]);
    }

    #[test]
    fn clip_is_silent_after_its_end() {
        let clips = prepared(vec![clip(0.0, 3)], 120.0);

        assert_eq!(render(&clips, 2, 2), [3.0, 0.0]);
    }

    #[test]
    fn overlapping_clips_add_up() {
        let clips = prepared(vec![clip(0.0, 4), clip(0.0, 4)], 120.0);

        assert_eq!(render(&clips, 1, 1), [4.0]);
    }

    #[test]
    fn clips_add_to_what_is_already_there() {
        let clips = prepared(vec![clip(0.0, 2)], 120.0);
        let mut out = [0.5, 0.5];
        clips.add_to(&mut out, 0);

        assert_eq!(out, [1.5, 2.5]);
    }

    #[test]
    fn a_slower_tempo_moves_the_start_but_not_the_speed() {
        // at 60 bpm beat 1 is sample 48000, and the clip still plays a
        // sample per sample
        let clips = prepared(vec![clip(1.0, 4)], 60.0);

        assert_eq!(render(&clips, 48000, 4), [1.0, 2.0, 3.0, 4.0]);
    }
}
