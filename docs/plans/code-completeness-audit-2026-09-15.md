# Code completeness audit — 2026-09-15

Scope: the 120 product requirements in
`playtest-adjustments-2026-09-12.md`, their QW task owners, the current Korangar
and Hercules diffs, and the stated done conditions in the execution runbook.

## Verdict

The project is **not feature complete** against the plan. The current changes
contain useful implemented slices, headless coverage, and several good pure
models, but earlier runbook updates promoted a number of model/fixture tests to
feature completion without production integration or required live evidence.
Those cards have been reopened in `qwen3-playtest-runbook.md`.

## Confirmed production slices

- Exact-quantity drop/trade dialog and duplicate pending-trade guard are wired
  through `InputEvent`, `ClientState`, a real window, and networking calls.
- Area-loot state is wired into world item clicks and cancellation events.
- Hotbar right-click/drag clearing is connected to the event loop and persistence
  packet path.
- Vendor rows reuse the existing item-tooltip path.
- Server-side player save/load/resetskill/refundskill permissions and commands
  are present.
- Party EXP configuration, player max-weight expansion, attacks above 90%
  weight, recovery changes, and respawn changes have server implementations.
- Hunt and general quest-location generators produce client data files.

These are implementation findings, not release acceptance. Their cards remain
open where the plan explicitly requires seated GUI, multi-client, boundary, or
restart verification.

## Blocking completeness findings

### Test-only client modules

The following modules are declared/exported but have no production consumer
outside their own unit tests (or another test-only helper):

- `state/breadcrumb.rs`
- `state/campaign_checkpoint.rs`
- `state/chest_discovery.rs`
- `world/library/equip_presentation.rs`
- `world/library/equipment_eligibility.rs`
- `world/library/hunt_schema.rs` and `journal_slice.rs`
- `world/library/warp_graph.rs`

Consequently QW-048–063 and QW-088 cannot be called complete. In particular,
there is no production breadcrumb HUD, route guidance, checkpoint
reconciliation, chest state rendering, or unified unusable-equipment
presentation from these modules.
`cargo clippy -p korangar --lib -- -D warnings` independently reports these as
dead code and fails with 43 warnings-as-errors; QW-100 is therefore open.

### 2026-09-16 integration update

`combat_chat.rs`, `party_colors.rs`, and `ui_sounds.rs` now have production
owners and call sites. Combat events feed the existing chat window; party rows
and minimap markers share resolved colors; UI sounds route through the audio
engine with persistent settings and edge gating. Their automated suites pass.
QW-081 and QW-083 remain open only for their stated live visual acceptance, and
QW-084 now has persisted, resettable, contrast-corrected local color overrides.
The original 43-warning
count above is historical; strict workspace Clippy remains open for current
dead-code debt plus an all-target constant-assertion lint.

### Recovery/rules defects and missing evidence

- Sitting recovery returns before the normal poison/overweight suppression path,
  so it can grant the 25% tick while the player is poisoned or at/above the
  no-regeneration weight threshold. This contradicts the approved rules.
- Respawn fill also runs before those suppression checks. Its exact ten-second
  behavior has no deterministic server test in the diff.
- `sit_regen_tick` is not visibly reset when leaving the sitting state, so a
  partial interval may carry across stand/sit cycles.
- Combat timestamps are recorded on HP damage, but the plan's offensive-skill
  and support-interaction cases are not covered by a server test.
- The max-weight multiplier is hard-coded in `status.c`, not configuration, and
  no boundary matrix proves pickup, trade, storage, cart, recovery, attack, and
  skill behavior at 70/90/100%.
- EXP values are configured, but QW-079 and the formula/restart/database gates
  remain open.

QW-071–079 therefore require correction and acceptance work before release.

### Data/schema gaps

- Equipment eligibility: resolved in QW-048/QW-049 via `Hercules/tools/gen-equipment-eligibility.py`
  (5,578 equippable items exported into `equipment_eligibility.tsv`, verified by `check-campaign.sh`,
  and integrated into `ItemBox` and `BuyWindow`).
- Hunt objective/guidance data is partly embedded as small Rust string fixtures;
  it is not the complete authoritative generated data pack requested by the
  plan.
- The warp graph parser operates on unit-test fixture strings and is not fed by
  a generated Hercules graph at runtime.
- Campaign checkpoint and reconciliation exist only as a client-side pure model;
  no durable server persistence or late-join protocol is implemented.

### Outstanding external/live gates

- Failed-skill names/captures, Increase AGI seated-source matrix, and all Windows
  install/repair cases remain open.
- Required GUI validation for hotbar, quantity dialogs, vendor tooltips, and
  area-loot remains open.
- Cosmetics needs an approved vertical slice.
- Two-client internet, Arc I/late-join, final gameplay-rules, and release gates
  remain open.

## Task reconciliation

The parent product plan intentionally remains unchecked; completion evidence is
owned by QW cards. On this audit, previously checked cards were reopened where
their done condition was not met. Pure decisions/models that genuinely met their
narrow card remain checked (for example QW-040, QW-043, QW-045, QW-070, QW-077,
QW-086, and QW-090).

Next implementation priority is QW-071/QW-072: correct recovery suppression and
timer lifecycle, add deterministic server coverage, then continue through the
reopened integration queue before QW-100.
