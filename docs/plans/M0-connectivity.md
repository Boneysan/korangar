# Implementation Plan — M0 Connectivity

| | |
|---|---|
| **Status** | Complete (2026-07-08) — login → char create → map walk validated live |
| **Milestone** | M0 — Korangar logs into Hercules_RO, creates a character, walks around Prontera |
| **Parent** | [PROJECT_PLAN.md](../PROJECT_PLAN.md) E1, [SOFTWARE_DESIGN.md](../SOFTWARE_DESIGN.md) §5–§7 |
| **Non-goal** | Fixing playability gaps beyond the first login/char/map loop |

## 1. Scope

Bring the local development stack to the first playable loop:

```
build server/client → login → character select/create → map connect → walk in Prontera
```

This plan closes E1.2–E1.8. E1.1 is already decided: Hercules moves to
`PACKETVER=20220406` because Korangar only supports `20220406`.

## 2. Preconditions

- Hercules repo exists at `~/GitHub/Hercules_RO`.
- Official client assets exist at `/mnt/h/RO/client/`.
- `slangc` is installed and visible on `PATH`.
- Rust nightly is active through `rust-toolchain.toml`.
- MariaDB/MySQL backing Hercules is running.

Current local workspace notes:
- `korangar/data.grf` and `korangar/rdata.grf` are currently symlinks to
  `/mnt/h/RO/client/`.
- `korangar/archive/data/sclientinfo.xml` already points at `127.0.0.1:6900`
  with `<version>55</version>` and Korangar-specific
  `<packet_version>20220406</packet_version>`.
- The default archive list is `data.grf`, `rdata.grf`, and `archive/`.
  Korangar reads `client/game_archives.ron` after changing cwd into `korangar/`,
  so the runtime settings path is `korangar/client/game_archives.ron`.
  The 2021 GRFs need explicit registration before asset coverage is considered complete.

Current progress:
- Hercules was rebuilt with `PACKETVER=20220406`, packet obfuscation disabled,
  and login/char/map servers verified listening on `6900`/`6121`/`5121`.
- Rust nightly, `slangc`, ALSA development headers, and `nasm` are installed.
- `cargo build -p korangar --features "debug unicode"` succeeds.
- Bounded startup with WSLg runtime variables reaches window/surface creation and
  shows the Korangar login window.
- Test account `korangar` / `korangar` reaches the character select screen,
  confirming login-server and char-server handoff.
- Character creation succeeds from Korangar.

## 3. Files Touched

Korangar:
- `korangar/archive/data/sclientinfo.xml`
- `korangar/client/game_archives.ron` if created/configured for this project, or the
  archive-list default in `korangar/src/loaders/gamefile/list.rs` if we choose a
  repo-pinned default.

Hercules_RO:
- `conf/import/battle.conf`
- Optional LAN profiles: `conf/import/char-server.conf`, `conf/import/map-server.conf`
- Build artifacts from `./configure --enable-packetver=20220406 && make`

## 4. Steps

1. Confirm current ports and server process state.
   - `login-server :6900`
   - `char-server :6121`
   - `map-server :5121`
   - DB connection succeeds.

2. Rebuild Hercules for Korangar's packet version.
   ```bash
   cd ~/GitHub/Hercules_RO
   ./configure --enable-packetver=20220406
   make
   ```
   Record the resulting `PACKETVER` in the M0 worklog.

3. Disable packet obfuscation server-side.
   - Add or update `conf/import/battle.conf`:
     ```conf
     packet_obfuscation: 0
     ```
   - Confirm no later import or battle config overrides it back to `2`.

4. Decide temporary asset strategy for M0.
   - Fastest path: keep existing symlinks for `data.grf` and `rdata.grf`.
   - If map load or random GRF access is slow, copy all four GRFs into
     `korangar/` on the WSL ext4 filesystem.
   - Register `renewal2021.grf` and `resources2021.grf` in the archive list once
     the copy/symlink decision is made.
   - Archive precedence is reverse load order: later entries are inserted at the
     front. For highest priority `archive/` over 2021 GRFs over base GRFs, the
     settings list should start with `data.grf` and end with `archive/`.

5. Verify `sclientinfo.xml`.
   - WSL dev profile: `127.0.0.1:6900`, official client version `55`, packet
     version `20220406`.
   - Windows/LAN profile, if needed later: LAN IP plus the Windows port-forward
     script. Do not block M0 on this.

6. Build Korangar.
   ```bash
   cargo build --features "debug unicode"
   ```
   If Cargo requires an explicit package in this virtual workspace, use:
   ```bash
   cargo build -p korangar --features "debug unicode"
   ```
   Use `--release` only after the debug build works.

7. Start Hercules and run the client.
   - Start login/char/map servers from Hercules.
   - Run Korangar with the debug packet inspector available.

8. Execute M0 demo.
   - Log in.
   - Create or select a character.
   - Enter Prontera or the configured start map.
   - Walk at least 10 tiles.
   - Send one chat message.
   - Logout cleanly.

## 5. Verification

M0 passes when:
- Client reaches map-server without packet-obfuscation disconnect.
- Character appears in-world and can move.
- Packet inspector shows no unknown packet that breaks framing during the login
  → char → map sequence.
- Server logs do not show auth, packetver, or obfuscation errors.
- Any observed gap is recorded as M1/M2 work, not left as a mystery.

## 6. Rollback / Recovery

- If the official 2019 client must be used again, rebuild Hercules back to
  `PACKETVER=20190605`; Korangar will stop working until Hercules returns to
  `20220406`.
- If `/mnt/h` GRF access is too slow, copy GRFs into WSL and update archive paths.
- If map connect fails immediately, first check `packet_obfuscation`, then packet
  version, then unknown-packet logs.
