# Software Design Document — Custom Ragnarok Online Client (Korangar-based)

| | |
|---|---|
| **Status** | Living architecture design |
| **Author** | boneysan |
| **Last updated** | 2026-07-05 |
| **Client codebase** | `~/GitHub/korangar` (fork of [vE5li/korangar](https://github.com/vE5li/korangar)) |
| **Server** | Hercules at `~/GitHub/Hercules_RO` (WSL2) |
| **Official client assets** | `H:\RO` (`/mnt/h/RO` from WSL) |
| **Project plan** | [PROJECT_PLAN.md](PROJECT_PLAN.md) — inventory, WBS, milestones, risks |
| **Feature roadmap** | [FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) — UI/product roadmap and packet-handler backlog |
| **DM interface design** | [DM_INTERFACE.md](DM_INTERFACE.md) — native tabletop tooling |

---

## 1. Overview

### 1.1 Purpose
Build a custom Ragnarok Online client on top of Korangar (a next-gen RO client
written in Rust) that connects to our self-hosted Hercules server, replacing the
official Windows client for day-to-day play and development.

### 1.2 Goals
- [ ] Client logs in, creates characters, and plays against the local Hercules server end-to-end.
- [ ] Client loads game assets (GRFs) from the existing official client install at `H:\RO`.
- [ ] Custom UI / quality-of-life features beyond the official client
      ([FEATURE_ROADMAP.md](FEATURE_ROADMAP.md)).
- [ ] Keep the fork mergeable with upstream Korangar where practical.

### 1.3 Non-goals
- Supporting the official kRO/iRO servers.
- Packet obfuscation / anti-cheat parity with the official client (server-side config disables it instead).
- Public-shard economy/social brokering such as auction house, market board, or LFG matchmaking.

### 1.4 Definitions
| Term | Meaning |
|---|---|
| **GRF** | Ragnarok's packed asset archive format (`data.grf`, `rdata.grf`). |
| **PACKETVER** | Date-coded protocol version shared between client and server; both sides must agree. |
| **Hercules** | Open-source RO server emulator (login, char, map, api servers). |
| **Korangar** | Rust RO client using wgpu for rendering; the base of this project. |

---

## 2. System Context

```
┌─────────────────────────────┐         ┌──────────────────────────────────┐
│  Windows host               │         │  WSL2                            │
│                             │         │                                  │
│  H:\RO  (official client    │  GRFs   │  ~/GitHub/korangar               │
│   install: data/, GRFs,     ├────────►│   custom client (this project)   │
│   laragon web stack,        │/mnt/h/RO│          │ TCP                   │
│   setup-wsl-portforward.ps1)│         │          ▼                       │
│                             │         │  ~/GitHub/Hercules_RO            │
│  (optional: run client      │         │   login-server   :6900           │
│   natively on Windows,      │◄───────►│   char-server    :6121           │
│   port-forward into WSL)    │         │   map-server     :5121           │
└─────────────────────────────┘         │   api-server                     │
                                        │   MariaDB/MySQL                  │
                                        └──────────────────────────────────┘
```

- Current Hercules config binds `char_ip` / `map_ip` to `192.168.20.60` (LAN address),
  with `setup-wsl-portforward.ps1` in `H:\RO` handling Windows→WSL forwarding.
- When the client also runs inside WSL, it can reach the servers directly via
  `127.0.0.1` — decide per deployment scenario (§7.2).

---

## 3. Client Architecture (Korangar)

See also [docs/CLIENT_SYSTEMS_OVERVIEW.md](CLIENT_SYSTEMS_OVERVIEW.md) and the full technical reference [docs/PACKET_EVENTS_CATALOG.md](PACKET_EVENTS_CATALOG.md) (NetworkEvent catalogue, every producing packet + verbatim handlers/structs, flows into world/particles/state/UI, DM chat + quest effect integration, extension guide).

Korangar is a Cargo workspace. Crates prefixed `ragnarok-` are engine-independent
libraries; crates prefixed `korangar-` implement the client.

| Crate | Responsibility | Expected customization |
|---|---|---|
| `korangar` | Main client: game loop, state machine (login → char select → in-game), rendering, settings | High — UI features, server selection |
| `korangar-networking` | Async connection handling for login/char/map servers, packet version dispatch | **High — packet version work (§5)** |
| `ragnarok-packets` | Packet definitions (structs + derive-generated ser/de) | High — custom/updated packets |
| `ragnarok-formats` | Parsers for RO file formats (maps, sprites, models) | Low |
| `ragnarok-bytes` / `ragnarok-macros` | Byte-level ser/de traits and derive macros | Low |
| `korangar-loaders` | GRF/asset loading pipeline | Medium — asset paths, custom GRF |
| `korangar-interface` | Retained UI framework | Medium — custom windows/widgets |
| `korangar-audio` | BGM and effect playback | Low |
| `korangar-video` | Video/cutscene playback | Low |
| `korangar-collision` | Picking and collision | Low |
| `korangar-container` / `korangar-debug` | Utility containers, debug/profiling tools | Low |

### 3.1 Key runtime facts
- Requires **Rust nightly** (`rust-toolchain.toml`) and `slangc` for shader compilation.
- Rendering via wgpu (Vulkan on Linux/WSL, Metal/DX12 elsewhere). See [docs/GRAPHICS_PIPELINE.md](GRAPHICS_PIPELINE.md) for a full overview of the multi-pass architecture, bind groups, light culling, shadows, and the forward lighting pipeline.
- Assets: Korangar expects `data.grf` and `rdata.grf` in `korangar/korangar/`
  by default; this project also needs `renewal2021.grf` and `resources2021.grf`
  registered in the archive settings. Built-in overrides live in
  `korangar/archive/data/`.
- Server list: `korangar/archive/data/sclientinfo.xml` — each entry names a
  server, its login IP/port, and packet version.
- Optional `debug` and `unicode` cargo features enable in-client developer tools.

### 3.2 Client state machine (as-is)
```
Startup → load archives → Login screen
  → LoginServer connect (:6900) → server/character-slot selection
  → CharServer connect (:6121)  → character select/create
  → MapServer connect (:5121)   → in-game loop
```
Transport mechanics (framing, keepalive, disconnect/error, reconnect gap) are
documented in §5.3; per-phase packet flows in §5.2.

---

## 4. Server Architecture (Hercules)

- Standard Hercules trio (`login-server`, `char-server`, `map-server`) plus the
  newer `api-server`; backed by MariaDB.
- Compile-time protocol: `PACKETVER` in `src/common/mmo.h`, currently the
  **default `20190605`** (no override found in `conf/import/`).
- Local config overrides live in `conf/import/*.conf` (ports and IPs listed in §2).
- Client-facing custom server systems are cataloged in [PROJECT_PLAN.md](PROJECT_PLAN.md)
  §1.2; the dedicated DM campaign UI is designed in [DM_INTERFACE.md](DM_INTERFACE.md).

---

## 5. Protocol / Packet-Version Strategy  ⚠️ *central design decision*

**Problem:** Korangar's networking layer supports exactly one packet version —
`SupportedPacketVersion::_20220406` (see `korangar-networking/src/packet_versions/`).
Hercules_RO is built with the default `PACKETVER 20190605`. These will not interoperate.

**Confirmed:** the official client at `H:\RO\client` is `2019-06-05fRagexe_patched.exe` —
the server's packetver was chosen to match it. Bumping the server (Option A) therefore
breaks the existing official client unless a matching 2022-04-06 patched exe is
obtained (WARP supports it). See [PROJECT_PLAN.md](PROJECT_PLAN.md) D1/D5.

**Options:**

| Option | Work | Trade-offs |
|---|---|---|
| **A. Rebuild Hercules with `PACKETVER=20220406`** *(recommended)* | Recompile server: `./configure --enable-packetver=20220406 && make` | One-line change; Hercules supports this version natively. Must re-verify the official client at `H:\RO` still matches or is retired. Client-side `data`/lua files may need updating for 2022-era features. |
| **B. Add 20190605 support to `korangar-networking`** | Implement a second `SupportedPacketVersion` variant + packet tables | Significant Rust work; diverges from upstream; only worth it if the official client at H:\RO must stay on 20190605 simultaneously. |
| **C. Dual packetver on Hercules** | Not supported — PACKETVER is compile-time | Rejected. |

**Decision:** Confirmed Option A (2026-07-05). Rebuilding Hercules with `--enable-packetver=20220406` is the path of least resistance since Korangar strictly implements `20220406`.

**Additional protocol notes:**
- The complete packet-to-`NetworkEvent` surface (including chat family used for DM commands and `QuestEffectPacket` for hazards) is catalogued in `docs/PACKET_EVENTS_CATALOG.md`.
- **Disable packet obfuscation** — Korangar does not implement it, so it must be
  off or the map-server connection fails. **Currently `packet_obfuscation: 2`
  (always on) in `conf/map/battle/client.conf`, with no override** — set
  `packet_obfuscation: 0` in `conf/import/battle.conf`. This is a Phase 0 blocker
  ([FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) §8), not just a note.
- Custom packets (if any) go in `ragnarok-packets` with matching Hercules-side
  handlers; list them here as they are added.

### 5.1 Packet obfuscation — *decision: disable server-side*

**Decision (2026-07-05):** keep `packet_obfuscation: 0` on Hercules; **do not**
implement obfuscation in Korangar. Rationale:
- It is an explicit **non-goal** (§1.3) and the anti-bot value is nil on a private
  friends-only server.
- Implementing it adds upstream-divergent, byte-exact protocol code that fails
  *silently* (a key/order desync surfaces as mysterious map disconnects).
- The only scenario that would justify it — the official 2019/2022 client and
  Korangar connecting to the *same* obfuscation-enabled server simultaneously —
  does not apply, since the official client is being retired (§5).

**How it works (for reference):** Hercules scrambles only the **2-byte packet
header** of client→map-server packets (payloads are untouched), via a
linear-congruential rolling key. Server side (`clif_parse_cmd_decrypt`), per
incoming map packet:
```c
cmd      = cmd ^ ((cryptKey >> 16) & 0x7FFF);       // recover real packet ID
cryptKey = (cryptKey * key[1] + key[2]) & 0xFFFFFFFF; // advance for next packet
```
`cryptKey` is seeded with `key[0]`. Login and char-server traffic are not obfuscated.

**If we ever reverse this decision, implement it as follows:**
1. **Obtain the key triple** `key[0..2]` for packetver `20220406` from Hercules
   (`src/map/packets_keys_main.h` / packet-keys conf) — we control these since the
   server is self-hosted. Store them as a per-version constant alongside the
   existing packet tables in `korangar-networking/src/packet_versions/`.
2. **Add cipher state** to the *map-server connection only*: a `MapCipher { key: u32 }`
   seeded with `key[0]`. The write loop at `korangar-networking/src/lib.rs:314`
   (`stream.write_all(&action)`) is shared across login/char/map, so gate the
   transform on the map connection.
3. **Apply the transform in the write task, in send order** — right before
   `write_all`, mutate the header bytes of each outgoing map packet:
   `action[0..2] ^= ((key >> 16) & 0x7FFF)` (little-endian u16), then advance
   `key = key.wrapping_mul(key1).wrapping_add(key2)`. It **must** live in the
   single write task, not at `packet_to_bytes` time, so the key advances exactly
   once per packet in strict order (the action channel already guarantees FIFO).
4. **Wire an enable flag** through `SupportedPacketVersion` so obfuscation is opt-in
   per server entry, matching the Hercules `packet_obfuscation` setting.
5. **Verify byte-exact from the first map packet** — confirm Hercules applies the
   cipher starting at the very first parsed map packet (no handshake offset). Any
   off-by-one in key advancement desyncs all subsequent packet IDs. Test by
   capturing a known-good official-client trace and diffing headers.

*Estimated effort: ~40-line module plus wiring; low–moderate, but the failure mode
is silent, so budget time for packet-trace verification.*

### 5.2 Protocol usage — packet flows

Design-altitude view of *which* packets move at each stage and *where they live in
code*. **The byte layouts are authoritative in `ragnarok-packets`, not here** — this
section is a map, not a dictionary. All names below are the actual Rust types
registered for `20220406` (verified 2026-07-05).

**Three sequential connections** (`C→S` = client sends, `S→C` = server sends):

**1. Login server (`:6900`)** — `connect_to_login_server`
```
C→S  LoginServerLoginPacket            (username / password)
S→C  LoginServerLoginSuccessPacket     → account_id, login_id1/2, sex, char-server list
     └ or LoginFailedPacket / LoginFailedPacket2  (reason enums → UnifiedLoginFailedReason)
```

**2. Character server (`:6121`)** — `connect_to_character_server`
```
C→S  CharacterServerLoginPacket        (account_id + login_ids carried over)
S→C  CharacterServerLoginSuccessPacket → slot count
C→S  (request list)                    → RequestCharacterListSuccessPacket  (character array)
C→S  SelectCharacterPacket             → CharacterSelectionSuccessPacket → map-server ip/port + character_id
     CreateCharacterPacket / DeleteCharacterPacket / SwitchCharacterSlotPacket  (+ *Failed variants)
```

**3. Map server (`:5121`)** — `connect_to_map_server`
```
C→S  MapServerLoginPacket              (account_id, character_id, login_id1, ClientTick, sex)
S→C  MapServerLoginSuccessPacket, then an initial burst:
       InitialStatsPacket, RegularItemListPacket / EquippableItemListPacket (Inventoy{Start,End}),
       UpdateSkillTreePacket, UpdateHotkeysPacket, ChangeMapPacket
C→S  MapLoadedPacket                   (client ready → entities begin streaming)
```

**In-game steady state** (map connection, repeating):

| Concern | C→S | S→C |
|---|---|---|
| Movement | `RequestPlayerMovePacket` | `PlayerMovePacket` (self), `EntityMovePacket` / `MovingEntityAppearPacket` (others) |
| Combat | `RequestActionPacket(Attack)`, `UseSkillAtIdPacket` | `DamagePacket`, `UpdateEntityHealthPointsPacket`, `DisplaySkillEffect*`, `ResurrectionPacket` |
| Chat | `GlobalMessagePacket` | `ServerMessagePacket`, `Broadcast(2)MessagePacket`, `OverheadMessagePacket`, `EntityMessagePacket` |
| NPC dialogue | *(menu/next replies)* | `NpcDialogPacket`, `DialogMenuPacket`, `NextButtonPacket`, `CloseButtonPacket` |
| Entities/world | — | `EntityAppear(2)Packet`, `EntityDisAppearPacket`, `GroundItemAppear*Packet`, `ItemPickupPacket`, `ItemDisappearPacket` |
| Character progression | `RequestStatUpPacket`, `LevelUpSkillPacket`, `RequestEquipItemPacket` | `UpdateStatPacket`, `StatusChangePacket`, `ParameterChangePacket`, `SpriteChangePacket` |
| Shop | `SelectBuyOrSell`, `PurchaseItems`, `SellItems`, `CloseShop` | *(shop list / result packets)* |
| Death / keepalive | `RestartPacket` (respawn), `RequestServerTickPacket` | `ServerTickPacket`, `MapServerPingPacket` |

**Where each concern lives in code (the pointer table):**

| Layer | Location |
|---|---|
| Outgoing sends (client actions) | `korangar-networking/src/lib.rs` — `connect_to_*`, `select_character`, `request_player_move`, `cast_skill`, `send_chat_message`, `request_item_equip`, … |
| Incoming handler registration | `korangar-networking/src/packet_versions/version_20220406.rs` — `register_{login,character,map}_server_packets` |
| Packet structs + **authoritative byte layout** | `ragnarok-packets` (derive-generated ser/de) |
| Version dispatch | `SupportedPacketVersion` enum + `packet_versions/mod.rs` |

**Notes:**
- Many map packets are registered with `register_noop` — parsed off the wire but
  not yet acted on (e.g. `QuestListPacket`, `AchievementListPacket`,
  `ReputationPacket`, `ClanInfoPacket`). This list is effectively Korangar's
  *feature-coverage frontier* and a useful backlog for
  [FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) §8 Phase 1 gap-cataloging.
- Custom DM packets ([DM_INTERFACE.md](DM_INTERFACE.md) §9.3 Phase B) attach to this same map-server flow — new types
  in `ragnarok-packets`, registered in the map handler, sent via a new `lib.rs` method.
- Only map-server packets are subject to obfuscation (§5.1); login/char are plaintext.

### 5.3 Transport & framing — how bytes become packets

§5.2 is *what* the two sides say; this is *how* the wire actually works. Verified
against `korangar-networking` + `ragnarok-packets/src/handler.rs` (2026-07-05).

**Threading & channels.** A single background thread (`spawn_networking_thread`)
runs a tokio `LocalSet` with **one task per connection** (login / char / map), each
owning its `TcpStream`. The main thread never touches the socket — it talks to each
task over two channels:
- **outgoing** (`action_*`): `send_*_packet` serializes header+payload with
  `packet_to_bytes` on the main thread and pushes the bytes; the task writes them to
  the socket **in FIFO order** (this ordering is what §5.1 obfuscation would rely on).
- **incoming** (`event_*`): the task parses packets and sends `NetworkEvent`s up to
  the game loop.

**Receive pipeline.** TCP read into an **8 KB buffer** → wrap in a `ByteReader` →
loop `packet_handler.process_one(...)` until the buffer is drained → drain
`NetworkEvent`s to the channel.

**Framing is by deserialization, not a length table.** `process_one` reads the
2-byte header, looks up the registered handler in a `HashMap<PacketHeader, …>`, and
runs it; the deserializer consumes exactly the packet's bytes:
- **Fixed-length** packets consume a known field set.
- **Variable-length** packets (`#[variable_length]`) carry a 2-byte length after the
  header and read the tail with `#[repeating_remaining]` (e.g.
  `LoginServerLoginSuccessPacket` → the char-server list).
- If bytes run short mid-parse, a **save-point** is restored and `process_one`
  returns `PacketCutOff`; the loop copies the partial packet to the front of the
  buffer (`cut_off_buffer_base`) and waits for the next read. This is what handles
  packets **split across TCP segments** and **multiple packets per segment**.

**⚠ Consequence — every server packet must be registered.** Because length is
derived from the deserializer, an **unregistered header cannot be framed**:
`process_one` returns `UnhandledPacket` and the loop **discards the rest of that
buffer**. So `register_noop` (§5.2) is not merely "ignore this" — it tells the
client the packet's *length* so it can advance to the next one. A packet the server
sends that Korangar hasn't registered **at all** desyncs and drops everything after
it in that read. Keeping the `20220406` registration complete is therefore a
**correctness requirement**, not just feature coverage — and the first thing to
suspect when in-game state mysteriously stops updating.

**Protocol quirks worth knowing:**
- **Bare account-id first-read:** the *char-server* connection reads a raw 4-byte
  `AccountId` **before** any framed packet (`read_account_id`), surfaced as
  `NetworkEvent::AccountId`. Login and map connections do not.
- **Keepalive + time sync:** each in-game connection pings every **10 s** — char
  with `CharacterServerKeepalivePacket`, map with `RequestServerTickPacket`, whose
  reply also drives **client-tick time synchronization** (`UpdateClientTick` →
  `estimated_client_tick`). An idle connection with no ping is dropped by the server.
- **MTU cap:** a single packet larger than one buffer's read is abandoned
  (`packet_start == 0` guard) to avoid getting wedged on a mis-parsed packet.
- **Debug hook:** every incoming/outgoing/unknown/failed packet passes through a
  `PacketCallback` — this backs the in-client packet inspector (`debug` feature),
  the tool of choice when the client and server disagree.

**Disconnect / error paths** *(fills the earlier §3.2 reconnect/error gap)*: a read of `Ok(0)` or a socket
error becomes `NetworkTaskError::ConnectionClosed` and the task ends; login/char
failures surface as `NetworkEvent::*ConnectionFailed` (reason enums, §5.2). There is
currently **no automatic reconnect/backoff** — each `connect_to_*` awaits and
replaces the prior task handle. Reconnect (especially a map-server drop mid-session)
is now a planned feature: [FEATURE_ROADMAP.md](FEATURE_ROADMAP.md)
§8 Phase 2 → Group play & session conveniences → Auto-reconnect.

### 5.4 Protocol gaps — packet families not yet defined  ⚠️

[FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) §8.3 catalogs packets that are
*registered but ignored*. This is the more dangerous
list: families **absent from `ragnarok-packets` entirely**, so per §5.3 the client
cannot even frame them — receiving one silently drops the rest of that read.
Verified by opcode + name sweep (2026-07-05).

| Family | Missing wire format | Consequence | Priority |
|---|---|---|---|
| **Party data** | group-list/roster, member HP & position updates, party chat (`0x0108`/`0x0109`), leave/kick/leader | **Live desync risk today**: the campaign runs party-locked (`$dm_active_party`); forming a party makes Hercules send these every session. Also means **party frames (MVP cut) are greenfield protocol work**, not extend-work. | **Phase 1 — define + register (noop at minimum) before the first party session** |
| **Whisper** | `0x0096` (C→S), `0x0097`/`0x0098` (S→C) | Blocks `/r` reply + history; **may break [DM_INTERFACE.md](DM_INTERFACE.md) `@dmsecret`** — verify whether `dm_voice.txt` sends real whispers or `dispbottom` | Phase 1 verify; define with party work |
| **Storage actions** | open/move-item/close (C→S) | Storage *contents* likely already framable — `RegularItemListPacket` (`0x0B09`) carries `inventory_type` (unified item lists) — but the client can't act on storage | Phase 2, with the Inventory & storage feature |
| **Trade** | `0x00E4`–`0x00F0` family | Already documented in [FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) Player trade | Post-MVP |

**Non-gap confirmed:** cast bars are covered — `UseSkillSuccessPacket` (`0x07FB`,
carries `delay_time`) is defined and noop-registered
([FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) §8.3 MVP row).

**Rule:** before any feature or server-script change that makes Hercules emit a new
packet class, sweep this list and `ragnarok-packets` first (risk 9, §11).

---

## 6. Asset Pipeline

### 6.1 Sources
- `H:\RO\client` (`/mnt/h/RO/client`): full official install — **four GRFs**
  (`data.grf`, `rdata.grf`, `renewal2021.grf`, `resources2021.grf`), `System/` lub
  databases, `BGM/`, `AI/`, `NavigationData/`. Full inventory in PROJECT_PLAN.md §1.1.
- Korangar's `archive/data/`: client-side overrides (icons, `sclientinfo.xml`).

### 6.2 Design
- [ ] Decide whether to **copy** `data.grf`, `rdata.grf`, `renewal2021.grf`, and
      `resources2021.grf` into `korangar/korangar/` (fast, uses WSL disk) or
      **symlink** to `/mnt/h/RO/...` (no duplication, but 9P filesystem I/O from
      WSL to Windows drives is slow — likely unacceptable for GRF random access;
      benchmark before committing).
- [ ] Custom content (item/mob sprites, descriptions) packaged as an extra GRF or
      loose `data/` overlay — define load order in Korangar's archive settings.
- [ ] Keep client-side DB files (iteminfo, etc.) in sync with Hercules `db/` —
      define the sync process (manual vs. scripted).

---

## 7. Configuration & Deployment

### 7.1 Client configuration
- `sclientinfo.xml` entry for the local server:
  ```xml
  <connection>
      <display>HerculesRO (local)</display>
      <address>127.0.0.1</address>   <!-- or 192.168.20.60 from Windows -->
      <port>6900</port>
      <version>55</version>
      <packet_version>20220406</packet_version>
      <langtype>1</langtype>
  </connection>
  ```
  This matches the current `korangar/archive/data/sclientinfo.xml` schema. The
  `packet_version` field is Korangar-specific; keep `<version>` as the small
  official client service version.

### 7.2 Deployment scenarios
| Scenario | Client location | Server address | Notes |
|---|---|---|---|
| Dev (all-WSL) | WSL, via WSLg or X | 127.0.0.1 | Primary development loop |
| Windows-native client | Windows build of Korangar | 192.168.20.60 + port-forward script | Better GPU/input; needs Slangc + VulkanSDK on Windows |
| LAN players | Windows | 192.168.20.60 | Requires `char_ip`/`map_ip` to stay LAN-reachable (already configured) |

### 7.3 Build
```bash
# Client (WSL)
cargo build --release --features "debug unicode"

# Server
cd ~/GitHub/Hercules_RO && ./configure --enable-packetver=20220406 && make
```

---

## 8. Feature Roadmap

The detailed roadmap moved to [FEATURE_ROADMAP.md](FEATURE_ROADMAP.md). This SDD
keeps only the architectural contract and cross-cutting risks.

Architectural implications retained here:
- **Phase 0 blockers:** packetver alignment, packet obfuscation disabled server-side,
  local GRF/archive configuration, and `sclientinfo.xml` connection setup.
- **Phase 1 protocol work:** unregistered packet families in §5.4 must be defined
  and at least noop-registered before features that trigger them are exercised.
- **Phase 2 feature work:** each roadmap item that touches network state should
  graduate into a focused implementation spec under `docs/specs/`, following
  [specs/buff-bar-slice.md](specs/buff-bar-slice.md).
- **Executable plans:** near-term milestone plans live under
  [plans/](plans/), starting with [plans/M0-connectivity.md](plans/M0-connectivity.md).

## 9. DM Interface

The DM/player tabletop feature design moved to [DM_INTERFACE.md](DM_INTERFACE.md).
The protocol-level contract remains:
- **Phase A:** use existing chat packets as command transport, then parse/suppress
  structured `[DMJ]{...}` echoes from Hercules scripts.
- **Phase B:** add custom `ragnarok-packets` + Hercules plugin support only after
  Phase A proves which state needs a real packet stream.
- Server-side permission checks remain authoritative; client windows are command
  generators and presentation surfaces.

## 10. Testing Strategy
- [ ] Unit: `cargo test` across workspace (packet ser/de round-trips in `ragnarok-packets`).
- [ ] Integration: scripted login against a disposable Hercules instance
      (Hercules_RO already has `athena-start`; consider a `docker`/tmux dev target).
- [ ] Manual test script per release: login, char create/delete, movement, NPC,
      combat, **player trade** (once implemented —
      [FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) §8 Phase 2 Player trade), logout.
- [ ] Protocol regression: capture known-good packet traces; replay on upgrade of either side.

## 11. Risks & Open Questions
| # | Risk / question | Mitigation / owner |
|---|---|---|
| 1 | Packet version mismatch (§5) blocks everything until Hercules is rebuilt as `20220406` | Rebuild first; decision is already recorded in §5 |
| 2 | Korangar is pre-alpha; core gameplay features may be missing | Phase-1 gap catalog; contribute upstream |
| 3 | GRF I/O over `/mnt/h` may be too slow | Benchmark; copy into WSL if needed |
| 4 | Hercules `PACKETVER` bump may break the official client still used from `H:\RO` | Confirm whether official client must keep working |
| 5 | Rust nightly / slangc toolchain drift | Pin via `rust-toolchain.toml`; note slangc version here |
| 6 | Does the `api-server` (emblems etc.) need client support? | Investigate Hercules api-server ↔ client contract |
| 7 | **Upstream `korangar-interface` API churn** — every custom window ([FEATURE_ROADMAP.md](FEATURE_ROADMAP.md), [DM_INTERFACE.md](DM_INTERFACE.md)) rides its macro/trait surface (`window!`, `CustomWindow`, `RustState` paths), and Korangar is pre-alpha; an upstream refactor breaks all our widgets at once. [DM_INTERFACE.md](DM_INTERFACE.md) §9.4's module isolation helps rebasing but does not shield against API changes. | Keep widgets thin over shared components (P7); pin the fork point before big UI pushes; budget rebase time per upstream sync |
| 8 | **[FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) §8.3 backlog misclassification** — the noop→handler backlog was bucketed by packet *name*; already caught one (`StateChangePacket` was option-flags, not buffs). | Re-verify the struct in `ragnarok-packets` before each promotion (rule recorded in §8.3) |
| 9 | **Silent framing desync from unregistered packets** (§5.3) — an unregistered server packet can't be length-framed, so the client drops the rest of that read; symptom is state silently not updating. Grows in likelihood as Hercules config/scripts ([DM_INTERFACE.md](DM_INTERFACE.md)) evolve. | Keep `20220406` registration complete; watch the `PacketCallback` unknown-packet hook / packet inspector during every new server feature |

## 12. Change Log
| Date | Change |
|---|---|
| 2026-07-05 | Initial skeleton |
| 2026-07-05 | Confirmed official exe = 2019-06-05 (ties server packetver); added §9 DM Interface based on Hercules_RO `dm_campaign` engine |
| 2026-07-05 | Restructured §8 Phase 2 into labeled UI sub-groups; added modern MMO features (HUD edit mode, floating combat text, nameplates, party frames, structured combat log, chat/toast/command palette, keybinds, gamepad); scoped out economy/mass-social features for a small friends group |
| 2026-07-05 | Added §8.1 Server-side dependencies (verified against Hercules_RO); confirmed party HP sharing + mob HP bars already on; surfaced `packet_obfuscation: 2` as a Phase 0 blocker (needs `0`) in §5 and §8 |
| 2026-07-05 | Added §5.1: resolved decision to disable packet obfuscation server-side rather than implement it in Korangar; documented the algorithm and a 5-step client implementation guide if the decision is ever reversed |
| 2026-07-05 | Added §5.2 Protocol usage — packet flows (login/char/map sequences + in-game loop) mapped to actual `20220406` packet types and their code locations; noted `register_noop` packets as the feature-coverage frontier for Phase 1 |
| 2026-07-05 | Added §8.2 UI/UX design principles (P1–P10 + the four UI categories) distilled from game-UI practice and mapped to our §8/§9 features; governs all Phase 2 widgets |
| 2026-07-05 | Extended §8.2 with group F (P11–P14: visual hierarchy, immediate feedback, error prevention, test-with-the-table) from a third UI-principles source; produced an annotated HUD + DM-console wireframe artifact |
| 2026-07-05 | Added §8 Phase 2 Core-windows group (inventory/storage, equipment+character sheet, skill tree, NPC dialogue) + Interaction-primitives group — framed as *extending* existing Korangar windows (drag-drop already wired); added a "feels modern" MVP cut |
| 2026-07-05 | Added §5.3 Transport & framing (threading/channels, framing-by-deserialization, the "every packet must be registered" correctness rule, account-id first-read, 10 s keepalive + time sync, MTU cap, disconnect paths + reconnect gap); resolved the §3.2 reconnect/error gap |
| 2026-07-05 | Added §8.3 Packet-handler backlog — all 51 `register_noop` packets from `version_20220406.rs` grouped and prioritized (MVP/High/Med/Low/out-of-scope) against Phase 2 features; wired into Phase 1 checklist |
| 2026-07-05 | Clarified scope note (direct 1-on-1 trade in scope, distinct from out-of-scope brokering); added Phase 2 Player-trade feature (secure trade window, post-MVP) — flagged as greenfield protocol work since the `0x00E4`–`0x00F0` trade packets aren't defined in `ragnarok-packets`; fixed §10 test line |
| 2026-07-05 | Wrote `docs/specs/buff-bar-slice.md` — first end-to-end implementation spec (StatusChangePacket → NetworkEvent → StatusEffects state → StatusBarWindow + per-frame tick), grounded in the real `korangar-interface` window/state pattern; serves as the template for §8.3 promotions |
| 2026-07-05 | Corrected §8.3: `StateChangePacket` is entity option-flags (sit/cloak/PK/effect), not a timed buff — moved from the MVP buff row to the World/map row; MVP buff row now `StatusChangePacket` + `StatusChangeSequencePacket` only, linked to the slice spec |
| 2026-07-05 | Added Phase 2 groups: Group play & session conveniences (auto-reconnect, auto-follow, auto-loot toggle, fast-travel/return, whisper reply+history, death recap; chat-bubbles flagged verify-only) and Shared party coordination (ready check, raid/target markers, assist targeting) — the latter flagged as riding one shared-state transport with map ping (§9.3); promoted §5.3 reconnect gap to a planned feature |
| 2026-07-05 | Closing pass: AFK/idle indicator on party frames; recorded macros + item/mob DB browser as considered-and-rejected in the scope note; refreshed §11 with risks 7–9 (upstream `korangar-interface` API churn, §8.3 name-based misclassification, silent framing desync from unregistered packets) |
| 2026-07-05 | Propagated §5.4 protocol-gap finding to §8: marked party frames as greenfield protocol work (undefined in `ragnarok-packets`) rather than just UI extension. |
| 2026-07-05 | Added QoL conveniences to Phase 2: Equipment loadouts, transmogrification, encounter recap, chat copy/paste, and gamepad radial menus. |
| 2026-07-05 | Added Discord Rich Presence and Photo Mode to Phase 2 Group play conveniences. |
| 2026-07-05 | Split roadmap/product content into [FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) and DM tooling into [DM_INTERFACE.md](DM_INTERFACE.md); kept this SDD focused on architecture, protocol, deployment, testing, and risks. |
