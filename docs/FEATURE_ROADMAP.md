# Feature Roadmap — Custom Ragnarok Online Client

| | |
|---|---|
| **Status** | Living roadmap |
| **Architecture** | [SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) |
| **Work plan** | [PROJECT_PLAN.md](PROJECT_PLAN.md) |
| **DM tooling design** | [DM_INTERFACE.md](DM_INTERFACE.md) |
| **DM client implementation** | [DM_CLIENT_IMPLEMENTATION.md](DM_CLIENT_IMPLEMENTATION.md) |

This document owns the feature roadmap, UI/UX principles, and packet-handler
promotion backlog for the Korangar-based HerculesRO client. The architecture and
protocol contract stay in [SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md).

## 8. Feature Roadmap

### Phase 0 — Baseline connectivity

Implementation plan: [plans/M0-connectivity.md](plans/M0-connectivity.md)

- [ ] Rebuild Hercules with `PACKETVER=20220406` to resolve the protocol mismatch
      ([SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) §5).
- [ ] **Disable packet obfuscation:** set `packet_obfuscation: 0` in
      `conf/import/battle.conf` (currently forced on at `2`;
      [SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) §5.1) — Korangar can't connect otherwise.
- [ ] Symlink or copy `data.grf`, `rdata.grf`, `renewal2021.grf`, and
      `resources2021.grf` from `/mnt/h/RO/client/` to the Korangar client directory
      ([SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) §6.2).
- [ ] Update `korangar/archive/data/sclientinfo.xml` to point to `127.0.0.1` and ensure it can connect locally.
- [ ] (Optional for LAN) Populate `conf/import/char-server.conf` and `map-server.conf` on Hercules with `char_ip`/`map_ip`.
- [ ] Login → char create → walk around Prontera. **This is the milestone demo.**

### Phase 1 — Playability parity
- [ ] Verify combat, NPC dialogue, warps, inventory, storage against Hercules.
- [ ] Catalog Korangar's missing features (it is pre-alpha) that block play; file issues.
- [ ] **Promote noop packet handlers → real handlers** per the prioritized backlog in **§8.3** (start with the three MVP rows: status/buffs, skill-damage feedback, stats).
- [ ] Close the party/whisper framing risk before the first group session
      ([plans/packet-gap-party-whisper.md](plans/packet-gap-party-whisper.md)).

### Phase 2 — Customization

> **Scope note:** this client targets a small group of friends on a private,
> DM-run server, not a public shard. *Automated* economy/mass-social features
> (auction house, market board, vending/trade brokering, LFG matchmaking) are
> intentionally **out of scope** — the group coordinates directly and the DM
> controls loot ([DM_INTERFACE.md](DM_INTERFACE.md)). **Direct 1-on-1 player trade is explicitly *in* scope**,
> though — it's how the group passes DM-distributed loot around, and it is *not*
> the same thing as the brokering features above. UX effort goes toward combat
> readability, party cohesion, trading, and the campaign, not scale. **The design principles governing every widget below are in §8.2.**
>
> **"Feels modern" MVP cut** — the smallest set that flips perception from *RO clone*
> to *modern client*: HUD edit mode · party frames · floating combat text · modern
> inventory · modern NPC dialogue. Ship these first; everything else is polish.
>
> **Considered and rejected** (2026-07-05, so they aren't re-proposed): a **macro
> system** — the command palette + keybind profiles cover the friends-group need,
> full macros are public-server scope creep; an **in-client item/mob database
> browser** — smart tooltips + the campaign journal cover it, and with 4 players
> the DM answers faster than a wiki UI would.

- [ ] **Login & connection:**
  - **Custom Server IP Input:** An input field on the login screen to manually specify the server IP and port, allowing users to connect to any server without editing `sclientinfo.xml`.
  - **Streamer/privacy mode:** hide player names and the custom server IP for screenshots/streams.

- [ ] **Foundation — UI framework:**
  - Scalable UI framework (resolution independence for 1440p/4K), with per-element UI scale.
  - Modern typography and window management (snapping, docking, locking).
  - **HUD edit mode:** a layout editor to drag/scale/lock every HUD element, with per-character saved profiles (cf. WoW Edit Mode / FFXIV HUD Layout). Generalizes the snapping/docking work above and is the substrate every widget below sits on — **build this first.**
  - Targeted spec: [specs/hud-edit-mode.md](specs/hud-edit-mode.md)

- [ ] **Combat feedback & readability:**
  - Modern action bars with keybind overlays and smooth cooldown animations.
  - **Floating combat text:** scrolling damage/heal numbers with crit emphasis, customizable position/scale/filtering.
  - **Nameplates:** overhead health bars + cast bars + aggro coloring. Supersedes the entity-outline idea below (keep hover outlines as a cheaper fallback).
  - Entity outlines on hover (red for enemies, green for players) to clarify chaotic combat.
  - **Buff/debuff bars** with duration timers, sorting, and right-click-to-cancel. *First slice specced: [specs/buff-bar-slice.md](specs/buff-bar-slice.md) — the template build for §8.3 promotions.*
  - **Cast bars** (self + target) with interrupt/uninterruptible indicators.
  - **Target / target-of-target / focus frames.**

- [ ] **Core windows — RO daily-drivers** *(extend existing Korangar windows, not build from scratch; surveyed 2026-07-05)*:
  - **Inventory & storage** — extend `inventory.rs` (today a bare ~58-line grid) with
    search, sort, category tabs, weight display, item lock, and quick-actions.
    *Drag-and-drop between windows already works* (`MouseInputMode::MoveItem` +
    `DropHandler` in `interface/components/item_box.rs`) — reuse it, don't rebuild it.
  - **Equipment & character sheet** — unify the separate `equipment.rs` (equip doll)
    and `stats.rs` into one character sheet with a real stat breakdown. This is where
    the **Smart Tooltips** stat-comparison (below) actually lives.
    - **Equipment Loadouts:** a one-click dropdown to save and swap between 3-4 gear profiles (e.g., "Combat", "Exploration"). Korangar loops over saved item IDs and sends `RequestEquipItemPacket`s.
    - **Transmogrification / Fashion Slots:** a "Costume" tab where dropping an item overrides the visible sprite/model of that slot without affecting stats (pure client-side visual override during entity draw).
  - **Skill tree** — extend `interface/windows/skill_tree/` (already has tabs +
    drag-to-hotbar via `DropSkillWrapper`) with search/filter and a build planner.
    *Interaction design:* Point allocation is done via explicit, immediate-feedback upgrade buttons next to each skill. Skill usage heavily favors modern drag-and-drop from the tree to the Action Bars over legacy double-click casting.
  - **NPC dialogue** — modernize `dialog.rs` (~258 lines): readable panel, portrait,
    highlighted/numbered choices. Doubles as the presentation home for [DM_INTERFACE.md](DM_INTERFACE.md)'s
    scene-director narration (`@dmsay` / `@dmcutscene` / `@dmspotlight`).

- [ ] **Interaction primitives** *(cross-window; make the whole UI feel modern)*:
  - **Drag-and-drop** — *already exists* (`MouseInputMode::MoveItem` + `DropHandler`,
    `interface/components/item_box.rs` / `skill_box.rs`); reuse across all windows.
  - **Right-click context menus** — RO leans on awkward click-modes; a shared context
    menu is a big modern win.
  - **Search** as a reusable component (inventory, storage, skills).
  - **Unicode-correct text** — enable/verify the `unicode` cargo feature (§3.1) so
    legacy RO encoding isn't a dated tell.

- [ ] **Party & session cohesion** *(ties into [DM_INTERFACE.md](DM_INTERFACE.md))*:
  - **Party/raid frames:** health/SP bars, class/role icons, **AFK/idle indicator**, range and out-of-line-of-sight indicators, click-to-target. Shares rendering with [DM_INTERFACE.md](DM_INTERFACE.md)'s initiative tracker and downed overlay. Protocol foundation exists now (`ragnarok-packets` + `NetworkEvent` + hidden `party_state`); the visible frame UI is still greenfield and needs live two-character validation first.
  - **Structured combat log** with event filtering — consumes the `[DMJ]` structured echo ([DM_INTERFACE.md](DM_INTERFACE.md) §9.3); doubles as the DM's verification surface for checks and damage.
  - **End-of-Encounter Recap:** when the DM finishes an encounter, pop up a quick "Encounter Summary" tab showing who took the most damage, who healed the most, and MVP actions.

- [ ] **Player trade** *(in scope — see scope note; distinct from brokering)*:
  - **Secure trade window:** drag items in, a zeny field, and **both-confirm**; on
    confirm the offer **locks and any last-second change is highlighted** — P13
    error-prevention applied (the classic RO trade-scam guard; still worth it among
    friends to catch honest mistakes). Main path for passing DM-distributed loot
    around ([DM_INTERFACE.md](DM_INTERFACE.md)).
  - **Build note — this is greenfield protocol work, not "extend."** Unlike the
    Core windows, trade is *unimplemented* in Korangar: the `0x00E4`–`0x00F0` trade
    packet family isn't defined in `ragnarok-packets`, so the client can't even
    *frame* an incoming trade request today (§5.3). Work: define the trade packets →
    register handlers in `version_20220406.rs` → add send methods in
    `korangar-networking/src/lib.rs` → build the window. **Hercules supports player
    trade natively**, so it's *client-only*. **Sits post-MVP** (§8.2/§8.3 MVP is
    combat-legibility first); the scope-note fix lands now, the window after.

- [ ] **Group play & session conveniences** *(client-only or already server-supported — cheap, high value for game nights)*:
  - **Auto-reconnect:** on a dropped connection, re-run login → char → map with
    cached credentials/selection and back-off, so a Wi-Fi hiccup doesn't end the
    session. Promotes the §5.3 reconnect *gap* to a real planned feature. *Client-only.*
  - **Auto-follow:** follow the DM / party lead. RO has no server-side follow, so
    it's client-side movement automation — repeatedly path to the target's tile via
    the existing move packets (§5.2). *Client-only, emergent.*
  - **Auto-loot toggle + loot-all / pickup radius:** `@autoloot` **already exists
    server-side** in Hercules — the client just needs a toggle/UI, not new mechanics.
  - **Player-side fast-travel / return:** quick warp to save point / known maps —
    players have no equivalent to the DM's `@dmwarp` / `@dmrecall` ([DM_INTERFACE.md](DM_INTERFACE.md) §9.1).
  - **Whisper reply (`/r`) + whisper history** in the tabbed chat (Information & communication).
  - **Death recap:** a focused "what killed me" view built from the structured
    combat log (Party & session cohesion).
  - **Discord Rich Presence:** dynamic status (e.g., "In Combat: Campaign Boss", "Exploring Prontera") by hooking the client into the Discord Game SDK and reading the `$dm_active_party` or `[DMJ]` echoes.
  - **Photo Mode (Group Screenshots):** a dedicated command to hide all UI, slightly detach the orbital camera for players, and apply depth of field for capturing campaign memories.
  - *Verify, don't build:* overhead **chat bubbles** likely already render
    (`OverheadMessagePacket` / `EntityMessagePacket` are handled, §5.2) — confirm first.

- [ ] **Shared party coordination** *(all ride one shared-state transport — see note)*:
  - **Ready check:** one-button "everyone ready?" before an encounter — broadcast
    query + collected replies.
  - **Raid / target markers:** mark a mob (skull/star) as "kill this first," visible
    to the whole party. Complements the [DM_INTERFACE.md](DM_INTERFACE.md) initiative / encounter panel.
  - **Assist targeting:** target the DM's / lead's current target.
  - **⚠ Transport dependency:** these three **plus the existing shared map ping**
    ([DM_INTERFACE.md](DM_INTERFACE.md) DM tooling) all need the *same* thing — a channel to sync small bits of party
    state between clients. Build it **once** ([DM_INTERFACE.md](DM_INTERFACE.md) §9.3 Phase A structured `[DMJ]` echo →
    Phase B custom packets) and ping + ready-check + markers + assist all come online
    together. Don't scatter separate transports.

- [ ] **Information & communication:**
  - **Tabbed, resizable chat** with item hyperlinks (hover → tooltip), timestamps, per-channel filters, and `/command` autocomplete.
    - **Chat Text-Selection & Copy/Paste:** highlight text inside the chat log to copy it, standard `Ctrl+C/V` support in the chat input, and `Shift-Clicking` an item/quest to paste a clickable hyperlink into chat.
  - **Toast/notification system:** level up, quest complete, loot, campaign beats — replacing chat-spam feedback (same philosophy as [DM_INTERFACE.md](DM_INTERFACE.md)'s dice cards).
  - **Command palette:** searchable overlay exposing client actions and the many `@dm…`/`@roll` commands ([DM_INTERFACE.md](DM_INTERFACE.md) §9.1) without memorization.

- [ ] **Navigation & quests:**
  - **Cross-Map Quest Guiding & Breadcrumbs:** A macro-level pathfinding system using a world graph of warp portals. When tracking a quest on another map, a ground-level glowing trail (breadcrumb ribbon) and minimap edge-arrows will guide you seamlessly through multiple maps and warp portals directly to the objective.
  - **Clickable NPC navigation links:** Parse RO dialogue navigation markup such as
    `<NAVI>[Hun]<INFO>izlude,122,207,</INFO></NAVI>` into clickable quest
    breadcrumbs. Same-map links should walk or mark the target directly; cross-map
    links should feed the breadcrumb route system above.
  - Targeted spec: [specs/navigation-quest-guiding.md](specs/navigation-quest-guiding.md)
    - Parse dialog text into structured segments: plain text, navigation label,
      destination map, and destination tile.
    - Render navigation labels as clickable dialog UI. Start with a small generated
      button beside/below the sentence; later replace it with true inline clickable
      text.
    - Add visual feedback when a navigation link is activated: walk indicator,
      ground marker, minimap marker, and eventually world-map support.
  - **Navigational Aids:** 3D floating markers over NPCs (`!` / `?`), minimap objective radiuses, and custom map waypoints.
  - **Enhanced minimap/world map:** zoom, tracking filters, shared party pings.

- [ ] **Input & accessibility:**
  - Full **keybind remap screen** with profiles (import/export).
  - **Gamepad/controller support** + virtual cursor.
    - **Radial Menus / Quick Wheels:** holding a bumper/button opens a modern radial wheel to quickly select potions, mounts, or out-of-combat tools without cluttering the main action bars.
  - **Accessibility:** colorblind-friendly modes, high-contrast text, opacity sliders, reduced-motion/screen-shake toggle, cutscene subtitles/captions, font scaling.
  - **Perf overlay:** FPS / latency / netgraph.

- [ ] **Tooltips & onboarding:**
  - **Smart Tooltips:** detailed item comparisons showing exact stat differences.
  - **Contextual tutorial overlays** for the group's first sessions.

- [ ] **Custom content pipeline** ([SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) §6.2): define packing/sync process.
  - Implementation plan: [plans/asset-pipeline.md](plans/asset-pipeline.md)

- [ ] **DM Tooling Expansions** *(see [DM_INTERFACE.md](DM_INTERFACE.md) for the full DM interface)*:
  - DM Free-Cam/Spectator mode to detach camera for scene surveying.
  - Context-aware map ping system for DMs and players (e.g., "Danger", "Move here").

- [ ] **Tabletop & Action Mechanics (Modernization)**:
  - **Integrated Skill Check Dialogue:** NPC dialogue options automatically detect skill checks (e.g., `[Charisma DC 15]`) and trigger the dice-roll UI inline, rather than requiring separate chat commands.
  - **Active Dodge Roll / Dash (Long-term Extension):** Pushing the engine toward a true Action RPG. A dedicated evasion keybind (`Spacebar`) providing a brief movement burst and i-frames to actively avoid hazard telegraphs. *Note: Requires heavy custom C-plugin work on the Hercules server to handle coordinate snapping and i-frames without rubber-banding.*
  - **Campfire / Short Rest System:** A deployable physical campfire where the party can sit to rapidly recover HP/SP, serving as a roleplay anchor.
  - **Dynamic Bestiary Journal:** A monster manual that unlocks exact HP, weaknesses, and lore for a creature only after fighting it or passing a DM Lore check. See `specs/bestiary-journal.md`.
  - **Action Camera (WASD Movement):** A toggle to lock the camera third-person, mapping movement to WASD and auto-attacks to left-click, completely removing the point-and-click requirement.
  - Detailed architecture in [plans/modern-mechanics.md](plans/modern-mechanics.md) §1 (plus related sections for dodge, gamepad, etc.).

### 8.1 Server-side dependencies

Most Phase 2 UI features are **pure client-side reskins** of data Korangar already
receives over the `20220406` protocol (damage, skill-cast, status-change, party,
and quest packets) — they need **no server change**. Verified against
`~/GitHub/Hercules_RO` on 2026-07-05:

**Already configured — no change needed:**
| Feature | Config | Current value |
|---|---|---|
| Party frames (member HP/SP) | `conf/map/battle/party.conf` → `party_hp_mode` | `0` (Aegis; updates on every HP change) ✅ |
| Nameplate mob HP bars | `conf/map/battle/monster.conf` → `show_monster_hp_bar` | `1` (all except Emperium/WoE/MVP) ✅ — set `7` to include MVP + WoE, and/or `show_mob_info: 1\|2` for text HP |

**Requires server work:**
| Feature | Current state | Change |
|---|---|---|
| Any map-server connection (blocks everything) | `packet_obfuscation: 2` in `conf/map/battle/client.conf`, no override | Set `0` in `conf/import/battle.conf` — Phase 0 blocker ([SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) §5, this doc §8). Decision to disable rather than implement client-side: [SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) §5.1 |
| Structured combat log, player dice cards, all DM windows | `$dm_client_mode` / `[DMJ]` echo **not implemented** in `npc/custom/dm_campaign/` | Script-only structured echo ([DM_INTERFACE.md](DM_INTERFACE.md) §9.3 Phase A) |
| Shared map ping ("Danger" / "Move here") | no transport | Custom packet (Phase B) or piggyback party chat |
| Live high-frequency state (initiative order, hazard ticks, encounter HP) | none | Custom packets in `ragnarok-packets` + Hercules plugin ([DM_INTERFACE.md](DM_INTERFACE.md) §9.3 Phase B) |
| Campaign custom items/mobs the UI references | server DB/script (already exists) | keep client-side DB in sync ([SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) §6.2) |

### 8.2 UI/UX design principles *(governing Phase 2 & DM tooling)*

Rules that constrain every widget in this roadmap and [DM_INTERFACE.md](DM_INTERFACE.md). Distilled from general game-UI
practice and applied to an RO + tabletop context. **Core thesis** — and the reason
[DM_INTERFACE.md](DM_INTERFACE.md) exists: the official client dumps state into chat and onto a fixed HUD; our
value is showing the *right* thing, in the *right* place, *only when it matters*.

**A — Does it need UI at all? (Signs & Feedback)**
- **P1. UI is not the only feedback channel.** Before adding a panel/icon, weigh
  in-world VFX, SFX, animation, or camera. E.g. hazard telegraphs ([DM_INTERFACE.md](DM_INTERFACE.md) §9.3) are drawn
  in-world, not as a HUD warning; a downed state can pair a screen-edge vignette
  with the overlay.
- **P2. Don't show everything all the time.** Context-gate visibility: mob-HP
  nameplates and buff/debuff bars appear in combat and fade otherwise; party
  frames stay (party is the campaign's core); DM windows only at GM level.

**B — The four UI categories (choose per element; mix freely):**
- **Non-diegetic** (overlay, full control): action bars, party/raid frames,
  minimap, HUD, toasts.
- **Spatial** (in-world, not fiction): floating combat text, nameplates, hazard
  telegraphs, NPC `!`/`?` markers, ground pathfinding arrows, map pings.
- **Meta** (overlay, but in fiction): render **dice cards** as physical tabletop
  dice/cards and the **campaign journal** as an in-world tome — reinforces the DnD
  fantasy instead of reading as generic HUD.
- **Diegetic** (in-world + in fiction): rare in RO; equipped-sprite changes already
  qualify. Don't force it — it's expensive and usually doesn't fit (§ref: Dead Space myth).

**C — Layout & attention:**
- **P3. Protect the critical focus area** — the player + immediate combat ring.
  Toasts, notifications, and the initiative bar must never occlude it; reserve
  corners/edges for permanent UI. Enforce via the HUD edit-mode grid (§8 Foundation).
- **P4. Minimize eye travel; design for player *flows*, not just screen position.**
  Party frames near the action; buff timers near the character; initiative order +
  whose-turn adjacent; dice-card results where the eye already is during a check.
- **P5. Place accents — one clear emphasis per state change** (crit flair on combat
  text, nat-20/nat-1 flair on dice cards, bloodied indicator). *"If everything is
  important, nothing is."*
- **P6. Group related elements, general → specific.**

**D — Consistency & restraint:**
- **P7. One reusable widget system, not per-window art.** Shared tiles/headers/
  lists/buttons built on `korangar-interface`; HUD edit-mode profiles depend on it.
- **P8. Respect familiar RO conventions (Jakob's Law).** "Modern" redesigns of
  inventory/hotbar/stat windows keep the layout RO players expect — innovate on
  feedback and readability, not on relearning muscle memory.
- **P9. Ergonomics over ornament.** RO's ornate aesthetic is welcome only where it
  doesn't steal functional space or legibility.

**E — Robustness:**
- **P10. Design for the worst case.** Bound every variable system so it can't cover
  critical UI: max simultaneous toasts, chat under spam, a long initiative list,
  stacked hazards, a full buff row. This is precisely the [DM_INTERFACE.md](DM_INTERFACE.md) §9.2 problem — the fix is
  bounded, structured widgets, not free text.

**F — Hierarchy, feedback, safety, testing:**
- **P11. Visual hierarchy.** Lead the eye with size, contrast, and whitespace — the
  biggest / highest-contrast element is the one that matters *right now*. (Extends
  P3/P5.)
- **P12. Immediate feedback on every action.** Every input gets instant
  confirmation — button press, cooldown sweep, hit flash, sound. A UI is a
  conversation, not a monologue. (Extends P1.)
- **P13. Error prevention.** Confirm destructive actions and disable invalid ones —
  DM `@dm reset`/`@dm cleanup`, character delete, item sell — so a misclick never
  breaks flow. (Extends P10.)
- **P14. Test with the table.** Usability-test with the actual friends group and
  iterate; on a private server *the group is the QA pool*. (→ §10.)

Accessibility and flexibility (colorblind-safe palettes, scalable text, gamepad
support, toggle-able HUD via edit mode) are realized directly by the §8 feature
groups rather than restated as principles here.

**Review trick:** overpaint the entire UI in one ugly bright color and ask "is the
game still legible and playable?" Use during Phase 2 UI review to catch clutter.

### 8.3 Packet-handler backlog (noop → real) — the Phase 1 gap catalog

`version_20220406.rs` registers **51** map/char packets with `register_noop`
(§5.2/§5.3): parsed for correct framing, but their data is dropped. They are
**framing-safe today** — this backlog is which to promote to real handlers to
unlock Phase 2 features, *not* a correctness list (that would be *unregistered*
packets). Grouped by feature area; classifications are **by packet name** and
should be confirmed against `ragnarok-packets` before implementing.

| Priority | Feature area → unlocks | noop packets to promote |
|---|---|---|
| **MVP** | Buffs/debuffs (timed) → buff bars (§8 Combat) | `StatusChangePacket`, `StatusChangeSequencePacket` — see [specs/buff-bar-slice.md](specs/buff-bar-slice.md). **Note:** `StateChangePacket` is *not* a buff packet (option-flags, moved to World/map below). |
| **MVP** | Skill/damage feedback → floating combat text, cooldowns (§8 Combat) | `DisplaySkillEffectAndDamagePacket`, `DisplaySkillCooldownPacket`, `DisplaySpecialEffectPacket`, `DisplayPlayerHealEffect`, `UseSkillSuccessPacket`, `ToUseSkillSuccessPacket`, `NotifyGroundSkillPacket` |
| **MVP** | Stats → character sheet, stat window (§8 Core windows) | `ParameterChangePacket`, `RequestStatUpResponsePacket`, `CriticalWeightUpdatePacket`, `UpdateAttackRangePacket` |
| **High** | Quests → campaign journal ([DM_INTERFACE.md](DM_INTERFACE.md)), quest tracker (§8 Nav) | `QuestListPacket`, `QuestNotificationPacket1`, `QuestRemovedPacket`, `HuntingQuestNotificationPacket`, `HuntingQuestUpdateObjectivePacket`, `NavigateToMonsterPacket`, `MarkMinimapPositionPacket` |
| **High** | Party/social → party frames (§8), friend list | Party roster/HP/position/chat/whisper packets are promoted; remaining work is party-frame UI, live validation, and `FriendOnlineStatusPacket`. |
| **High** | Progression toasts → notification system (§8 Info) | `AchievementListPacket`, `AchievementUpdatePacket`, `DisplayGainedExperiencePacket` |
| **Med** | Equipment view → character sheet, equip-swap (§8 Core windows) | `UpdateShowEquipPacket`, `EquippableSwitchItemListPacket`, `EquipAmmunitionPacket`, `AmmunitionActionPacket` |
| **Med** | Char-select screen polish | `CharacterListPacket`, `CharacterSlotPagePacket`, `CharacterBanListPacket`, `LoginPincodePacket` |
| **Med** | World/map + entity option-state (sit/cloak/PK/effect visuals) | `StateChangePacket` (option-flags, *not* buffs), `ChangeMapCellPacket`, `MapTypePacket`, `EntityStopMovePacket`, `DisplayEmotionPacket`, `DisplayImagePacket` |
| **Low** | Clan/reputation (mostly N/A for a friends group) | `ClanInfoPacket`, `ClanOnlineCountPacket`, `ReputationPacket` |
| **Low** | Config/misc | `UpdateConfigurationPacket`, `MessageTablePacket`, `ConnectionRefusedPacket` |
| **Out of scope** | Economy/mail (§8 scope note) — keep as noop | `OpenMarketPacket`, `NewMailStatusPacket` |
| **Keep noop** | Opaque, header-only framing placeholders | `Packet0b18` (registered in two handlers), `Packet8302` |

**Reading it:** the three MVP rows are the ones that make combat *legible* and are
the natural companions to the §8.2 MVP cut. `StatusChangePacket` in particular is a
prerequisite for buff/debuff bars — no buff timers without it.

---
