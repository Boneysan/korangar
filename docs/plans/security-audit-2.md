# Security audit — second pass, 2026-08-17

| | |
|---|---|
| **Status** | Independent re-audit. Remediations 2026-08-18. Later passes: [security-audit-3.md](security-audit-3.md), [security-audit-4.md](security-audit-4.md) |
| **Auditor** | Second pass, after [security-audit.md](security-audit.md). Not a re-read of that document — the live tree, the live processes, and the live `login` table were checked again |
| **Trigger** | The first pass had been written and six findings fixed. A second set of eyes before friends arrive |
| **Scope of THIS pass** | Re-verify every first-pass claim against current code and the running box. Then cover what the first pass left out: API server, MariaDB bind, loaded NPC/DM scripts, fork packet deltas, host tools |
| **NOT covered — say so rather than imply coverage** | Fuzzing of korangar decoders, upstream Hercules CVEs in `clif.c` beyond this fork's deltas, the Lua/GRF asset path, a Windows run of the pack |

**Remediated 2026-08-18:** C1 rotated (see first-pass note); N1 session party 0; N2 `dm_*` flag names; N3 API `bind_ip` 127.0.0.1; N4 MariaDB loopback; N5 logs locked and the pre-MD5 dump removed; N7 test NPCs unloaded; N8 spawn cap; N9 group 5 no longer inherits `@ban`/`@item`; N10 `network.conf` loopback; N11 fail-ban on; N12 leftover buffer reset; T4-adjacent `DM_TriggerEvent` id-1 is in pass 3.

The first pass's **risk acceptance for the LAN beta still stands**, with the same revoke conditions (port-forward, untrusted tailnet member, anything worth stealing). This document does not re-litigate that. It records what the first pass got right, what it missed, and what is still live.

**Do not put the live interserver password in this file.** It lives in gitignored `Hercules/conf/import/{char,map,api}-server.conf`. Writing it here would publish it.

---

## Method

Checked against the box as it was running on 2026-08-17:

- `login-server` `*:6900`, `char-server` `*:6121`, `map-server` `*:5121`, `api-server` `*:7121` — all four up
- `mariadbd` on `*:3306` (IPv4 and IPv6)
- macOS Application Firewall enabled, "Block all incoming" off
- Live `login` table read as `ragnarok`@`localhost`
- `Boneysan/korangar` and `Boneysan/Hercules` are both **public**
- Import configs are gitignored; `git log` on those paths and `git log -S` on the rotated interserver userid/password are empty — they have **never** been committed

---

## Scorecard of the first pass

| ID | First-pass claim | This pass |
|---|---|---|
| **C1** | Two group-99 accounts, passwords in the public repo | **Remediated 2026-08-18.** Live hashes no longer match the published strings. `headless2` is group 0 |
| **H1** | Plaintext `user_pass` | **CONFIRMS fixed.** `use_MD5_passwords: true` in import; all four rows are 32-char hashes |
| **H2** | Password on the wire in the clear | **CONFIRMS.** Korangar still sends `CA_LOGIN` 0x0064 with a 24-byte plaintext field |
| **M1** | `ip_rules` off | **CONFIRMS fixed.** Import has `enable: true` and allow-lists `127.0.0.1`. The allow-list **does** exempt localhost from the DDoS heuristic (`connect_ok == 2`) |
| **M2** | Interserver still `s1`/`p1` | **CONFIRMS fixed in import**, and the secret is **not** in git (the first pass did not claim it was). Previous 24-char attempt is in old logs |
| **M3** | `quick-xml` 0.39.2 | **CONFIRMS.** Ours is 0.41; 0.39.2 remains via `wayland-scanner` (Linux build-time only) |
| **M4** | Remember-password unlabelled | **CONFIRMS mitigated.** Label reads "(saved unencrypted)". Still plaintext on disk if ticked |
| **M5** | Client panic on short first TCP segment | **PARTIAL.** The `unwrap` is gone. Leftover-buffer accounting is still wrong — see N12 |
| **M6** | Missing visual-effect asset crashed the client | **CONFIRMS fixed.** Logs and skips |
| **G1–G4** | Group 0 has no commands; `@dm*` is level ≥ 1; no campaign SQL; pack refuses `login_settings.ron` | **CONFIRMS** |
| **packet_obfuscation is 0** | Stated as a project decision | **REFINES.** Stock `client.conf` is `2`. Import does not override it. `packets_keys_main.h` has **no keys for PACKETVER 20220406**, so the XOR is `^ 0` and the setting is a no-op. That is why Korangar can connect |
| **Pack has no checksum** | SLSA gap | **STALE.** `make-pack.sh` writes `SHA256SUMS`. `Play.ps1` does not run the verifier |
| **DM scripts have no injection** | No `query_sql` | **PARTIAL.** No SQL. `@dmflag set` is unsanitized `setd` — see N2 |

---

## CRITICAL

### C1. Two full-admin accounts, passwords still published — still open

Live `login` table, 2026-08-17, after the MD5 migration:

| account | group | `user_pass` |
|---|---|---|
| `korangar` | **99** | MD5 of the username (32 hex, matches) |
| `headless2` | **99** | MD5 of `headless2pw` (matches) |
| `headless3` | 0 | MD5 of `headless3pw` (matches) |
| `korangar_inter` | 0, sex `S` | 32-char hash (interserver) |

The published strings are still in the **public** `Boneysan/korangar` tree:

- `korangar-networking/examples/headless-tester/main.rs` (defaults)
- `tools/testing/run-integration-tests.sh` (SQL insert, `group_id` 99)
- `docs/plans/gui-verification-pass.md`, this family's first-pass doc

`tools/audits/committed-secrets.sh` only patterns `headless2pw` / `headless3pw`. The `korangar`/`korangar` pair is **invisible to CI**. `headless3pw` baseline lines for `main.rs` / `run-integration-tests.sh` are stale — those hits are gone.

Local (gitignored) copies currently hold `headless2` / `headless2pw` with remember-password on:

- `korangar/korangar/client/login_settings.ron`
- `client2/client/login_settings.ron`

MD5-at-rest does **not** mitigate this. The attacker types the published password; the server hashes it for them.

What they get: every atcommand (`all_commands: true`), including `@item`, `@kick`, `@ban`, `@reloadscript`. What they do **not** get: a shell, raw SQL, or arbitrary file read — re-checked against `ACMD()` in `atcommand.c`. Game world, not the host.

**Fix:** change both passwords, drop `headless2` to group 0 (or delete it), update the harness in the same commit, and add the *old* strings to `committed-secrets.sh` so they cannot come back.

---

## HIGH — new in this pass

### N1. `@dmmode on` with no party unlocks the campaign for every solo

```525:533:../../../Hercules/npc/custom/dm_campaign/shared/dm_console.txt
	if (.@action$ == "on") {
		$dm_mode = 1;
		$dm_active_party = getcharid(CHAR_ID_PARTY);
```

Campaign NPCs gate with:

```70:70:../../../Hercules/npc/custom/dm_campaign/act_01/arc_01_prontera.txt
	if (!$dm_mode || getcharid(CHAR_ID_PARTY) != $dm_active_party) { close; }
```

If the DM is not in a party, `$dm_active_party` is `0`. Every unpartied player then matches `0 == 0` and can take quests, EXP, zeny, and the Sigil Ring. The Session Board (`arc_01_prontera.txt:32`) uses the same test and will tell them a session is underway.

**Exploit:** DM types `@dmmode on` before forming a party, or after leaving one.

**Fix:** refuse `@dmmode on` unless `getcharid(CHAR_ID_PARTY) > 0`. Treat `$dm_active_party == 0` as "no session" in every gate.

### N2. `@dmflag set` is unsanitized `setd`

```817:820:../../../Hercules/npc/custom/dm_campaign/shared/dm_console.txt
	if (.@action$ == "set") {
		.@value = atoi(@dm_atcmd_p$[.@offset + 2]);
		.@done = callfunc("DM_InstanceSetFlag", .@flag$, .@value);
```

The name is a raw atcommand token. `DM_InstanceSetFlag` → `DM_PartyApplyFlag` → `setd(.@flag$, .@value)` on every online party member (`dm_quests.txt:94–117`, `dm_flags.txt:3–11`). No `dm_` prefix check.

`@dmflag set Zeny 1000000000` and `@dmflag set BaseLevel 99` work. `setd` cannot change login `group_id` (`setgroupid` is never called from `npc/`), but it is a full economy/stat editor for anyone with GM **level** ≥ 1 — including Support (group 2), who do not otherwise have `@item` / `@zeny`.

**Fix:** allow only names matching `^dm_`.

### N3. API server is listening on `*:7121` and the firewall allows it

`api-server` is up (started by `athena-start`). Incoming connections to that binary are **permitted**. Korangar does not need this process.

| URL | Flags | Note |
|---|---|---|
| `GET /test/url` | `REQ_DEFAULT` | No auth. Reflects `User-Agent` into HTML. `NULL` UA is undefined `snprintf` (`handlers.c:628–639`) |
| `POST /userconfig/load` | `REQ_API` | Account must be online; **no auth token**. Sequential AIDs (`2000000`…). Hotkeys/emotes, not passwords |
| other POSTs | `REQ_API_AUTH` | 16-byte token compared with `memcmp` |

**Fix:** do not start `api-server` for game nights, or set `bind_ip: "127.0.0.1"` in `conf/import/api-server.conf`.

---

## MEDIUM — new in this pass

### N4. MariaDB listens on `*:3306`; the published SQL password is one GRANT from working

`mariadbd` is bound to all interfaces. Incoming is permitted. `conf/global/sql_connection.conf` (tracked, public) has `ragnarok` / `ragnarok`.

Connecting via the LAN address is **rejected**: the user is `ragnarok`@`localhost` only (`ERROR 1130`). Loopback TCP with those credentials works, and that user has `SELECT, INSERT, UPDATE, DELETE` on `ragnarok.*` — enough to `UPDATE login SET group_id = 99`.

The port is open and the password is published. Only the MySQL user table stops remote login. That is one `GRANT` away from a full dump.

**Fix:** `bind-address = 127.0.0.1` in MariaDB. Do not add `ragnarok`@`%`.

### N5. Login logs used to print passwords in the clear

Stock Hercules (`src/login/login.c:1180, 1185`) logs stored pass + received pass on failure. `lclif.c` hashes the incoming password **before** `mmo_auth` when MD5 is on, so post-migration lines are hashes. Pre-migration they are plaintext.

Verified in `log/server-20260816-222759.log`: `Invalid password (account: 'headless2', pass: 'headless2pw', received pass: 'headless2p', ...)`. The 24-character interserver attempt (silently truncated at `NAME_LENGTH`) is in the same log family.

`log/*.log` is gitignored. `log/*.out` is not. `log/db-snapshot.sql` is gitignored and is a pre-MD5 dump (`s1`/`p1`, plaintext player passwords).

**Fix:** delete or lock down `log/server-*.log`, `log/*.out`, and `log/db-snapshot.sql`.

### N6. World-map boss deaths credit the last-hitter, not the session

Kill labels such as `OnMistressDead` (`arc_06_yuno.txt:295–303`) call `DM_PartyExp` / `DM_InstanceSetFlag` on the attached killer's party. The NPC sits on the real map (`juperos_01`). Same pattern on the other arc set-pieces.

If the set-piece is not instanced, a random last-hit takes the arc EXP and flags. Session party may get nothing.

### N9. Group 5 "Dungeon Master" inherits `@ban` / `@kill` / `@item`

`tools/promote-dm.sh` sets group 5. That group inherits Event Manager (`@item`, `@zeny`) **and** Law Enforcement (`@ban`, `@block`, `@jail`, `@kill`). The comment in `groups.conf` says the seat is table-control without full admin. Inheritance contradicts that. A promoted friend-DM can ban the host.

Not an in-game exploit by a default player. It is the only intended GM path.

### N10. `network.conf` allows a char/map/api server from any IP

```22:25:../../../Hercules/conf/network.conf
allowed: (
	"0.0.0.0:0.0.0.0",
)
```

The login server warns about this on every start. Combined with bind-all, anyone who learns the interserver secret (gitignored import files, or old logs) can attach a fake char/map/api server. Stock Hercules default; this fork did not tighten it.

**Fix:** restrict `allowed` to `127.0.0.1:255.0.0.0`.

### N11. Open `_M`/`_F` registration; dynamic fail-ban off

`new_account: true` is stock and not overridden — friends-distribution wants it. `dynamic_pass_failure.enabled: false` is a **tracked** fork change (`login-server.conf:115–116`, commit "Disable dynamic password failure ipban for development/testing environment"). Source default is on.

Rate limit is 1 register / 10 s. Anyone who can reach 6900 can mint accounts and guess forever. Tolerable on a closed LAN; the first thing that breaks if 6900 is reachable from outside.

---

## LOW — new in this pass

### N7. Identify Test NPC is live in Prontera

`npc/custom/identify_test.txt` is listed in `scripts_custom.conf`. Any player, unlimited, gets an unidentified Cotton Shirt / Knife / Guard / Headset. Test fixture left in the world. `headless_dialog_test.txt` is also loaded; it only echoes `input()` and warps to Payon.

### N8. `@dm spawn` has no count cap

`@dm spawn 1002 20000` will stall the map-server (`dm_console.txt:864–882`). Same class as stock `@item` quantity. GM level ≥ 1.

### N12. Short-read leftover, and uncapped `repeating(count)`

After a short first TCP segment is completed, `cut_off_buffer_base` is not reset if the rest of the buffer parses cleanly (`korangar-networking/src/lib.rs:351–401`). The next `read` can re-include already-consumed bytes. Availability (odd disconnect after login on wifi), not memory unsafety. Still no test.

`#[repeating(achievement_count)]` and similar do `Vec::with_capacity` with no cap (`ragnarok-macros`). A malicious or MITM server can OOM the client. Outside the current threat model — friends trust this server.

### Other lows

- `@roll` is group 0 and `mapannounce`s the current map. Bounds are capped. `fudge` / `hidden` correctly require GM 1. The old `sscanf` leak is fixed.
- `tools/promote-dm.sh` and `create-account.sh` interpolate `$USERNAME` / `$PASSWORD` into SQL and hardcode `ragnarok`/`ragnarok`. Local shell only.
- `crossbeam-epoch` is still 0.9.18 (first-pass L1, unfixed).
- `rust-state` is a git dep with no `rev` in `Cargo.toml` (lockfile pins it).
- Unsigned binaries (first-pass L4). `Play.ps1` does not run `Verify.ps1`.
- `.github/workflows/security.yml` only runs on `main`. Long-lived agent branches do not get `cargo audit` except via the weekly cron on `main`.

---

## First-pass leftovers that stay accepted

These were already graded and accepted for the LAN beta. Re-verified, not re-opened.

| ID | State |
|---|---|
| **C1** | Still open. Acceptance still bound by reachability. Same revoke list |
| **H2** | Won't fix in-protocol. VPN is the transport fix. Packet obfuscation is a no-op for 20220406 (see scorecard) |
| **M4** | Mitigated by label. OS keychain deliberately not done |

Tailscale still does **not** close the LAN path. Servers bind `INADDR_ANY`. The first-pass note on that is still correct; this pass adds that **7121 and 3306** are in the same boat, and that 3306 is currently saved only by `ragnarok`@`localhost`.

---

## Verified clean — this pass, not assumed

| Item | Evidence |
|---|---|
| Fork inbound parser `0x0F00` | No payload. One call to `unit->skillcastcancel`. Length 2 in `packets_len.h`. `MAX_PACKET_DB` includes this slot |
| ZC writers `0x0EFF` / `0x0EFE` | `WFIFOHEAD` + `sizeof`. Name copy is `safestrncpy(..., NAME_LENGTH)` |
| `@item` multi-word peel | `safestrncpy`, `%99[^\"]`, `strtol` with endptr, `max_numbers` cap against a 16-int stack array |
| No `system` / `popen` / `exec` in `src/` | Grep hits are billing *message* strings only |
| No `query_sql` in loaded custom scripts | Only `woe_controller.txt` and `hunting_missions.txt`, both commented out of `scripts_custom.conf` |
| No player → group 99 path | `setgroupid` unused in `npc/`. `@adjgroup` is admin-only. `promote-dm.sh` hardcodes group 5. `@dmflag set group_id 99` would create a character script variable, not the login group |
| Group 0 has no atcommands | `groups.conf:79–91`. `@dm*` is `bindatcmd(..., 1, 99, 1)` plus `DM_RequireDM` |
| `httpsample` not loaded | `plugins.conf` — all plugins commented |
| Interserver secret never committed | `git log` on the import files is empty; `git log -S` on the rotated userid/password is empty |
| `make-pack.sh` refuses credentials | Dies if `login_settings.ron` or `window_cache*.ron` reach the pack. On-disk `dist/Windows/` has neither |
| `attachrid` loops restore the origin | `dm_quests.txt`, `dm_rewards.txt`, `dm_traps.txt`, `DM_HazardArea` detach then re-attach. No stuck-RID path found |
| Healer / warper / jobmaster / reset / item mall | Commented out in `scripts_custom.conf` |

---

## CWE map (this pass only)

| Finding | CWE |
|---|---|
| C1 published admin passwords | CWE-798 Hard-coded Credentials |
| N1 session party `0 == 0` | CWE-863 Incorrect Authorization |
| N2 unsanitized `setd` | CWE-94 / CWE-116 (script injection of a variable name) |
| N3 unauthenticated API + IDOR | CWE-306 / CWE-639 |
| N4 MySQL on all interfaces + published password | CWE-284 / CWE-798 |
| N5 passwords in logs | CWE-532 Insertion of Sensitive Information into Log File |
| N10 interserver from any IP | CWE-284 |
| N11 fail-ban off | CWE-307 Improper Restriction of Excessive Authentication Attempts |
| First-pass H2 cleartext on the wire | CWE-319 |

---

## Recommended order

1. **Rotate C1** and stop committing the new password. Drop `headless2` to group 0. Extend `committed-secrets.sh` to the old values (the username-as-password pair needs an exact-string pin, not a generic `korangar` match).
2. **Refuse `@dmmode on` without a party**, and reject `$dm_active_party == 0` in every campaign gate.
3. **Whitelist `dm_*` in `@dmflag set`.**
4. **Stop `api-server` for game nights**, or bind it to loopback.
5. **`bind-address = 127.0.0.1`** for MariaDB. Tighten `network.conf` `allowed` to loopback.
6. Unload the two Prontera test NPCs. Cap `@dm spawn`. Narrow group 5 so it does not inherit `@ban` / `@kill`.
7. Delete or lock down `log/server-*.log`, `log/*.out`, and `log/db-snapshot.sql`.

Nothing in 2–7 matters as much as 1 while two published passwords grant full admin.

---

## What this pass still did not cover

- Fuzzing korangar's packet decoders (the first pass named this; still not done)
- Upstream Hercules CVEs in `clif.c` outside the fork deltas
- The Lua/GRF asset path (a tampered `lua_files.7z` is local code execution — pack integrity is the control)
- A real Windows run of the friends pack (still never executed on Windows)
- Whether `pf` would be the right LAN lock once the tailnet includes anyone not fully trusted — the first pass already recorded that decision
