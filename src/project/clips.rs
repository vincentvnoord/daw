//! Audio clips on audio tracks: loading them, moving them, and sending each
//! track's clips to the studio.

use std::{error::Error, path::Path, sync::Arc};

use crate::{
    audio::{
        sampler::sample::Sample,
        studio::{
            StudioCommand,
            clips::{Clips, ScheduledClip},
        },
        transport::beats_at,
    },
    project::{Project, Track},
};

/// Samples per peak in a clip's overview. Zoomed out, a pixel of the timeline
/// covers about a thousand samples, so a few peaks make up each pixel.
const PEAK_WINDOW: usize = 256;

/// Audio placed on an audio track. It plays at its own speed whatever the
/// tempo, so its length in beats changes with the tempo.
pub struct Clip {
    pub name: String,
    /// In beats.
    pub start: f64,
    /// Mono, at the project's sample rate. Shared with the studio.
    pub audio: Arc<Vec<f32>>,
    /// The loudest sample in each `PEAK_WINDOW` samples, for drawing the
    /// waveform without going through every sample each frame.
    peaks: Vec<f32>,
}

impl Clip {
    /// Loads a WAV file as a clip starting at `start`, resampled to
    /// `sample_rate`.
    pub fn load(path: &Path, start: f64, sample_rate: f32) -> Result<Self, Box<dyn Error>> {
        let sample = Sample::load(path, sample_rate)?;
        let name = path
            .file_stem()
            .map_or("Clip".into(), |s| s.to_string_lossy().into_owned());
        Ok(Self::new(name, start, sample.data))
    }

    pub fn new(name: String, start: f64, audio: Arc<Vec<f32>>) -> Self {
        let peaks = audio
            .chunks(PEAK_WINDOW)
            .map(|chunk| chunk.iter().fold(0.0, |peak: f32, s| peak.max(s.abs())))
            .collect();
        Self {
            name,
            start,
            audio,
            peaks,
        }
    }

    /// How long it lasts at `bpm`.
    pub fn length_in_beats(&self, bpm: f64, sample_rate: f64) -> f64 {
        beats_at(self.audio.len() as u64, bpm, sample_rate)
    }

    /// The loudest sample from `from` up to `to`, near enough: whole peak
    /// windows are compared, so it can reach a little past either end.
    pub fn peak(&self, from: usize, to: usize) -> f32 {
        if from >= to {
            return 0.0;
        }
        let first = from / PEAK_WINDOW;
        let last = (to - 1) / PEAK_WINDOW + 1;
        self.peaks
            .get(first..last.min(self.peaks.len()))
            .unwrap_or_default()
            .iter()
            .fold(0.0, |peak, &p| peak.max(p))
    }
}

impl Project {
    /// Loads a WAV file onto audio track `track` at `start`, in beats. Does
    /// nothing for an instrument track.
    pub fn add_clip(
        &mut self,
        track: usize,
        path: &Path,
        start: f64,
    ) -> Result<(), Box<dyn Error>> {
        // loading and resampling happen here, never on the audio thread
        let clip = Clip::load(path, start.max(0.0), self.sample_rate)?;
        let Some(audio) = self.tracks.get_mut(track).and_then(Track::audio_mut) else {
            return Ok(());
        };

        audio.clips.push(clip);
        self.send_clips(track);
        Ok(())
    }

    /// Moves clip `index` of track `from` to `start` on track `to`, and sends
    /// both tracks' clips.
    pub fn move_clip(&mut self, from: usize, index: usize, to: usize, start: f64) {
        // only audio tracks play clips
        if self.tracks.get(to).and_then(Track::audio).is_none()
            || self
                .tracks
                .get(from)
                .and_then(Track::audio)
                .is_none_or(|a| index >= a.clips.len())
        {
            return;
        }

        let Some(mut clip) = self.tracks[from].audio_mut().map(|a| a.clips.remove(index)) else {
            return;
        };
        clip.start = start.max(0.0);
        if let Some(audio) = self.tracks[to].audio_mut() {
            audio.clips.push(clip);
        }

        self.send_clips(from);
        if to != from {
            self.send_clips(to);
        }
    }

    /// Sends audio track `track`'s clips to its input slot. Returns `false`
    /// when the command queue is full.
    fn send_clips(&mut self, track: usize) -> bool {
        let Some(audio) = self.tracks.get(track).and_then(Track::audio) else {
            return false;
        };

        let clips = audio
            .clips
            .iter()
            .map(|clip| ScheduledClip {
                start: clip.start,
                samples: Arc::clone(&clip.audio),
            })
            .collect();
        let command = StudioCommand::SetClips {
            slot: audio.slot,
            clips: Clips::new(clips),
        };
        self.commands.push(command).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(samples: Vec<f32>) -> Clip {
        Clip::new("Test".to_owned(), 0.0, Arc::new(samples))
    }

    #[test]
    fn peak_is_the_loudest_sample_in_the_range() {
        let mut samples = vec![0.1; PEAK_WINDOW * 4];
        samples[PEAK_WINDOW * 2 + 5] = 0.8;
        let clip = clip(samples);

        assert_eq!(clip.peak(0, PEAK_WINDOW * 2), 0.1);
        assert_eq!(clip.peak(PEAK_WINDOW * 2, PEAK_WINDOW * 3), 0.8);
    }

    #[test]
    fn a_negative_sample_counts_by_its_size() {
        let clip = clip(vec![0.2, -0.6, 0.3]);

        assert_eq!(clip.peak(0, 3), 0.6);
    }

    #[test]
    fn peak_past_the_end_is_silent() {
        let clip = clip(vec![0.5; 10]);

        assert_eq!(clip.peak(PEAK_WINDOW * 5, PEAK_WINDOW * 6), 0.0);
        assert_eq!(clip.peak(4, 4), 0.0);
    }

    #[test]
    fn length_in_beats_follows_the_tempo() {
        // one second of audio
        let clip = clip(vec![0.0; 48000]);

        assert_eq!(clip.length_in_beats(120.0, 48000.0), 2.0);
        assert_eq!(clip.length_in_beats(60.0, 48000.0), 1.0);
    }
}
