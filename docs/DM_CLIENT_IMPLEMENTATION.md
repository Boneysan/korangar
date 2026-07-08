# DM Client Implementation Guide

**Purpose**: This is the technical implementation bridge for E7 (DM interface & custom features) in [PROJECT_PLAN.md](PROJECT_PLAN.md) and the designs in [DM_INTERFACE.md](DM_INTERFACE.md). It maps:

- Server campaign engine ([DM_SERVER_FUNCTIONS.md](DM_SERVER_FUNCTIONS.md) + Hercules_RO `npc/custom/dm_campaign/`)
- Wire surface ([PACKET_EVENTS_CATALOG.md](PACKET_EVENTS_CATALOG.md), protocol/ docs, packet-gap plan)
- World / in-world visuals ([WORLD_MAPS_ENTITIES.md](WORLD_MAPS_ENTITIES.md))
- Graphics / effects ([GRAPHICS_PIPELINE.md](GRAPHICS_PIPELINE.md))
- Base client architecture ([SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md), [CLIENT_SYSTEMS_OVERVIEW.md](CLIENT_SYSTEMS_OVERVIEW.md))

**Goal**: Enough concrete guidance, data flows, code patterns, and prerequisites so the custom DM UI (dice cards, journal, trackers, DM console windows, hazard telegraphs) can be built while keeping the fork rebaseable.

**Status**: Phase A (chat + existing packets + QuestEffect) is immediately actionable after M1. Phase B (custom packets) is gated on proving Phase A UX.

## 1. Isolation & Rebaseability Rules (non-negotiable)

From CLAUDE.md and DM_INTERFACE §9.4:

- All custom DM code **must** live under:
  - `korangar/src/dm/` (pure state, parsers, emitters, data models — no direct graphics or heavy deps)
  - `korangar/src/interface/windows/dm/` (windows + widgets that use `korangar-interface`)
- Never modify core `src/lib.rs` event match, `ClientState` fields, or base windows except through additive paths or the approved extension points.
- Use `#[hidden_element]` + `RustState` derives for new DM state so the inspector can see it without polluting the main view.
- Static campaign data (arcs/beats/mobs/quests) is **generated at build time** from the Hercules_RO tree (single source of truth). Do not hardcode in client.
- Permission is **always server-enforced** via `DM_RequireDM`. Client UI is convenience + visibility gating (GM level check is nice-to-have client-side only).

Any change that touches generic RO paths (e.g. new `NetworkEvent` variants used by DM) must be justified as generally useful or clearly isolated.

## 2. Recommended Module Structure

```
korangar/src/
├── dm/
│   ├── mod.rs                 // pub use re-exports
│   ├── state.rs               // DmState (or per-feature modules)
│   ├── parser.rs              // Phase A [DMJ] + chat result parser
│   ├── commands.rs            // CommandEmitter (builds "name : @dm..." strings)
│   ├── data/                  // generated (beats, quest defs, mob ids, etc.)
│   │   ├── campaign.rs        // build.rs output
│   │   └── quests.rs
│   └── types.rs               // InitiativeEntry, CheckResult, Hazard, BeatId, etc.
├── interface/
│   └── windows/
│       └── dm/
│           ├── mod.rs
│           ├── campaign_board.rs
│           ├── check_console.rs
│           ├── dice_card.rs     // widget + manager
│           ├── initiative_bar.rs
│           ├── hazard_board.rs
│           ├── session_hud.rs
│           └── ... (one file per major window)
└── state/
    └── mod.rs                 // extend ClientState with dm: DmState
```

Add to `korangar/src/lib.rs` (minimal surface):
- Import `dm` module.
- In `handle_network_events`: route relevant `ChatMessage` (and future events) to `dm::parser::handle_incoming(...)`.
- Call `dm::state.tick(client_tick)` in the per-frame update.
- Expose a `dm_command_emitter` or route UI actions through existing `networking_system.send_chat_message` initially.

Keep the touch points tiny and documented.

## 3. ClientState Extensions (Reactive DM State)

Current `ClientState` (from `src/state/mod.rs`) already has good patterns (entities, chat_messages, status_effects, hotbar, inventory, skill_tree, dialog_window).

Proposed additive extension (add under a `dm` field or top-level for simplicity while hidden):

```rust
// In ClientState
#[hidden_element]
dm_state: DmState,

// Example skeleton (in src/dm/state.rs)
#[derive(RustState, StateElement, Default)]
pub struct DmState {
    pub active: bool,                    // driven by @dm mode / $dm_active_party echo
    pub party: Vec<PartyMember>,         // from party packets + [DMJ] when available
    pub initiative: Vec<InitiativeEntry>,
    pub current_beat: Option<BeatId>,
    pub flags: HashMap<String, bool>,    // from @dmflag status
    pub downed: Option<DownedState>,     // per-player or local
    pub hazards: Vec<HazardMarker>,      // id + position + type (fed by QuestEffect + [DMJ])
    pub inspiration_tokens: u8,
    pub last_check_results: VecDeque<CheckResult>, // for dice cards + combat log
    // ...
}
```

- All DM UI binds via `client_state().dm_state().xxx()`.
- Clear on map change / disconnect (like entities, particles).
- StatusEffects was recently added as a template — copy that pattern (tickable, path-based).

For Phase A, many fields will be populated by parsing chat + QuestEffect packets.

## 4. Phase A Transport (Zero New Packets — Use What Exists)

**Command direction (UI → server)**:
- DM window button click → `dm::commands::emit_dm_check(...)` (or similar).
- Builds string `format!("{} : @dm check {} {} {}", player_name, target, stat, dc)`.
- Sends via existing `networking_system.send_chat_message(player_name, text)` (wraps `GlobalMessagePacket`).
- Server `bindatcmd` + `DM_RequireDM` executes. Result comes back as chat.

See [PACKET_EVENTS_CATALOG.md](PACKET_EVENTS_CATALOG.md) "Outgoing" and "DM Campaign Integration Points".

**Feedback direction (server → UI)**:
- All output arrives as one or more `NetworkEvent::ChatMessage { text, color }`.
- In `lib.rs` (or a dedicated DM router):
  ```rust
  NetworkEvent::ChatMessage { text, .. } => {
      if let Some(dmj) = dm::parser::try_parse_dmj(&text) {
          dm_state.apply(dmj);
          // optionally suppress or reformat the raw text in chat_messages
      } else if dm::parser::is_dm_result(&text) {
          dm_state.record_check_result(...);
      } else {
          chat_messages.push(...);
      }
  }
  ```
- `QuestEffectPacket` (0x0446) for map markers/hazards (already wired: `AddQuestEffect` → `particle_holder.add_quest_icon`). See WORLD_MAPS_ENTITIES.md for the `values()` decoupling fix that made this reliable even without a live entity.

**Structured echo convention (server script side, client parser)**:
When `$dm_client_mode` (or equivalent) is true, server emits a parallel line:
```
[DMJ]{"t":"check","who":"Wynne","stat":"agi","roll":17,"mod":3,"dc":15,"pass":true,"nat":null}
```
Parser ignores the human text version or rewrites the chat entry.

This is exactly as described in DM_INTERFACE §9.3 and DM_SERVER_FUNCTIONS.

## 5. Priority E7 Feature Data Flows (Phase A)

| Feature (E7.x) | Primary Packets/Events | State Path | In-World / UI Notes |
|---|---|---|---|
| Player dice cards (E7.2) | ChatMessage (from @roll / @dm check) + future [DMJ] | `dm_state.last_check_results` + dice card manager | Animated card widget (reuse particle/effect style or pure interface). Nat-20/1 flair. |
| Campaign quest journal (E7.3) | Quest* packets (many noops today; promote per catalog + FEATURE_ROADMAP §8.3) + OngoingQuestInfoList data | `dm_state.quests` or extend quest system | Build-time merge from `campaign_quest_journal_entries.lua` (see Hercules tools). Native view supersedes official lub. |
| Inspiration + downed (E7.4) | Chat + [DMJ] + StatusChange (for any linked effects) | `dm_state.inspiration_tokens`, `downed` | Overlay + indicator near hotbar. |
| Initiative tracker (E7.5) | [DMJ] from @dminitiative + party packets (gap) | `dm_state.initiative` (reorderable for DM) | HUD bar. Reuses party HP when available. |
| Check console + rewards (E7.6) | Outgoing via chat emitter; incoming chat/[DMJ] | Check params + history | DM-only window (GM level gate). |
| Campaign board / beats / ledger (E7.7) | [DMJ] status + static data | `dm_state.current_beat`, flags | Tree from generated `campaign.rs`. Click emits @dmbeat/@dmdecide. |
| Encounter / hazard / scene (E7.8-9) | QuestEffectPacket + AddEntity + chat | `dm_state.hazards` + active encounter | In-world: extend particles or skill units for pulsing telegraphs. Map pings via markers. |
| Session HUD (E7.10) | Chat + mode echoes | `dm_state.active`, party | Minimal always-visible or toggle. |

**Party dependency note**: The campaign is party-locked (`$dm_active_party`). Promote the party family (see `plans/packet-gap-party-whisper.md` and PACKET_EVENTS_CATALOG "Party Packet Targets") early for roster/HP/position. Until then, synthesize party list from chat echoes or manual @dm status.

## 6. Packet Prerequisites & Promotion Path

See [PACKET_EVENTS_CATALOG.md](PACKET_EVENTS_CATALOG.md) (full NetworkEvent + producing packets) and FEATURE_ROADMAP §8.3 (noop backlog).

**DM-critical to promote (Phase 1 / M2)**:
- Party roster/HP/position/chat (0x0AE4/0x0AE5 family + 0x0109 etc.) — framing safety first.
- Quest family (0x0AFF, 0x0B0C etc. already partially added as noops per handoff note).
- StatusChange* (MVP buff row; template in specs/buff-bar-slice.md).
- More quest notification / objective packets for journal.

**Process** (repeatable, from buff spec and packet catalog):
1. Capture with `KORANGAR_PACKET_LOG` or packet inspector.
2. Cross-ref Hercules `packets_struct.h` + `packets2022_len_main.h` (use hercules-20220406.md workflow).
3. Add/ refine struct in `ragnarok-packets/src/lib.rs`.
4. Change `register_noop` → `register` that produces `NetworkEvent::XXX`.
5. Handle in `lib.rs` → mutate DM or base state.
6. Add packet roundtrip test.
7. Update backlog + this doc.

`register_length_fallbacks` keeps everything framed even while noops exist.

## 7. In-World DM Visuals (Hazards, Markers, Scenes)

- **QuestEffectPacket** (0x0446): Primary for "!" / colored markers. Already works decoupled.
- **Skill units** (`NotifySkillUnitPacket`): Good path for pulsing AoE hazards/telegraphs. Reuse `effect_holder`.
- **Particles / custom effects**: Extend `world/particles` or `renderer/effect` for DM-specific (e.g. hazard pulse geometry). See WORLD_MAPS_ENTITIES.md for quest icon render changes and GRAPHICS_PIPELINE for effect passes.
- **Map pings / spotlights**: Can start as ground markers + chat; later use picker + custom draw.
- **AddEntity for temporary scene actors**: Use existing Npc/Player paths with special job_ids or effect state.

Coordinate positions come from the packet (TilePosition / WorldPosition) or server @dmhazard coords translated client-side.

DM windows can also drive client-local previews (e.g. hazard placement picker) before sending the command.

## 8. UI Widget & Window Patterns

- Base on `korangar_interface` (see existing `dialog.rs`, `hotbar.rs`, `status_bar.rs`).
- Use declarative `window! { ... split! { ... } }` macros.
- For reactive lists (initiative, dice history): bind to `Vec` in DmState via paths.
- Dice cards: treat like floating particles or dedicated overlay window. Support physics-lite animation if desired (or simple timed cards).
- GM-only windows: gate creation/open on GM level (available via character or server echo). Use `WindowClass` for management.
- HUD edit mode (Phase 2 foundation): every new HUD element (initiative bar, inspiration) must participate in the layout system.

See specs/buff-bar-slice.md for an end-to-end slice pattern (packet → event → state → widget → tick).

Command palette (mentioned in roadmap): can be a searchable overlay that knows the @dm verbs (static list from generated data) and emits via the command emitter.

## 9. Build-Time Campaign Data

- Server side owns `CAMPAIGN.md`, `planning/`, `campaign_quest_journal_entries.lua`, mob/quest defs.
- Client needs a small codegen step (similar to `tools/generate_packet_lengths.sh`):
  - Parse relevant pieces → `dm/data/campaign.rs` (acts/arcs/beats tree, quest ID ranges 20000-20234, recommended mob job_ids for encounter palette).
  - Run at `cargo build` time for the korangar crate (see existing `build.rs`).
- Quest journal can merge the lub data or the lua source.

Keep generation hermetic so rebase of upstream Korangar doesn't fight it.

## 10. Current Status Snapshot (as of latest docs + code audit)

- **Server**: Fully mapped (DM_SERVER_FUNCTIONS + live scripts).
- **Packets / events**: Excellent coverage via PACKET_EVENTS_CATALOG + recent quest/status work + length fallbacks.
- **World / visuals**: Quest effects + particles ready for markers/hazards; entities solid.
- **DM code in client**: None yet (no `dm/` dir, no DM fields in ClientState, no dm/ windows — all standard Korangar windows present).
- **M0 connectivity**: Advanced (per plans/M0-connectivity.md: login/char working, map load in progress).
- **Party risk**: Explicitly called out; dedicated plan exists.
- **Recent client work**: Status effects + bar slice, quest icon render fix, packet catalogue, various deep technical docs.
- **Gaps for full E7**: This doc + concrete first slices (dice cards or journal are lowest risk starters). Phase A parser, command emitter, generated data, GM gating.

MVP player-facing pieces (dice cards, journal, inspiration) can be built against chat + QuestEffect + existing quest packets without waiting for full party or DM console.

## 11. Recommended First Implementation Order (after M1)

1. E7.1 (server [DMJ] echo) — script change only.
2. E7.2 + E7.3 (dice cards + journal) — parse chat, minimal state, simple widgets. Uses existing ChatMessage + quest data.
3. Wire parser + emitter in lib.rs (small surface).
4. E7.4 (inspiration/downed indicators).
5. Promote critical noops (party + more quests/status) per packet-gap + FEATURE §8.3.
6. DM-only windows (check console, board) gated behind a debug/GM flag.
7. In-world hazard polish.
8. Phase B custom packets only if chat parsing becomes painful under load.

## 12. Cross-References & Further Reading

- Design: DM_INTERFACE.md, FEATURE_ROADMAP.md §8 (esp. 8.2 principles + 8.3 backlog), PROJECT_PLAN.md E7.
- Server: DM_SERVER_FUNCTIONS.md (full command table + script modules).
- Wire: PACKET_EVENTS_CATALOG.md (especially Chat family, QuestEffectPacket, AddEntity), plans/packet-gap-party-whisper.md, protocol/*.
- World/Visuals: WORLD_MAPS_ENTITIES.md (AddQuestEffect handling, particles), GRAPHICS_PIPELINE.md (effects, lighting for scenes).
- State/UI patterns: CLIENT_SYSTEMS_OVERVIEW.md §4-5, specs/buff-bar-slice.md (template), existing windows (dialog, status_bar, chat).
- Assets/Customs: plans/asset-pipeline.md.
- Base architecture: SOFTWARE_DESIGN.md.

Update this guide (and the roadmaps) as slices land. When implementing the first DM window, start by copying patterns from `status_bar.rs` + `dialog.rs` and route through a thin `dm/` shim.

This, combined with the other deep technical maps created for the project, provides the concrete "how" to turn the DM campaign vision into working, maintainable code.