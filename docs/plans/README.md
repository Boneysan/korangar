# Implementation Plans

**Parent hub**: [docs/README.md](../README.md) (start here for the full documentation index).

> **2026-08-12 — two tracks are separate:**
>
> 1. **Headless suite** — acceptance **closed** for planned depth. Resume:
>    [../tools/testing/headless-next-steps.md](../tools/testing/headless-next-steps.md)
> 2. **GUI live pass** — still has open rows (Hermode, Auto Spell, …). Resume:
>    [gui-verification-pass.md](gui-verification-pass.md) (**open-only table at top**)
>
> Also read [../RESUME-HERE.md](../RESUME-HERE.md).

This directory contains executable implementation plans derived from the design
docs. Keep plans short, milestone-scoped, and close them or replace them as work
lands.

| Plan | Purpose |
|---|---|
| [qwen3-playtest-runbook.md](qwen3-playtest-runbook.md) | **Active continuous Qwen3 execution queue:** one bounded task at a time, exact evidence gates, non-blocking review checkpoints, and a single `NEXT` pointer |
| [qwen3-restart-2026-09-13.md](qwen3-restart-2026-09-13.md) | **Authoritative senior reconciliation:** accepted completions, incomplete evidence, blockers, dirty-worktree snapshot, and exact restart order |
| [playtest-adjustments-2026-09-12.md](playtest-adjustments-2026-09-12.md) | September playtest product backlog and acceptance intent; execute it through the Qwen3 runbook above |
| [recovery-rules-implementation.md](recovery-rules-implementation.md) | QW-071–079: server-authoritative recovery, weight, checkpoint/respec, EXP, and boundary verification |
| [equipment-eligibility-integration.md](equipment-eligibility-integration.md) | QW-047–049: complete Hercules equipment export and shared client presentation |
| [campaign-journal-and-routing-integration.md](campaign-journal-and-routing-integration.md) | QW-050–053/056–063: generated campaign data, journal, tracker, minimap, and routing |
| [campaign-checkpoint-protocol.md](campaign-checkpoint-protocol.md) | QW-054–056: durable server checkpoint and preview/confirm late-join reconciliation |
| [combat-chat-integration.md](combat-chat-integration.md) | QW-080–081: typed combat events, bounded state, filters, and existing-chat integration |
| [party-colors-and-ui-sounds.md](party-colors-and-ui-sounds.md) | QW-082–084: stable accessible party colors and transition-driven UI audio |
| [chest-presentation-integration.md](chest-presentation-integration.md) | QW-088–090: authoritative chest state, world presentation, Field Notes, and Marks |
| [M0-connectivity.md](M0-connectivity.md) | First login → char → map loop against local Hercules |
| [asset-pipeline.md](asset-pipeline.md) | GRF/archive/data sync decisions for M1 |
| [packet-gap-party-whisper.md](packet-gap-party-whisper.md) | Protocol-safety plan for missing party and whisper packet families |
| [M1-p0-verification.md](M1-p0-verification.md) | E3.1 live P0 verification checklist — **34/34 closed** (macOS 2026-07) |
| [animation-fidelity.md](animation-fidelity.md) | Post-runtime animation fidelity: layer composition, event cursor, weapon visuals, skill/status recipe batches |
| [gui-verification-pass.md](gui-verification-pass.md) | **GUI live queue (IN PROGRESS).** Boundary 5 (event → pixel). Blocks A–D largely done; **open: Block E Hermode, N20 Auto Spell**, known fails N23/N24. Open-only table at top of file. Improvements/findings inline |
| [phase-d-live-verification.md](phase-d-live-verification.md) | Phase D live GUI checklist — **CLOSED 2026-07-21**, all 8 rows PASS |
| [phase-e1-live-verification.md](phase-e1-live-verification.md) | Phase E1 live GUI checklist — **CLOSED 2026-07-22**, all 7 rows PASS on mechanism |
| [classic-effect-fidelity.md](classic-effect-fidelity.md) | Classic skill effects — E1/E2 live-verified closed; Moonlit/Hermode live work lives in gui-verification-pass |
| [testing-completeness.md](testing-completeness.md) | What headless green means / does not mean |
| [work-backlog.md](work-backlog.md) | Standing inventory: §1 live debt reconciled 2026-08-12; §2+ unbuilt features |
| [friends-distribution.md](friends-distribution.md) | **Private friends pack** (E8.3): Google Drive folder, no public release. Packaging not built yet |
| [security-audit.md](security-audit.md) | First security pass (2026-08-17). C1 rotated 2026-08-18; six other findings fixed the same day as the audit |
| [security-audit-2.md](security-audit-2.md) | Independent second pass (2026-08-17). Re-verifies the first pass; adds API / MariaDB / DM-script findings. Remediations 2026-08-18 |
| [security-audit-3.md](security-audit-3.md) | Third pass (2026-08-17). Lua/GRF, session tokens, official NPCs, campaign economy. Remediations 2026-08-18 |
| [security-audit-4.md](security-audit-4.md) | Fourth pass (2026-08-17). API memory safety/resource limits, chat exhaustion, concrete client packet panics. Remediations 2026-08-18 |

Use [docs/specs](../specs/) for implementation specs that describe a specific
feature slice in code-level detail.

Recent targeted specs for DM / protocol:
- `party-packets.md` — full promotion plan + Hercules layouts for 0x0AE4/0x0AE5 + whisper.
- `dm-phase-a-chat-integration.md` — parser, [DMJ], emitter, lib.rs wiring.
- `dm-ui-window-template.md` — isolation, WindowClass, reactive template.

Future / Phase 2 specs:
- `hud-edit-mode.md` — foundational layout system for all modern HUD + DM elements.
- `navigation-quest-guiding.md` — cross-map breadcrumbs, NAVI parsing, in-world ribbons, pings.
- `campaign-quest-journal.md` — quest UI (not built; headless owns packets only).

Use [docs/protocol](../protocol/) for Hercules-derived packet references and
packet audit lookup workflows.
