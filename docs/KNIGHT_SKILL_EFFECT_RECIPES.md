# Knight classic skill-effect recipes

This matrix records the six skills provisioned on `EffectKnight` F1–F6. It is
an implementation/acceptance reference for Korangar, not copied upstream
client code. Asset names were verified against the configured official RO
GRFs; layering and trigger semantics were cross-checked against
roBrowserLegacy's independent skill/effect tables.

| Key | Skill (ID) | Actor / weapon action | Caster layer | Target / travel layer | Audio / camera |
|---|---|---|---|---|---|
| F1 | Magnum Break (7) | Normal attack with equipped weapon | Procedural `ring_yellow.tga` + `대폭발.tga` expanding cylinders; orange point light | Caster-centered area recipe, including empty hits | `effect\\ef_magnumbreak.wav`; 50 ms camera quake |
| F2 | Pierce (56) | Normal spear attack; Knight spear table value 2 maps to `Attack3` | `pierce.str` at body height; warm point light | `earthhit.str` on the struck entity | Weapon ACT event / spear hit sound |
| F3 | Brandish Spear (57) | Normal spear attack | Attached `brandish2.str`; warm point light | Detached `brandish.str` at the damage target | `effect\\knight_brandish_spear.wav` |
| F4 | Spear Stab (58) | Normal spear attack | `spearstab.str` at body height; warm point light | Damage packet supplies the target and timing | `_enemy_hit_normal1.wav` |
| F5 | Spear Boomerang (59) | Throwing `Attack1`, intentionally without the held-weapon layer | `spearboomerang.str` at head height; warm point light | Official `이팩트\\창.spr` travels source-to-target over 140 ms | `effect\\knight_spear_boomerang.wav` |
| F6 | Bowling Bash (62) | Normal attack with equipped weapon | `bowling.str` at head height; warm point light | Eight alternating `lens1.tga` / `lens2.tga` radial hit streaks | `_enemy_hit_normal1.wav` at caster; `effect\\ef_hit2.wav` at target |

## Trigger and deduplication rules

- Magnum Break is emitted from successful skill use so its caster-centered
  recipe still plays when no target takes damage.
- The five targeted skills use the damage event because it carries source,
  target, skill id, and actor-action duration.
- Area/multi-hit damage can report once per target. A short source/skill gate
  emits the caster layer once while retaining per-target hit layers.
- Equipped weapon and shield appearances are promoted for both initial entity
  creation and later appearance updates. The local player's equipped inventory
  is also authoritative when the server's local character snapshot omits the
  appearance value.

## Out-of-range casting acceptance

For an entity-targeted skill, a click outside `attack_range` must not be sent as
an immediate cast and silently fail. Korangar now paths to the closest walkable
tile within the learned range, retains the skill id, level, range, and entity,
then casts when movement stops. It recalculates if the target moved and drops
the buffer if the target disappears.

## Asset acceptance

`loads_classic_knight_spear_layer` is ignored in the fast suite because it
opens the multi-gigabyte configured archives. It verifies the Knight male spear
SPR/ACT plus every STR, texture, projectile sprite, and WAV referenced above.
