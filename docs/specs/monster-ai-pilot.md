# Tactical monster pilot — GDD §6.10 / server slices S5–S9

**Parent:** [GDD §6.10](../GDD.md#610-implementation-path-added-v02) and [next-slices plan](../plans/gdd-next-slices.md). The pilot tests readable, varied PvE on three tiers before broad rollout. It is separate from DM Session encounters.

## Profiles and implementation boundary

Keep a reviewed family-profile table in `Hercules/planning/` with monster ID, map, role, mode bits, skill/trigger parameters, telegraph, cooldown, retreat threshold, and rollback switch. Generate or validate the corresponding `db/re/mob_skill_db.conf` records. Data-only roles come first: Aggressor (`MD_AGGRESSIVE`/target-change plus a cast attack), Support (`MST_FRIEND`/friend-HP trigger), Controller (status/ground skill), Coward (`NPC_RUN` at low HP), and Opportunist (target-weak/cast reaction). Protector means assist/defend behavior in this pilot; true interposition is a later C design. Do not apply global `monster_ai` bits to improve one family if they also change unrelated mobs.

The client cast telegraph slice must be live before a new area attack ships. Eligibility: an actual server cast with a footprint larger than one cell, real target position, and finite cast time. Visual duration ends on execution/cancel. A warning without an actionable movement window is retuned or removed. Keep most ordinary monsters simple so farming does not become a boss fight.

## Pilot sequence

| Tier | Maps and proposed profiles | Observation gate |
|---|---|---|
| Early | `prt_fild08`: Poring/Lunatic/Fabre, one gentle Aggressor and one Coward pattern. | A new character can identify the tell and continue farming without repeated deaths or long pauses. |
| Mid | `orcsdun01`: Orc Zombie relentless pressure, Orc Skeleton variation; test Orc Archer keeper on adjacent `orc_fild` only after S9. | Solo and party can distinguish roles and counter a cast without reading a guide. |
| Late | `gl_prison`/`gl_knt01`: Raydric pressure/assist, Raydric Archer keeper, Evil Druid support. | Party prioritizes support/ranged targets; difficulty comes from composition, not hidden one-shots. |

For each map, snapshot spawn list, monster skill data, time-to-kill, deaths, potion use, and solo/party feedback before changes. Land one family at a time with a config or data rollback. Keep a short observation log with skill IDs, cast times, telegraph result, and player response. Advance only after the previous tier has a live pass.

## C-side extensions, after data pilot

Skirmisher: after a bounded number of hits, choose a reachable cell at distance `d`, step once, then resume normal chase; cooldown prevents oscillation. Ranged Keeper: maintain a preferred band and fall back to normal attack if no legal retreat cell exists. Hazard awareness: only opted-in monsters add bounded cost to traversing active server ground effects; unintelligent undead ignore them. Never make the client decide collision or damage. Count path searches and CPU time, and cap additional searches per AI tick. If a map fails pathing or load checks, disable the opted-in profile and preserve stock AI.

## Boss follow-on

Eddga on `pay_fild11` is the GDD's **recommendation**, not part of S5–S9. Begin the boss slice only after mid/late pilot tells are readable. Use one signature fire pressure, adds, a movement check, water-element preparation, an HP-threshold escalation, and a recovery window; measure the open-world encounter separately from the DM Session beat. Keep ordinary MVP spawns stock until that slice passes.

## Acceptance

- Script/data parse and server startup pass; no accidental mode changes outside opted-in monster IDs.
- Live client shows correct cast footprints and cancellation; no warning for instant/single-cell skills.
- Compare baseline to each tier's solo and party run; record deaths, time-to-kill, potion use, and whether players noticed/countered the intended role.
- On S9, stress a dense map, verify bounded path work and no stuck/oscillating monsters; config rollback restores stock behavior.
