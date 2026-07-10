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
