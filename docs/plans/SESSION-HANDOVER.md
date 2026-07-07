# Session Handover — GL Backend, Audio, Packet Desync

**Date:** 2026-07-06 (evening session)
**Status:** Paused mid-investigation. Graphics + audio working. MariaDB was
started manually and a live retest was performed. That retest rejected the
`0x089C` map-enter attempt, proving the shuffle-specific 20220406 mapping must
come from `packets_shuffle_main.h`, not the broad late `packets.h` block alone.
The corrected packet set has been rebuilt and live-retested successfully.

**2026-07-07 continuation:** `cargo test -p ragnarok-packets --lib` passes
(9 tests), and `cargo build -p korangar --features "debug unicode"` passes.
The full `cargo test -p ragnarok-packets` still fails before tests because the
`pcap` example links `-lpcap`, which is not installed.

**2026-07-07 later continuation:** live packet logging pushed the map parser
past two more server packets. `0x0A3B` was identified as
`ZC_EQUIPMENT_EFFECT` and `0x097B` as `ZC_PERSONAL_INFOMATION`; both are now
parsed and ignored so the audit can continue to later map traffic. The client
is still receiving NPC/entity traffic inconsistently, so the remaining work is
the packet-gap audit rather than the original login/map-entry desync.

**2026-07-07 live-client continuation:** MariaDB/Hercules were running and the
user logged into Korangar against the local server. Live testing in Izlude found
three client issues now addressed in this commit: server-side warp trigger NPCs
no longer render as broken shadow/`!` entities, RO `<NAVI><INFO>` dialogue
markup is stripped to readable labels until clickable quest breadcrumbs are
implemented, and the skill tree has a clearer "Rank up skills" flow with visible
`+` rank controls. Skill names/tabs now fall back to ASCII labels when Korean
client Lua strings would render as square boxes. Follow-up: restart the live
Korangar process after pulling, and regenerate `korangar/lua_files.7z` if the
local archive list changes.

This is a working handover note, not a permanent design doc. Delete it once
the packet work lands and the findings are folded into
[packet-gap-party-whisper.md](packet-gap-party-whisper.md) and CLAUDE.md.

---

## 1. What works now (verified with screenshots + server logs)

- **GPU rendering in WSL** — full 3D world renders on the RTX via GL-over-D3D12.
- **Login → char select → enter map** — reaches the in-game world.
- **Sound effects** — working.
- **Background music** — working (login theme + map BGM audible).

## 2. Fixes made this session (ALL UNCOMMITTED — see §5)

### GL backend rendering (root cause of the "black world, UI visible" bug)
The forward pass used **MSAA**, whose multisample *resolve* silently produces
black output on Mesa's d3d12 GL driver. Plus several GLSL limitations had to be
worked around. Files:
- `korangar/src/lib.rs` — pass the winit display handle to wgpu's
  `InstanceDescriptor` (EGL needs it to make a presentable surface; without it
  the client panicked at `surface.rs:59`).
- `korangar/src/graphics/capabilities.rs` — report only `Msaa::Off` on the GL
  backend; also (temporarily removed then irrelevant) BC7 gate — BC7 works, no
  gate needed.
- `korangar/src/graphics/engine.rs` — clamp any saved MSAA setting the adapter
  doesn't support down to `Off` in `on_resume`.
- `korangar/src/graphics/mod.rs` — forward bind group: added bindings 9 & 10
  (unfiltered float views of the directional + point shadow maps for raw depth
  reads), depth entries → `Float{filterable:false}`, nearest sampler →
  `NonFiltering`.
- `korangar/shaders/**` — one-texture-multiple-samplers → texel-snapped linear
  sampling; `GatherCmp` → `SampleCmp` PCF; raw depth reads via separate
  non-shadow texture bindings. (rectangle, model, entity, indicator, forward
  module, sdsm, debug_buffer shaders.)

Native Vulkan/DX12/Metal behavior is preserved (MSAA still offered there; PCF
is equivalent quality).

### Audio
- `korangar/client/audio_settings.ron` — `mute_on_focus_loss: false` (was true;
  it muted whenever the window lost focus). User preference — revert if desired.
- `korangar/src/loaders/archive/folder/mod.rs` — `FolderArchive::load_mapping`
  now uses `WalkDir::follow_links(true)` so symlinked asset dirs are scanned.
- **`korangar/bgm` symlink → `/mnt/h/RO/client/BGM`** — music files are read
  from the working dir, NOT the GRF archives. This symlink is why music works.
  It is currently UNTRACKED and machine-specific (points into `/mnt/h`). Decide:
  gitignore it, or make BGM path configurable.

### Packet protocol
- `ragnarok-packets/src/lib.rs` — **`MapServerLoginPacket`: corrected to the
  20220406 main shuffle layout.** The exact Hercules block is in
  `src/map/packets_shuffle_main.h`, not only `src/map/packets.h`. The correct
  packet is `0x0436` with fields at offsets `2,6,10,18,22`, so Korangar now
  sends `account_id`, `character_id`, `login_id1`, `login_id2`, `client_tick`,
  and `sex` for a 23-byte map-enter packet.
- `ragnarok-packets/src/lib.rs` — **corrected the 20220406 main map opcodes
  from the exact `packets_shuffle_main.h` block:**
  - `MapServerLoginPacket` / `pWantToConnection`: `0x0436` (23 bytes)
  - `RequestServerTickPacket` / `pTickSend`: `0x0360` (6 bytes)
  - `RequestPlayerMovePacket` / `pWalkToXY`: `0x035F` (5 bytes)
  - `RequestActionPacket` / `pActionRequest`: `0x0437` (7 bytes)
  - `ItemPickupRequestPacket` / `pTakeItem`: `0x0362` (6 bytes)
  - `RequestDetailsPacket` / `pGetCharNameRequest`: `0x0368` (6 bytes)
  - `UseSkillAtIdPacket` / `pUseSkillToId`: `0x0438` (10 bytes)
- `ragnarok-packets/src/lib.rs` — added live-decoded server packets
  `EquipmentEffectPacket` (`0x0A3B`) and `PersonalInformationPacket`
  (`0x097B`) so the parser can keep advancing through map traffic.
- `korangar-networking/src/packet_versions/version_20220406.rs` — registered
  the same packets as no-ops on the 20220406 map handler.
- `korangar-networking/src/lib.rs` — **DEBUG INSTRUMENTATION** (temporary):
  `KORANGAR_PACKET_LOG` env var hex-dumps every outgoing map packet. Added to
  `send_map_server_packet`, the keep-alive path, and the map-enter direct send.
  Decide whether to keep (behind the env flag) or strip before committing.

## 3. Packet desync result: fixed for login/map-entry

**Previous symptom:** immediately after entering the map, the client was
disconnected and auto-reconnected in a loop until it gave up. Server log
(map-server): `Received unsupported packet (packet 0x0000 (0x0000), 2 bytes
received), disconnecting session #7`. Before the `MapServerLoginPacket` layout
fix it was 4 stray bytes with *varying* garbage opcodes (0x4106, 0x8105, …).

**Current state:** the `0x089C` live retest failed. The client sent:
```
map-enter 19 bytes: 9c 08 84 84 1e 00 f4 49 02 00 86 e3 dc 2b 64 00 00 00 01
map-send    6 bytes: 38 04 64 00 00 00
keepalive   6 bytes: 38 04 b7 1e 01 00
```

The map server rejected it with:
```
clif_parse: Received unsupported packet (packet 0x8484 (0x8484), 23 bytes received), disconnecting session #7.
```

That means Hercules consumed only the 2-byte `0x089C` packet, then interpreted
the next two account-id bytes (`84 84`) as another packet header. The generated
length table confirms `packetLen(0x089c, 2)`, while `packetLen(0x0436, 23)`.

**Successful outgoing map packets after the corrected rebuild** (via
`KORANGAR_PACKET_LOG=1`):
```
map-enter 23 bytes: 36 04 84 84 1e 00 f4 49 02 00 f2 93 0c 64 57 de a3 47 64 00 00 00 01
map-send    6 bytes: 60 03 64 00 00 00   (0x0360 tick, value 100)
keepalive   6 bytes: 60 03 f3 db 1a 00   (0x0360 tick)
map-send    5 bytes: 5f 03 ...           (0x035f movement)
```
The client loaded `iz_int`, then later `int_land`, and continued sending
accepted movement/keepalive packets without the previous map-server disconnect.

(The `87 01…`/`00 02…` keepalives in the log are the char/login-server
connections, not the map — the log line doesn't tag which connection.)

## 4. NEXT STEP

**Next concrete step:** continue the packet-gap audit for other client-to-map
actions, then decide whether to keep the `KORANGAR_PACKET_LOG` instrumentation
behind the debug/env gate or strip it before committing.

If Hercules is not already running, launch it first:
```bash
cd ~/GitHub/Hercules_RO
./run-servers.sh
```

Then launch/test:
```bash
cd ~/GitHub/korangar
env -u WAYLAND_DISPLAY KORANGAR_PACKET_LOG=1 GALLIUM_DRIVER=d3d12 \
  MESA_D3D12_DEFAULT_ADAPTER_NAME=NVIDIA WGPU_BACKEND=gl \
  ./target/debug/korangar
```

If login says the account is already online after a force-kill/rejected
session, clear the ghost online flags:
```bash
mariadb -u ragnarok -pragnarok ragnarok -e "UPDATE login SET online=0 WHERE userid='korangar'; UPDATE \`char\` SET online=0 WHERE account_id=(SELECT account_id FROM login WHERE userid='korangar');"
```

To find the effective opcode/length for ANY shuffled packet at 20220406, check
the exact `PACKETVER` block in:
`~/GitHub/Hercules_RO/src/map/packets_shuffle_main.h`.

The generated length table is also authoritative:
`~/GitHub/Hercules_RO/src/common/packets/packets2022_len_main.h`.

Handy checks:
```bash
rg -n "packet\\(0x0436|packet\\(0x0360|packet\\(0x0437|packet\\(0x0438" \
  ~/GitHub/Hercules_RO/src/map/packets_shuffle_main.h
rg -n "packetLen\\(0x0436|packetLen\\(0x089c" \
  ~/GitHub/Hercules_RO/src/common/packets/packets2022_len_main.h
```

If a later action desyncs: keep `KORANGAR_PACKET_LOG=1`, capture the first
unsupported packet from map-server, and diff the sent bytes against
`packets_shuffle_main.h` plus `packets2022_len_main.h`. This is really the
[packet-gap](packet-gap-party-whisper.md) audit continuing; more
client-to-map packets may still need checking against the 20220406 shuffle
block.

## 5. Cleanup / commit plan for next session

Uncommitted (see `git status`): 22 files + the `korangar/bgm` symlink.
Suggested commits once the packet bug is fixed:
1. **GL backend support** — lib.rs, capabilities.rs, engine.rs, graphics/mod.rs,
   all `shaders/**`. (Big but one logical change.)
2. **Audio** — folder/mod.rs follow_links, audio_settings.ron. Decide BGM
   symlink handling (gitignore + document, or config option).
3. **Packet fixes** — ragnarok-packets plus the temporary
   `KORANGAR_PACKET_LOG` instrumentation in networking.
   **Strip or gate the `KORANGAR_PACKET_LOG` instrumentation** in
   networking/src/lib.rs first.
4. CLAUDE.md already updated with the GL story.

Also: `korangar/client/graphics_settings.ron` still says `msaa: X4` — that's
fine, the new clamp handles it at runtime. `audio_settings.ron` mute change is a
dev convenience.

## 6. How to reproduce / test (WSL specifics)

- Launch: `./run-wsl.sh` (release-ish) or for packet debugging:
  `env -u WAYLAND_DISPLAY KORANGAR_PACKET_LOG=1 GALLIUM_DRIVER=d3d12
  MESA_D3D12_DEFAULT_ADAPTER_NAME=NVIDIA WGPU_BACKEND=gl
  ./target/debug/korangar` (needs `--features "debug unicode"` build for the
  packet log + readable phase logs).
- Screenshot the window: find its id via python-xlib, then
  `ffmpeg -f x11grab -window_id <id> -i :0 -frames:v 1 out.png`. Use
  `env -u WAYLAND_DISPLAY` so it runs on X11 (grabbable) not Wayland.
- Drive UI with python-xlib XTEST (synthetic mouse/keyboard). NOTE: the debug
  build window is 1280×720; the release window differed — use fractional coords.
- Hercules servers: `~/GitHub/Hercules_RO/{login,char,map}-server` run from that
  dir; ports 6900/6121/5121. MariaDB must be started (`sudo service mariadb
  start` — no systemd in this WSL). Test account `korangar`/`korangar`, char
  `test` in `iz_int`.
- **"Rejected from server" at char-select** = ghost online flag from a
  force-killed session. Retry once, or `UPDATE char SET online=0; UPDATE login
  SET online=0;` (creds in `~/GitHub/Hercules_RO/conf/global/sql_connection.conf`
  — DB writes are permission-gated, run manually).
- Server logs mirrored to scratchpad this session:
  `/tmp/claude-.../scratchpad/{login,char,map}-server.log` (scratchpad is
  session-specific and will be gone next session — restart the servers with
  logging if needed).
