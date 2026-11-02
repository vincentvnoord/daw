use std::sync::Arc;

pub mod channel;
pub mod meter;

use crate::{
    atomic::AtomicF32,
    audio::{
        frame::Frame,
        mixer::{
            channel::{Channel, ChannelId, ChannelRoute},
            meter::Meter,
        },
        source::StereoSource,
    },
};

/// Most channels the mixer can hold. Space for all of them is reserved up
/// front, so adding a channel while audio runs never allocates on the audio
/// thread.
pub const MAX_CHANNELS: usize = 64;

pub struct MixerParams {
    pub gain: AtomicF32,
    pub meter: Meter,
}

pub struct Mixer {
    channels: Vec<Channel>,
    params: Arc<MixerParams>,
    gain: f32,
    peak: f32,
}

impl Mixer {
    pub fn new(params: Arc<MixerParams>) -> Self {
        let gain = params.gain.get();
        Self {
            channels: Vec::with_capacity(MAX_CHANNELS),
            params,
            gain,
            peak: 0.0,
        }
    }

    /// Adds a channel. Gives it back when the mixer is full.
    pub fn add(&mut self, channel: Channel) -> Result<(), Channel> {
        if self.channels.len() >= MAX_CHANNELS {
            return Err(channel);
        }

        self.channels.push(channel);
        Ok(())
    }

    /// Takes a channel out of the mixer. The channels after it move up one
    /// place; nothing is reallocated.
    pub fn remove(&mut self, id: ChannelId) -> Option<Channel> {
        let index = self.index_of(id)?;
        Some(self.channels.remove(index))
    }

    /// Where the channel with this id currently sits.
    pub fn index_of(&self, id: ChannelId) -> Option<usize> {
        self.channels.iter().position(|c| c.id() == id)
    }

    pub fn add_input(&mut self, channel_idx: usize, input: f32) {
        if let Some(ch) = self.channels.get_mut(channel_idx) {
            ch.add_input(input);
        }
    }

    pub fn resolve(&self, route: &ChannelRoute) -> Option<usize> {
        route.get_channel().and_then(|id| self.index_of(id))
    }
}

impl StereoSource for Mixer {
    fn begin_buffer(&mut self) {
        self.params.meter.record(self.peak);
        self.peak = 0.0;

        self.gain = self.params.gain.get();
        for channel in &mut self.channels {
            channel.begin_buffer();
        }
    }

    fn next_frame(&mut self) -> Frame {
        let sum: Frame = self.channels.iter_mut().map(|c| c.process()).sum();
        let out = sum * self.gain;
        self.peak = self.peak.max(out.left.abs().max(out.right.abs()));

        out
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_1_SQRT_2;

    use super::*;

    fn mixer_with(channels: usize) -> (Mixer, Vec<ChannelId>) {
        let mut mixer = Mixer::new(Arc::new(MixerParams {
            gain: AtomicF32::new(1.0),
            meter: Meter::new(),
        }));
        let ids = (0..channels)
            .map(|_| {
                let (channel, params) = Channel::create(1.0);
                assert!(mixer.add(channel).is_ok());
                params.id
            })
            .collect();

        (mixer, ids)
    }

    #[test]
    fn route_follows_its_channel_when_an_earlier_one_is_removed() {
        let (mut mixer, ids) = mixer_with(3);
        let route = ChannelRoute::new(Some(ids[2]));
        assert_eq!(mixer.resolve(&route), Some(2));

        assert!(mixer.remove(ids[0]).is_some());

        assert_eq!(mixer.resolve(&route), Some(1));
    }

    #[test]
    fn route_to_a_removed_channel_goes_nowhere() {
        let (mut mixer, ids) = mixer_with(2);
        let route = ChannelRoute::new(Some(ids[1]));

        assert!(mixer.remove(ids[1]).is_some());

        assert_eq!(mixer.resolve(&route), None);
    }

    #[test]
    fn full_mixer_gives_the_channel_back() {
        let (mut mixer, _) = mixer_with(MAX_CHANNELS);
        let (channel, _) = Channel::create(1.0);

        assert!(mixer.add(channel).is_err());
    }

    #[test]
    fn inputs_are_summed_into_the_mix() {
        let (mut mixer, _) = mixer_with(2);
        mixer.begin_buffer();

        mixer.add_input(0, 0.25);
        mixer.add_input(0, 0.25);
        mixer.add_input(1, 0.5);
        let frame = mixer.next_frame();

        // centred, each side gets 1/sqrt(2) of the mono signal
        assert!((frame.left - FRAC_1_SQRT_2).abs() < 1e-6);
        assert!((frame.right - FRAC_1_SQRT_2).abs() < 1e-6);
    }

    #[test]
    fn inputs_are_cleared_after_each_frame() {
        let (mut mixer, _) = mixer_with(1);
        mixer.begin_buffer();

        mixer.add_input(0, 1.0);
        mixer.next_frame();

        assert_eq!(mixer.next_frame().left, 0.0);
    }
}
