# GDD combat input and telegraph slice contract

**Parent:** [GDD §5.13](../GDD.md#513-implementation-path-added-v02), [next-slices plan](../plans/gdd-next-slices.md) rows 2/3/10, and [monster AI pilot](monster-ai-pilot.md). Hercules owns hit, cast, movement, and skill legality. The client provides immediate selection feedback and shows server-originated windups.

## Cast telegraph lifecycle (slice 2)

The 20220406 `UseSkillSuccessPacket` (0x07FB) and `UseSkillAckPacket` (0x0B1A) both contain source entity, destination entity, target position, skill ID, and duration. Their current event mapping drops the destination fields. Preserve them as typed data in `NetworkEvent::SkillCast`; do not infer the target from the mouse or the caster's current position.

| Transition | Client effect |
|---|---|
| Cast packet with `cast_ms > 0` | Start/update one cast bar per source. If the skill's server layout covers more than one cell and a valid target cell exists, draw one footprint there for that cast. |
| Duplicate/updated cast packet | Replace the source's existing cast state; do not stack decals. |
| `SkillCastCancelled`, source despawn, map change | Remove that source's telegraph immediately. |
| Authoritative execution or cast-duration end | Remove windup footprint; persistent skill-unit visuals, if any, follow their own server events. |
| Unknown skill layout, bad target position, instant or single-cell skill | Cast bar or ordinary effect only; no invented warning geometry. |

Check target-cell semantics against live ground, entity-centered, and moving-target skills. The footprint comes from existing `Map::render_skill_footprint` layouts. A client preview must never suggest damage outside the server's actual cells. Limit simultaneous warnings and preserve meaning under reduced effects.

## Monster selection (slice 3)

Clicking a monster currently attacks immediately, while `player_target.rs` handles **players only**. Add a separate selected-monster ID and frame. Tab selects the nearest visible, alive hostile; repeated Tab advances through a stable sorted snapshot, Shift+Tab reverses. Sort by tile distance, then screen-center angle, then entity ID. Rebuild on spawn/despawn; skip unclickable/hidden actors. Selection alone sends no attack packet; clicking a monster retains current attack behavior. Clear selection on map change, target disappearance, or logout. Race/size/element chips come from the versioned bestiary entry and respect the server knowledge policy.

## One timed action (slice 10)

`BufferedAction` already chains actions while walking into range; preserve that behavior. A new local press during animation lock occupies one replaceable slot `{action, queued_at, expires_at}`, starting with 200 ms expiry. Newest press replaces oldest; explicit move/cancel, death, logout, map change, lost target, or refusal clears it. When the local action ends or the player reaches range, validate target/cell, range, map, and current server-known state before sending once. Never replay after expiry or across map changes. Autoattack's continuous loop remains separate from this one-shot slot.

Use a visible but restrained queued-action cue; a rejected action clears it with the existing skill-fail explanation. Verify rapid attack→skill, skill→skill, ground cast while moving, cancellation, lag, and the boundary at 199/201 ms on the live stack. This slice does not predict damage or begin a speculative server cast animation.
