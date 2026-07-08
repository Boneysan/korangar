# Implementation Specs

**Parent hub**: [docs/README.md](../README.md)

This directory contains targeted, code-level implementation specifications. These are concrete guides meant to be followed when building specific features or promoting packets.

Use these alongside the higher-level plans in `../plans/`.

## Current Specs

See the main [Documentation Hub](../README.md) for the full categorized list and descriptions. All specs here are linked from the relevant roadmap and implementation documents.

- `buff-bar-slice.md` — Reference template for packet → event → state → widget promotion.
- `party-packets.md` — Party and whisper packet definitions + promotion plan.
- `dm-phase-a-chat-integration.md` — DM `[DMJ]` parser, command emitter, and integration.
- `dm-ui-window-template.md` — DM window isolation pattern and template.
- `hud-edit-mode.md` — HUD layout editor (foundational for Phase 2 UI).
- `navigation-quest-guiding.md` — Quest navigation, breadcrumbs, and pings.

When adding a new spec:
1. Create the file here.
2. Add it to the hub in `../README.md`.
3. Link it from the relevant roadmap/plan document.