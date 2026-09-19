# Recovery and progression rules — implementation plan

**Owners:** QW-071–079  
**Repositories:** Hercules and Korangar  
**Approved policy:** `progression-approvals-needed.md`

## Goal

Implement the approved recovery, weight, checkpoint, respec, and EXP rules as
server-authoritative behavior, expose truthful client feedback, and prove every
boundary after restart. This plan replaces the assumption that the current
`status.c` diff is complete.

## Existing code to reconcile

- Hercules `src/map/status.c`: `status_mark_combat`, `status_natural_heal`,
  max-weight multiplier, respawn fill.
- Hercules `src/map/pc.c` / `pc.h`: respawn state, `pc_skilldown`.
- Hercules `src/map/atcommand.c`, `conf/groups.conf`: save/load and respec.
- Hercules `conf/import/battle.conf`: EXP configuration.
- Korangar `docs/player-save-and-respec.md`, Menu commands, EXP messages.
- Headless scenarios in `korangar-networking/examples/headless-tester/scenarios/`.

## Required server design

1. Add named battle settings instead of unexplained constants:
   `campaign_combat_timeout_ms`, `campaign_sit_recovery_interval_ms`,
   `campaign_sit_recovery_percent`, `campaign_respawn_percent`,
   `campaign_respawn_fill_ms`, and `campaign_max_weight_multiplier`.
   Register them in `src/map/battle.h` and `src/map/battle.c`; set approved
   values only in `conf/import/battle.conf`.
2. Centralize the decision in a helper that returns why natural recovery is
   blocked: dead, poison/bleeding status, overweight, or none. Run it before
   standing, walking, sitting, and respawn-fill branches.
3. Track recovery mode transitions explicitly. Clear `sit_regen_tick` on stand,
   death, map change, logout, and any suppression transition. Do not carry a
   partial sitting interval through a stand/sit cycle.
4. Mark combat on positive HP damage dealt or taken and on an accepted
   offensive skill. Healing/support only refreshes combat when its target is
   already in combat. Map change, death, and reconnect must have defined state.
5. Sitting recovery replaces ordinary recovery. At each complete interval heal
   exactly 25% of maximum HP and SP, clamp to maximum, and emit one update.
6. Respawn gives 50% immediately and schedules the balance over exactly ten
   seconds. Damage cancels the outstanding fill. Poison/overweight handling must
   follow the approved policy and be exercised explicitly.
7. Apply the weight multiplier to the intended final capacity consistently,
   including skills/equipment bonuses. Keep warnings at 70%, soft state at 90%,
   and the authoritative pickup/trade hard stop at 100%.
8. Keep `@resetskill` and `@refundskill` server-authoritative. Reject prerequisite
   violations, refresh skill packets, and make stale hotbar entries harmless.
9. Keep monster/quest rates at 100, party even-share bonus at 25% per additional
   eligible member, range at 15 levels, and existing shared-Zeny behavior.

## Client work

- Use existing weight/status/EXP packets; do not duplicate the rules locally.
- Add explicit 70/90/100% labels and server-error text where current UI is
  ambiguous.
- Ensure EXP toast and HUD totals use the same authoritative packet delta.
- Document Menu Save/Load/Reset behavior and surface command rejection text.

## Verification matrix

- Recovery: stand/walk/sit × normal/49%/50%/89%/90%/100% weight × poison on/off.
- Combat: deal, receive, offensive cast, support idle target, support engaged
  target, timeout refresh/expiry, death, map change, reconnect.
- Respawn: exact immediate amount, time samples, damage cancellation, max clamp.
- Weight: pickup, trade, storage, cart, attacks, skills, death at one unit below,
  exactly at, and one unit above every boundary.
- Respec: prerequisite chain, zero-level removal, stale hotbar, relog.
- EXP: solo and 2/3-player, in/out of range, base/job, quest award, Zeny,
  rounding, relog and database total.

Add focused headless scenarios for observable behavior and a small deterministic
server test seam for formula/state transitions. Restart Hercules before the
acceptance pass to prove configuration loading.

## Done

- No recovery branch bypasses poison/weight suppression accidentally.
- All configurable values survive restart and match the approved document.
- Hercules build, focused scenarios, and the full relevant headless suite pass.
- QW-071–079 contain command logs and exact before/after values, not source-only
  assertions.

