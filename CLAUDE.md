# Korangar — agent notes

Rust Ragnarok Online client (wgpu 29 + winit). This fork's goal is a usable
custom UI for a friends group + DM campaign.

**PAUSED 2026-07-26 — the next session is a LIVE PASS, not new features.** Seven
commits from 2026-07-26 are pushed but **nothing in them has been seen on screen**:
the aiming footprint, ground+support walk-into-range, item names everywhere,
ammo-item projectiles, cast cancel (right-click/Escape), and Moonlit/Hermode. The
checklist, the stack bring-up order, and the traps are in
[docs/RESUME-HERE.md](docs/RESUME-HERE.md). Read that first.

**Start here for all documentation:**
- [docs/README.md](docs/README.md) — Master index/hub with categories, search guidance, and links to everything.
- Then `docs/CLIENT_SYSTEMS_OVERVIEW.md` and `docs/SOFTWARE_DESIGN.md` for architecture.

See the Documentation Hub above for the full list (DM tools, packets, world, graphics, plans, specs, etc.).

**NEXT animation task (2026-07-26):** The **engine track (phases A–D) is
closed**, and so is most of the data track. E1 **DONE 7/7 live**. E2 **DONE** —
both unit batches plus **Hunter traps**, which now spawn as real RSM props
(`trap_model_file()` / `TRAP_MODEL_FILES` in `world/unit_recipe.rs`; models
preloaded into the map's shared buffer at load, placed at runtime as ordinary
`ModelInstruction`s). E4 — **every grounded item done live**: opt1/opt2 tints, attached
looping status STRs, Freeze1/Freeze2 split, and sprite freeze extended to
stun/sleep (`status_freezes_animation` — Hercules immobilises every opt1 state
bar STONEWAIT/BURNING). What remains is **blocked on reference material, not
effort**: a specific held pose, and refresh variants — do not eyeball either. E3 **partial** — architecture complete, the gap is
table coverage (~79 mapped `EffectId` references against 1124 enum variants);
unmapped ids log under `KORANGAR_PACKET_LOG`. Phase F (timing polish) not
started and two of its three items are gated on Phase A fixtures.
**Newest, and the first thing to look at: the ground-skill aiming footprint
(2026-07-26) is wired and test-verified but has never been seen on screen.**
While a ground skill is armed the cursor draws its **real area** — squares from
`skill_db`'s `Layout`, the fifteen custom shapes hardcoded in Hercules'
`skill_init_unit_layout`, and the four direction-dependent walls (Fire Wall, Ice
Wall, Earth Strain, Fire Rain) oriented via a port of `map_calc_dir`. Lives in
`world/skill_layout.rs` + `Map::render_skill_footprint`. It also tints the
footprint red when out of range. **Resolve skill ids through
`docs/skills.json`, never from the Hercules constant names — nine of eighteen
were wrong on the first pass.**
Next (cheapest first): live-verify that footprint (does Land Protector Lv10's
225-cell area read as a shape or a solid slab?); then the cast circles (`Lockon`
+ six `Beginspell` recipes exist, never looked at — but they are procedural
placeholders over generic ring textures, so expect a rebuild, not a tick);
then E3 coverage from the `KORANGAR_PACKET_LOG` unmapped-id log. Plan:
[docs/plans/animation-fidelity.md](docs/plans/animation-fidelity.md) §6.

**Three follow-ups closed 2026-07-26 (code + tests only — NONE live-verified):**
- **Out-of-range casts no longer vanish — all three targeting modes now walk into
  range.** Hercules drops an out-of-range cast with a bare `return 0` and **no**
  `clif->skill_fail`, both for ground (`unit.c` `unit_skilluse_pos2`) and for entity
  targets (`unit_skilluse_id2`), so the cast was silently lost. **Attack** already
  had `cast_or_path_entity_skill`; **Ground/Trap** got `cast_or_path_ground_skill` +
  `BufferedAction::CastGroundSkill`; **Support** (Heal, Blessing) was casting with no
  range check at all and now routes through the entity path too — a self-target is
  distance 0, so self-buffs still fire instantly. All three report in chat when no
  walkable cell gets close enough. Casting itself still **roots** the character,
  which is correct RO behaviour (see the cast-cancel note below).
- **No user-facing message may print a raw item id.** `ZC_ACK_TOUSESKILL` causes
  71/72 now cross the crate boundary as `NetworkEvent::SkillFailedMissingItem`
  (`korangar-networking` has no item DB) and the client names the item. Same for
  the trade-window rows and the weapon-refine result, all via `resolve_item_name`
  — which also filters the **`NOTFOUND` sentinel**, a second failure mode distinct
  from a missing entry that will print verbatim if you only check `try_get`.
- **Ranged projectiles use the ammo item's sprite** (per-arrow-type, plus firearms
  and shuriken). See
  [docs/protocol/inventory-and-ranged-attacks.md](docs/protocol/inventory-and-ranged-attacks.md).

**Mid-cast interruption is NOT an RO feature — it would be a fork addition.**
An earlier session note claimed "RO cancels a cast by moving". **That is false**,
and Hercules is faithful here on both counts:
- **You cannot move while casting.** `unit_can_move` (`unit.c:1230`) roots the
  caster; the only exceptions are the Sage skill `SA_FREECAST` and skills flagged
  `FreeCastNormal` / `FreeCastReduced` in `skill_db.conf`. With Free Cast you walk
  *and keep casting* — movement still never cancels.
- **There is no player-initiated cast cancel.** Nothing skill-cancel-shaped exists
  in `src/map/packets.h`. Casts end by completing, by damage interrupting them
  (the skill's `castcancel` flag — what Phen card / Bragi protect against), or via
  the Sage skill `SA_CASTCANCEL`, which `skill.c:1534` whitelists as the one skill
  usable mid-cast.

So right-click/Esc-to-abort is a **deliberate fork feature**, not a fidelity gap.
**Approved and BUILT 2026-07-26** as a fork addition: `CZ_CANCEL_CAST` (`0x0F00`)
plus the matching Hercules delta — see §3b for the four touch points and its three
silent-failure traps. Client side: `cancel_own_cast` in `lib.rs`, bound to
right-click and Escape, ordered *after* the armed-skill cancel so both gestures
still clear a reticle first. It does not clear the cast bar optimistically —
Hercules broadcasts `clif->skillcastcancel`, which returns as
`SkillCastCancelled`. **Movement still never cancels, and casting still roots**;
that half is authentic and was left alone deliberately.

**Two rendering findings worth knowing before touching effects or props:**
- **A multiplicative tint cannot desaturate** — multiplying by grey only darkens.
  Status effects that drain colour (petrification) need the sprite mixed toward
  its luminance; `StatusTint` carries `color` *and* `desaturation`, and the
  entity shader drains hue **before** the tint multiplies in.
- **RO effect textures are alpha-keyed and the original relies on alpha
  TESTING.** Most STR layers blend additively, where a transparent texel is black
  and free — which hid the omission for the whole E1/E2 programme. Layers that
  blend One/Zero (`silence.str` layer 6, every layer of `sleep.str`) write the
  source verbatim and painted a black square until the effect shaders got a
  discard. Both `effect.slang` and `effect_bindless.slang` carry it.

**Effect fidelity — read this before touching skill visuals.** RO ships effects in
**two families**: `.str` keyframe scripts (`data\texture\effect\`) and sprite/
procedural effects the exe hardcodes. The authoritative skill→effect→asset mapping
is **reverse-engineered in roBrowserLegacy's DB tables** (SkillEffect/EffectTable/
SkillUnit) — use those as *data*, never guess GRF filenames (guessing failed live).
E1 wizard skills and E2 batch-1 persistent units are mapped and live-verified;
recipes live in `world/skill_recipe.rs` (one-shot) and `world/unit_recipe.rs`
(persistent units). See [docs/plans/classic-effect-fidelity.md](docs/plans/classic-effect-fidelity.md).
Caution: `get_files_with_extension` **under-reports the GRFs** — probe with
`file_exists`, or parse the GRF table directly.
Fixes shipped while closing D (all on `agent/platform-connectivity-controls`):
armed players stand in the ReadyFight stance so weapon+shield render at idle
(`78f57915`), and relax to Idle on town/safe maps (`abb519f1`); respawn/resurrect
revive the sprite (`78f57915`); ammo (arrows) equips/stacks/shows count and a
normal bow attack draws a flying arrow (`d3f7c5dd`, `2b637bac`) — see
[docs/protocol/inventory-and-ranged-attacks.md](docs/protocol/inventory-and-ranged-attacks.md).
Ranged projectiles now draw the *ammunition item's* sprite (2026-07-26), covering
per-arrow-type, firearms and shuriken; only the grenade launcher lacks a sprite.

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

The project's main automated regression gate is a **123-scenario headless client**
(`korangar-networking/examples/headless-tester`). Acceptance passed 2026-07-13 with a
double-run green gate. Docs live in **`tools/testing/`** (not `docs/`):
[headless_test_plan.md](tools/testing/headless_test_plan.md) is canonical,
[headless_findings.md](tools/testing/headless_findings.md) is the bug log,
[testing_guide.md](tools/testing/testing_guide.md) is the overall reference.

Run it with the servers already up:

```sh
cargo run --release --example headless-tester -p korangar-networking -- --scenario all
```

Coverage (123): session/lifecycle 8 · GM commands 9 · movement 5 · combat 3 · skills 44
(39 job-class sweeps + teleport/weapon-refine menus) · items 12 · dialogue 5 · social 16 ·
DM tooling 14 · observer parity 7.

Nine of the social sixteen guard the 2026-08-02 party work, and two of those exist
because the failure they catch is **silent**: `party-member-vitals` fails if the
Hercules `KORANGAR_PARTY_SP_TO_GROUPM` delta is lost in an upstream merge (the
server falls back to the narrow 0x080E form, `spell_points` becomes `None`, and
the party SP bar simply never appears again), and `party-sp-only-broadcast`
covers the `case SP_SP:` trigger on its own, since widening the packet without
the trigger leaves SP riding along on HP updates.

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
3b. **Server-source deltas (rebuild + re-apply after any upstream Hercules merge).** We keep a small number of patches in the sibling `Hercules/` tree; they are invisible from this repo, so check here first when server-sent data looks wrong. **Rebuild with `Hercules/dev.sh build`, not a bare `make`** — the map-server target is `map_sql`, and `make map` fails with *"No rule to make target"*, a message containing no "error" that slips through a grepped build log and leaves you testing a stale binary; `dev.sh build` checks the binary's mtime instead. Restart with `./dev.sh restart && ./dev.sh wait`.
   - `src/map/status.c`, `status_get_val_flag()` — added `SC_VOLCANO` / `SC_DELUGE` / `SC_VIOLENTGALE` (`val_flag |= 1 | 2`, 2026-07-24). Without it Hercules sends `val1 = 1, val2 = 0` for these, so the status window could only render "+0". All three share one icon (`SI_GROUNDMAGIC`), so their values are the only way to show what they grant. Requires `make -j8` and a server restart.
   - **`SC_LANDPROTECTOR`** (2026-07-24) — a fork-invented status so Land Protector tells the player their ground magic is suppressed, and for how long. Officially it grants nothing (it acts on the ground, not on people). Spans **five** places, and missing any one fails *silently*: `src/map/status.h` (`sc_type` enum slot), `db/constants.conf` (**both** `SC_LANDPROTECTOR: 728` matching the enum slot **and** `SI_LANDPROTECTOR: 1150`), `db/re/sc_config.conf` (icon, no `CalcFlags`), `db/re/skill_db.conf` (`StatusChange:` on `SA_LANDPROTECTOR`), and `src/map/skill.c` (`skill_unit_onplace` / `skill_unit_onout`). **The trap:** `sc_config.conf` and `skill_db.conf` resolve status names via `script->get_constant()`, so without the `SC_` constant both bindings are skipped with only a `ShowWarning` — and server stdout goes to **`log/server-latest.log`** (written by `dev.sh start`; older runs used an appended `log/athena-start.out`), not `log/map.log` (which stays empty). Our `onplace` also uses the group's *remaining* time rather than upstream's `sg->limit`, so re-entering a half-expired field shows the true countdown.
   - **`CZ_CANCEL_CAST` = packet `0x0F00`** (2026-07-26) — a fork-invented *client→server* packet letting the player abort their own cast with right-click / Escape. Official RO has no such packet (and forbids moving while casting), so nothing upstream could be reused; see the cast-cancel note above. Four places: `src/map/clif.c` (`clif_parse_CancelCast` → `unit->skillcastcancel(&sd->bl, 0)`, plus the `clif->pCancelCast =` registration), `src/map/clif.h` (interface member), `src/map/packets.h` (`packet(0x0f00,clif->pCancelCast,0)`), and the **length** entry. Three traps:
     1. **A client packet with no length entry makes `clif_parse` disconnect the session**, not warn — so a missing length looks like "right-click kicks me to login".
     2. The length therefore lives in **`src/common/packets_len.h`**, which is hand-maintained, *not* in `common/packets/packets<year>_len_*.h` — those are autogenerated and banner-marked "do not commit manual changes", so a regeneration would silently drop it.
     3. **`0x0F00` is exactly `MAX_PACKET_DB`.** Anything higher is rejected by both `packets_addLen`'s assert and `packetdb_addpacket`, and `packets->db[]` is sized `MAX_PACKET_DB + 1`. Do not "tidy" this to a rounder number like `0x0F01`.
   - **`LOOK_AMMO` broadcast** (2026-07-27 … 2026-07-29) — **five** touch points across three files, because official RO reports ammunition for nobody but yourself and the Korangar client picks its projectile sprite from the ammo item. Rides the unused `LOOK_FLOOR` slot (aliased by macro in `src/map/map.h`, deliberately **not** added to `enum look` — a new member would raise `LOOK_MAX`, which is `MAX_STYLIST_TYPE`). The touch points: `clif_changelook` (persist into `vd->ammo`), `clif_inventoryItems` (seed at login), `clif_getareachar_unit` (re-send on enter-view — **unconditionally, including zero**, or an observer who missed an unequip is never corrected), `pc_equipitem`/`pc_unequipitem` (broadcast on change), and `clif_parse_LoadEndAck` (**re-broadcast after `clif->spawn`** — the seed runs ~95 lines before `map->addblock`, so its AREA send reaches nobody). **The trap that cost the most:** `status_set_viewdata` memcpy's a whole `view_data` over `sd->vd` when a player is **disguised**, zeroing `vd->ammo`; un-disguising returns through the branch that assigns fields individually, which never re-assigns it, so it stayed zero forever. `src/map/status.c` now re-derives it there beside weapon/shield. **Any future fork field added to `view_data` inherits this problem.** Audits: [docs/plans/observer-parity-audits.md](docs/plans/observer-parity-audits.md).
   - **`ZC_PARTY_INVITE_SENDER` = packet `0x0EFF`** (2026-08-02) — a fork-invented
     *server→client* packet naming who sent a party invite. Official
     `ZC_PARTY_JOIN_REQ` carries only the party id and party name, so an official
     client can say "you are invited to join <party>" but never "<player> invites
     you". Sent from `clif_party_invite` **immediately before** the invite; the
     client pairs the two by party id and keeps the name keyed to that id, so a
     stale name from an earlier unanswered invite cannot be shown against a
     different party. Three places: `src/map/packets_struct.h` (struct + header),
     `src/map/clif.c` (`clif_party_invite`), and the **length** in
     `src/common/packets_len.h` — the hand-maintained file, never the generated
     `packets<year>_len_*.h`. **Deliberately a companion packet rather than
     widening 0x00FE:** the official packet keeps its official shape, so a stock
     client is unaffected and an upstream change to it cannot conflict, and the
     feature **degrades gracefully** — without 0x0EFF the invite still works and
     the popup falls back to naming only the party (client-side the name is an
     `Option`). `0x0EFF` sits above the highest official packet (`0x0BC0`) and
     below `MAX_PACKET_DB` (`0x0F00`, taken by `CZ_CANCEL_CAST`). Guarded by the
     `party-invite-sender` scenario, because losing it is silent — the invite
     keeps working and only the name quietly disappears.
   - **Party-member SP (`KORANGAR_PARTY_SP_TO_GROUPM`)** (2026-08-02) — the
     client draws HP **and SP** bars over party members, but official
     main-branch servers never report a party member's SP. Only the **Zero**
     branch got the wide 22-byte `ZC_NOTIFY_HP_TO_GROUPM` (**0x0bab**) carrying
     `sp`/`maxsp`; main sends the narrow 14-byte **0x080E**. The delta is one
     macro in `src/map/packets_struct.h` plus **three** guard sites in
     `src/map/clif.c` — and the third is the one that is easy to miss:
     1. the struct/header selection (`packets_struct.h`),
     2. `clif_party_hp` and `clif_hpmeter_single`, which *already* assign
        `sp`/`maxsp` under the same guard — no new code, just a wider condition,
     3. **`clif_updatestatus`'s `case SP_SP:` (`clif.c:3861`)**, which is
        what *triggers* `clif->party_hp` when SP changes. Without it the packet
        is only sent when **HP** moves, so the SP bar freezes between hits and
        looks like a client bug.
     **Why this is cheap and not a fork-invented packet:** 0x0bab is already
     `packetLen(0x0bab, 22)` in Hercules' own `packets2022_len_main.h` and in the
     client's generated `lengths_20220406.rs`, so no length entry is needed and a
     client without the handler consumes it through the known-length fallback
     instead of desyncing. **Do not touch the fourth guard site** (`clif.c:19648`,
     `ZC_BATTLEFIELD_NOTIFY_HP`) — that is a different struct which still has no
     `sp` fields, and widening it writes past the end. Client side: 0x0BAB is
     `PartyMemberVitalsPacket`, 0x080E stays registered for stock servers, and
     both feed `NetworkEvent::PartyMemberHealth { spell_points: Option<..> }`
     where `None` **keeps** any SP already known rather than blanking it.
     Requires `dev.sh build` and a server restart.
   - **`ip_rules` disabled** (2026-07-30) — `conf/common/socket.conf`. Hercules'
     anti-flood counts connections **per IP**, and on this box everything is
     127.0.0.1 — the headless suite *and* the servers. The suite opens three
     sockets per login (login/char/map), six for a paired scenario, and
     `connect_as` retries a failed login four times, so `ddos.count` (5 per
     3000 ms) is trivially exceeded; the flag then sticks for 10 minutes.
     **The damaging part is not the client being refused: the char-server
     connects to the login-server from 127.0.0.1 too**, so its own reconnect is
     refused, the log fills with `there is no char-server online`, every login
     fails, and the harness retrying drives it straight back in. It presents as
     **random mid-scenario disconnects with a perfectly clean packet ledger**,
     which is the tell — a connection-flavoured failure plus `0 failed` in the
     ledger is an environment problem, never a protocol one. One partial suite
     run: 38 DDoS warnings / 126 char-server refusals / 105 link losses / 16
     failures. After disabling: 0 / 0 / 0 and 113 passed. **Two traps:** the
     `allow_list` entry for 127.0.0.1 in `conf/import/socket.conf` has existed
     since 2026-07-11 and does **not** prevent it; and the DDoS *warning* is
     printed whether or not the flag is honoured, so comparing warning counts
     between a good and a bad run proves nothing — count
     `there is no char-server online` instead.
   - **Multi-word item names in `@item`** (2026-07-26) — `src/map/atcommand.c`.
     `@item Iron Arrow 500` silently gave **one Iron** (998): unquoted, the command
     scanned `%99s %12d`, took `Iron`, failed on `Arrow`, and still returned ≥ 1 so
     it reported success. Only the quoted form supported spaces. Now
     `atcommand_item_search` + `atcommand_item_parse` back all four of `@item`,
     `@itembound`, `@item2`, `@itembound2`. **The trap:** a trailing integer is not
     safely the quantity — **1797 items have a display name ending in a digit**
     (`Vesper Core 01`, `Vita500`), so the parser resolves *longest name first*,
     trying the whole string before peeling one trailing integer at a time. **The
     second trap, which longest-first creates:** the old ID lookup was
     `itemdb->exists(atoi(name))`, and `atoi("1770 500")` is `1770`, so the whole
     string resolved as an ID and `@item 1770 500` silently gave **one** item. An
     ID is now only accepted when the string is numeric end to end (`strtol` +
     endptr). Caught by a throwaway harness, not by the compiler. Display
     names already matched case-insensitively; Aegis names stay case-sensitive per
     `case_sensitive_aegisnames: true`. Requires `make -j8` and a server restart.
   - **`ZC_SKILL_FAIL_REASON` = packet `0x0EFE`** (2026-08-07) — a fork-invented
     *server→client* packet naming the **runtime** reason behind a cause-0 skill
     failure. `ZC_ACK_TOUSESKILL` carries a `useskill_fail_cause`, and Hercules
     sends `USESKILL_FAIL_LEVEL` (0) for a great many outcomes that have nothing
     to do with skill level, **because Gravity never numbered them**: 21 of the
     33 states in `skill_check_condition_castbegin`'s switch report 0 and only
     one has a dedicated cause. The official client says "Skill level is not high
     enough" to all of them, and so did we.
     **Know the split before touching this.** A *static* precondition (`State:`
     in `skill_db.conf` — needs a shield, a falcon, a stance) needs **no packet**:
     it is on disk at both ends, and `tools/generate_skill_states.py` carries it
     into `korangar-networking/src/packet_versions/skill_states.rs` (42 skills,
     13 states). This packet is for what only the server knows as it decides —
     did the roll miss, was anybody in range, is there enough experience.
     Five places: `src/map/packets_struct.h` (struct + header), `src/map/clif.h`
     (`enum skill_fail_reason` **and** the interface member), `src/map/clif.c`
     (`clif_skill_fail_reason` + its `clif->` registration), the **length** in
     `src/common/packets_len.h` — the hand-maintained file, never the generated
     `packets<year>_len_*.h` — and **11 call sites** in `skill.c` / `unit.c`,
     each a one-line swap onto `clif->skill_fail_reason`.
     `enum skill_fail_reason` is **deliberately separate from
     `useskill_fail_cause`**: that numbering is Gravity's, and values invented in
     it would collide with a future official cause. **Append only**, and keep it
     in step with `SkillFailReason` in `ragnarok-packets`.
     Sent immediately *before* the failure it explains and paired **by skill id**,
     the same shape as `ZC_PARTY_INVITE_SENDER`, so a stock client is unaffected
     and it **degrades gracefully** — the client keeps its older skill-id
     inference as the fallback. The reason is *taken* on use, so one left behind
     by a suppressed failure cannot explain a later, unrelated one; that is
     covered by `skill_fail_reason_0x0efe_explains_the_following_failure`.
     Requires `dev.sh build` and a server restart.
   - **Realigned message glosses** (2026-08-07) — `src/map/messages_main.h`,
     `messages_re.h`, `messages_zero.h`, **15 lines each, identical in all
     three**. These files are **banner-marked "This file is autogenerated,
     please do not commit manual changes"**, so this is the same trap as
     `packets<year>_len_*.h`: a regeneration drops the delta and **says nothing**.
     Each id in these files is documented with its Korean text and an English
     gloss, and two regions paired the wrong two together:
     **0x576–0x582**, thirteen ids whose English lagged its Korean by one slot
     (`MSG_USESKILL_FAIL_HOLYWATER` read "Unable to use the skill to exceed the
     number of Ancilla", `MSG_NO_CHATTING` read "This skill requires other skills
     to be used", and so on down to `MSG_FAILED_MOBILE_LOCKSERVER`, whose own
     gloss had fallen off the end and had to be written); and **0x745/0x746**,
     a straight swap. No server behaviour changes — nothing reads these comments
     — but `korangar/tools/generate_message_table.py` does, and 0x746 is
     `MSG_ITEM_REUSE_LIMIT_SECOND`, which **every item with a `Delay` in
     `item_db.conf` sends**: an Yggdrasil Berry used twice inside five seconds
     told the player "Content has been saved in [SaveData_ExMacro5]".
     **Guarded at both ends, because losing it is silent and the symptom is
     plausible text rather than no text:** the generator carries a
     `CORRECTED_UPSTREAM` sentinel table and **exits non-zero without writing**
     if the header ever reverts, and `msgstringtable.rs` pins the same ids in
     `item_reuse_delay_is_not_the_macro_save_message` /
     `shifted_gloss_run_stays_realigned`. No rebuild or restart needed.
     **How they were found, which generalises:** the Korean line carries ASCII
     anchors (`SaveData_ExMacro`, the digits in "3시간"/"5시간") that must
     reappear in its own gloss — if they land on a *neighbour's* gloss instead,
     the run is shifted, and no translation is required to see it.
   Type `0` is deliberate in the `skillcastcancel` call: a voluntary abort must not be blocked by the skill's `castcancel` flag or by Phen / `no_castcancel` (that is `type&2`, for damage interrupts), and the skill to drop is `ud->skill_id`, not `SA_CASTCANCEL`'s `skill_id_old` (`type&1`). SP is untouched because it is charged at cast *end*. Requires `make -j8` and a server restart.
4. **Rebaseability**: Keep custom UI features isolated in `korangar/src/interface/windows/dm/` (and state in `korangar/src/dm/`) as much as possible to ensure the fork remains rebaseable against upstream Korangar.
5. **No Upstream IP**: Per `wiki/Contributing.md`, do not include code taken directly from or inspired by GRAVITY's intellectual property.
