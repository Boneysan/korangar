# Session notes — 2026-09-16

## Implemented

- Hercules recovery/progression constants moved into battle configuration.
- Combat state, support interaction, sitting timer resets, poison/weight
  suppression, respawn fill, and final-capacity weight multiplication were
  implemented.
- Recovery/config contract executable and focused headless coverage were added.
- Combat chat now receives typed damage, heal, status, failure, EXP, and loot
  events and has a bounded filtered channel in the existing chat window.
- UI activation/rejection sounds gained persistent enable/volume controls and
  transition gating.
- Stable party colors now share one resolved RGB value between party rows and
  minimap markers.
- Seven detailed integration plans were added for the remaining recovery,
  eligibility, campaign, checkpoint, presentation, and chest work.

## Verified

- `cargo fmt --all -- --check`: pass.
- `cargo test --workspace`: pass (Korangar library: 468 passed, 17 ignored;
  remaining workspace and doc tests passed).
- Hercules `make -j2`: pass.
- Hercules `test_combat_recovery`: pass, but it is a contract/model test and
  does not yet call the production status helpers directly.
- Final focused headless passes: sitting recovery, respawn, checkpoint
  save/load, ×5 capacity, skill relog/refund/hotbar, solo/party EXP,
  persisted DM EXP, attack, incoming damage, and skill refusal.

Strict workspace Clippy is still red: the first all-target failure is the
pre-existing constant assertion in `ragnarok-packets/src/lib.rs`, and the normal
build still reports the already-audited unused campaign/checkpoint/chest/
eligibility code. QW-100 remains open.

## Reconciliation

The runbook was corrected after review. QW-071, 072, 075, 079, 081, and 083
remain open because their stated production-path, full boundary, or live GUI
conditions are not yet proven. QW-073, 076, 078, 080, and 082 have implementation
and automated evidence sufficient for their current cards.

Next: make the recovery test seam call production functions, then finish the
recovery/weight live matrix before moving to QW-084.
