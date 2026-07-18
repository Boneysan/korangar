# Korangar Documentation Hub

This directory contains all design documents, technical deep dives, implementation plans, and specs for the Korangar project (this fork is specialized as a **Native Tabletop / DM Campaign engine** for the "Seal Cascade" D&D campaign).

## How to Navigate & Search

- **Searchability**: All documentation is plain Markdown. It is highly searchable:
  - In your editor/IDE: Use the built-in search (VS Code, Zed, etc.).
  - Terminal (recommended for agents): `rg "keyword" docs/` or `grep -r "keyword" docs/` (ripgrep is excellent for this repo).
  - GitHub: Use the repo search bar (filter by path `docs/`).
  - No dedicated doc site (e.g. no MkDocs) — raw files are the source of truth.

- **Start here for agents / new developers**:
  1. `CLAUDE.md` (root) — Project-specific rules and context.
  2. This `docs/README.md` — Overview of everything.
  3. `docs/CLIENT_SYSTEMS_OVERVIEW.md` — High-level map of the codebase.
  4. `docs/SOFTWARE_DESIGN.md` — Architecture and key decisions.

- **Cross-references**: Documents link to each other extensively. Follow the "See also" sections.

## Documentation Categories

### Core Architecture & Systems
| Document | Purpose |
|----------|---------|
| [CLIENT_SYSTEMS_OVERVIEW.md](CLIENT_SYSTEMS_OVERVIEW.md) | High-level map of all major systems (world, UI, networking, state, loaders). Prioritized for DM work. Includes improvement focus per area. |
| [SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) | Overall architecture, protocol strategy, decisions, and technical constraints. |
| [UI_FRAMEWORK_EXTENSION.md](UI_FRAMEWORK_EXTENSION.md) | How the retained UI (korangar-interface) works + concrete patterns for adding windows/components. |
| [STATE_MANAGEMENT_GUIDE.md](STATE_MANAGEMENT_GUIDE.md) | ClientState reactivity, RustState/StateElement paths, how to add new state safely. |
| [GRAPHICS_PIPELINE.md](GRAPHICS_PIPELINE.md) | Deep dive into the wgpu forward renderer, lighting, shadows, and shaders. |
| [WORLD_MAPS_ENTITIES.md](WORLD_MAPS_ENTITIES.md) | Map loading (RSW/GND/GAT), entity system, quest effects/markers, and DM visual integration. |
| [ANIMATION_SYSTEM.md](ANIMATION_SYSTEM.md) | Actor animation: native SPR/ACT layer rules, weapon sprites, exact basic-attack selection, source/target clocks, delayed impacts, skill actor states, sound events, audit tooling, and known gaps. |
| [PACKET_EVENTS_CATALOG.md](PACKET_EVENTS_CATALOG.md) | Complete catalogue of `NetworkEvent`s, every producing packet, handlers, data flows, and DM usage. |
| [LOADERS_ASSET_PIPELINE_INTERNALS.md](LOADERS_ASSET_PIPELINE_INTERNALS.md) | Detailed loaders, async system, archive backends, and Library. |
| [INPUT_CAMERA_SYSTEMS.md](INPUT_CAMERA_SYSTEMS.md) | Input handling, MouseInputMode, full camera trait + implementations, picking, pathing. |
| [ORIGINAL_CLIENT_CONTROLS.md](ORIGINAL_CLIENT_CONTROLS.md) | Compatibility matrix and implementation order for matching original RO mouse, keyboard, chat, window, and interaction controls. |
| [KORANGAR_INTERFACE_INTERNALS.md](KORANGAR_INTERFACE_INTERNALS.md) | Deep internals of layout resolver, element system, macros, and extension points. |
| [MAIN_LOOP_RENDERER_PERFORMANCE.md](MAIN_LOOP_RENDERER_PERFORMANCE.md) | End-to-end frame, renderer separation, timing, and performance characteristics. |
| [DEBUG_AUDIO_VIDEO_SUBSYSTEMS.md](DEBUG_AUDIO_VIDEO_SUBSYSTEMS.md) | Audio engine, video playback, and the full set of debug tools (packet inspector, state inspector, profilers). |
| [RO_OFFICIAL_CLIENT_STRUCTURE.md](RO_OFFICIAL_CLIENT_STRUCTURE.md) | Mapping of the real RO client (assets, packets, Lua data) at H:\RO for reference. |
| [MACOS_WORKFLOW.md](MACOS_WORKFLOW.md) | Running the Hercules server + client natively on macOS: setup, process management, and a fixed startup panic (AppKit `drawRect:` race). |
| [PLATFORM_BRINGUP.md](PLATFORM_BRINGUP.md) | Cross-platform bring-up runbook (Windows/Linux/WSL/macOS): server-first verification, account bootstrap, `loginlog` debugging, known client bugs (login wedge, render-loop kick), backend expectations per platform. |

### DM / Tabletop Campaign Tools (Primary Focus)
| Document | Purpose |
|----------|---------|
| [DM_INTERFACE.md](DM_INTERFACE.md) | Design for native DM/player UI (dice cards, initiative, campaign board, hazards, etc.). |
| [DM_CLIENT_IMPLEMENTATION.md](DM_CLIENT_IMPLEMENTATION.md) | Technical implementation guide: isolation rules, Phase A chat/[DMJ] flows, state model, parser, visuals. |
| [DM_SERVER_FUNCTIONS.md](DM_SERVER_FUNCTIONS.md) | Full map of the Hercules server-side DM campaign engine (`@dm*` commands, scripts, state). |
| [BESTIARY.md](BESTIARY.md) | Enhanced bestiary (1759+ entries): core stats, **precise skill damage formulas** (ported from battle.c e.g. NPC_*ATTACK ratio=100+100*(lv-1)), **separate PhysDPS/MagicDPS columns**, expanded **Drops/MvpDrops/MvpExp**. For DM balance & DPS. Full data in `bestiary.json`. |
| [CARDS.md](CARDS.md) | Complete cards table (1012+ cards): ID, name, equip location, effect script, dropped by monsters with chances (linked to bestiary mobs). Full data in `cards.json`. |
| [ITEMS.md](ITEMS.md) + `items.json` + `loot_groups.json` | Master items database (13k+ entries) with drops from mobs. Primary source for building level-appropriate loot/reward tables (filter by type, price, mob level from bestiary). loot_groups for boxes/packages. |
| [DM_DATA_GUIDE.md](DM_DATA_GUIDE.md) | How to integrate bestiary/cards/items data into client for DM journal, encounters, rewards, loot. Essential for E7 features. |

### Protocol, Packets & Networking
| Document | Purpose |
|----------|---------|
| [protocol/README.md](protocol/README.md) | Index for protocol references. |
| [protocol/hercules-20220406.md](protocol/hercules-20220406.md) | Hercules packet source map and audit workflow. |
| [protocol/packet-length-fallbacks.md](protocol/packet-length-fallbacks.md) | How automatic framing via length tables works. |
| [PACKET_EVENTS_CATALOG.md](PACKET_EVENTS_CATALOG.md) | (See Core section — also the authoritative packet-to-event map.) |

### Testing & Verification

> These live in **`tools/testing/`**, outside `docs/` — they are easy to miss. The
> **107-scenario headless suite** is the project's main automated regression gate
> (acceptance passed 2026-07-13, double-run green).

| Document | Purpose |
|----------|---------|
| [../tools/testing/testing_guide.md](../tools/testing/testing_guide.md) | Overall project testing reference — **start here**. |
| [../tools/testing/headless_test_plan.md](../tools/testing/headless_test_plan.md) | Canonical headless test plan: scenario inventory, what each phase covers, and why headless findings transfer to the graphical client. |
| [../tools/testing/headless_findings.md](../tools/testing/headless_findings.md) | Bug log + port-back tracking; every finding classified by layer (shared crate / harness / server). Fill in whenever a scenario fails. |
| [../tools/testing/headless_remaining_test_design.md](../tools/testing/headless_remaining_test_design.md) | Implementation-ready design for remaining lifecycle, social, DM, repair, and skill-menu coverage. |
| [../tools/testing/headless_mock_client_plan.md](../tools/testing/headless_mock_client_plan.md) | Original design doc; implementation status tracked there. |
| [plans/M1-p0-verification.md](plans/M1-p0-verification.md) | The **GUI** live-verification checklist — complements headless (see the axes note below). |

**Three different axes — do not conflate them:**

1. **Implemented?** → [PROJECT_PLAN.md](PROJECT_PLAN.md) §2's "Korangar" column (✅/🔶/❌).
2. **Wire protocol proven?** → the headless suite (`--scenario all`). It links the *same*
   `ragnarok-packets` + `korangar-networking` crates as the real client, so a green
   scenario means the packets and event mapping are correct.
3. **Proven through the GUI?** → [plans/M1-p0-verification.md](plans/M1-p0-verification.md).

Headless **cannot** catch a bug in how the main client *consumes* an event (the UI/state
layer in `korangar/src/`) — but it does establish that the wire data is correct, which
isolates any remaining bug to the UI layer. A row can be headless-green and still
unchecked for GUI; that means "the protocol works, the window hasn't been driven by hand."

To run (servers must be up first):

```sh
cargo run --release --example headless-tester -p korangar-networking -- --scenario all
```

### Implementation Plans, Roadmaps & Specs
| Document | Purpose |
|----------|---------|
| [FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) | Feature roadmap, UI/UX principles, packet promotion backlog. |
| [PROJECT_PLAN.md](PROJECT_PLAN.md) | Milestones (M0–M4), detailed task breakdown (E1–E9), decisions, risks. |
| [plans/README.md](plans/README.md) | Index of executable near-term plans. |
| [plans/M0-connectivity.md](plans/M0-connectivity.md) | Connectivity milestone plan. |
| [plans/asset-pipeline.md](plans/asset-pipeline.md) | GRF and custom asset loading strategy. |
| [plans/packet-gap-party-whisper.md](plans/packet-gap-party-whisper.md) | Critical party/whisper packet safety plan. |
| [plans/M1-p0-verification.md](plans/M1-p0-verification.md) | E3.1 live P0 verification checklist (M1). |
| [plans/modern-mechanics.md](plans/modern-mechanics.md) | Technical sketches for future action RPG / tabletop mechanics (WASD camera, skill checks, gamepad, etc.). |
| [specs/](specs/) | Targeted implementation specs (see below). |

### Targeted Implementation Specs (in `specs/`)
These are concrete, code-level guides ready for implementation:

- [combat-animation-pipeline.md](specs/combat-animation-pipeline.md) — Native-client-backed pipeline and recipe contract for attacks, skills, effects, projectiles, persistent units, reactions, audio, and complete player/mob coverage.
- [buff-bar-slice.md](specs/buff-bar-slice.md) — Template for packet promotion (status effects).
- [party-packets.md](specs/party-packets.md) — Detailed plan + Hercules layouts for party (0x0AE4/0x0AE5) and whisper.
- [dm-phase-a-chat-integration.md](specs/dm-phase-a-chat-integration.md) — `[DMJ]` parser, command emitter, lib.rs integration.
- [dm-ui-window-template.md](specs/dm-ui-window-template.md) — Isolation pattern + copy-paste template for DM windows.
- [hud-edit-mode.md](specs/hud-edit-mode.md) — Foundational HUD layout editor (required for most Phase 2 UI).
- [navigation-quest-guiding.md](specs/navigation-quest-guiding.md) — Cross-map breadcrumbs, NAVI parsing, in-world ribbons, pings.
- [atb-structured-rounds.md](specs/atb-structured-rounds.md) — Future work (E7.14): DM-toggleable ATB encounter rounds (movement/action locks via `setpcblock`, `[DMJ]`-synced charge-bar strip).
- [campaign-quest-journal.md](specs/campaign-quest-journal.md) — E7.3: quest data pipeline from `quest_db.conf`, journal window, HUD tracker; consumes QuestAdded/QuestRemoved/QuestList.
- [bestiary-unlock-persistence.md](specs/bestiary-unlock-persistence.md) — Bestiary unlocks: client-local RON file (Phase 1), server-authoritative party-wide sync (Phase 2).
- [initiative-encounter-panel.md](specs/initiative-encounter-panel.md) — E7.5+E7.8: initiative tracker + encounter panel, client windows *and* the missing `dm_checks.txt`/`dm_combat.txt` server commands.
- [proficiency-checks.md](specs/proficiency-checks.md) — Future work (E7.16): `d20 + base_stat/15 + proficiency` check formula, class skills as proficiencies, mechanical stealth/perception outcomes.

### Other / Historical
- [protocol/2026-07-07-handoff.md](protocol/2026-07-07-handoff.md) — Packet audit handoff note.
- [2026-07-10-handover.md](2026-07-10-handover.md) — Client/server bring-up handover covering connectivity, controls, and the bundled English item-name path.
- [2026-07-13-session-notes.md](2026-07-13-session-notes.md) — Sprite lighting modes, emote bubbles, jobname.lub sprite resolution, entity-sprite audit.
- [2026-07-14-session-notes.md](2026-07-14-session-notes.md) — Headless→graphical port-back closure (instanced-map resolution, skill-asset audit), E7 first slice (Bestiary Journal + Loot Generator), main merge. **Start here to resume DM-feature work.**
- [2026-07-15-session-notes.md](2026-07-15-session-notes.md) — E7 backlog design pass (five targeted specs) + bestiary data pipeline (mob lore, stat/tactical fields for the tiered lore reveal).
- [2026-07-17-session-notes.md](2026-07-17-session-notes.md) — **M1-008 round 2**: `ZC_NOTIFY_GROUNDSKILL` wired (Thunderstorm/Storm Gust play at the cast position), classic falling-bolt volleys for Fire/Cold Bolt (`FallingBolts`), per-hit `firehit`/`windhit` wiring, portal vortex tuning, `probe-thunderstorm` diagnostic scenario.
- [2026-07-16-session-notes.md](2026-07-16-session-notes.md) — **E3.1 GUI pass + follow-ons**: checklist driven by hand 17 → 32 of 34 rows; skill targeting; M1-016 emote-owner/height fixes; dynamic player/mob anchors; and the 81-entry `Alt+L` emote picker. Also the STR base+morph interpolator fix, classic effect remap, warp portal vortex, and mob-emote chat silence. Includes live results, why headless missed the GUI defects, and test-environment traps. **Start here to resume M1 verification.**
- [dm-atcommand-feedback.md](dm-atcommand-feedback.md) — `dispbottom` / `@dm` chat via packet `0x017F`; pair with Hercules `planning/dm-mode-troubleshooting.md`.
- [storage-window.md](storage-window.md) — Kafra storage grid, drag packets, `close2` then `openstorage` UX.

## For Agents (Claude, Grok, etc.)

Always start with:
- Root `CLAUDE.md` (rules + running instructions).
- This hub (`docs/README.md`).
- Then the relevant deep dive for your task (use the tables above).

When creating new documentation:
- Place it in the appropriate category.
- Add a link here.
- Add cross-references from related docs.
- Keep custom DM work isolated (see rules in CLAUDE.md and DM_CLIENT_IMPLEMENTATION.md).

## External References
- Wiki: [wiki/](https://github.com/vE5li/korangar/tree/main/wiki) (Installation, Troubleshooting, Contributing).
- Upstream Korangar: See the main project for base client details.

This hub is the single source of truth for "where is the documentation?" Questions about the project should be answerable by starting here + targeted `rg` searches. 

Last major update: Documentation expansion for packets, DM implementation, and future plans (2026).
#
