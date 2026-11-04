//! How the UI changes the studio while audio runs: commands come in on one
//! queue, and whatever they leave behind goes back on another.

use rtrb::Consumer;

use crate::audio::{
    mixer::channel::{Channel, ChannelId},
    schedule::Schedule,
    studio::{
        MAX_INPUTS, MAX_INSTRUMENTS, Studio, clips::Clips, input::InputSlot,
        instrument::InstrumentSlot,
    },
};

/// Changes the UI asks the studio to make while the audio is running.
pub enum StudioCommand {
    AddChannel(Channel),
    RemoveChannel(ChannelId),
    AddInstrument(InstrumentSlot),
    AddInput(InputSlot),
    /// Gives input slot `slot` another recording device to read from, or none.
    SetInput {
        slot: usize,
        samples: Option<Consumer<f32>>,
    },
    /// Replaces the clips input slot `slot` plays.
    SetClips {
        slot: usize,
        clips: Clips,
    },
    /// Replaces what plays along the transport.
    SetSchedule(Schedule),
}

/// Things the studio no longer uses. They go back to the UI thread to be
/// dropped there, because freeing memory on the audio thread can glitch.
#[allow(dead_code)] // only held so it gets dropped on the UI thread
pub enum Garbage {
    Channel(Channel),
    Instrument(InstrumentSlot),
    Input(InputSlot),
    InputQueue(Consumer<f32>),
    Clips(Clips),
    Schedule(Schedule),
}

impl StudioCommand {
    /// Makes the change on `studio`. Returns what it replaced, or what there
    /// was no room or no slot for, to be dropped on the UI thread.
    fn apply(self, studio: &mut Studio) -> Option<Garbage> {
        match self {
            Self::AddChannel(channel) => studio.mixer.add(channel).err().map(Garbage::Channel),
            Self::RemoveChannel(id) => studio.mixer.remove(id).map(Garbage::Channel),
            Self::AddInstrument(slot) => {
                push_within(&mut studio.instruments, slot, MAX_INSTRUMENTS)
                    .err()
                    .map(Garbage::Instrument)
            }
            Self::AddInput(slot) => push_within(&mut studio.inputs, slot, MAX_INPUTS)
                .err()
                .map(Garbage::Input),
            // with no such slot the new queue or clips go back unused
            Self::SetInput { slot, samples } => match studio.inputs.get_mut(slot) {
                Some(input) => input.set_input(samples),
                None => samples,
            }
            .map(Garbage::InputQueue),
            Self::SetClips { slot, clips } => {
                Some(Garbage::Clips(match studio.inputs.get_mut(slot) {
                    Some(input) => input.set_clips(clips),
                    None => clips,
                }))
            }
            Self::SetSchedule(schedule) => {
                let old = studio
                    .sequencer
                    .set_schedule(schedule, &mut studio.instruments);
                Some(Garbage::Schedule(old))
            }
        }
    }
}

impl Studio {
    /// Applies every command waiting, and sends back what they leave behind.
    pub(super) fn handle_commands(&mut self) {
        while let Ok(command) = self.commands.pop() {
            if let Some(garbage) = command.apply(self) {
                self.discard(garbage);
            }
        }
    }

    fn discard(&mut self, garbage: Garbage) {
        // if the UI hasn't emptied the queue it gets dropped here after all,
        // which is better than keeping it forever
        let _ = self.garbage.push(garbage);
    }
}

/// Pushes `item` unless `items` already holds `max`, in which case it comes
/// back. With `max` reserved up front, the push never reallocates.
fn push_within<T>(items: &mut Vec<T>, item: T, max: usize) -> Result<(), T> {
    if items.len() >= max {
        return Err(item);
    }

    items.push(item);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rtrb::RingBuffer;

    use super::*;
    use crate::{
        atomic::AtomicF32,
        audio::{
            mixer::{MixerParams, meter::Meter},
            studio::StudioStats,
            transport::{TransportParams, metronome::MetronomeParams},
        },
    };

    fn studio() -> Studio {
        let (_, commands) = RingBuffer::new(1);
        let (garbage, _) = RingBuffer::new(1);
        Studio::new(
            Arc::new(MixerParams {
                gain: AtomicF32::new(1.0),
                meter: Meter::new(),
            }),
            Arc::new(TransportParams::new()),
            Arc::new(MetronomeParams::new()),
            commands,
            garbage,
            Arc::new(StudioStats::default()),
            48000.0,
        )
    }

    #[test]
    fn a_full_list_gives_the_item_back() {
        let mut items = vec![1, 2];

        assert_eq!(push_within(&mut items, 3, 2), Err(3));
        assert_eq!(push_within(&mut items, 3, 3), Ok(()));
        assert_eq!(items, [1, 2, 3]);
    }

    #[test]
    fn clips_for_a_missing_slot_go_back_as_garbage() {
        let command = StudioCommand::SetClips {
            slot: 0,
            clips: Clips::empty(),
        };

        assert!(matches!(
            command.apply(&mut studio()),
            Some(Garbage::Clips(_))
        ));
    }

    #[test]
    fn removing_an_unknown_channel_leaves_nothing_behind() {
        let (channel, params) = Channel::create(1.0);
        drop(channel);

        let command = StudioCommand::RemoveChannel(params.id);

        assert!(command.apply(&mut studio()).is_none());
    }
}
