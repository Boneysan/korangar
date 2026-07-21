# NEXT — Phase E1 live client verification

| | |
|---|---|
| **Status** | **CODE CLOSED 2026-07-22 — live GUI pending** |
| **Date code closed** | 2026-07-22 |
| **Branch** | `agent/platform-connectivity-controls` |
| **Parent plan** | [animation-fidelity.md](animation-fidelity.md) §6 Phase E1 |
| **Reference** | [ANIMATION_SYSTEM.md](../ANIMATION_SYSTEM.md) §6–7 |

## Do this first

Phase E1 **code is closed**. Do **not** start E2 until this live checklist is
walked and results are written back into `animation-fidelity.md` §6 E1.

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
| Visual fidelity | Deferred — better textures / native geometry in later E passes; not a wiring bug |

## Live checklist (in-game)

Use a Mage or Wizard (GM `@job 2` / `@job 9` + `@allskill`, or Effect roster).
Field map with a durable dummy (`@monster 1905` Barricade). Watch for **visible
geometry**, not only a point light or sound.

| # | Skill | Action | Expect | Pass? |
|---|-------|--------|--------|-------|
| 1 | Napalm Beat (11) | Cast on target | Violet expanding rings at target + sound | |
| 2 | Soul Strike (13) | Cast multi-level | Purple orbs fly caster→target (one per hit), then hit STR | |
| 3 | Frost Diver (15) | Cast on target | Ice projectile travels, freeze hit on impact | |
| 4 | Fire Ball (17) | Cast on target | Fire travel ball, then firehit | |
| 5 | Jupitel Thunder (84) | Wizard cast | Yellow travel ball, lightning/wind hits | |
| 6 | Earth Spike (90) | Wizard cast | Single rising ground spike under target | |
| 7 | Heaven's Drive (91) | Wizard cast | Ring of rising spikes around target | |

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

## After live pass

1. Fill the table above with date + observer.
2. Flip E1 exit to **live-met** in `animation-fidelity.md` §6.
3. Optional: commit doc-only “Phase E1 live-verified”.
4. **Next:** Phase E2 (persistent skill units) per animation-fidelity §6.
