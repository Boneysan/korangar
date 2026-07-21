# Korangar — agent notes

Rust Ragnarok Online client (wgpu 29 + winit). This fork's goal is a usable
custom UI for a friends group + DM campaign.

**Start here for all documentation:**
- [docs/README.md](docs/README.md) — Master index/hub with categories, search guidance, and links to everything.
- Then `docs/CLIENT_SYSTEMS_OVERVIEW.md` and `docs/SOFTWARE_DESIGN.md` for architecture.

See the Documentation Hub above for the full list (DM tools, packets, world, graphics, plans, specs, etc.).

**NEXT animation task (2026-07-22):** Phase D is **DONE** (live-green 2026-07-21).
**Phase E1 code is closed** (Napalm/Soul Strike/Frost Diver/Fire Ball/Jupitel/
Earth Spike/Heaven's Drive procedural recipes) — needs a **live GUI pass** then
E2 (persistent skill units). Plan: [docs/plans/animation-fidelity.md](docs/plans/animation-fidelity.md) §6.
Fixes shipped while closing D (all on `agent/platform-connectivity-controls`):
armed players stand in the ReadyFight stance so weapon+shield render at idle
(`78f57915`), and relax to Idle on town/safe maps (`abb519f1`); respawn/resurrect
revive the sprite (`78f57915`); ammo (arrows) equips/stacks/shows count and a
normal bow attack draws a flying arrow (`d3f7c5dd`, `2b637bac`) — see
[docs/protocol/inventory-and-ranged-attacks.md](docs/protocol/inventory-and-ranged-attacks.md).
Known open sub-follow-ups: gun-bullet/shuriken projectiles, per-arrow-type sprites.

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

## Running on macOS

See `docs/MACOS_WORKFLOW.md` for the full workflow (starting the Hercules
server via `athena-start`, building/running the client, process management)
and `docs/PLATFORM_BRINGUP.md` for the cross-platform bring-up runbook
(account bootstrap, `loginlog` debugging, known client bugs — read it before
standing up Windows/Linux/WSL).
Short version: no special env vars needed (wgpu picks Metal natively) —
`cargo run --release --bin korangar` from `korangar/korangar/`. That doc
also covers a startup panic (`engine.rs:766` unwrap on `None` surface, from
AppKit calling `drawRect:` before `resumed()` finishes) that's fixed in-tree.

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

## Testing — the headless suite

The project's main automated regression gate is a **107-scenario headless client**
(`korangar-networking/examples/headless-tester`). Acceptance passed 2026-07-13 with a
double-run green gate. Docs live in **`tools/testing/`** (not `docs/`):
[headless_test_plan.md](tools/testing/headless_test_plan.md) is canonical,
[headless_findings.md](tools/testing/headless_findings.md) is the bug log,
[testing_guide.md](tools/testing/testing_guide.md) is the overall reference.

Run it with the servers already up:

```sh
cargo run --release --example headless-tester -p korangar-networking -- --scenario all
```

Coverage (107): session/lifecycle 8 · GM commands 9 · movement 5 · combat 3 · skills 44
(39 job-class sweeps + teleport/weapon-refine menus) · items 12 · dialogue 5 · social 7 ·
DM tooling 14.

**What it proves, and what it doesn't.** The headless tester links the *same*
`ragnarok-packets` and `korangar-networking` crates as the graphical client, so a
packet/framing/event-mapping bug found headlessly is fixed for the real client too (it
must still get a regression unit test in `ragnarok-packets`). It **cannot** catch a bug in
how `korangar/src/` *consumes* an event — the UI/state layer. So headless-green means "the
wire data is correct", which usefully isolates any remaining bug to the UI.

This is why a feature can be headless-green while its row in
`docs/plans/M1-p0-verification.md` (the **GUI** live pass) is still unchecked. Those are
different axes; see the axes note in [docs/README.md](docs/README.md). Do not report a
headless pass as "verified working in the client".

## Architecture & Development Rules

When writing code or adding features, agents must adhere to these project-specific constraints:

1. **Tabletop Scope**: This fork is the "Seal Cascade" D&D campaign engine. When designing UI or features, prioritize the tabletop/DM tools outlined in `docs/DM_INTERFACE.md` over generic RO MMO features (like auction houses or matchmaking).
2. **Packet Registration**: Due to Korangar's framing-by-deserialization design, an unregistered packet header would desync the read buffer. **Framing is now handled automatically**: `register_length_fallbacks` (called last in each of the three `register_*_server_packets` functions — login, character, and map) consumes any known-length server packet that lacks a dedicated handler, using a table auto-generated from Hercules' own length tables (`tools/generate_packet_lengths.sh` → `lengths_20220406.rs`). See `docs/protocol/packet-length-fallbacks.md`. The server **must be built with `--enable-packetver=20220406`** (a default Hercules build is 20190605 and is wire-incompatible at the map handoff — see `docs/PLATFORM_BRINGUP.md` item 0; an earlier version of this note wrongly said the server was 20190605). Regenerate the table if the server's PACKETVER ever changes. You only need to define/register a packet in `ragnarok-packets` + `version_20220406.rs` when the client actually needs its **contents** (a fallback-consumed packet produces no `NetworkEvent`); use `register_noop` for a modeled-but-unhandled packet.
3. **Packet Obfuscation**: The server (`Hercules_RO`) is configured with `packet_obfuscation: 0`. **Do not** attempt to implement packet obfuscation in the Korangar networking layer.
4. **Rebaseability**: Keep custom UI features isolated in `korangar/src/interface/windows/dm/` (and state in `korangar/src/dm/`) as much as possible to ensure the fork remains rebaseable against upstream Korangar.
5. **No Upstream IP**: Per `wiki/Contributing.md`, do not include code taken directly from or inspired by GRAVITY's intellectual property.
