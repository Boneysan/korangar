# Plan — Private friends client distribution

| | |
|---|---|
| **Status** | **Pack rebuilt 2026-09-02** from current `agent/platform-connectivity-controls` — fresh `.exe`, VC++ runtime bundled, split manifests, `Setup` script, in-pack README. Cross-build re-proven the same day. **Still never run on Windows** (§10) |
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
4. Types `TheirName_create` and a password, and lands in Prontera.

They never install Rust, nightly, slangc, Git, or the official RO client. They
never edit `sclientinfo.xml`. They never type `cargo`. If any of those leak into
the instructions, the pack is not done.

Later updates are a ~30 MB re-download of the binary + `archive/`, not another
4 GB of GRFs.

**Non-goals for the first pack**

- Auto-updater / patcher (E8.4). Drive replace-in-place is the updater.
- Account website / FluxCP (E8.5). `_create` self-registration is enough.
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
| **S8** | Accounts | `_create` self-registration, documented in the Drive Doc. Pre-create GM accounts yourself | No control panel for v0. The fork replaced upstream's `_M`/`_F` with `_create` (2026-09-05) — the client asks for sex per character, so registration should not |
| **S9** | Link stability | **Never delete-and-reupload** a shared file. Use Drive “Manage versions”, or overwrite files *inside* the shared folder | A new upload mints a new link; every old text is dead |
| **S10** | CPU baseline | **AVX2 required** (`x86-64-v3` + `+aes`, from `.cargo/config.toml`). Documented, checked by the launcher, not lowered | Confirmed in the binary: 10,265 `vpaddd`, 5,273 `vpbroadcastd`, 2,144 `vinserti128`, 168 `vpermd`, plus FMA. Without AVX2 the process dies on its first vectorised instruction — `STATUS_ILLEGAL_INSTRUCTION`, no window, no message, nothing to read. Operator decision 2026-09-02: every PC in this group is post-2013 Intel or Ryzen, so ship v3 and *say so* rather than rebuild at `x86-64-v2` |
| **S11** | Visual C++ runtime | **Bundle `VC_redist.x64.exe`** in the client half; `Setup` installs it, `Play` refuses with a readable message if it is missing | The exe imports `MSVCP140.dll`, `VCRUNTIME140.dll`, `VCRUNTIME140_1.dll`, which are **not** part of a clean Windows (the `api-ms-win-crt-*` imports are — those are the Universal CRT). Static linking was tried first and **cannot work**: `+crt-static` fails with `undefined symbol: __declspec(dllimport) nearbyint` from `libmach_dxcompiler_rs`, wgpu's prebuilt DX12 shader compiler, which is built against the dynamic CRT |
| **S12** | Manifest naming | **`SHA256SUMS-client` and `SHA256SUMS-assets`** — never a shared `SHA256SUMS` | The halves are merged into one folder by design, so a shared name is a collision. Explorer offers Replace-or-Skip and the loser takes its half's coverage with it: keep the client copy and `Play.ps1` dead-ends on *"SHA256SUMS does not list lua_files.7z"* (unfixable by re-downloading, which is what it tells you to do); keep the assets copy and the exe, launchers and all 49 `archive/` files go unverified forever |

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

Live in [`tools/packaging/windows/`](../../tools/packaging/windows/).

**Windows is a `Play.bat` that calls a `Play.ps1`, and the split is deliberate.**
A bare `.ps1` is *worse* than a `.bat` for this audience: double-clicking one
**opens it in Notepad** (that is the default file association), Windows' default
execution policy on client SKUs is `Restricted`, and anything downloaded from
Drive carries Mark-of-the-Web, which blocks scripts under `RemoteSigned` too. The
`.bat` double-clicks and runs anywhere, so it is the door; PowerShell does the
work behind it, launched `-ExecutionPolicy Bypass` so no friend is ever told to
change a machine setting.

**Written for Windows PowerShell 5.1**, the one in the box — `Play.bat` invokes
`powershell`, not `pwsh`, because a friend's machine has 5.1 and may not have 7.
Nothing newer than PowerShell 3.0 cmdlets, no ternaries, no `??`, no `&&`, no
three-argument `Join-Path`, and **pure ASCII with no BOM**, because 5.1 decodes a
BOM-less script as ANSI and would turn a stray typographic dash into mojibake
inside the error messages.

What the checks buy, which is the reason not to just `start korangar.exe`: the
two realistic first-launch failures both produce useless symptoms. **The Assets
download is separate and enormous**, so the likely miss is running the OS folder
without copying Assets in beside it — the script names the missing GRFs and says
where to put them. And **every path the client reads is CWD-relative**, so a
wrong working directory sends `Client::init` into its `cd korangar` checkout
heuristic, which means nothing on a friend's machine.

Verified 2026-08-17 by running all six paths (no exe / no assets / partial assets
/ no `archive` / no `server.ron` / complete) against fake pack folders. **Caveat:
those runs used pwsh 7.6 on macOS — the 5.1 compatibility is by construction and
review, not by execution.** Run it once on a real Windows box before the pack
ships; that is the same §10 gate the `.exe` needs anyway.

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

### 7.0 Where the server address lives — all three of them

Asked often enough to belong at the top. The address appears in **three** files
and **all three must agree**, or a friend authenticates and then fails the
char-server handoff with a symptom that names nothing:

| What | File | Set by | Survives a re-clone? |
|---|---|---|---|
| What the **client** dials | `dist/Windows/client/server.ron` → `address:` / `port:` | `make-pack.sh --server <ip[:port]>` writes it; port defaults to 6900 | n/a — regenerated with the pack |
| What **char-server** advertises | `Hercules/conf/import/char-server.conf` → `inter.char_ip` | you, by hand | **NO — gitignored** |
| What **map-server** advertises | `Hercules/conf/import/map-server.conf` → `inter.map_ip` | you, by hand | **NO — gitignored** |

Currently all three read `100.96.4.37` — this host's Tailscale address. See §7.4a.

Friends never type an address and never edit `server.ron`. Moving the server
means re-running `make-pack.sh --server <new>` and re-sending the 80 MB client
half — which is exactly why the pack is split.

**Deliberately NOT changed when you move:** `login_ip`, and the map-server's own
`inter.char_ip`. Both describe how the servers reach *each other* on one box;
pointing them outward routes local traffic over the network for nothing.

> [!WARNING]
> The two Hercules values live in **gitignored** files. No commit records them,
> a fresh clone loses them silently, and the symptom is every friend failing at
> character select. The tables here and in §7.4a are the only durable copy —
> re-apply them after any re-clone.

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

Friend install is four sentences — that is all they need in the Drive Doc:

0. **Use a password you use nowhere else.** One sentence, and it belongs in the
   Doc verbatim rather than paraphrased: *"Please pick a password you don't use
   for anything else -- this is a hobby game server, not a bank, and the login is
   not encrypted."* Ragnarok's login protocol sends the password in clear, and
   `<passwordencrypt>` cannot be combined with the hashed storage this server
   uses. **The VPN is what encrypts it in transit**; this sentence covers anyone
   who plays before the tailnet exists, or on a LAN.
1. Install Tailscale from https://tailscale.com/download
2. Click the invite the GM sent and sign in
3. Wait until the GM’s machine shows as Connected, then launch Play

They never type an IP.

### 7.4a CURRENT SETUP (2026-09-05): Tailscale, everyone

**Operator decision 2026-09-05: every player joins the tailnet, including
anyone sitting on the host's own LAN.** The group has remote friends, and a
LAN address cannot serve them. One address, one pack, one config.

| | |
|---|---|
| `conf/import/char-server.conf` | `inter.char_ip: "100.96.4.37"` |
| `conf/import/map-server.conf` | `inter.map_ip: "100.96.4.37"` |
| `conf/network.conf` | `lan_subnets` gained `"192.168.20.49:255.255.255.0"` |
| `conf/import/char-server.conf` | `char_configuration.player.deletion.delay: 60` (stock is **86400**) |
| `conf/import/socket.conf` | `ip_rules.allow_list` gained `"100.64.0.0/10"` — the tailnet |
| Pack | `dist/*/client/server.ron` → `address: "100.96.4.37"` |
| Tailnet | `100.96.4.37` — this Mac, the only node as of 2026-09-05; no subnet routes advertised |
| Verified | **not yet** — no peer has ever joined the tailnet, so no client has made the char/map handoff over it |

**Why a LAN player joins the tailnet too.** `server.ron` holds exactly one
login address per pack, so serving both would mean two packs kept in sync
forever, and a pack that fails confusingly if it reaches the wrong person. The
cost to them is close to nothing: Tailscale connects same-subnet peers
**directly across the LAN**, not through a relay, so the added latency is a
fraction of a millisecond of WireGuard encryption and a userspace hop. It is
not *zero* — if a direct connection cannot be established (AP isolation and
guest Wi-Fi are the usual culprits) Tailscale falls back to a DERP relay, which
sends LAN traffic out to the internet and back. `tailscale status` names the
path per peer (`direct` vs `relay`), and `tailscale ping <host>` reports which
one a packet took. Check it once on game night rather than assuming.

**The `lan_subnets` entry is inert today** and is there on purpose. A tailnet
client's source address is its `100.x`, which does not match the LAN subnet, so
it takes the WAN branch and is handed `100.96.4.37` — correct. The entry only
fires for a client whose source address really is on `192.168.20.0/24`, i.e.
one dialling the LAN address directly. It costs nothing and means that if the
group ever does want a LAN-only pack, only the pack needs rebuilding and not
the server config.

**Deliberately NOT changed:** `login_ip`, and the map-server's own
`inter.char_ip`. Both describe how the servers reach *each other* on this one
box; pointing them outward would route local traffic over the network for
nothing.

**Flood protection versus your own friends.** `connect_check` trips at **10
connections in 3 seconds** and then refuses that IP for **ten minutes**
(`ddos_count`, `ddos_interval`, `ddos_autoreset` in `src/common/socket.c`). A
client that retries on a failed login reaches ten in seconds, so on 2026-09-05 a
friend who kept re-entering `Name_create` after his account already existed
locked himself out entirely — the log shows `DDoS Attack detected` followed by
`Connection refused: IP isn't authorized`.

The tailnet range is now in `allow_list`, and an allow-list match is an
**unconditional** accept: `connect_check_` returns 1 for `connect_ok == 2` even
for an IP flagged as DDoS. Safe, because nothing reaches `100.64.0.0/10` without
the host adding it in the Tailscale admin console, and the protection still
applies to every other source. **The flag lives in the login server's memory**,
so clearing an existing one means restarting *that* server — which does not
disturb anyone already in the world, since the map server holds those sessions.

**Character deletion delay.** Stock Hercules makes a player wait a full day
before a character can be deleted — an anti-theft measure for a public server,
where the threat is someone else wiping your characters. Here the realistic
case is a friend mistyping a name or picking the wrong job on the first evening
and being stuck with it until tomorrow. Set to **60 seconds**: still deliberate
(the client asks again after the timer), but it cannot cost anyone a session.
**CI cannot tell you this value** — `run-integration-tests.sh` overrides it to
`0`, so the suite has never seen what the live server uses.

> [!WARNING]
> **`conf/import/**/*.conf` is gitignored upstream**, so `char_ip`, `map_ip` and
> the deletion delay live in untracked files. They do not survive a fresh clone
> and no commit records them — this table is the only durable copy. Re-apply it
> after any re-clone. (`conf/network.conf` *is* tracked, so the `lan_subnets`
> line survives on its own.)

**What this replaced:** `192.168.20.49`, this host's LAN address, set
2026-08-17 for a physical game night. It was unreachable from the internet, and
it was a DHCP lease — if the router had reassigned it, every pack already handed
out would have broken at once with a connection refused. The tailnet address is
assigned per node and stays put, which removes that failure mode as well.

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

All three ports must accept. Then they launch Play, register `Name_create`,
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

**Not the current arrangement** — see §7.4a, where the 2026-09-05 decision was
that everyone joins the tailnet including LAN players. Kept because it is the
fallback if Tailscale is ever unavailable, and because step 2 is already
applied. If everyone is on your Wi-Fi, you could skip Tailscale:

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
   Everyone needs this, including anyone on the GM's own
   Wi-Fi. Leave it running whenever you play -- without it
   you reach the login screen and then fail at character
   select.

2. Download the Assets folder (once — it is large) and the
   Windows or macOS folder.

3. Put both in the same folder on your computer, so Play.bat
   (or Play.command) sits next to data.grf.

4. Double-click Play.

   Google Drive will warn that it cannot scan the file for
   viruses. Click Download anyway.

   Windows: if SmartScreen appears, More info → Run anyway.
   Mac: right-click Play.command → Open the first time.

5. At the login screen type a username ending in _create
   (for example Sam_create) and any password. That creates
   your account, which is just "Sam" -- you type _create once
   and log in as Sam from then on. Then make a character;
   you pick its gender and look on that screen.

If something breaks, send the GM a screenshot of the window
and what you clicked. Do not edit any files.
```

Session / class / `@roll` material stays in the campaign player guide, not
here. When this pack exists, rewrite the “Getting Connected” section of
`Hercules/planning/player-guide.md` so it no longer mentions the 2019 official
client or `clientinfo.xml`.

---

## 9. Packaging — built

One operator command produces the Drive uploads. **Built and verified
2026-08-17** as [`tools/packaging/make-pack.sh`](../../tools/packaging/make-pack.sh):

```sh
tools/packaging/make-pack.sh --server 100.x.y.z            # full pack
tools/packaging/make-pack.sh --server 100.x.y.z --build    # cross-build first
tools/packaging/make-pack.sh --server 100.x.y.z --skip-assets   # update pack
```

Measured output **2026-09-02**: **`dist/Windows` 80 MB (60 files), `dist/Assets`
3.7 GB (199 files)**, both manifests verifying clean. The whole run takes ~22
seconds, because APFS clones the GRFs instead of copying them and Apple Silicon
hashes SHA-256 in hardware — so there is no reason to skip a regeneration.

### What is in the pack, and why each piece is there

| `dist/Windows/` | Why it ships |
|---|---|
| `korangar.exe` | The client. Shaders are `include_bytes!`-compiled in (`shader_compiler.rs:8`), so no slangc, no Vulkan SDK, nothing for a friend to install |
| `Setup.bat` + `Setup.ps1` | **The one thing a friend runs first.** Checks AVX2, installs the VC++ runtime, finds the Assets download and merges it in, verifies all 259 files, clears the mark-of-the-web, offers to launch |
| `Play.bat` + `Play.ps1` | Every launch. Only the cheap checks — AVX2, runtime, assets present, `lua_files.7z` hash, `archive/`, `client/server.ron` |
| `Verify.bat` + `Verify.ps1` | On demand. Reads **both** manifests and names every corrupt or missing file |
| `READ ME FIRST.txt` | Requirements, install, `_create` accounts, the throwaway-password warning, troubleshooting |
| `VC_redist.x64.exe` | S11 — the client cannot be statically linked. ~25 MB, gitignored, fetched from `aka.ms` on first pack |
| `archive/` | 49 files: fonts, UI textures, language RONs, `msgstringtable.txt`, `sclientinfo.xml`, Lua scaffolding |
| `client/server.ron` | Written from `--server`. **This is the only place a friend's client learns the address** |
| `client/game_archives.ron` | Written, never copied — the working copy points outside the pack |
| `SHA256SUMS-client` | S12 |

| `dist/Assets/` | Why it ships |
|---|---|
| `data.grf`, `rdata.grf` | 3.2 GB + 307 MB of Gravity assets |
| `renewal2021.grf`, `resources2021.grf` | Not in this tree — copied from `RO/client/` |
| `lua_files.7z` | Hashed at load against the manifest (security audit 3, T7) |
| `BGM/` | 192 mp3s, 345 MB. Loaded **off the filesystem** case-insensitively, not out of a GRF, so it cannot be folded into an archive |
| `Verify.bat`, `Verify.ps1` | Byte-identical to the client copies, so the merge collision is harmless |
| `SHA256SUMS-assets` | S12 |

**Six things it refuses to get wrong, each of which fails silently otherwise:**

- **A contents checklist runs at the end of every pack.** Every file above is
  asserted present. This exists because the failure mode is not a broken build —
  it is a friend discovering the gap. Add to the list whenever the pack gains a
  file; that is cheaper than remembering.
- **Both manifests are rewritten every run, including under `--skip-assets`.**
  A manifest that lags the folder it describes is worse than none: it reports a
  good download as corrupt. The pack that sat on disk from 2026-08-17 did exactly
  that — `Play.ps1` had been edited after the manifest was written, so
  `shasum -c` failed on it, and the first thing a friend runs would have called a
  perfectly good download broken.
- **The assets manifest is asserted to cover `BGM/`.** The 2026-08-17 manifest
  listed five files and left 345 MB unverified.
- **The launcher/manifest names are pinned to each other.** Renaming one side
  only is silent until a friend hits it.

- **`client/game_archives.ron` is written, never copied.** The working copy
  points at `../../../RO/client/renewal2021.grf` — paths that escape the pack and
  break only on someone else's machine. The script then *asserts* no `..`
  survived.
- **It aborts if `login_settings.ron` or `window_cache*.ron` reach `dist/`.**
  That file holds a real username and password in plaintext.
- **`--build` puts Homebrew LLVM on `PATH` itself**, so the keg-only trap in §9
  cannot bite whoever runs it next.

`/dist/` is in `.gitignore`; 3.7 GB of Gravity assets must never be offered to
git.

**Every pack ships `SHA256SUMS` plus `Verify.bat` / `Verify.ps1`.** Windows has
no `sha256sum`, so without the verifier the manifest would be unreadable to the
people it exists for. Tell friends to run `Verify` if the game behaves strangely
after downloading: Drive truncates large files and resumes badly, and a 3.7 GB
asset folder is exactly the kind of thing that arrives subtly incomplete. It
reports `All N files match`, or names each corrupt file.

**Zip `Windows/`; upload `Assets/` as a folder.** GRFs are already compressed, so
re-zipping 3.6 GB costs a long wait and saves almost nothing.

**macOS is still not built** — the script is Windows-only. When it is, it needs
the same shape: a `Setup.command`, a `Play.command`, the same two manifests, and
the same contents checklist extended with the macOS entries.

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

**The console window — half decided 2026-09-02.** The binary is *console*
subsystem, so Windows opens a terminal behind the game.
`#![windows_subsystem = "windows"]` removes it.

The reason not to just do that was that the console was the **only** diagnostic
channel the client had: no log file anywhere, no panic hook, so a panic went to
stderr and nowhere else. And it is not only panics — the unconditional
diagnostics are 101 sites tagged `[login]`, `[disconnect]`, `[skill-effect]`,
`[towninfo]`, which is exactly what you want when a friend says "it will not
connect". (A further 60 `println!`s turned out to be `#[cfg(test)]` scaffolding,
not runtime output — worth knowing before anyone counts them again.)

**Done:** `korangar/src/logging.rs`. Every one of those 101 sites now goes
through `client_log!`, which writes to the console **and** to `korangar.log`
beside the executable; one previous run is kept as `korangar.log.previous`,
because the first thing anybody does after a crash is try again. A panic hook
writes the payload and location before deferring to the default handler. The
hook opens its own file handle rather than sharing the mutex — a deadlock
inside a panic hook would replace a readable crash with a hang.

**Still open, deliberately:** whether to set `windows_subsystem = "windows"`.
**Keep the console for the first Windows session**, because nothing in that
binary has ever executed and three §10 rows (the AVX2 check, the bundled
redist, the IP-bound auth) are unexercised code whose failures you will want to
read live. Flip it afterwards — the log file is what makes that safe, and
`READ ME FIRST.txt` already tells friends to send `korangar.log`, so the
support workflow does not change when the terminal goes away.

Do **not** teach friends to clone the repo as a fallback. If someone cannot
run the pack, you run it for them on a call.

---

## 10. Gate before inviting the group

A pack that has not been opened on a **second machine** is not shipped.

Nothing below can be checked from the Mac. The `.exe` cross-compiles, links, and
carries the right imports — and none of that is evidence that it *runs*. Three
of these rows exist specifically because their failure is invisible here.

On a friend’s laptop (or a second account / second OS):

- [ ] Friend accepted the Tailscale invite; `tailscale status` on the host shows them
- [ ] Friend can `nc` (or equivalent) to host `100.x` ports 6900, 6121, 5121
- [ ] Drive download hits the virus-scan interstitial; they click through.
- [ ] **`Setup` runs start to finish and prints "Ready to play."** This is the
      one that covers the most ground: CPU check, runtime install, asset merge,
      259-file verification.
- [ ] **The AVX2 check answers correctly on a machine that HAS AVX2** — i.e. it
      does not false-positive and refuse a good PC. `IsProcessorFeaturePresent`
      is unexercised code; it returns false on Windows 7 by design, and nobody
      has watched it return true.
- [ ] **The client starts on a PC that never had the VC++ runtime.** Test on a
      machine that has not had a game or dev tool installed, or the bundled
      redist is untested by construction — most PCs already have it.
- [ ] **Every player is on the tailnet**, including anyone on your own Wi-Fi.
      As of 2026-09-05 the tailnet has exactly one node (this Mac) and no peer
      has ever joined, so the char/map handoff over Tailscale is **completely
      unexercised** — this row is the first time it will ever run.
- [ ] **`tailscale status` says `direct`, not `relay`, for the LAN player.**
      A relayed peer sends LAN traffic out to the internet and back. If it says
      `relay`, suspect AP isolation or blocked client-to-client UDP on the
      router before blaming the game. `tailscale ping <host>` names the path.
- [ ] Launcher starts the client without a terminal `cd`.
- [ ] Login with a fresh `Name_create` creates an account. (`_M`/`_F` is gone:
      a fresh `Name_m` must now be *rejected* as an unknown account, not
      silently registered.)
- [ ] Character create → Prontera (login-only is not enough — that hides a bad `char_ip`).
- [ ] **Two friends on two machines log in at once.** The auth path is now
      IP-bound (`node->ip == ip`, from the security-audit remediations), which
      loopback testing structurally cannot exercise. If it misbehaves, *every*
      remote tester is rejected at char-select.
- [ ] You (on the host) can see them, whisper works, party invite works.
- [ ] One fight, one NPC shop. Logout and back in.

Before any of that, on the host:

- [ ] **Rebuild Hercules** (`dev.sh build`). The binaries on disk were compiled
      2026-08-18 00:54:42; the security remediations landed at 01:05:07, eleven
      minutes later, so the running server does not contain them.
- [x] **Bump `tools/testing/hercules-revision`.** Done 2026-09-05: it had gone
      from `c07c4b235` (2026-08-07, nine commits behind) to `cda581b12`, the
      `stable` merge of Hercules PR #3. It now tracks **`stable` rather than an
      agent branch**, so it stops drifting every time a branch moves and the
      paired suite pairs with the fork that is actually released.

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

E8.4 (auto-update) and E8.5 (FluxCP) stay deferred. Drive + `_create` cover
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
