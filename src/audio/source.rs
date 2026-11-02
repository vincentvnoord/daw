use crate::audio::frame::Frame;

pub trait AudioSource: Send + 'static {
    fn next_sample(&mut self) -> f32;
    fn begin_buffer(&mut self) {}
}

pub trait StereoSource: Send + 'static {
    fn next_frame(&mut self) -> Frame;
    /// Called before each buffer of `frames` frames is asked for.
    fn begin_buffer(&mut self, _frames: usize) {}
}
