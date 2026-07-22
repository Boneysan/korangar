# NEXT — Phase E1 live client verification

| | |
|---|---|
| **Status** | **LIVE-VERIFIED 2026-07-22 — all 7 rows PASS on mechanism** |
| **Date code closed** | 2026-07-22 |
| **Date live-verified** | 2026-07-22 (Claude-driven, `cliclick` + `screencapture`) |
| **Branch** | `agent/platform-connectivity-controls` |
| **Parent plan** | [animation-fidelity.md](animation-fidelity.md) §6 Phase E1 |
| **Reference** | [ANIMATION_SYSTEM.md](../ANIMATION_SYSTEM.md) §6–7 |

## Do this first

Phase E1 **code is closed**. Do **not** start E2 until this live checklist is
walked and results are written back into `animation-fidelity.md` §6 E1.

> **Scope note added 2026-07-22 — what this pass can and cannot tell you.**
> Every visual below is a **procedural stand-in**, and we now know those are
> *not* what the original client draws. The classic single-target spells are
> sprite animations with no `.str` equivalent in the GRFs, so no amount of
> texture tuning makes these authentic — see
> [classic-effect-fidelity.md](classic-effect-fidelity.md).
>
> This checklist therefore verifies **mechanism**: that the effect spawns at
> the right place, at the right time relative to the damage number, with its
> texture actually loading. It does **not** verify fidelity. A row can pass
> here and still look wrong, and that is expected — fidelity is tracked
> separately and needs the skill→sprite mapping.

## What shipped (code)

- `TravelBall` projectile recipe (Fire Ball / Frost Diver / Jupitel)
- `SoulStrikeOrbs` multi-orb travel (orb count = packet `hit_count`)
- `SkillBurstStyle::{NapalmBeat, EarthSpike, HeavensDrive}` target geometry
- Recipes for skill IDs 11, 13, 15, 17, 84, 90, 91 in `skill_recipe.rs`
- Unified projectile spawn in `spawn_damage_caster_skill_effect`
- Unit tests + `all_mapped_skill_effect_assets_exist` green for E1 textures

### Mitigations (post double-check)

| Issue | Mitigation |
|---|---|
| Travel vs hit-STR timing | Travel duration / Soul Strike orb packing uses `impact_delay_ms` so arrival ≈ impact due-tick |
| Multi-packet travel (Jupitel) | `UniqueEffectSlot::TravelProjectile` once-per-cast (~0.22 s); FallingBolts stay per-packet |
| AOE target spam (HD / Napalm) | `UniqueEffectSlot::TargetAoe` — one ring per cast; per-target `earthhit` / hit sounds still fire |
| Visual fidelity | **Improved 2026-07-22** via GRF probe (`probes_e1_procedural_effect_textures`): see texture table below |

### GRF texture upgrades (probe-backed)

| Effect | Was (weak stand-in) | Now (confirmed in GRF) |
|---|---|---|
| Fire Ball travel | `불화살1.tga` (bolt frame) | `fire_blast.bmp` |
| Frost Diver travel | `icearrow.tga` | `ice.tga` |
| Jupitel travel | `lens1.tga` | `번개4.bmp` |
| Soul Strike orbs | `purpleslash.tga` | `pok1.tga` |
| Napalm Beat | `purpleslash.tga` | `ring_blue.tga` |
| Earth Spike / Heaven's Drive | `lens1`/`lens2` | `bd_stonecurse.tga` + `crystallization\cry_stone_01.tga` |

Re-probe: `cargo test -p korangar --lib probes_e1_procedural_effect_textures -- --ignored --nocapture`

## Test character — ALREADY SET UP 2026-07-22

**`test` (char_id 150000) is ready; no `@job`/`@allskill` needed.** It was
promoted Mage → **Wizard (class 9)** and all seven E1 skills are bound to
**F1–F7 in checklist order**:

| Key | F1 | F2 | F3 | F4 | F5 | F6 | F7 |
|---|---|---|---|---|---|---|---|
| Skill | Napalm Beat 11 | Soul Strike 13 | Frost Diver 15 | Fire Ball 17 | Jupitel 84 | Earth Spike 90 | Heaven's Drive 91 |
| Lv | 10 | 10 | 10 | 10 | 10 | 5 | 5 |

Earth Spike and Heaven's Drive cap at 5 — that is their real `MaxLevel`, not a
shortfall. Every slot has a matching learned level (no dead bindings).

**Changed from its original state**, should you want to revert:
`class 2 → 9`, `int 1 → 99`, `dex 1 → 99`, and skills 84/90/91 granted.
INT/DEX were raised because at DEX 1 the cast times made the Wizard spells
effectively untestable. Base/job level untouched (50 / 1).

The hotbar is **server-side** — `Hotbar::clear()` repopulates from the server's
hotkey packet on map login, so Hercules' `hotkey` table is the source of truth,
not any client file. Edit it with the character **offline** or the map server
will overwrite on save. Prior stale rows pointed at skills the character did not
have (83/88/89, left from an earlier Wizard stint) and were cleared.

Still needed at the keyboard: `@warp prt_fild07` and `@monster 1905`.
**macOS trap:** F-keys need `fn` held unless the system setting is flipped.

## Live checklist (in-game)

Field map with a durable dummy (`@monster 1905` Barricade). Watch for **visible
geometry**, not only a point light or sound.

| # | Skill | Action | Expect | Pass? |
|---|-------|--------|--------|-------|
| 1 | Napalm Beat (11) | Cast on target | Violet expanding rings at target + sound | ✅ 2026-07-22 — violet ring geometry at target, dmg 185 |
| 2 | Soul Strike (13) | Cast multi-level | Purple orbs fly caster→target (one per hit), then hit STR | ✅ 2026-07-22 — orbs visibly travel caster→target, then dmg 480. Orb-count-vs-`hit_count` not separable from stills |
| 3 | Frost Diver (15) | Cast on target | Ice projectile travels, freeze hit on impact | ✅ 2026-07-22 — ice crystal in flight, icy freeze burst at impact, dmg 318 |
| 4 | Fire Ball (17) | Cast on target | Fire travel ball, then firehit | ✅ 2026-07-22 — ball tracked caster→target over 3 frames, fire burst at target |
| 5 | Jupitel Thunder (84) | Wizard cast | Yellow travel ball, lightning/wind hits | ✅ 2026-07-22 — bolt launches at cast end, blue lightning burst at target, dmg 1,644 |
| 6 | Earth Spike (90) | Wizard cast | Single rising ground spike under target | ✅ 2026-07-22 — single spike at target cell, dmg 1,590 |
| 7 | Heaven's Drive (91) | Wizard cast | Ring of rising spikes around target | ✅ 2026-07-22 — cluster of spikes around target cell (visibly wider than row 6), dmg 910 |

### Commands / notes

```text
@job 2          # Mage (or 9 Wizard for 84/90/91)
@allskill
@monster 1905   # Barricade
```

- If only a point light shows: texture load failed — check client log for
  `[skill-effect] failed to load…`.
- Soul Strike orb count should scale with skill level / packet `div`.
- Fire Bolt / Cold Bolt falling volleys are **prior** work; no need to re-check
  unless a regression is suspected.

## Live pass results (2026-07-22)

All 7 rows pass **on mechanism**: each effect spawned at the target, in step with
the damage number, with its texture loading (no `[skill-effect] failed to load`
lines in the client log for the whole session). Fidelity remains out of scope per
the scope note above.

### Traps hit while driving this pass — read before the next GUI session

1. **`@monster 1905` Barricade is the right dummy after all.** It takes magic
   normally (Def 0 / Mdef 0 / 600 500 HP, `db/re/mob_db.conf`) and, being
   immobile, is the only reliably *hoverable* target. Wandering spawns (Poring
   1002) move out from under the cursor between the screenshot that locates them
   and the keypress that casts. Green Plant 1080 is immobile but its sprite is
   too small for the entity picker to hit.
2. **Synthetic F-keys do not reach the client through AppleScript.**
   `osascript … key code 122` is swallowed by macOS (F-keys default to
   media/brightness; `com.apple.keyboard.fnState` is unset). Those presses
   produce *no* effect at all — not even an arm — which reads exactly like "the
   hotbar is broken". Non-F keys (Home, Escape) do get through, which is what
   makes the failure confusing.
   Working method: post the key **straight to the client's pid** with
   `CGEvent.postToPid` — a ~25-line Swift helper (`fnkey <pid> <keycode>`,
   `CGEventFlags.maskSecondaryFn`). Mouse moves/clicks via `cliclick` are fine;
   so is `osascript key code 36` for Return **in the chat input**, but `cliclick`
   `kp:return`/`kp:enter` is not.
3. **Confirm a cast actually fired by reading chat, not by watching the sprite.**
   `Aiming <skill> — click a target` means the cursor was *not* over an entity
   and the skill only armed (M1-006). Several early "no effect" readings were
   really "no cast".
4. **Capture window must be shifted for slow casts.** Jupitel Thunder's cast
   outlasts a 18-frame `screencapture` burst (~250 ms/frame); the projectile only
   launches at the very end. Sleep ~1.9 s after the keypress, *then* burst.
5. Ground-target rows (6, 7) always arm — press the F-key, then click to place
   (per M1-006), so they need the two-step drive, not the hover fast-path.

### Follow-up found during this pass (not an E1 blocker)

**"Skill level is not high enough."** appeared in chat on an Earth Spike cast.
The skill tree shows Earth Spike / Heaven's Drive learned at **5/5** (their real
`MaxLevel`), but every hotbar slot renders **10**, and the cast path sends
`learnable_skill.maximum_level`. Both rows still landed on retry, so this is
intermittent rather than a hard block — worth a look when Phase E2 starts.

## After live pass

1. ✅ Table filled — 2026-07-22, observer Claude (scripted `cliclick` +
   `screencapture`, user-supervised session).
2. ✅ E1 exit flipped to live-met in `animation-fidelity.md` §6 (status line,
   `NEXT AGENT` field, and the E1 section all updated).
3. ⬜ **Not committed.** `docs/plans/phase-e1-live-verification.md`,
   `M1-p0-verification.md`, `animation-fidelity.md` and `docs/plans/README.md`
   are modified in the working tree on `agent/platform-connectivity-controls`
   (clean at `b6ceb25a` before this pass). Commit doc-only when ready.
4. **Next — recommended order:**
   1. **Skill→sprite mapping** ([classic-effect-fidelity.md](classic-effect-fidelity.md))
      *before* E2. E1 passes on mechanism but still doesn't look like RO, and no
      skill is mapped to a sprite yet, so in-game visuals are unchanged. Prove
      end-to-end on Soul Strike → `이팩트\soule` first (anchor handling is the
      likely first bug). Extraction is blocked on GRF decryption — drive
      `GameFileLoader::get()` from a small Rust bin, not the Python script.
   2. **M1-017 logout crash** — **CLOSED live 2026-07-22.** Fixed by closing windows
      on `LoggedOut` before `skill_tree().clear()`, plus optional skill-points / tab
      skills paths. Live: Skill Tree open → Log out → character select.
   3. **Phase E2** (persistent skill units) per animation-fidelity §6 — unblocked.
   4. Leftovers for the next GUI session: M1-009's "vs equipped" delta (the
      equip gesture was never worked out — double-click does not equip), the
      arrow-projectile eyeball (needs a bow, so not `test`), and the
      "Skill level is not high enough" hotbar-level anomaly noted above.
