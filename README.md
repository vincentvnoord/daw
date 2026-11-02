# Vincent Studio

A small DAW written in Rust: synth and sampler tracks, a mixer with gain, pan and meters, live audio inputs, and a transport with a metronome. Built on [cpal](https://github.com/RustAudio/cpal) and [egui](https://github.com/emilk/egui).

## Running

The Nix flake provides the Rust toolchain and the system libraries (ALSA, PipeWire, Wayland/X11, OpenGL):

```sh
nix develop      # or `direnv allow` once
cargo run --release
```

Play notes with the computer keyboard: `Z`–`M` and `S`–`J` for the lower octave, `Q`–`P` and the number row for the upper one. Space starts and stops the transport.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

`testsamples/` holds a few WAV files for trying the sampler; the default project loads the kick from there.
