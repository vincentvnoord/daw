use std::{error::Error, sync::Arc};

use cpal::traits::{DeviceTrait, HostTrait};

use crate::{
    audio::{
        capture::{input_channels, open_input},
        studio::{
            MAX_INPUTS, StudioCommand,
            input::{InputParams, InputSlot, input_queue},
        },
    },
    project::{Clip, Project, Track, TrackSource},
};

/// The UI's side of an audio track in the studio: its input slot, the
/// recording device it monitors, if one is chosen, and its clips.
pub struct AudioHandle {
    /// Index of the track's input slot in the studio.
    pub slot: usize,
    pub params: Arc<InputParams>,
    pub input: Option<LiveInput>,
    /// In no particular order, like placements.
    pub clips: Vec<Clip>,
}

/// A recording device an audio track is listening to.
pub struct LiveInput {
    /// The device's name, plus which of its inputs when it has more than one.
    pub name: String,
    /// Keeps the device recording; dropping it stops the input.
    _stream: cpal::Stream,
}

/// One channel of a recording device, which an audio track can listen to.
/// An audio interface with two inputs shows up as two of these.
pub struct InputDevice {
    /// The device's name.
    pub name: String,
    /// Which of its inputs, from 0.
    pub channel: usize,
    /// How many inputs the device has.
    pub channels: usize,
    device: cpal::Device,
}

impl InputDevice {
    /// "Input 2", or nothing for a device with a single input.
    pub fn channel_label(&self) -> Option<String> {
        (self.channels > 1).then(|| format!("Input {}", self.channel + 1))
    }

    /// What to call this input: the device's name, plus which
    /// of its inputs when it has more than one.
    pub fn input_name(&self) -> String {
        match self.channel_label() {
            Some(label) => format!("{} · {label}", self.name),
            None => self.name.clone(),
        }
    }
}

/// Every input of every recording device on the system. Asking the system
/// can take a moment, so call this when the list is needed, not every frame.
pub fn input_devices() -> Vec<InputDevice> {
    let devices = match cpal::default_host().input_devices() {
        Ok(devices) => devices,
        Err(err) => {
            eprintln!("failed to list input devices: {err}");
            return Vec::new();
        }
    };

    devices
        .flat_map(|device| {
            let name = device
                .description()
                .map_or_else(|_| device.to_string(), |d| d.name().to_owned());
            let channels = input_channels(&device);
            (0..channels).map(move |channel| InputDevice {
                name: name.clone(),
                channel,
                channels,
                device: device.clone(),
            })
        })
        .collect()
}

impl Project {
    /// Adds an audio track with no input yet, playing into a new channel
    /// named after it. Its input is heard as soon as one is chosen, until the
    /// track is muted. Returns its index in `tracks`, or `None` when the
    /// studio is full.
    pub fn add_audio_track(&mut self) -> Option<usize> {
        let slot = self.tracks.iter().filter(|t| t.audio().is_some()).count();
        // room for both the channel and the slot, so we never end up with
        // one without the other
        if slot >= MAX_INPUTS || self.commands.slots() < 2 {
            return None;
        }

        let name = self.unique_track_name("Audio");
        // at unity: a recorded signal is quiet already, unlike a synth at full scale
        let channel = self.add_channel_with_gain(name.clone(), 1.0);
        let (input, params) = InputSlot::create(channel);
        if self.commands.push(StudioCommand::AddInput(input)).is_err() {
            return None;
        }

        self.tracks.push(Track {
            name,
            source: TrackSource::Audio(AudioHandle {
                slot,
                params,
                input: None,
                clips: Vec::new(),
            }),
            placements: Vec::new(),
        });
        Some(self.tracks.len() - 1)
    }

    /// Makes audio track `track` listen to `device`, or to nothing. The old
    /// device stops recording. Does nothing for an instrument track.
    pub fn set_track_input(
        &mut self,
        track: usize,
        device: Option<&InputDevice>,
    ) -> Result<(), Box<dyn Error>> {
        let sample_rate = self.sample_rate as u32;
        let Some(audio) = self.tracks.get(track).and_then(Track::audio) else {
            return Ok(());
        };

        // opened first, so a device that fails to open leaves the old one on
        let (input, samples) = match device {
            Some(device) => {
                let (producer, consumer) = input_queue();
                let stream = open_input(&device.device, device.channel, sample_rate, producer)?;
                let input = LiveInput {
                    name: device.input_name(),
                    _stream: stream,
                };
                (Some(input), Some(consumer))
            }
            None => (None, None),
        };

        let slot = audio.slot;
        self.commands
            .push(StudioCommand::SetInput { slot, samples })
            .map_err(|_| "too many changes at once, try again")?;
        if let Some(audio) = self.tracks.get_mut(track).and_then(Track::audio_mut) {
            audio.input = input;
        }
        Ok(())
    }
}
