# Platform Bring-up Runbook (Windows / Linux / WSL / macOS)

Lessons from bringing the client + Hercules server up on a fresh platform
(first done end-to-end on macOS, 2026-07-10). Work through this checklist in
order when standing up a new OS target — each item is something that
actually bit us. Items are tagged **[platform-specific]** or **[all
platforms]**.

Companion docs:
- `MACOS_WORKFLOW.md` — the macOS-specific workflow this runbook grew out of.
- Root `CLAUDE.md` — WSL2 workflow (`run-wsl.sh`, GL backend caveats).

## 0. THE #1 RULE: build Hercules with `--enable-packetver=20220406` [all platforms]

The client speaks **PACKETVER 20220406**. A default Hercules build
(`./configure` with no flags) is **PACKETVER 20190605** (`src/common/mmo.h`)
and is silently wire-incompatible: login and character screens mostly work,
then the **map handoff fails** (the 23-byte `CZ_ENTER` 0x0436 desyncs a
20190605 map server), the client bounces back to the char server with a
spent auth token, and you get a baffling "Rejected from server". This burned
half a day on the macOS bring-up — the docs disagreed about the server
version (`CLAUDE.md` said 20190605, `plans/M1-p0-verification.md` said
20220406) and the fresh Mac clone defaulted to the wrong one.

**macOS** (Homebrew deps; paths are not on the default search path, and
`configure` needs them):

```sh
brew install mariadb pcre           # once
cd Hercules
CPPFLAGS="-I/opt/homebrew/include" LDFLAGS="-L/opt/homebrew/lib" \
  ./configure --enable-packetver=20220406
make -j8
```

**Linux / WSL** (Debian/Ubuntu):

```sh
sudo apt install build-essential zlib1g-dev libpcre3-dev \
  libmariadb-dev libmariadb-dev-compat   # once
cd Hercules
./configure --enable-packetver=20220406
make -j$(nproc)
```

**Windows**: the Visual Studio solution (`Hercules.sln`) reads `PACKETVER`
from `src/common/mmo.h`; either edit the `#define PACKETVER` there or add
`PACKETVER=20220406` to the project's preprocessor definitions. (Untested —
the intended Windows path runs the server under WSL instead.)

**Verify** after building — all three must agree:
```sh
# 1. What the binary was built with (packet_obfuscation warning aside):
grep -rn "enable-packetver" config.log | head -1   # should show 20220406
# 2. What the client expects:
#    korangar/archive/data/sclientinfo.xml → <packet_version>20220406</packet_version>
# 3. The client's length table header comment:
#    korangar-networking/src/packet_versions/lengths_20220406.rs (PACKETVER=20220406)
```

If you change the server's PACKETVER, regenerate the client's framing table:
```sh
cd korangar && ./tools/generate_packet_lengths.sh <HERCULES_DIR> 20220406 main
```
(The script needs a C compiler; it uses `cc -E`, works with clang and gcc.)

### English item names [all platforms]

English item names are bundled into the client as a compact generated table
derived from `docs/items.json` (itself generated from the Hercules item DB).
The GRF item table remains the source for icon/resource paths. There is no
runtime dependency on an original-client `System/itemInfo_EN.lua` file.

```sh
# Regenerate after updating docs/items.json:
jq -r '.[] | "\(.Id)\t\(.Name | gsub("_"; " "))"' docs/items.json \
  > korangar/src/world/library/hercules_item_names.tsv
```

On startup, confirm the `[itemInfo]` line lists the GRF base table plus
`bundled Hercules names (EN overlay, 13182 items)`. Do not restore the former
`archive/System/itemInfo_EN.lua` external symlink.

## 1. Server first, and verify it independently [all platforms]

Bring up MariaDB + Hercules and confirm health *before* touching the client,
so client bugs can't be confused with server bugs:

- Exactly one PID per server: `pgrep -fl "login-server|char-server|map-server|api-server"`.
  Duplicate/orphaned processes cause crash-loops (`socket.c rfifoskip`
  assertions, char-server connections REFUSED) that look like protocol bugs.
- Listening ports: login `6900`, char `6121`, map `5121`, api `7121`.
- `athena-start stop` only kills PIDs from `.pid` files it wrote itself —
  manually-launched servers must be killed by hand.

## 2. The SQL `loginlog` table is the source of truth for auth [all platforms]

Console logs may be redirected, buffered, or lost depending on how the
servers were launched. The database always has the answer:

```sql
SELECT time, ip, user, rcode, log FROM loginlog ORDER BY time DESC LIMIT 10;
```

This is how you distinguish, from the server's point of view:
- *no row* → the client never sent a (parseable) login packet
- `Unregistered ID` → connectivity works, account missing
- `Incorrect Password` → connectivity works, wrong credentials
- `login ok` → auth fine; problems are further down the chain (char/map)

## 3. Account bootstrap on a fresh database [all platforms]

A fresh Hercules DB contains **only** `s1` (the inter-server account,
sex `S` — not playable). Nobody can log in until a real account exists.
Either:

- **`_M`/`_F` suffix self-registration** (requires `new_account: true` in
  `conf/login/login-server.conf`, which is on): log in as `<name>_M`; the
  suffix is stripped and account `<name>` is created with the typed password.
- **Direct SQL** (what we use for the DM account; `use_MD5_passwords: false`
  so `user_pass` is plaintext):
  ```sql
  INSERT INTO login (userid, user_pass, sex, email, group_id)
  VALUES ('korangar', '<password>', 'M', 'a@a.com', 99);  -- 99 = GM/admin
  ```

Watch out: `dynamic_pass_failure` IP-ban is enabled (7 failures in 5 min →
5-minute ban). Repeated bad-password testing can lock you out silently.

## 4. FIXED CLIENT BUG: silent wedge after a rejected login [all platforms]

**Symptom (before the fix)**: first login attempt fails (wrong password /
unknown account), the client shows nothing, and every subsequent click of
the Login button does nothing — no error, no packet sent (verify via
`loginlog`: no new rows).

**Root cause — a packet-mapping gap, not a state machine bug**: Hercules
built with PACKETVER >= 20180627 sends login refusals as
`AC_REFUSE_LOGIN_R3` (**0x0B02**), but the client only modeled
`LoginFailedPacket` (0x0081) and `LoginFailedPacket2` (0x083E). 0x0B02
appeared solely in the auto-generated length-fallback table, and a
fallback-consumed packet produces **no NetworkEvent** (see
`protocol/packet-length-fallbacks.md`) — so no error window opened and
`disconnect_from_login_server` never ran, leaving the session wedged.

**Fix (in-tree, 2026-07-10)**: `LoginFailedPacket3` (0x0B02, same 26-byte
layout as 0x083E) in `ragnarok-packets`, registered in
`version_20220406.rs`. Failed logins now show "Incorrect username or
password" and can be retried immediately.

**Lesson for future packet work**: when a server behavior seems to be
"silently ignored" by the client, check whether the packet is only in
`lengths_20220406.rs` — that means it's consumed without producing an
event. See CLAUDE.md rule 2.

## 4b. Login auth token expires after 30 seconds [all platforms]

Hercules `AUTH_TIMEOUT` (login.c) is **30 seconds**: the one-shot token
issued at login is discarded if the client doesn't connect to the char
server in time. Sitting at the server-selection screen too long then
clicking a server yields `HC_REFUSE_ENTER` (0x006C) — shown as "Rejected
from server" — and **no retry from that screen can ever succeed**.

The client now handles this (2026-07-10): on a character server rejection
it drops the dead session and returns to the login window, so you can just
log in again. (Previously it stranded you on the server-select screen; the
auto-reconnect path in `CharacterServerDisconnected` also blindly
`unwrap`ed saved login data — both fixed in `korangar/src/lib.rs`.)

## 4c. PACKETVER-dependent headers in the pre-game flow [all platforms]

**Resolution first**: the real fix for all of this was rebuilding the server
with `--enable-packetver=20220406` (item 0) — the Mac Hercules had been a
default 20190605 build by mistake. The legacy receive handlers below were
added while diagnosing that and are **kept as defense-in-depth**: they're
inert against a 20220406 server (it never sends those headers), but they
make the client survive an accidentally default-built server well enough to
reach the map handoff and fail *loudly* there instead of silently at login.

Mismatches found while the server was at 20190605, all verified against the
Hercules source — the client now accepts *both* header generations:

| Meaning | Client modeled | 20190605 server actually sends | Status |
|---------|----------------|--------------------------------|--------|
| Login refusal | 0x0081, 0x083E | **0x0B02** (`AC_REFUSE_LOGIN_R3`) | fixed: `LoginFailedPacket3` |
| Character list | 0x0B72 (175-byte entries) | **0x099D** (155-byte entries) | fixed: `RequestCharacterListLegacySuccessPacket` |
| Char created | 0x0B6F (175-byte entry) | **0x006D** (155-byte entry) | fixed: `CreateCharacterLegacySuccessPacket` |

The 155 vs 175 byte difference: hp/max_hp are u32 and sp/max_sp u16 below
PACKETVER 20201007, u64 for all four above (`CharacterInformationLegacy` vs
`CharacterInformation`; sizes locked by a unit test). Everything else in
the chain was verified byte-exact against the Hercules source: 0x0064
login, 0x0AC4 accept (64+160·n), 0x0065 CH_ENTER (17), 0x082D slot info,
0x0A39 make-char (36), 0x0AC5 zone-server info (156), 0x0436 map enter,
0x02EB map accept (13).

Also fixed: `CharacterListPacket` (0x006B, a noop) used to parse typed
175-byte entries — it now consumes raw bytes so it can't desync on either
server generation. And `register_length_fallbacks` now runs on **all
three** connections (was map-only), so any future unmodeled packet is
logged instead of silently dropping the read buffer.

**When standing up a new server build**: if its PACKETVER differs, rerun
`tools/generate_packet_lengths.sh` AND re-check this table — headers and
struct sizes shift at 20170315, 20180627, 20201007 among others.

## 4d. "Already online" ghost sessions & logging in too early [all platforms]

Two related traps around server startup, both by-design Hercules behavior:

**Don't log in during the startup window.** The char-server accepts
connections as soon as it listens, but the map-server takes ~30–40 s (on
this Mac) to load its 1156 maps and register with the char-server. A
character select during that window fails the map handoff — and by then the
char-server has already flagged the account online.

Wait for the char-server console line before the first login:
```
[Status]: Map-server loading complete.
```

**The ghost online flag clears itself via a refused login.** Once an
account is stuck online (failed handoff, killed client, crashed map
server), the next login attempt makes the login server send a kick to the
char-servers, arm a 30 s failsafe timer, and refuse that attempt with
error 8 (`login.c` `login_auth_ok`). The char-servers answer "not here"
almost immediately, clearing the flag — **just log in again a second
later**. The client's error text now says exactly that ("Account was still
flagged online — the server is clearing it, try logging in again"). If it
somehow persists past 30 s, restart the login-server (its online table is
in memory).

## 5. Verifying the protocol path without the client [all platforms]

A ~20-line python script speaks `CA_LOGIN` (0x0064) directly:

```python
import socket, struct
pkt = struct.pack("<HI24s24sB", 0x0064, 55, b"korangar", b"<password>", 0)
s = socket.create_connection(("127.0.0.1", 6900), timeout=5)
s.sendall(pkt); resp = s.recv(4096)
print(hex(struct.unpack("<H", resp[:2])[0]))  # 0x0ac4/0x0069 = accepted, 0x006a/0x083e = refused
```

This cleanly splits "server-side problem" from "client-side problem" — it's
how we proved the macOS client wedge (item 4) was client-side. Note the
server answers plaintext 0x0064 even though the client uses PACKETVER
20220406 framing; `check_client_version: false` in the login conf.

## 6. Render loop: first-frame lifecycle differs per platform

Two related lessons from macOS (see `MACOS_WORKFLOW.md` for full detail):

- **[macOS]** AppKit can deliver a `RedrawRequested` (via `drawRect:`)
  *during* `create_window`, before `resumed()` creates the wgpu surface →
  panic (`engine.rs` `get_window_size` unwrap). Fixed in-tree with an
  `is_ready_to_render()` guard.
- **[all platforms]** The redraw loop is **self-sustaining**: each handled
  `RedrawRequested` schedules the next via `window.request_redraw()`. Any
  code path that skips a frame (like the guard above) MUST still request a
  redraw, and `resumed()` kicks the loop explicitly after surface creation.
  Symptom of getting this wrong: window opens, audio plays, screen stays
  blank forever. If a new platform shows "music but no graphics", check
  whether the first `RedrawRequested` was dropped before the surface existed.

## 7. Graphics backend expectations per platform

| Platform | Expected adapter line in startup log | Notes |
|----------|--------------------------------------|-------|
| macOS    | `using adapter <GPU> (metal)`        | No env vars needed. |
| Windows  | `... (dx12)` or `... (vulkan)`       | Native build blocked by BitDefender (see CLAUDE.md); use cargo-xwin cross-compile from WSL. |
| Linux    | `... (vulkan)`                       | Should be the happy path. |
| WSL2     | `using adapter D3D12 (<GPU>) (gl)`   | MUST use `./run-wsl.sh` (forces GALLIUM_DRIVER=d3d12 + WGPU_BACKEND=gl); plain `cargo run` silently falls back to llvmpipe CPU rendering. GL lacks MSAA resolve (black world) and binding arrays — client has fallbacks. |

Always check the adapter line first on a new platform — a wrong backend
produces symptoms (blank/black screens, terrible FPS) that masquerade as
client bugs.

## 8. Client configuration recap [all platforms]

- Server address/port: `korangar/archive/data/sclientinfo.xml`
  (`127.0.0.1:6900`). No CLI flags exist for this (binary takes only
  `--sync-cache`/`--help`/`--version`).
- Assets (`data.grf`, `rdata.grf`, `lua_files.7z`, `client/`,
  `archive/`) live in `korangar/korangar/` — the binary adjusts its working
  directory itself, so it can be launched from anywhere, but the assets must
  be present in that directory.
- Missing-but-nonfatal on all platforms so far: `NotoSansKR.ttf`,
  `Towninfo_EN.lub`, `graphics_settings.ron` / `game_archives.ron`
  (defaults are used; warnings are expected on first run).

## Bring-up checklist (condensed)

0. **Build Hercules with `--enable-packetver=20220406`** (item 0). Verify:
   `grep -o 'DPACKETVER=[0-9]*' Makefile | head -1`.
1. MariaDB up; Hercules schema imported.
2. Start servers; verify one PID each + all four ports listening. **Wait
   for "Map-server loading complete"** in the char-server output (~40 s)
   before logging in (item 4d).
3. Create a playable account (SQL or `_M` trick). GM: `group_id = 99`.
4. (Optional) scripted CA_LOGIN probe → expect `login ok` in `loginlog`.
5. Build client; launch; **check adapter line** matches the table above.
6. Log in. A failed login shows "Incorrect username or password" and can be
   retried directly (item 4).
7. Pick a character server **within 30 seconds** of logging in (item 4b);
   if rejected, the client returns to the login window — just log in again.
8. Verify char select / creation appears; then map entry.
