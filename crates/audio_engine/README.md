# crates/audio_engine

Rust crate for audio decoding + playback utilities.

## Implemented
- **`decode_to_pcm`** — decode audio files (MP3, WAV, FLAC, OGG/Vorbis) to interleaved f32 PCM via Symphonia
- **`mixdown_mono`** — mix multi-channel audio to mono
- **`PcmBuffer`** — sample rate, channel count, interleaved frames

## Planned
- Playback clock + transport controls
- Time-stretch / pitch-shift for practice mode
