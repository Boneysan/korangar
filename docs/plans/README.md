# Implementation Plans

**Parent hub**: [docs/README.md](../README.md) (start here for the full documentation index).

This directory contains executable implementation plans derived from the design
docs. Keep plans short, milestone-scoped, and close them or replace them as work
lands.

| Plan | Purpose |
|---|---|
| [M0-connectivity.md](M0-connectivity.md) | First login → char → map loop against local Hercules |
| [asset-pipeline.md](asset-pipeline.md) | GRF/archive/data sync decisions for M1 |
| [packet-gap-party-whisper.md](packet-gap-party-whisper.md) | Protocol-safety plan for missing party and whisper packet families |
| [M1-p0-verification.md](M1-p0-verification.md) | E3.1 live P0 verification checklist against Hercules |
| [animation-fidelity.md](animation-fidelity.md) | Post-runtime animation fidelity: layer composition, event cursor, weapon visuals, skill/status recipe batches |
| [phase-d-live-verification.md](phase-d-live-verification.md) | Phase D live GUI checklist — **CLOSED 2026-07-21**, all 8 rows PASS |
| [phase-e1-live-verification.md](phase-e1-live-verification.md) | Phase E1 live GUI checklist — **CLOSED 2026-07-22**, all 7 rows PASS on mechanism; read its "Traps hit while driving this pass" before any new GUI session |
| **[classic-effect-fidelity.md](classic-effect-fidelity.md)** | **NEXT (2026-07-22): derive the skill→sprite mapping.** Sprite backend shipped (`44927a32`) but **no skill is mapped yet**, so in-game visuals are unchanged. Start with Soul Strike → `이팩트\soule`. Extraction needs GRF decryption — drive `GameFileLoader::get()` from a small Rust bin, *not* the Python script |

Use [docs/specs](../specs/) for implementation specs that describe a specific
feature slice in code-level detail.

Recent targeted specs for DM / protocol:
- `party-packets.md` — full promotion plan + Hercules layouts for 0x0AE4/0x0AE5 + whisper.
- `dm-phase-a-chat-integration.md` — parser, [DMJ], emitter, lib.rs wiring.
- `dm-ui-window-template.md` — isolation, WindowClass, reactive template.

Future / Phase 2 specs:
- `hud-edit-mode.md` — foundational layout system for all modern HUD + DM elements.
- `navigation-quest-guiding.md` — cross-map breadcrumbs, NAVI parsing, in-world ribbons, pings.

Use [docs/protocol](../protocol/) for Hercules-derived packet references and
packet audit lookup workflows.
