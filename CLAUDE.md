# Korangar — agent notes

Rust Ragnarok Online client (wgpu 29 + winit). This fork's goal is a usable
custom UI for a friends group + DM campaign.

**Start here for all documentation:**
- [docs/README.md](docs/README.md) — Master index/hub with categories, search guidance, and links to everything.
- Then `docs/CLIENT_SYSTEMS_OVERVIEW.md` and `docs/SOFTWARE_DESIGN.md` for architecture.

See the Documentation Hub above for the full list (DM tools, packets, world, graphics, plans, specs, etc.).

**DM Data Assets (key for upcoming E7 work)**: See new `docs/DM_DATA_GUIDE.md` for how bestiary.json, items.json, cards.json integrate with bestiary journal, encounters, rewards, loot tables. Codex should use these for data-driven DM features.

## Running on this machine (WSL2)

Use `./run-wsl.sh`. Do NOT plain `cargo run` — it will silently fall back to
llvmpipe (CPU) software rendering.

Why: Ubuntu's Mesa does not ship the Dozen (dzn) Vulkan-on-D3D12 driver, so
Vulkan inside WSL has no hardware device. The OpenGL-on-D3D12 driver IS
available but must be forced. The script sets:

- `GALLIUM_DRIVER=d3d12` — use the D3D12 passthrough GL driver instead of llvmpipe
- `MESA_D3D12_DEFAULT_ADAPTER_NAME=NVIDIA` — pick the RTX 5060 Ti, not the Intel iGPU
- `WGPU_BACKEND=gl` — wgpu would otherwise pick the (software-only) Vulkan device

Verify hardware rendering by checking the startup log for
`using adapter D3D12 (NVIDIA GeForce RTX 5060 Ti) (gl)`.

Caveats of the GL backend: `TEXTURE_BINDING_ARRAY`, `PARTIALLY_BOUND_BINDING_ARRAY`,
and non-uniform indexing are unsupported; Korangar detects this and uses fallback
paths. Fine for UI/feature work — do not use WSL for performance judgments.

GL support required client-side fixes (2026-07-06, all in-tree now; native
Vulkan/DX12/Metal behavior preserved):

- wgpu needs the winit display handle at instance creation (`display:` in
  `InstanceDescriptor`) or EGL cannot create a presentable surface.
- GLSL cannot bind one texture to multiple samplers, read a depth texture
  without a comparison, or (via naga) translate `GatherCmp` — shaders use
  texel-snapped linear sampling, separate non-shadow bindings (9/10 in the
  forward pass group) for raw depth reads, and `SampleCmp`-based PCF instead.
- **MSAA resolve silently produces black frames** on GL (the black-screen
  symptom: UI visible, world black). `Capabilities` reports only `Msaa::Off`
  on GL and `GraphicsEngine::on_resume` clamps saved settings accordingly.
- Runtime BC7 texture compression works on GL; no special handling.

## Audio in WSL

Works via ALSA → PulseAudio → WSLg (`PULSE_SERVER=unix:/mnt/wslg/PulseServer`).
This required (already done on this machine, 2026-07-06):

1. `sudo apt install libasound2-plugins` — the ALSA pulse plugin
2. `~/.asoundrc` routing `pcm.!default` / `ctl.!default` to `pulse` — the plugin
   is not the default device without this

If ALSA errors like `cannot find card '0'` appear at startup, one of these
two pieces is missing.

## Windows builds

Native Windows toolchain development is blocked by BitDefender flagging build
tooling. The intended path for native-performance testing and distribution is
cross-compiling from WSL with `cargo-xwin` (target `x86_64-pc-windows-msvc`)
and running the `.exe` on the Windows side (not yet set up).

## Architecture & Development Rules

When writing code or adding features, agents must adhere to these project-specific constraints:

1. **Tabletop Scope**: This fork is the "Seal Cascade" D&D campaign engine. When designing UI or features, prioritize the tabletop/DM tools outlined in `docs/DM_INTERFACE.md` over generic RO MMO features (like auction houses or matchmaking).
2. **Packet Registration**: Due to Korangar's framing-by-deserialization design, an unregistered packet header would desync the read buffer. **Framing is now handled automatically**: `register_length_fallbacks` (called last in `register_map_server_packets`) consumes any known-length server packet that lacks a dedicated handler, using a table auto-generated from Hercules' own length tables (`tools/generate_packet_lengths.sh` → `lengths_20220406.rs`). See `docs/protocol/packet-length-fallbacks.md`. This means the server is **PACKETVER 20190605** (main), not 20220406 — regenerate the table if that changes. You only need to define/register a packet in `ragnarok-packets` + `version_20220406.rs` when the client actually needs its **contents** (a fallback-consumed packet produces no `NetworkEvent`); use `register_noop` for a modeled-but-unhandled packet.
3. **Packet Obfuscation**: The server (`Hercules_RO`) is configured with `packet_obfuscation: 0`. **Do not** attempt to implement packet obfuscation in the Korangar networking layer.
4. **Rebaseability**: Keep custom UI features isolated in `korangar/src/interface/windows/dm/` (and state in `korangar/src/dm/`) as much as possible to ensure the fork remains rebaseable against upstream Korangar.
5. **No Upstream IP**: Per `wiki/Contributing.md`, do not include code taken directly from or inspired by GRAVITY's intellectual property.
