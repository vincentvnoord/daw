# Vincent Studio

A DAW in Rust: cpal for audio, egui (eframe) for the UI. Run everything inside `nix develop` (or direnv); the flake provides the toolchain and the ALSA, PipeWire and windowing libraries.

## Collaboration

The user writes the code and is learning by building. Guide and review: explain the design, point at the files, sketch the shape. Implement only when asked. Leave building, testing and formatting to the user after small changes.

## Architecture

Two threads, and the `project` / `audio` split follows them:

- `src/project/` is the UI thread's side: `Project` owns the tracks, channels and inputs as handles, and every change to the studio goes through a method on it.
- `src/audio/` is the audio thread's side: `Studio` (in `audio/studio/`) is the one `StereoSource` the engine plays. `audio` never imports from `project` or `ui`.
- `src/ui/` draws the `Project`; panels in `ui/`, reusable controls in `ui/widgets/`.

The threads talk in exactly four ways. A new feature picks one of these:

| Path | Direction | For |
| --- | --- | --- |
| `Arc<…Params>` with atomics (`atomic::AtomicF32`) | both | continuous values: gain, pan, bpm, playhead. Read once per buffer in `begin_buffer`. |
| `StudioCommand` queue | UI → audio | structural changes: add/remove channels, instruments, inputs |
| `Garbage` queue | audio → UI | anything the studio drops, freed on the UI thread |
| per-instrument `NoteEvent` ring buffer | UI → audio | live keyboard notes |

Signal flow per sample: instrument and input slots produce mono `f32` → `Mixer::add_input` on the channel their `ChannelRoute` resolves to → `Channel` applies gain and pan into a stereo `Frame` → master gain → metronome click added after the mixer.

## Real-time rules

The audio thread never allocates, frees, locks or blocks. In practice:

- Build objects on the UI thread and send them whole in a `StudioCommand`.
- Reserve capacity up front (`MAX_CHANNELS`, `MAX_INSTRUMENTS`, `MAX_INPUTS`) so pushes never reallocate.
- Send removed objects back through `Garbage`.
- Refer to things across threads by id (`ChannelId`), resolved to an index once per buffer, so a removal never shifts a route onto the wrong target.

## Timeline

Design and build order agreed so far: tracks own one instrument each and send to a channel (many tracks can share one); sequences hold notes in beats and are placed on tracks; the UI flattens the tree into a sorted schedule the studio plays against the `Transport`. Scheduled notes must start on their exact sample, as the metronome's clicks do.

## Style

- Doc comments say what a thing is for and why; inline comments explain the non-obvious, in lowercase. Match the density of the surrounding file.
- Tests live in a `mod tests` at the bottom of the file they test, named as plain sentences (`route_follows_its_channel_when_an_earlier_one_is_removed`).
- CI runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`.
