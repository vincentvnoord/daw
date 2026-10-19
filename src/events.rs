#[derive(Clone, Copy, Debug)]
pub enum NoteEvent {
    On(u8),
    Off(u8),
}
