# Plan — Private friends client distribution

| | |
|---|---|
| **Status** | Planned (2026-08-12) — packaging not built; this is the source of truth. **Windows cross-build PROVEN 2026-08-17** (§9), not yet run on Windows |
| **Audience** | The operator (you). Not a player handout. |
| **Milestone** | E8.3 in [PROJECT_PLAN.md](../PROJECT_PLAN.md) — the *friends* reading of it, not a public release |
| **Parent** | [PROJECT_PLAN.md](../PROJECT_PLAN.md) E8, [asset-pipeline.md](asset-pipeline.md), [MACOS_WORKFLOW.md](../MACOS_WORKFLOW.md) |
| **Player-facing leftover** | [Hercules `planning/player-guide.md`](../../../Hercules/planning/player-guide.md) still describes the 2019 official client. Rewrite that install section when the first pack ships. |

This is a **private game-night pack** for personal friends. It is not a public
product, not a Steam/GitHub release, and not an invitation to strangers. Gravity
owns `data.grf` / `rdata.grf` / BGM; bundling them is the normal private-server
arrangement among people you know, and it is still asset redistribution. Keep
the Drive folder to named friends or an unlisted link. Do not index it, do not
put the GRFs on a public GitHub Release, do not file this under “ship to the
internet.”

The public CI workflow (`.github/workflows/release.yml`) stays what it is: a
**developer** binary zip with no Gravity assets and a `server.ron.example`.
Friends never download that.

---

## 1. The bar

A friend who has never opened a terminal:

1. Opens the Google Drive folder you texted them.
2. Downloads two things the first time (Assets + their OS folder), or one
   first-time zip that already merges them.
3. Double-clicks `Play`.
4. Types `TheirName_m` or `TheirName_f` and a password, and lands in Prontera.

They never install Rust, nightly, slangc, Git, or the official RO client. They
never edit `sclientinfo.xml`. They never type `cargo`. If any of those leak into
the instructions, the pack is not done.

Later updates are a ~30 MB re-download of the binary + `archive/`, not another
4 GB of GRFs.

**Non-goals for the first pack**

- Auto-updater / patcher (E8.4). Drive replace-in-place is the updater.
- Account website / FluxCP (E8.5). `_m` / `_f` registration is enough.
- Apple Developer / Authenticode signing. Ad-hoc + “right-click Open” is enough.
- Feature parity (quest journal, party chat polish, status icons, remaining DM
  chrome). Those are playability, not installability. First session can use a
  voice call.
- Waiting on M3 / M4. The install pack and a two-machine smoke test are the gate.

---

## 2. Decisions

| ID | Decision | Choice | Why |
|---|---|---|---|
| **S1** | Who is this for | Named personal friends only | Copyrighted assets; no support surface for strangers |
| **S2** | How they get the files | A **shared Google Drive folder** (or equivalent: Mega, Dropbox). Not GitHub Releases | ~4 GB; GitHub’s per-file cap is 2 GB; GRFs must not be public |
| **S3** | One zip vs split | **Split: `Assets/` (once) + per-OS client (every build).** Optional extra: a merged first-time zip | Updating the client must not force a 4 GB re-download |
| **S4** | How they reach the server | **Tailscale (or ZeroTier) first.** Port-forward + public IP only if someone cannot run a mesh VPN | `char_ip` / `map_ip` are `127.0.0.1` today; a friend who logs in is then told to connect to *their* localhost |
| **S5** | What they launch | `Play.bat` (Windows) / `Play.command` or a `.app` (macOS) that `cd`s to its own directory, then starts the binary | Every path the client reads is **CWD-relative**. Finder’s CWD is `/`. Explorer is usually the exe folder, but a launcher is still cheaper than a support thread |
| **S6** | Build flavour | `--release`, feature `unicode` only. **No `debug`** | `release.yml` currently builds `--features "unicode,debug"` — that ships the packet inspector to friends |
| **S7** | Server address in the pack | Pre-filled `client/server.ron` with the host’s Tailscale / LAN IP. Friends do not edit it | `sclientinfo.xml` is EUC-KR and baked to `127.0.0.1`. The override exists exactly for this |
| **S8** | Accounts | `_m` / `_f` self-registration, documented in the Drive Doc. Pre-create GM accounts yourself | No control panel for v0 |
| **S9** | Link stability | **Never delete-and-reupload** a shared file. Use Drive “Manage versions”, or overwrite files *inside* the shared folder | A new upload mints a new link; every old text is dead |

Revisit S4 if the whole group is on the same LAN for a physical game night —
then `192.168.x.x` and a firewall hole on 6900 / 6121 / 5121 is simpler than
Tailscale. The pack still needs a filled-in `server.ron`; the value just
changes.

---

## 3. Why a checkout is not a pack

Current operator layout (works for *you*):

| Path | What it is | Ship? |
|---|---|---|
| `korangar/target/release/korangar` (~28 MB) | Release binary; shaders are compiled *into* it | Yes |
| `korangar/korangar/data.grf` (3.0 GB), `rdata.grf` (292 MB) | Gravity base archives | Yes, inside `Assets/` |
| `../../../RO/client/renewal2021.grf` (7 MB), `resources2021.grf` (66 MB) | Still outside the tree; `client/game_archives.ron` points at them by relative path | Yes — **copy into the pack**. Do not leave those `../../../RO/…` paths in a shipped `game_archives.ron` |
| `korangar/korangar/BGM/` (345 MB) | Loose MP3s. The audio engine opens them from disk, not from a GRF | Yes |
| `korangar/korangar/archive/` (7.5 MB) | Overlays + `sclientinfo.xml` | Yes |
| `korangar/korangar/lua_files.7z` (2.9 MB) | Generated Lua archive; first launch will rebuild it if missing | Yes, ship yours |
| `korangar/korangar/client/login_settings.ron` | Saved `korangar` / `korangar` (and the observer seat’s `headless2`) | **No** |
| `korangar/korangar/client/game_archives.ron` | Dev paths | **No** — write a clean relative list into the pack |
| `cache.7z` | Optional first-run texture cache | Optional. Nice if you pre-build `--sync-cache` on the same OS family; not required |

`client/server.ron` is the supported way to point a build at a host (see
`korangar/src/loaders/server/mod.rs`). A malformed file panics on purpose
rather than silently reconnecting to `127.0.0.1`.

Working-directory trap, from `Client::init`: if `archive/` is not in CWD, the
process tries `cd korangar` and gives up. That is a checkout heuristic, not a
packaging story. The launcher in §5 is the friends-facing fix; resolving paths
from `current_exe()` later would make the launcher optional.

---

## 4. Drive folder (what you upload)

Share **one folder**. Viewer access. Either named Google accounts or an unlisted
“anyone with the link.” Not “anyone on the internet can find this.”

```
Seal Cascade/                          ← the link you text them
  READ ME FIRST                       ← a Google Doc, pinned, readable on a phone
  Windows/
    Play.bat
    korangar.exe
    archive/
    client/
      server.ron                      ← already filled
      game_archives.ron               ← relative names only
  macOS/
    Play.command                      ← or Seal Cascade.app
    korangar
    archive/
    client/
      …
  Assets/                             ← download once; almost never changes
    data.grf
    rdata.grf
    renewal2021.grf
    resources2021.grf
    BGM/
    lua_files.7z
```

First night they put **Assets and their OS folder in the same directory**, so
`data.grf` sits next to `Play.bat` / `Play.command`. The relative
`game_archives.ron` is then:

```ron
(
    archives: [
        "data.grf",
        "rdata.grf",
        "renewal2021.grf",
        "resources2021.grf",
        "archive/",
    ],
)
```

Lookup priority is the reverse of this list (`GameFileLoader::add_archive`
inserts at index 0). `archive/` wins, then the 2021 GRFs, then `rdata`, then
`data`. That matches [asset-pipeline.md](asset-pipeline.md) D-A2.

**Optional first-time zip.** If “put two folders together” is too much
instruction, also upload `SealCascade-Windows-full.zip` / `…-macOS-full.zip`
that already merge Assets + Client. Keep the split folders for updates.

**Do not upload**

- `login_settings.ron`
- `client2/` (observer seat, absolute paths, headless credentials)
- `target/`, source, `shaders/` (already in the binary)
- A `game_archives.ron` that mentions `/Volumes/…` or `../../../RO/`
- The `debug` build

---

## 5. Launchers

Windows `Play.bat` (same directory as the exe):

```bat
@echo off
cd /d "%~dp0"
start "" korangar.exe
```

macOS `Play.command` (chmod +x before zipping):

```sh
#!/bin/sh
cd "$(dirname "$0")"
exec ./korangar
```

A real `.app` whose wrapper `cd`s into `Contents/Resources` is nicer (double-
click from Finder without a Terminal window). Not required for the first pack.

Unsigned + downloaded from Drive means:

- **Windows:** SmartScreen → More info → Run anyway.
- **macOS:** right-click → Open the first time, or
  `xattr -d com.apple.quarantine ./korangar`. Drive sets the quarantine xattr
  on anything it downloads.

Put both sentences in the Doc. They will hit both.

---

## 6. Google Drive procedure

**First upload**

1. Build the pack locally (or run the packager in §9 once it exists).
2. Create the folder structure in Drive. Upload Assets first; it takes a while.
3. Pin `READ ME FIRST`. Share the **folder**, not a single zip.
4. Text friends the folder link plus “read the Doc, download Assets once, then
   your OS folder.”

**Every later client build**

1. Rebuild `--release` (`unicode` only).
2. Replace `Windows/korangar.exe` and/or `macOS/korangar`, plus `archive/` if
   overlays changed.
3. If you wrapped those in a zip, use the file’s **Manage versions** to drop
   the new zip on the *same* Drive item.
4. Message the group: “new client, re-download the Windows/Mac folder only.”

**Do not** delete the shared file and upload a fresh one. The link dies.

**Drive-specific friction you will hit**

- Files over the virus-scan limit (~100 MB; some accounts trip around 25 MB)
  show *“can't scan this file for viruses.”* That is expected for GRFs and for
  any full zip. The Doc must say **click Download anyway**.
- Do not let Drive convert `READ ME FIRST.txt` into a Docs file mid-upload.
  Author it as a Doc from the start.
- Drive for Desktop is optional luxury: a friend who adds the shared folder
  gets binary drops automatically. Prefer they **copy** out to
  `Documents/SealCascade` rather than run from a half-synced folder. Unzip-to-
  Desktop is the default instruction.

---

## 7. Server reachability — Tailscale (do this)

Drive only solves “how they get the files.” After login, Hercules **tells the
client** the char- and map-server IPs. Those are still localhost:

- `Hercules/conf/char/char-server.conf` → `char_ip: "127.0.0.1"`
- `Hercules/conf/map/map-server.conf` → `map_ip: "127.0.0.1"`

A friend can authenticate and then immediately fail the char-server handoff
with a useless symptom. Tailscale is how they reach you without opening the
router. This section is the implementation runbook (decision **S4**).

Do **not** turn the host into an exit node or a subnet router. Friends only
need to reach three ports on the machine that runs Hercules.

### 7.1 What you are building

```
Friend laptop                    Your Mac (Hercules host)
─────────────                    ────────────────────────
Tailscale client  ← encrypted →  Tailscale client
       │                                │
       │  client/server.ron             ├── login-server :6900
       │  address = your 100.x          ├── char-server  :6121
       │  (or MagicDNS name)            └── map-server   :5121
       ▼
   korangar.exe
```

Inter-server traffic (char → login, map → char) stays on `127.0.0.1`. Only
the **advertised** addresses that get sent *to the game client* change.

### 7.2 Tailscale on the host (you)

1. Install Tailscale on the Mac that runs Hercules:
   [https://tailscale.com/download](https://tailscale.com/download)
   (`brew install --cask tailscale` is fine).
2. Sign in. That creates your **tailnet**. Personal plan is free for **6
   users** (you + five friends) — enough for a 3–6 player campaign.
3. In the Tailscale menu, turn **MagicDNS** on if it is not already
   (it is on by default).
4. Record two values from the Tailscale admin console
   ([https://login.tailscale.com/admin/machines](https://login.tailscale.com/admin/machines))
   or from a terminal:

   ```sh
   tailscale ip -4
   tailscale status
   ```

   You want the host’s `100.x.x.x` and its MagicDNS name
   (`your-mac.tailxxxx.ts.net`). The 100.x address is stable for that
   machine until Tailscale is reinstalled.
5. In the admin console, open the host machine → **Disable key expiry**.
   Otherwise the node falls off the tailnet mid-campaign and every client
   dies at once.
6. Set Tailscale to open at login on the host. Hercules is unreachable if
   Tailscale is not running, even if the three servers are up.
7. Leave **exit node** and **subnet routes** off.

### 7.3 Invite friends (pick one)

**Invite to the tailnet** (preferred if the group is ≤ 5 friends besides
you). Admin console → **Users** → **Invite users**. Each friend installs
Tailscale, clicks the invite, signs in with their own Google/Microsoft/
GitHub account. They can then reach every machine on the tailnet that ACLs
allow (default: all of them).

**Share only the host node** if you are over the 6-user free cap, or you
do not want friends to see your other devices. On the host machine in the
admin console → **Share**. They accept on their Tailscale account and see
*only that machine*. Node sharing does not count against the user cap the
same way.

Either way: a friend who has Tailscale installed but has **not** accepted
an invite/share cannot reach `100.x`. The game will look like “can’t
connect to server.”

Friend install is three sentences — that is all they need in the Drive Doc:

1. Install Tailscale from https://tailscale.com/download
2. Click the invite the GM sent and sign in
3. Wait until the GM’s machine shows as Connected, then launch Play

They never type an IP.

### 7.4 Hercules: advertise the Tailscale IP

Edit **import files only**. Do not touch the stock
`conf/char/char-server.conf` / `conf/map/map-server.conf`.

`Hercules/conf/import/char-server.conf` — add `inter.char_ip`. Keep
`login_ip` at `127.0.0.1` (char talks to login on the same box):

```conf
char_configuration: {
    @include "conf/import/integration-sql.conf"
    enable_char_creation: true
    player: {
        deletion: {
            delay: 0
            level: 0
        }
    }
    inter: {
        char_ip: "100.x.x.x"   // host Tailscale IPv4 from `tailscale ip -4`
        // login_ip stays 127.0.0.1 — do not set it to 100.x
        // bind_ip stays unset so the process listens on all interfaces
    }
}
```

`Hercules/conf/import/map-server.conf`:

```conf
map_configuration: {
    @include "conf/import/integration-sql.conf"
    inter: {
        map_ip: "100.x.x.x"    // same host Tailscale IPv4
        // char_ip in this file is how *map* reaches *char* — leave 127.0.0.1
    }
}
```

`bind_ip` must stay commented out in the stock files. If it is `127.0.0.1`,
Tailscale packets never land.

`conf/network.conf` `lan_subnets` should stay:

```
lan_subnets: (
	"127.0.0.1:255.0.0.0",
)
```

That is deliberate. Hercules’s LAN check works like this
(`socket_lan_subnet_check` in `src/common/socket.c`): if the connecting
client’s IP matches an entry, the server advertises the **first number in
that entry**, not `char_ip`. So `"100.64.0.0:255.192.0.0"` would send
every friend to `100.64.0.0`, which is the network address and is wrong.

- You, on the host, connect from `127.0.0.1` → match the existing entry →
  get `127.0.0.1` (local play still works).
- A friend connects from `100.a.b.c` → no match → get `char_ip` /
  `map_ip` (the Tailscale address you set above).

Do **not** add the Tailscale CGNAT range to `lan_subnets` unless the first
field is *your actual* `100.x.x.x`. Adding `100.64.0.0` is a trap.

`conf/import/socket.conf`: `ip_rules.enable` is already `false` in
`conf/common/socket.conf` (headless-suite DDoS false positives). Leave it
off. A six-person tailnet does not need the flood heuristic, and turning
it back on without a correct allow-list will ban friends and then refuse
the char-server’s own localhost reconnect.

### 7.5 Host firewall

macOS System Settings → Network → Firewall: allow incoming for
`login-server`, `char-server`, `map-server` (or disable the firewall on
the Tailscale interface). Ports:

| Server | Port |
|---|---|
| login | 6900 |
| char  | 6121 |
| map   | 5121 |

Confirm they are listening on all interfaces, not just loopback:

```sh
lsof -nP -iTCP -sTCP:LISTEN | grep -E '6900|6121|5121'
```

You want `*:6900` (or the `100.x` address), not `127.0.0.1:6900` only.

Restart Hercules after any `char_ip` / `map_ip` change:

```sh
cd Hercules && ./dev.sh restart && ./dev.sh wait
```

A running char-server keeps handing out the old address until it is
restarted.

### 7.6 What goes in the friends pack

Bake this as `client/server.ron`. They do not edit it.

```ron
(
    address: "100.x.x.x",
    port: 6900,
    name: "Seal Cascade",
)
```

`address` is resolved at login, so the host’s MagicDNS name
(`your-mac.tailxxxx.ts.net`) also works and survives a Tailscale IP change
better than a raw `100.x`. Prefer MagicDNS in `server.ron`; keep the
numeric IP in Hercules `char_ip` / `map_ip` (those fields are read at
server start).

If the host Tailscale IP ever changes (reinstall, new machine):

1. `tailscale ip -4` on the host
2. Update the two import files
3. `./dev.sh restart && ./dev.sh wait`
4. Rebuild the client pack’s `server.ron` (or just the MagicDNS name, if
   you used that) and replace the OS folder on Drive

### 7.7 Prove it before game night

On the **host**:

```sh
tailscale status          # friends show as active once they have accepted
./dev.sh log 20           # login/char/map actually up
```

On a **friend machine** (or your phone with Tailscale, as a cheap probe):

```sh
ping -c 3 100.x.x.x
nc -vz 100.x.x.x 6900
nc -vz 100.x.x.x 6121
nc -vz 100.x.x.x 5121
```

All three ports must accept. Then they launch Play, register `Name_m`,
and must reach **Prontera**, not just the login window. Login-only success
with a hang at char select almost always means `char_ip` is still
`127.0.0.1`.

### 7.8 Troubleshooting

| Symptom | Likely cause |
|---|---|
| “Can’t connect” at login | Friend has no Tailscale, or invite not accepted, or host Tailscale not running |
| Login works, then kicked / “rejected from server” / returns to login | `char_ip` / `map_ip` still `127.0.0.1`, or servers bound to loopback |
| Works for you, not for them | You are hitting the `127.0.0.1` lan_subnet path; they need the advertised `100.x` |
| Worked last week, dead today | Host key expired (disable expiry), or Tailscale not at login, or IP changed and pack not updated |
| Random “DDoS Attack detected” in login log | Someone re-enabled `ip_rules`; turn it back off |
| `there is no char-server online` | Char-server died or cannot reach login on `127.0.0.1` — not a Tailscale problem |

### 7.9 Same-LAN game night instead

If everyone is on your Wi-Fi, skip Tailscale:

1. Your LAN IPv4 (`ipconfig getifaddr en0`) goes in `inter.char_ip` /
   `inter.map_ip`.
2. Add `"192.168.x.x:255.255.255.0"` to `lan_subnets`, where the first
   field is **your** LAN IP (same trap as §7.4 — not `.0`).
3. Bake that LAN IP into `server.ron`.
4. Host firewall still has to allow 6900 / 6121 / 5121.

Do not leave `char_ip`/`map_ip` at `127.0.0.1` just because bind is open —
the *advertised* address is what the client uses.

---

## 8. Friend-facing copy (`READ ME FIRST`)

Keep this to a phone screen. No packet versions, no GRF, no “cwd.”

```
Seal Cascade — first time

1. Install Tailscale from https://tailscale.com/download
   and click the invite the GM sent you. Wait until the
   GM's computer shows as Connected.
   (Skip this if the GM said you are on the same Wi-Fi.)

2. Download the Assets folder (once — it is large) and the
   Windows or macOS folder.

3. Put both in the same folder on your computer, so Play.bat
   (or Play.command) sits next to data.grf.

4. Double-click Play.

   Google Drive will warn that it cannot scan the file for
   viruses. Click Download anyway.

   Windows: if SmartScreen appears, More info → Run anyway.
   Mac: right-click Play.command → Open the first time.

5. At the login screen type a username ending in _m or _f
   (for example Sam_m) and any password. That creates your
   account. Then make a character.

If something breaks, send the GM a screenshot of the window
and what you clicked. Do not edit any files.
```

Session / class / `@roll` material stays in the campaign player guide, not
here. When this pack exists, rewrite the “Getting Connected” section of
`Hercules/planning/player-guide.md` so it no longer mentions the 2019 official
client or `clientinfo.xml`.

---

## 9. Packaging work (not built yet)

One operator command should produce the Drive uploads. Suggested:

```
tools/package-client.sh --os windows|macos|both --server 100.x.x.x
```

It should:

1. `cargo build --release -p korangar --features unicode` (no `debug`).
2. Copy the four GRFs into `dist/Assets/` (including the two that today live
   under `RO/client/`).
3. Copy `BGM/`, `lua_files.7z`, `archive/`.
4. Write a clean relative `client/game_archives.ron`.
5. Write `client/server.ron` from `--server`.
6. Omit `login_settings.ron` and every other `client/*.ron` that is a
   personal setting. Themes can ship as defaults if they are not
   machine-specific.
7. Drop `Play.bat` / `Play.command` next to the binary.
8. Emit `dist/Windows/`, `dist/macOS/`, `dist/Assets/`.
9. Optionally zip a merged first-time archive per OS.

macOS: `chmod +x` the binary and `Play.command` before zipping. Consider
`codesign --sign -` (ad-hoc) so Gatekeeper is slightly less angry.

### Windows — the cross-build works, and here is the recipe

**Proven 2026-08-17 on this Mac**: `korangar.exe`, 50 MB, `PE32+ executable
(console) x86-64`, built from macOS with **no code changes, no patched
dependencies and no features removed**. `CLAUDE.md`'s "not yet set up" is now
only true of the packaging around it.

```sh
rustup target add x86_64-pc-windows-msvc
brew install llvm nasm          # see the traps below
cargo install cargo-xwin

export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
cargo xwin build --release --target x86_64-pc-windows-msvc \
  --bin korangar --features unicode --xwin-include-atl
```

**Four traps, each of which cost an attempt:**

1. **Homebrew LLVM is keg-only**, so `clang-cl` and `llvm-lib` are *not* on
   `PATH` in a fresh shell. Without them `blake3`, `mlua-sys` and `rav1d` all
   fail in their build scripts — three failures, one cause. Apple's clang does
   not provide either tool.
2. **`--xwin-include-atl` is required** (something in the graph, almost certainly
   `mach-dxcompiler-rs`, needs `atls.lib`) **and it is silently ignored when the
   xwin cache already has a `DONE` marker.** The flag looks broken; it is not.
   Delete `~/Library/Caches/cargo-xwin/xwin/DONE` and re-run — the extracted
   files are reused, so this is not another 1.1 GB download.
3. **`nasm` must be linked, not merely installed.** `rav1d` assembles x86 asm and
   a `brew install` that reports "already installed, it's just not linked" leaves
   you with an assembler error that reads nothing like a packaging problem.
4. **`--features unicode` only** (decision S6). Not `unicode,debug`.

**What is NOT proven: that the `.exe` runs.** A cross-compiled binary that links
is not a binary that works, and nothing here can execute it. That is the §10
second-machine gate, and for Windows it is the *only* evidence that counts.

**Consider the CI path instead.** `release.yml` already has a **native**
`windows-2025` job, which is more reproducible than one developer's Mac and needs
no xwin cache at all. It currently builds `--features "unicode,debug"`, so it
ships the packet inspector — fix that (or add a friends-pack job) and CI becomes
the better source for the pack's `.exe`. Keep the cross-build for quick local
iteration.

**Open decision — the console window.** The binary is *console* subsystem, so
Windows opens a terminal behind the game. `#![windows_subsystem = "windows"]`
removes it and also removes the only place a panic message would ever appear.
Worth choosing deliberately before friends see it.

Do **not** teach friends to clone the repo as a fallback. If someone cannot
run the pack, you run it for them on a call.

---

## 10. Gate before inviting the group

A pack that has not been opened on a **second machine** is not shipped.

On a friend’s laptop (or a second account / second OS):

- [ ] Friend accepted the Tailscale invite; `tailscale status` on the host shows them
- [ ] Friend can `nc` (or equivalent) to host `100.x` ports 6900, 6121, 5121
- [ ] Drive download hits the virus-scan interstitial; they click through.
- [ ] Launcher starts the client without a terminal `cd`.
- [ ] Login with a fresh `Name_m` creates an account.
- [ ] Character create → Prontera (login-only is not enough — that hides a bad `char_ip`).
- [ ] You (on the host) can see them, whisper works, party invite works.
- [ ] One fight, one NPC shop. Logout and back in.

If that pass fails, fix the pack or the advertised IPs. Do not add a fourth
friend to a broken handoff.

Playability gaps that are **not** blockers for this gate: party chat polish,
quest journal, status-effect icons, remaining DM windows. Tell them to use
voice.

---

## 11. Work breakdown

Sizes match the rest of E8 (S ≤ ½ day, M ≤ 2 days).

| ID | Task | Size | Depends |
|---|---|---|---|
| F1 | Copy `renewal2021.grf` + `resources2021.grf` into a known pack input path (next to `data.grf`) so the tree is self-contained | S | — |
| F2 | `tools/package-client.sh` as in §9 | M | F1 |
| F3 | `Play.bat` + `Play.command` (and optional macOS `.app`) | S | F2 |
| F4 | Tailscale + Hercules advertise-IP: follow §7 end-to-end on the host, then one friend `nc` to 6900/6121/5121 | S | S4 |
| F5 | Create the Drive folder, upload Assets once, pin `READ ME FIRST` | S | F2, F4 |
| F6 | Rewrite `Hercules/planning/player-guide.md` “Getting Connected” for this pack | S | F5 |
| F7 | Two-machine smoke test (§10) | S | F5 |
| F8 | (Later) resolve client data paths from `current_exe()` so CWD no longer matters | M | not required for v0 |

E8.4 (auto-update) and E8.5 (FluxCP) stay deferred. Drive + `_m`/`_f` cover
them for a friends group.

---

## 12. What a friend should never see

- `git clone`, Rust nightly, slangc, Vulkan SDK, `cargo run`
- `sclientinfo.xml` or anything about EUC-KR / packet version
- Your `login_settings.ron` or the `korangar` / `headless2` passwords
- `../../../RO/client/…` or `/Volumes/T7/…`
- A request to “just install the 2019 official client”

If the Doc or the zip contains any of those, it is an operator checkout, not a
friends pack.
