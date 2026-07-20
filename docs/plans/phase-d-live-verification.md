# NEXT — Phase D live client verification

| | |
|---|---|
| **Status** | **OPEN — first task for the next agent (Claude / Codex / Grok)** |
| **Date code closed** | 2026-07-18 |
| **Branch** | `agent/platform-connectivity-controls` |
| **Parent plan** | [animation-fidelity.md](animation-fidelity.md) §5 Phase D |
| **Reference** | [ANIMATION_SYSTEM.md](../ANIMATION_SYSTEM.md) §1, §6, §7 |

## Do this first

Phase D **code is closed**. Do **not** start Phase E or more engine work until
this live checklist is walked and the results are written back into
`animation-fidelity.md` §5 (date + observer + pass/fail per row).

If something fails live, fix the bug, re-verify, then close D. Only after D is
live-green should work move to Phase E (skill/status recipes) or Phase F.

## What shipped (code)

- Item ID appearance (Hercules LOOK_WEAPON / LOOK_SHIELD nameids) via
  `equipped_weapon_look` / `equipped_left_hand_look`
- `weapon_view_from_item_id` / `effective_weapon_view` / dual-wield 25..=30
- Per-item SPR probe → dual class → single class → none
- `_검광` trails gated to native class views (1–7, 16–18, 25–30); per-item
  bases still probe `{base}_검광`
- Multi weapon-family layer swap (base + trail + off-hand)
- Unit tests green; GRF path probe includes Mjolnir `기사_*_1530` + trail
- Native RE: Ragexe `0x7C4F90` / `0x7C4B30`, trail table `0x976EC0`
  (`../../RO/client/2019-06-05fRagexe_patched.exe`)

## Live checklist (in-game)

Rebuild/relaunch Korangar against the local Hercules + effect roster (or any
GM-capable character). Record each row.

| # | Character / setup | Action | Expect | Pass? |
|---|-------------------|--------|--------|-------|
| 1 | EffectKnight (or Knight), **sword** | Idle, walk, auto-attack | Class sword sprite; **`_검광` trail** on attack if SPR exists; one attack sound | |
| 2 | Knight, **spear** | Auto-attack / Pierce | Spear class sprite holds through swing; trail if native view allows (spear = view 4 → yes) | |
| 3 | Knight, `@item 1530` **Mjolnir** equip | Idle + attack | **Per-item** path `기사_{sex}_1530` (not generic club only); trail only if `…_1530_검광` exists (class mace view 8 has **no** class trail) | |
| 4 | Knight, sword + **Guard** (2101) | Face camera / face away | Sword + shield; shield **behind** body when facing away (Phase C regression) | |
| 5 | EffectSinX, **katar** / Jur | Dual-wield attack | Katar pair layer; Attack3; sound once per swing | |
| 6 | SinX, **two daggers** (left+right) | Equip both, attack | Dual view 25 (`단검_단검`) or two dagger layers; Attack3 for Assassin family | |
| 7 | Any, **bow** or **mace** class only | Attack | Weapon shows; **no** class `_검광` (native skip) | |
| 8 | Unequip / re-equip weapon | Swap | No full body reload jank; head stays put (C4 regression) | |

### Commands / notes

```text
@item 1530          # Mjolnir (per-item probe)
@item 2101          # Guard
# Dual daggers: equip two dagger-class items on SinX (right + left hand)
```

- Path probes use `file_exists` (lowercase). Archive listing under-reports.
- If Mjolnir shows as generic club, inventory/appearance is not feeding item
  nameid into `common.weapon` — check `equipped_weapon_look` on SetInventory /
  equip events.
- If dual daggers still look like one dagger and Attack2, `effective_weapon_view`
  is not seeing left-hand item on `common.shield`.

## After live pass

1. Fill the table above in this file or paste results into
   `animation-fidelity.md` §5 with **date + observer**.
2. Flip Phase D exit to **live-met** in that plan.
3. Clear the “Phase D live GUI” bullet in `ANIMATION_SYSTEM.md` §7.
4. Optional: commit doc-only “Phase D live-verified”.
5. **Next engine/data work:** Phase E (skill/status batches) per
   `animation-fidelity.md` §6 — independent of D live except for campaign
   visibility priority.

## Live pass results — IN PROGRESS (resumed 2026-07-21)

Observer: Claude + user (bigzome). macOS Metal build. Rows 1–3 done 2026-07-20 on
EffectKnight (prt_fild08). Rows 4 & 8 done 2026-07-21 on EffectKnight; rows 5–6
done 2026-07-21 on EffectSinX (prt_fild07). Target: Barricade (`@monster 1905`,
600k HP) as a stationary punching bag; zoom in; burst-capture (`screencapture -x`
every ~0.1s) mid-swing. **Only Row 7 (bow/mace, no class trail) remains** — paused
2026-07-21 before it (plan: EffectStalker + a bow, view 11 → expect no class
`_검광` trail). Driving via `cliclick` + `screencapture` bursts; user handles GM
commands / equipping.

**Row 4 geometry note:** a tall Barricade due-south occludes a Knight standing
north of it; stand diagonally (upper-left, Barricade to the SE) so the Knight
faces down-toward-camera and stays visible for the shield-in-front check.

**Method note (important):** equipped weapon/shield sprites only draw during the
**attack** motion (see BLOCKER below), so every sprite check must be captured
mid-swing, not idle.

| # | Row | Result | Notes |
|---|-----|--------|-------|
| 1 | Knight + sword | **PASS** | Class sword sprite held through swing; `_검광` trail is a prominent pale fan on the overhead slash; one sound/swing. |
| 2 | Knight + spear | **PASS (nuance)** | Spear sprite holds through the poke; one sound/swing. Trail: **Lance (1410)** correctly shows none — it has a per-item base `기사_남_1410` but no per-item trail `1530`-style companion, so the resolver picks the per-item base and finds no `1410_검광` (faithful to native). **Plain Spear (1404)** has no per-item base → falls to class base `창` → class trail `창_검광` **is attached** (verified in code + file_exists probe); visually subtle on a poke vs the sword's slash fan. Not a regression. |
| 3 | Knight + Mjolnir (1530) | **PASS** | Per-item hammer sprite used (resolver prefers per-item base, same path as Lance); item confirmed live by Mjolnir's Lord-of-Vermilion/chain-lightning procs (also proves the nameid reaches `common.weapon`, the row's main worry). Per-item trail `1530_검광` exists and is attached (`is_per_item` always probes it); visually swamped by the lightning procs. One weapon-hit sound/swing. |
| 4 | Knight sword + Guard | **PASS** (2026-07-21) | EffectKnight, sword (right) + Guard shield (left). **Facing-away** attack: shield correctly behind the back (2026-07-20). **Facing-toward** attack (2026-07-21): stood upper-left of the Barricade so the Knight faces down-toward-camera and isn't occluded — shield clearly drawn **in front of the body** on the left arm. Shield z-order correct both directions ⇒ Phase C4 concern resolved. (Tall-Barricade occludes a Knight standing due-north of it; diagonal positioning avoids that.) |
| 5 | SinX katar/Jur | **PASS** (2026-07-21) | EffectSinX with **Jur** (katar, item 1250 — two-handed, fills both hand slots in the Equipment window). Mid-swing capture: katar blade drawn forward, crouched Assassin lunge (Attack3 family), prominent white `_검광` trail arc (katar = view 16, in native trail set 16–18 → correct). User confirmed **one attack sound per swing**. Katar auto-attacked continuously (single-hit `Damage`/`CriticalHit` type → drives the loop). |
| 6 | SinX two daggers | **PASS** (2026-07-21) | Two Main Gauche (1207) dual-wielded — **Equipment window confirms both Left hand + Right hand = daggers**. Mid-swing capture: two dagger sprites + white `_검광` trail arc (dual view 25 `단검_단검`, in native trail set 25–30 → correct), Attack3 Assassin lunge, damage number displays. **NOTE:** exposing this row uncovered a real dual-wield attack bug (see below) — fixed 2026-07-21; auto-attack then verified to continue. |
| 7 | bow/mace (no class trail) | not started | |
| 8 | unequip/re-equip | **PASS** (2026-07-21) | EffectKnight, weapon swaps via the inventory right-click menu (Equip/Unequip). User confirmed real-time **no head jump, no body flash/reload**; burst frames show the head/body pinned to the same position across the swap. C4 regression not present. Caveat: the equipped weapon isn't drawn while idle (idle-gear blocker below), so the *weapon* swap is only fully visible mid-attack — the head/body reload check (the C4 concern) is unaffected by that. |

### BUG FOUND + FIXED 2026-07-21 — dual-wield normal attack "swings once then stops"

Discovered while setting up Row 6: with two daggers equipped, a normal attack
swung exactly once and would not auto-repeat (the katar auto-attacked fine).

**Root cause.** Continuous ("auto") attack is re-driven by the player's own damage
ack: the `DamageEffect` network event (`korangar/src/lib.rs:~2888`) re-issues
`player_attack`. A dual-wield / Double-Attack normal hit is sent by Hercules as
`DamageType::MultiHitDamage` (=8), carrying a second damage value
(`damage_amount_2`, commented *"Assassin dual wield damage"* on both damage
packets). The handlers in
`korangar-networking/src/packet_versions/version_20220406.rs` only mapped
`Damage` and `CriticalHit` → `DamageEffect`; every other type, including
`MultiHitDamage`, fell through to `_ => None`. No `DamageEffect` ⇒ no re-trigger
(swing once, stop) **and** the dual-wield hit's damage number never rendered.

**Fix.** Added `MultiHitDamage | MultiHitDamageEndure` and `CriticalMultiHit`
arms to **both** `DamagePacket1` (0x008A) and `DamagePacket3` (0x08C8) handlers —
emit a `DamageEffect` with `hit_count = number_of_hits.max(2)`, damage
`damage_amount + damage_amount_2`, `is_critical` set for the critical variant.
Regression test `dual_wield_multi_hit_damage_0x08c8_surfaces_damage_effect` in
`korangar-networking/src/lib.rs`. **Live-verified 2026-07-21**: dual-wield
auto-attack now continues; damage numbers display. **Committed + pushed 2026-07-21
on `agent/platform-connectivity-controls`** (this fix + doc only; the Phase D
diagnostic scaffolding below stayed out of the commit).

### BLOCKER — equipped weapon & shield not rendered on the idle player

**Symptom:** a standing/idle player shows an empty-handed body; the equipped sword
AND shield appear only during the attack motion. Authentic RO draws equipped
weapon/shield at all times. Confirmed via zoomed capture with sword + Guard equipped.

**Ruled out (code + `animation-audit`):**
- `compose_action_motion` (`world/animation/mod.rs:1248`) composes **all** gear
  layers for **every** action; no attack-gate; idle handled explicitly (body
  motion 0, ~line 1271).
- `decode_animation_layer` (`loaders/animation/mod.rs:124`) builds **all** ACT
  actions for a layer.
- `animation-audit`: the Knight sword's idle action **has** frames
  (`검 group 0` = `[3,3,3,3,3,8,8,8]`, none zero) — data not empty.

**Two suspects to check next (30-min runtime diagnostic: debug-print the local
player's `animation_data.layers` — count / `path_key` / current-action frame
count — while idle):**
1. The weapon/shield layer is **not present** in the *local* player's live
   `animation_data` at idle — equip path is
   `UpdateEquippedPosition → refresh_entity_player_gear → apply_weapon_layers_swap`
   (`this_entity` historically fragile; cf. `ca0cedd3`).
2. **Body↔weapon action-structure MISMATCH** — `animation-audit` flags the weapon
   layer's per-action frame counts not aligning with the body by index; the
   `body_action_index % layer.animations.len()` mapping in `compose_action_motion`
   could misresolve the idle frame. (This is a broader lead, may affect more than idle.)

Fix deferred by decision 2026-07-20 (finish the checklist first). Does **not**
require re-deriving the original client — the draw model is already RE'd in Phase C/D.

**Diagnostic scaffolding added (revert or promote before commit):**
`korangar/src/lib.rs::probe_weapon_parts` (+16 lines) and
`korangar/examples/trail_probe.rs` — file_exists probes used to prove the trail
SPR findings above.

## Fast re-check commands (no GUI)

```bash
cd korangar
cargo test -p korangar --lib
cargo test -p korangar --lib loads_classic_weapon_layers_for_roster -- --ignored
# from korangar/korangar/ with GRFs present:
cargo run --release --bin weapon-sprite-audit
cargo run --release --bin animation-audit   # body↔layer action-structure mismatches
```
