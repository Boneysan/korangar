# Security audit — third pass, 2026-08-17

| | |
|---|---|
| **Status** | Independent third pass. Remediations 2026-08-18. A fourth pass is in [security-audit-4.md](security-audit-4.md) |
| **Parent** | [security-audit.md](security-audit.md) (first), [security-audit-2.md](security-audit-2.md) (second) |
| **Trigger** | Look for surfaces the first two passes left unnamed, not re-litigate C1 |
| **Scope of THIS pass** | Loaded official NPCs, login/char session tokens, Lua/GRF/pack integrity, campaign economy and instance isolation |
| **NOT covered** | Fuzzing korangar decoders, a Windows run of the pack, upstream Hercules CVEs in `clif.c` outside session/auth |

**Remediated 2026-08-18:** T1 Lua sandbox (no `io`/`os`/`package`); T2 auth IP bind + token log stripped; T3 party-scoped grant latch on named sites and boss deaths; T4 `DM_TriggerEvent` uses stored id − 1; T5 `load_gm_scripts: false`; T6 Izlude Hypnotist level-50 gate; T7 GRF inflate cap, `lua_files.7z` hashed at load against `SHA256SUMS`, `Play.ps1` refuses a missing or mismatching copy; T8 campaign map/quest allowlists; T9 `nowarp`/`noteleport` on set-piece maps.

The LAN-beta risk acceptance still stands, with the same revoke list. This pass does not re-open C1, H2, `@dmmode`/`@dmflag`, the API server, or MariaDB bind. Those remain as written.

**Do not put the live interserver password in this file.**

---

## Method

Checked against the live tree on 2026-08-17. Renewal is on. `npc/re/scripts_main.conf` is the index; `npc/scripts_removed.conf` is empty, so the official corpus is loaded. `load_gm_scripts: true`.

---

## HIGH — new

### T1. Lua runs with `io` / `os` / `package`. A swapped `lua_files.7z` is host code execution

Every library load does `Lua::new()` then `state.load(&data).exec()`:

```155:165:../../korangar/src/world/library/mod.rs
        let state = Lua::new();
        for file in files {
            let data = game_file_loader.get(file)...;
            state.load(&data).exec()?;
```

mlua 0.11 `Lua::new()` loads `StdLib::ALL_SAFE`: all libraries except `debug`/`ffi`. That **includes** `os.execute`, `io.open`, `io.popen`, `dofile`, `loadfile`, and `require` of `.lua` from CWD. There is no `set_memory_limit` and no `Lua::new_with(TABLE|STRING|MATH)`.

`lua_files.7z` is the payload:

- If the file exists next to the exe, it is trusted as-is (`gamefile/mod.rs` `load_patched_lua_files`). No hash, no signature.
- It is **not** part of `calculate_hash()`, so it does not invalidate `cache.7z`.
- `make-pack.sh` copies it into `dist/Assets/` only.
- `dist/Windows/SHA256SUMS` hashes the exe and `archive/` scaffolding `.lua` files. It does **not** hash `lua_files.7z`.
- **`dist/Assets/SHA256SUMS` does not exist.** The 3.6 GB half, including `lua_files.7z`, cannot be verified.
- `Play.ps1` requires the four GRFs. It does not require `lua_files.7z`. Missing file → client rebuilds Lua from unsigned GRF `.lub`s.

A Drive swap, or planting `lua_files.7z` beside a friend's `korangar.exe`, runs with that friend's Windows user rights on first `Library::new`.

The first pass named this path as uncovered and recommended `SHA256SUMS`. The Windows half now has a manifest; the Assets half, which is the one that can execute, does not.

**Fix:** `Lua::new_with` without `io`/`os`/`package`. Hash `lua_files.7z` at load (or always rebuild from hashed GRFs). Ship `Assets/SHA256SUMS`. Make `Play.ps1` refuse a missing or mismatching copy.

### T2. Auth nodes are not bound to the client IP

```336:337:../../../Hercules/src/login/login.c
		node->sex        == sex_num2str(sex) /*&&
		node->ip         == ip_*/ )
```

The same IP check is commented out on char (`char.c:4440–4441`) and map (`char.c:3821–3823`). `login_id1` / `login_id2` are `rnd() + 1` (31-bit MT19937, seed `time + tick + pid`). They ride the wire in `AC_ACCEPT_LOGIN` in the clear. Nodes are single-use and live 30 seconds.

**Practical on this LAN:** sniff `AC_ACCEPT_LOGIN`, send `0x65` to the char-server from any other host within 30s. First consumer wins. The thief does not need the victim's IP and does not need the password.

The char-server also prints the live tokens:

```
request connect - account_id:%d/login_id1:%u/login_id2:%u
```

`rnd()` is the same generator as `generate_token`. Recovering a 32-bit seed from your own login and predicting the next friend's IDs is theoretically possible; sniffing is the attack that matters here.

A client **cannot** skip the char-server. Map always asks char. That path is closed.

**Fix:** uncomment the three IP checks. Stop logging `login_id*`. Seed `rnd` from `/dev/urandom` if you ever leave a trusted LAN.

This sits next to first-pass H2, not under it. H2 is "they read the password." T2 is "they steal the session after login without the password."

### T3. Campaign rewards check the speaker, pay the whole party

Story NPCs test `questprogress` / flags on **the attached talker**, then `DM_GivePartyItem` / `DM_PartyExp` / `DM_GivePartyZeny` every online member. `DM_InstanceSetFlag` / `DM_InstanceQuestComplete` only update people who are online.

A mule invited later, or a member who was offline at grant time, still has flag 0 / quest 0. They talk, the whole party is paid again.

Verified:

| Site | Speaker check | Party grant |
|---|---|---|
| `arc_01_prontera.txt:351–354` | `!dm_arc01_sigil_ring_obtained` | Sigil Ring (50001) for everyone, then the flag |
| `arc_01_prontera.txt:410–416` `OnDeviruchiDead` | none | 60k/25k EXP every death |
| `arc_06_yuno.txt` Krenn | speaker lacks bribe/exposed | 25,000 zeny |
| `arc_19_finale.txt` | `!dm_campaign_complete` | 1,000,000 / 500,000 EXP |
| `OnSurtDead` | none | 5M / 3M EXP |

Same character cannot replay after their own flag is set. The farm is a fresh alt in the party.

50001 is trade-locked and not unique. Inventory can hold extras. It is not in the `@dmreward` pool.

**Fix:** gate grants on a party- or instance-scoped "already paid" flag, set it **before** the grant, and skip `On*Dead` if that flag is set. For the ring, also `countitem(50001)`.

### T4. `DM_TriggerEvent` uses the stored instance id, not the real one

Storage is intentional (`$dm_inst_<party> = iid + 1` so id 0 is trackable). `DM_InstanceEnd` subtracts 1. `DM_TriggerEvent` does not:

```122:130:../../../Hercules/npc/custom/dm_campaign/shared/dm_common.txt
		.@iid = getd("$dm_inst_" + .@party_id);
		if (.@iid > 0) {
			.@inst_npc$ = instance_npcname(.@npc$, .@iid);
```

`instance_npcname` always returns `dup_<id>_<blid>` if the **world** NPC exists. It does not check that the clone exists. The `!= ""` fallback never runs.

In play:

- One instance (real id 0, stored 1): `@dmbeat` fires `dup_1_*`, which does not exist. Pressure/rift **silently never start**.
- Two+ instances: party A (stored 1) fires **party B's** clone (real id 1). Cross-table event.
- DM not in a party: lookup skipped; world NPC is used.

**Fix:** pass `.@iid - 1` into `instance_npcname`, matching `DM_InstanceEnd`.

---

## MEDIUM — new

### T5. Official GM consoles are loaded, passwords in the script tree

`load_gm_scripts: true`. `F_GM_NPC` requires `getgmlevel() >= 99`, then a password that is public in this repo: `1854` (most consoles), `1357` (Izlude Arena), `8028`, `68392411` (god-seal vars), `"dmc2008"`, `"dragonslayer"`, `"07godsefes"`. Several sit in the open world, not `sec_in02`. A few 3rd-job "Button Girl" NPCs skip the password entirely and only check GM 99.

Effects: reset/force job quests, wipe Mora/13.3 logs, set `$God1–4`, force instance entry, spawn Endless Tower / Nidhoggur stones.

Irrelevant to a group-0 friend. Additive to C1: after you rotate the published admin passwords, **do not leave a group-99 account that walks around towns**, or turn `load_gm_scripts: false`.

### T6. Izlude Hypnotist: first-class respec has no level cap

`npc/re/other/resetskill.txt` is loaded. The dialogue says first class **and** BaseLevel < 50. Enforcement only applies the level-50 cutoff to Taekwon / Gunslinger / Ninja. Swordman through Thief at **any** BaseLevel, including 99, get free unlimited `resetstatus` / `resetskill` (inventory weight must be 0).

Stock Hercules. Campaign characters will find this NPC.

### T7. Assets pack has no checksum. GRF inflate is uncapped

`dist/Assets/SHA256SUMS` is missing. `Verify.ps1` works on the Windows half only. Extra files are ignored; the manifest is unsigned.

GRF zlib (`archive/native/mod.rs`) does `Vec::with_capacity(uncompressed_size)` then `read_to_end` with no cap. A malicious GRF can OOM the client. Distinct from the already-reported `repeating()` OOM.

`FolderArchive` follows symlinks. `game_archives.ron` accepts any path at load. The shipped pack list is relative and clean; the working-tree file still has `../../../RO/client/…`. `make-pack.sh` rewrites it and dies on `..`.

### T8. `@dminstance start` / `@dmwarp` take any map. `@dmquest` takes any quest id

DM's **current** party, not `$dm_active_party`. `@dminstance start prontera` clones every NPC on that map (Kafras, shops, warps). `@dmquest complete 1000` completes official Book of Ymir. `@dmexp` is uncapped. `@dmreward` high tiers include Empelium.

Already need GM level ≥ 1. Worse because group 5 inherits Law Enforcement and Event Manager.

### T9. Set-piece maps have no `nowarp` / `noteleport`

Campaign scripts add no mapflags. `prt_sewb4` (Listening Chamber) is even `pairship_startable`. Players can Fly Wing / Teleport out with the Sigil Ring and vault loot. Instanced copies inherit the same flags.

### T10. Character delete accepts the default email / birthdate

New accounts get `email = a@a.com` and `birthdate = 0000-00-00`. With a **stolen session** (T2), packet `0x68` accepts `a@a.com` or empty; delete-2 accepts `000000` after the 24h delay. Level gate is 0. Cannot delete a stranger from a cold connection.

Pincode is **off** in this fork (`char-server.conf:219`). If it is ever turned on, there is a stock race: `sd->auth = true` before the PIN is demanded, and select only rejects `pincode_enable == -1`. Leave it off until that is closed.

---

## LOW / informational — new

- **Release GM panel** (`CommandsWindow`, Ctrl+O) and the loot/bestiary windows send hardcoded `@item` / `@monster` / `@blvl`. Server group still enforces. Packet inspector and map-warp UI are `debug`-only. Chat is the arbitrary `@` path, which is normal RO.
- **`min_chat_delay: 0`.** Color codes are not stripped. Group 0 cannot run atcommands. No `OnWhisperGlobal` NPC is loaded.
- **Emblem upload** is SQL-only and token required. The guild-master check occurs only after GIF decoding, whose frame-size check is commented out. Fourth-pass [P4-1](security-audit-4.md#p4-1-emblemupload-performs-attacker-amplified-gif-allocation-before-resource-and-guild-checks) reclassifies this as a high allocation/decode DoS; the API should not be running on a non-loopback interface (second-pass N3).
- **Cash shop** is stock dummy fruit/potions. No loaded NPC writes `#CASHPOINTS`. Campaign never touches it.
- **`item_db2.conf`** live custom item is only 50001 Sigil Ring, fully trade-locked. Sample GM gear above it is commented out.
- **Quest IDs 20001–20233** exist only in the campaign block. Last-hit cannot complete job-change quests. `@dmquest` can (T8).
- **`Play.ps1`** is clean: existence checks, `Unblock-File`, `Start-Process`. No `iex`, no download, no Defender changes.
- **RON settings** deserialize into fixed structs. No typetag gadget.
- **ffmpeg** is `--sync-cache` only, fixed `pipe:0`/`pipe:1` argv. Friends launch does not pass it.
- **Shipped client makes no HTTP(S) calls.**
- **Group 0** still cannot slot-change, rename, or sex-change (DB counters default 0; sex change is a map→char GM packet).
- **`start_zeny` is 0.** Drop/EXP rates are 100%. Jobmaster/healer/warper still commented out.
- **`F_GM_NPC` itself is fail-closed** for GM < 99. `atcommand()` / `setgroupid` / `fopen` / `query_sql` are absent from loaded official scripts.

---

## CWE map (this pass)

| Finding | CWE |
|---|---|
| T1 unsandboxed Lua + unsigned archive | CWE-94 / CWE-494 |
| T2 auth not bound to IP | CWE-384 Session Fixation / CWE-345 |
| T3 speaker-only reward gate | CWE-863 Incorrect Authorization |
| T4 wrong instance id | CWE-682 Incorrect Calculation |
| T6 missing level gate | CWE-863 |
| T7 missing Assets checksum / zip-bomb | CWE-494 / CWE-400 |
| T10 default email as delete proof | CWE-640 Weak Password Recovery |

---

## Recommended order (on top of passes 1–2)

Passes 1–2 still start with **rotate C1**. Then, from this pass:

1. **Sandbox Lua** and put `lua_files.7z` (and the GRFs) in a real Assets manifest that `Play.ps1` checks.
2. **Uncomment the three auth IP checks.** Stop logging `login_id*`.
3. **Make campaign grants idempotent** (party flag set before the item/EXP). Guard `On*Dead`.
4. **Subtract 1 in `DM_TriggerEvent`.**
5. `load_gm_scripts: false` for game nights, or keep group 99 off the map. Fix or disable the Izlude Hypnotist. Add `nowarp`/`noteleport` to set-piece maps.
6. Restrict `@dminstance` / `@dmwarp` / `@dmquest` to campaign allowlists.

---

## Coverage after three passes

| Surface | Where recorded |
|---|---|
| Credentials, MD5, pack hygiene, Rust advisories | Pass 1 |
| API, MariaDB, `@dmmode` / `@dmflag`, fork packets, host tools | Pass 2 |
| Lua/GRF, session tokens, official NPCs, campaign economy, instances | **This pass** |
| Still not done | Decoder fuzzing, Windows execution of the pack, upstream `clif.c` CVE sweep |
