# Tactical monster pilot — GDD §6.10 / server slices S5–S9

**Parent:** [GDD §6.10](../GDD.md#610-implementation-path-added-v02) and [next-slices plan](../plans/gdd-next-slices.md). The pilot tests readable, varied PvE on three tiers before broad rollout. It is separate from DM Session encounters.

## Profiles and implementation boundary

Keep reviewed profiles keyed by map and monster in `Hercules/db/re/mob_ai_profile_db.conf` (or a future layered data path) with role parameters and an obvious rollback. Use `mob_skill_db.conf` for skills only when those skills are appropriate everywhere that monster appears; never add a globally-scoped skill/mode to implement a map-only role. Data-only roles come first: Aggressor (`MD_AGGRESSIVE`/target-change plus an optional cast attack), Support (`MST_FRIEND`/friend-HP trigger), Controller (status/ground skill), Coward (`NPC_RUN` at low HP), and Opportunist (target-weak/cast reaction). Protector means assist/defend behavior in this pilot; true interposition is a later C design. Do not apply global `monster_ai` bits to improve one family if they also change unrelated mobs.

**Current source pilot (2026-09-24):** `Hercules/db/re/mob_ai_profile_db.conf` is an optional map+monster keyed table loaded at map-server startup by `mob.c`. On `prt_fild08`, Fabre gets the Aggressor mode and Poring uses NPC_RUN at ≤35% HP (4 cells, 8-second retry cooldown). On `orcsdun02`, Orc Archer has an opt-in RangedKeeper profile: when its player target closes inside 5 cells, it may take one legal adjacent step outward, with a 1.5-second cooldown even when no suitable step/path can be started. This profile also opts into local hazard-weighted A*: active Fire Wall, Fire Pillar, Meteor, Lord of Vermilion, Storm Gust, Quagmire, Hunter traps, and NPC ground/fire attack cells add path cost. Hazard checks are cached within a 17×17 region around the path origin; beyond that region route cost remains stock. On `orcsdun01`, Orc Skeleton has an opt-in Skirmisher profile: after three positive player hits, it tries one legal outward step within a three-cell band and consumes a three-second cooldown even if no route can be started. Hit counters reset on spawn/revive and cap at the configured threshold. Runtime behavior is keyed to map and class, leaving shared monster records and copies on other maps unchanged. Removing the file and restarting map-server restores stock behavior. This is source implementation only: all live combat/farming/path-load acceptance is still required; the broader Orc family composition and Late profiles are not yet configured.

The client cast telegraph slice must be live before a new area attack ships. Eligibility: an actual server cast with a footprint larger than one cell, real target position, and finite cast time. Visual duration ends on execution/cancel. A warning without an actionable movement window is retuned or removed. Keep most ordinary monsters simple so farming does not become a boss fight.

## Pilot sequence

| Tier | Maps and proposed profiles | Observation gate |
|---|---|---|
| Early | `prt_fild08`: Poring/Lunatic/Fabre, one gentle Aggressor and one Coward pattern. | A new character can identify the tell and continue farming without repeated deaths or long pauses. |
| Mid | `orcsdun01`: Orc Zombie relentless pressure, Orc Skeleton variation; test Orc Archer keeper on `orcsdun02` only after the opt-in keeper code is accepted. | Solo and party can distinguish roles and counter a cast without reading a guide. |
| Late | `gl_prison`/`gl_knt01`: Raydric pressure/assist, Raydric Archer keeper, Evil Druid support. | Party prioritizes support/ranged targets; difficulty comes from composition, not hidden one-shots. |

For each map, snapshot spawn list, monster skill data, time-to-kill, deaths, potion use, and solo/party feedback before changes. Land one family at a time with a config or data rollback. Keep a short observation log with skill IDs, cast times, telegraph result, and player response. Advance only after the previous tier has a live pass.

## C-side extensions, after data pilot

The Skirmisher prototype is now implemented for opted-in profiles: after the configured number of positive player-hit callbacks, it scans at most eight adjacent cells, chooses one legal cell farther from its current player target without exceeding the configured band, and requests one path. The bounded hit counter and per-mob cooldown prevent oscillation; failed movement consumes the trigger and falls through to stock AI. The RangedKeeper prototype similarly makes one legal outward step when a player enters its preferred range, with a cooldown on success or failure. The Orc Archer also opts into local hazard-weighted A*: a reviewed list of harmful/controlling active skill units contributes a fixed path cost, with each cell checked at most once in the bounded 17×17 region around the path origin. Beyond that region, cost is stock; this is not a global hazard map. Dense-map/path/load testing and live behavior remain open. Never make the client decide collision or damage. Count path searches and CPU time, and cap additional searches per AI tick. If a map fails pathing or load checks, disable the opted-in profile and preserve stock AI.

## Boss follow-on

Eddga on `pay_fild11` is the GDD's **recommendation**, not part of S5–S9. Begin the boss slice only after mid/late pilot tells are readable. Use one signature fire pressure, adds, a movement check, water-element preparation, an HP-threshold escalation, and a recovery window; measure the open-world encounter separately from the DM Session beat. Keep ordinary MVP spawns stock until that slice passes.

## Acceptance

- Script/data parse and server startup pass; no accidental mode changes outside opted-in monster IDs.
- Live client shows correct cast footprints and cancellation; no warning for instant/single-cell skills.
- Compare baseline to each tier's solo and party run; record deaths, time-to-kill, potion use, and whether players noticed/countered the intended role.
- On S9, stress a dense map, verify bounded path work and no stuck/oscillating monsters; config rollback restores stock behavior.
