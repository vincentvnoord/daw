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

    /// The clips' audio at sample `position` of the transport. Clips that
    /// overlap add up.
    pub fn sample_at(&self, position: u64) -> f32 {
        self.starts
            .iter()
            .zip(&self.clips)
            .filter(|&(&start, _)| position >= start)
            .filter_map(|(&start, clip)| clip.samples.get((position - start) as usize))
            .sum()
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

    #[test]
    fn clip_starts_on_the_sample_its_beat_falls_on() {
        // at 120 bpm beat 1 is sample 24000
        let clips = prepared(vec![clip(1.0, 4)], 120.0);

        assert_eq!(clips.sample_at(23999), 0.0);
        assert_eq!(clips.sample_at(24000), 1.0);
        assert_eq!(clips.sample_at(24001), 2.0);
    }

    #[test]
    fn clip_between_samples_starts_on_the_sample_after() {
        // at 130 bpm beat 1 is at sample 22153.8
        let clips = prepared(vec![clip(1.0, 4)], 130.0);

        assert_eq!(clips.sample_at(22153), 0.0);
        assert_eq!(clips.sample_at(22154), 1.0);
    }

    #[test]
    fn clip_is_silent_after_its_end() {
        let clips = prepared(vec![clip(0.0, 3)], 120.0);

        assert_eq!(clips.sample_at(2), 3.0);
        assert_eq!(clips.sample_at(3), 0.0);
    }

    #[test]
    fn overlapping_clips_add_up() {
        let clips = prepared(vec![clip(0.0, 4), clip(0.0, 4)], 120.0);

        assert_eq!(clips.sample_at(1), 4.0);
    }

    #[test]
    fn a_slower_tempo_moves_the_start_but_not_the_speed() {
        // at 60 bpm beat 1 is sample 48000, and the clip still plays a
        // sample per sample
        let clips = prepared(vec![clip(1.0, 4)], 60.0);

        assert_eq!(clips.sample_at(48000), 1.0);
        assert_eq!(clips.sample_at(48003), 4.0);
    }
}
