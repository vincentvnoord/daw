use std::sync::Arc;

use crate::{
    audio::{
        mixer::{
            MAX_CHANNELS,
            channel::{Channel, ChannelId, ChannelParams, DEFAULT_GAIN},
        },
        studio::StudioCommand,
    },
    project::{Project, Track},
};

/// The UI's side of a mixer channel.
pub struct ChannelHandle {
    pub name: String,
    pub params: Arc<ChannelParams>,
}

impl ChannelHandle {
    pub fn id(&self) -> ChannelId {
        self.params.id
    }
}

impl Project {
    /// Adds a mixer channel while the audio is running. Returns its id, or
    /// `None` when the mixer is full or the command queue is.
    pub fn add_channel(&mut self, name: String) -> Option<ChannelId> {
        self.add_channel_with_gain(name, DEFAULT_GAIN)
    }

    /// Like `add_channel`, with the fader starting at `gain`.
    pub fn add_channel_with_gain(&mut self, name: String, gain: f32) -> Option<ChannelId> {
        if self.channels.len() >= MAX_CHANNELS {
            return None;
        }

        // built here so the audio thread only has to push it into the mixer
        let (channel, params) = Channel::create(gain);
        if self
            .commands
            .push(StudioCommand::AddChannel(channel))
            .is_err()
        {
            return None;
        }

        let id = params.id;
        self.channels.push(ChannelHandle { name, params });
        Some(id)
    }

    /// A name for a new channel that nobody named.
    pub fn next_channel_name(&self) -> String {
        format!("Channel {}", self.channels.len() + 1)
    }

    /// Removes a mixer channel while the audio is running. Tracks that
    /// played into it get no output.
    pub fn remove_channel(&mut self, id: ChannelId) {
        if self
            .commands
            .push(StudioCommand::RemoveChannel(id))
            .is_err()
        {
            return;
        }

        self.channels.retain(|c| c.id() != id);
        for route in self.tracks.iter().map(Track::route) {
            if route.get_channel() == Some(id) {
                route.set_channel(None);
            }
        }
    }

    pub fn channel(&self, id: ChannelId) -> Option<&ChannelHandle> {
        self.channels.iter().find(|c| c.id() == id)
    }
}
