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

## Fast re-check commands (no GUI)

```bash
cd korangar
cargo test -p korangar --lib
cargo test -p korangar --lib loads_classic_weapon_layers_for_roster -- --ignored
# from korangar/korangar/ with GRFs present:
cargo run --release --bin weapon-sprite-audit
```
