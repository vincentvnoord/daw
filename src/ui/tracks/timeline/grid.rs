use eframe::egui::Rect;

pub(super) const PIXELS_PER_BEAT: f32 = 24.0;

/// Turns beats into x positions and back, and knows which part is on screen
/// so only that part gets drawn.
pub(super) struct Grid {
    /// Where beat 0 is.
    origin: f32,
    pub(super) visible: Rect,
    /// At the current tempo, for clips: their audio is in samples.
    pub(super) samples_per_beat: f64,
}

impl Grid {
    pub(super) fn new(origin: f32, visible: Rect, samples_per_beat: f64) -> Self {
        Self {
            origin,
            visible,
            samples_per_beat,
        }
    }

    pub(super) fn x(&self, beat: f64) -> f32 {
        self.origin + beat as f32 * PIXELS_PER_BEAT
    }

    pub(super) fn beat(&self, x: f32) -> f64 {
        ((x - self.origin) / PIXELS_PER_BEAT) as f64
    }

    /// Every whole beat on screen, with its x.
    pub(super) fn visible_beats(&self) -> impl Iterator<Item = (u64, f32)> + '_ {
        let first = self.beat(self.visible.left()).floor().max(0.0) as u64;
        (first..)
            .map(|beat| (beat, self.x(beat as f64)))
            .take_while(|&(_, x)| x <= self.visible.right())
    }
}
