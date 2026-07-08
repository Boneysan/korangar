# Debug Tools, Audio & Video Subsystems

**Purpose**: Documentation of the less central but still important subsystems: audio, video playback, and the rich set of debug tools.

These are "obscure" in the sense that they are not on the critical path for DM features or core engine work, but engineers still need to understand them when they touch related areas.

## Audio (korangar-audio)

Standalone crate: `korangar-audio/`

Architecture (from crate structure):
- `manager.rs` — central `AudioEngine`
- `sound/` — `StaticSound`, `StreamingSound`
- `track/`
- `backend/` — cpal (desktop + wasm), with platform specifics.
- `listener.rs`, `frame.rs`, `decibels.rs`, etc.

Key concepts:
- 3D spatial audio (for world sounds, BGM?).
- Streaming vs static.
- Parameters (volume, pitch, etc.).
- Tweening.

**Integration**:
- In `lib.rs`: `audio_engine.play_sound_effect(...)`, ambient sounds, background music.
- Triggered from world entities, effects, UI clicks, etc.
- On map change: `audio_engine.clear_ambient_sound()`.

**WSL note** (from CLAUDE.md):
- Requires `libasound2-plugins` + `~/.asoundrc` routing to PulseAudio via WSLg.
- `PULSE_SERVER=unix:/mnt/wslg/PulseServer`

**Extension**:
- Add new sound types by extending the manager.
- Spatial audio parameters come from entity positions and listener.

For DM: Useful for custom hazard sounds, dice roll SFX, etc.

## Video (korangar-video)

Small crate for IVF playback (simple container, likely VP8/VP9 or similar).

Used for cutscenes / login videos?

See `korangar-video/src/ivf/`.

Integration is limited — mostly a playback component that can be attached to the renderer or a dedicated surface.

Not heavily used in the DM campaign flow today.

## Debug Tools

These are extremely valuable for engineers and are gated behind the `debug` feature.

### Major Debug Windows

Located in `src/interface/windows/` with `#[cfg(feature = "debug")]`:

- `packet_inspector.rs` — Shows every incoming/outgoing packet with pretty printing (uses `Packet::to_element`).
- `ClientStateInspector` — Walks the entire `ClientState` via `StateWindow`.
- `ThemeInspector`
- `FrameInspector`
- `Profiler` window
- `RenderOptions`
- `CacheStatistics`
- `Commands` window (generic debug commands)

### Packet History & Callback

In `src/networking/mod.rs`:

- `PacketHistoryCallback` implements `PacketCallback`.
- Every packet (in/out/unknown/error) is recorded with full deserialized form.
- Rendered using the same element system as normal UI.

This is one of the best debugging tools for protocol work.

### Profiling

- `korangar-debug` crate.
- `#[korangar_debug::profile]` attributes.
- Ring buffer + statistics.
- Visible in the profiler window.

Used in loaders, render paths, input, etc.

### Other Debug Aids

- `KORANGAR_PACKET_LOG=1` → hex dumps of packets.
- Picker debug visualization.
- Pathing mesh generation (in debug builds for entities).
- Various inspectors that stay open across map changes for convenience.

## How These Fit Into the Architecture

- Audio is a side-effect system driven from world + UI events.
- Video is a specialized playback path (rarely used).
- Debug tools are built on the same UI + state primitives as everything else, which is why they can inspect packets and `ClientState` so effectively.

## Extension Advice

- When adding a new DM feature, consider whether it should produce audio or have debug visualization.
- New debug windows should follow the same `CustomWindow` pattern.
- For audio, prefer going through the `AudioEngine` rather than raw cpal.

## Cross References

- `CLAUDE.md` (WSL audio setup)
- `CLIENT_SYSTEMS_OVERVIEW.md` (mentions audio/video as lower priority)
- `PACKET_EVENTS_CATALOG.md` (packet inspector is the best way to explore events)
- `GRAPHICS_PIPELINE.md` (some debug visualization lives in render passes)

These subsystems are intentionally lightweight so they don't interfere with the core DM + engine work.
