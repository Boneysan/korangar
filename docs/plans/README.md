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
| [security-audit.md](security-audit.md) | First security pass (2026-08-17). C1 still open; six other findings fixed that day |
| [security-audit-2.md](security-audit-2.md) | Independent second pass (2026-08-17). Re-verifies the first pass; adds API / MariaDB / DM-script findings |
| [security-audit-3.md](security-audit-3.md) | Third pass (2026-08-17). Lua/GRF, session tokens, official NPCs, campaign economy |

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
