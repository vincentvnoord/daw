use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::atomic::AtomicF32;

pub mod metronome;

/// Tempo a new project starts at.
pub const DEFAULT_BPM: f32 = 120.0;
pub const MIN_BPM: f32 = 20.0;
pub const MAX_BPM: f32 = 300.0;

pub struct TransportParams {
    pub playing: AtomicBool,
    pub bpm: AtomicF32,
    /// Where the playhead is, in samples. Only the audio thread writes it,
    /// once per buffer.
    pub position: AtomicU64,
    /// Set by the UI to send the playhead back to the start.
    pub rewind: AtomicBool,
}

impl TransportParams {
    pub fn new() -> Self {
        Self {
            playing: AtomicBool::new(false),
            bpm: AtomicF32::new(DEFAULT_BPM),
            position: AtomicU64::new(0),
            rewind: AtomicBool::new(false),
        }
    }

    pub fn toggle_playing(&self) {
        self.playing.fetch_xor(true, Ordering::Relaxed);
    }
}

/// The clock of the studio: whether it's playing, and where the playhead is.
pub struct Transport {
    params: Arc<TransportParams>,
    playing: bool,
    position: u64, // position in samples
    bpm: f64,
    sample_rate: f64,
    /// Whether the playhead was moved this buffer, rather than played to.
    jumped: bool,
    /// Whether playing stopped this buffer.
    stopped: bool,
}

impl Transport {
    pub fn new(params: Arc<TransportParams>, sample_rate: f64) -> Self {
        let bpm = params.bpm.get() as f64;
        Self {
            params,
            playing: false,
            position: 0,
            bpm,
            sample_rate,
            jumped: false,
            stopped: false,
        }
    }

    /// Picks up what the UI changed and tells it where the playhead is.
    pub fn begin_buffer(&mut self) {
        self.jumped = self.params.rewind.swap(false, Ordering::Relaxed);
        if self.jumped {
            self.position = 0;
        }

        let was_playing = self.playing;
        self.playing = self.params.playing.load(Ordering::Relaxed);
        self.stopped = was_playing && !self.playing;
        self.set_bpm(self.params.bpm.get() as f64);

        self.params.position.store(self.position, Ordering::Relaxed);
    }

    /// Changes the tempo without moving the playhead to another beat: the
    /// position in samples is scaled so the position in beats stays the same.
    fn set_bpm(&mut self, bpm: f64) {
        let bpm = bpm.clamp(MIN_BPM as f64, MAX_BPM as f64);
        if bpm == self.bpm {
            return;
        }

        self.position = (self.position as f64 * self.bpm / bpm).round() as u64;
        self.bpm = bpm;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Whether the playhead was moved this buffer (sent back to the start),
    /// so whatever plays along it has to find its place again.
    pub fn jumped(&self) -> bool {
        self.jumped
    }

    /// Whether playing stopped this buffer, so held notes can be released.
    pub fn stopped(&self) -> bool {
        self.stopped
    }

    pub fn advance(&mut self, frames: u64) {
        if self.playing {
            self.position += frames;
        }
    }

    pub fn position_in_beats(&self) -> f64 {
        beats_at(self.position, self.bpm, self.sample_rate)
    }

    /// The index of the beat that starts at the current sample, if one does.
    /// Stopped, the position doesn't move, so no beat starts.
    pub fn beat_starting_now(&self) -> Option<u64> {
        if !self.playing {
            return None;
        }

        let beat = self.position_in_beats().floor() as u64;
        if self.position == 0 {
            return Some(beat);
        }

        let previous = beats_at(self.position - 1, self.bpm, self.sample_rate).floor() as u64;
        (beat != previous).then_some(beat)
    }
}

/// How many beats `position` samples are at this tempo.
pub fn beats_at(position: u64, bpm: f64, sample_rate: f64) -> f64 {
    position as f64 / sample_rate * bpm / 60.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(bpm: f32, playing: bool) -> Arc<TransportParams> {
        let params = TransportParams::new();
        params.bpm.set(bpm);
        params.playing.store(playing, Ordering::Relaxed);
        Arc::new(params)
    }

    fn transport_at(bpm: f32, position: u64) -> Transport {
        let mut transport = Transport::new(params(bpm, true), 48000.0);
        transport.begin_buffer();
        transport.advance(position);
        transport
    }

    #[test]
    fn one_beat_at_120_bpm() {
        assert_eq!(transport_at(120.0, 24000).position_in_beats(), 1.0);
    }

    #[test]
    fn doesnt_move_when_stopped() {
        let mut transport = Transport::new(params(120.0, false), 48000.0);
        transport.begin_buffer();
        transport.advance(24000);

        assert_eq!(transport.position_in_beats(), 0.0);
    }

    #[test]
    fn beat_starts_on_its_first_sample() {
        assert_eq!(transport_at(120.0, 0).beat_starting_now(), Some(0));
        assert_eq!(transport_at(120.0, 23999).beat_starting_now(), None);
        assert_eq!(transport_at(120.0, 24000).beat_starting_now(), Some(1));
        assert_eq!(transport_at(120.0, 24001).beat_starting_now(), None);
    }

    #[test]
    fn no_beat_starts_when_stopped() {
        let mut transport = transport_at(120.0, 24000);
        transport.params.playing.store(false, Ordering::Relaxed);
        transport.begin_buffer();

        assert_eq!(transport.beat_starting_now(), None);
    }

    #[test]
    fn beat_between_samples_starts_on_the_sample_after() {
        // at 130 bpm a beat is 22153.8 samples long
        assert_eq!(transport_at(130.0, 22153).beat_starting_now(), None);
        assert_eq!(transport_at(130.0, 22154).beat_starting_now(), Some(1));
        assert_eq!(transport_at(130.0, 22155).beat_starting_now(), None);
    }

    #[test]
    fn changing_tempo_keeps_the_beat_position() {
        let mut transport = transport_at(120.0, 36000); // 1.5 beats
        transport.params.bpm.set(60.0);
        transport.begin_buffer();

        assert_eq!(transport.position_in_beats(), 1.5);
    }

    #[test]
    fn rewind_goes_back_to_the_start() {
        let mut transport = transport_at(120.0, 36000);
        transport.params.rewind.store(true, Ordering::Relaxed);
        transport.begin_buffer();

        assert_eq!(transport.position_in_beats(), 0.0);
        assert!(!transport.params.rewind.load(Ordering::Relaxed));
    }

    #[test]
    fn rewind_counts_as_a_jump_for_one_buffer() {
        let mut transport = transport_at(120.0, 36000);
        assert!(!transport.jumped());

        transport.params.rewind.store(true, Ordering::Relaxed);
        transport.begin_buffer();
        assert!(transport.jumped());

        transport.begin_buffer();
        assert!(!transport.jumped());
    }

    #[test]
    fn stopping_is_seen_for_one_buffer() {
        let mut transport = transport_at(120.0, 1000);
        assert!(!transport.stopped());

        transport.params.playing.store(false, Ordering::Relaxed);
        transport.begin_buffer();
        assert!(transport.stopped());

        transport.begin_buffer();
        assert!(!transport.stopped());
    }

    #[test]
    fn ui_sees_the_position() {
        let mut transport = transport_at(120.0, 1000);
        transport.begin_buffer();

        assert_eq!(transport.params.position.load(Ordering::Relaxed), 1000);
    }
}
