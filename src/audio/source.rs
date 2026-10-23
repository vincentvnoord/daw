pub trait AudioSource: Send + 'static {
    fn next_sample(&mut self) -> f32;
    fn begin_buffer(&mut self) {}
}
