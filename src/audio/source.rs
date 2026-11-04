use crate::audio::frame::Frame;

/// Most frames a source is asked for at once. Buffers are sized to it up
/// front, so the audio thread never allocates.
pub const MAX_BLOCK_FRAMES: usize = 1024;

pub trait AudioSource: Send + 'static {
    fn begin_buffer(&mut self) {}
    /// Fills `out`, overwriting what was there.
    fn process(&mut self, out: &mut [f32]);
}

pub trait StereoSource: Send + 'static {
    /// Called once per device buffer of `frames` frames, before `process`
    /// is asked for it in one or more pieces.
    fn begin_buffer(&mut self, _frames: usize) {}
    /// Fills `out`, overwriting what was there.
    fn process(&mut self, out: &mut [Frame]);
}
