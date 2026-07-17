# 2026-07-17 — M1-008 round 2: ground-cast effects, bolt volleys, portal tuning

Follow-up to the 2026-07-16 STR renderer fix, driven by live feedback:
Thunderstorm showed nothing, Fire/Cold Bolt lacked their falling projectiles,
the warp vortex read as too small. (Mob emote chat silence was confirmed
working.)

## Thunderstorm root cause: `ZC_NOTIFY_GROUNDSKILL` was noop

The original client plays ground-cast area effects (Thunderstorm, Storm
Gust) from `ZC_NOTIFY_GROUNDSKILL` (0x0117) at the targeted position —
independent of damage. Hercules sends it for every ground cast
(`skill_castend_pos2` default branch → `clif->skill_poseffect`), verified in
source and live: a new **temporary** headless scenario `probe-thunderstorm`
(skills.rs; runs on its own GM account `probe`/`probe`, created directly in
the `login` table, so it does not kick a GUI session) showed the packet
arriving with the cast position. Korangar had it registered as noop, so the
storm never played; per-target damage still arrived (skill 21, `div` 10).

Wiring now matches the original (cross-checked against roBrowserLegacy's
skill/effect tables, semantics only):

- New `NetworkEvent::GroundSkillEffect { skill_id, source_entity_id, level,
  position }`; lib.rs plays `thunderstorm.str` (21) / `stormgust.str` (89)
  at `map.get_world_position(position)`.
- `NetworkEvent::DamageEffect` gained `hit_count` (`div`, ≥1).
- Per-hit table `wizard_hit_effects`: Soul Strike → soulexpansion stand-in;
  Fire Bolt → random `firehit1-3.str` **delayed 0.5 s** to meet the volley;
  Lightning Bolt (20) → `lightning.str` + random `windhit1-3.str`;
  Thunderstorm → random `windhit1-3.str` per target. Cold Bolt's classic hit
  is sound-only (no STR). Load failures now `eprintln` (`[skill-effect]`)
  instead of failing silently.

## Classic falling-bolt volleys (`world/effect/bolts.rs`)

`FallingBolts` (EffectBase): `hit_count` projectiles staggered 0.15 s, each
falling 0.5 s from ~22 units above (slight randomized sideways offset) onto
the target — the classic `ef_firebolt`/`ef_coldbolt` code-drawn effect.
Fire Bolt animates `effect\불화살1-6.tga` at 30 ms/frame; Cold Bolt uses
`effect\icearrow.tga`. Sprites rotate to their on-screen direction of
travel; additive blend. `EffectWithLight` gained a `start_delay` parameter
(hit bursts waiting for the volley).

## Also

- Portal vortex enlarged (outer r6→5 h16, inner r3.6→3 h24, alpha up) after
  "little warp thing" feedback; awaiting another look.
- Sounds (`ef_firearrow%d.wav` etc.) are still not played — the classic
  tables name them; future work.
- The probe scenario + `probe` account are temporary diagnostics; remove
  before the next acceptance gate (scenario count parity: suite is 106).

All `cargo test -p korangar --lib` green (80); all mapped STRs parse with 0
unconsumed bytes; release rebuilt and relaunched for live verification.

## Live verification + future work

**Live-verified by the user 2026-07-17: Thunderstorm, Fire Bolt, and Cold
Bolt all render.** (Storm Gust and the emote silence were confirmed in the
previous round; the warp vortex was visible and has since been enlarged.)

Per the user's direction, a **classic skill-effect coverage pass** is now a
scoped High row in `FEATURE_ROADMAP.md`: audit the whole skill catalog for
the same three wiring-gap classes the Wizard kit hit — unmapped ground-cast
STRs (`ground_skill_effect` has only 21/89), missing per-hit STRs
(`wizard_hit_effects` has only 13/19/20/21), and code-drawn effects that
were never STR files (`ef_*` recipes in roBrowserLegacy). Plus skill-unit
visuals beyond Firewall/Pneuma, `DisplaySpecialEffectPacket`, cast circles,
and sounds. The roadmap row records the proven method (wire probe → STR
dump → reference tables → live check).
