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

## Modernization Charter

This fork is authorized to make substantial improvements to controls, UI,
rendering, campaign presentation, accessibility, and DM/player workflows. The
official Ragnarok client is a compatibility baseline and content reference, not
a design ceiling. New systems may replace awkward official-client interaction
patterns when they produce a clearer and more enjoyable private game-night
experience.

Large improvements should follow these guardrails:

- Preserve Hercules as the authority for movement, combat, inventory, quests,
  permissions, and persistent state.
- Preserve a compatible fallback for core play where practical (for example,
  click-to-move remains available when WASD ships).
- Put disruptive behavior behind explicit settings and choose safe defaults.
- Do not reuse official shortcuts silently; use the compatibility matrix and a
  future remapping screen.
- Introduce protocol changes only after existing packets/scripts prove
  insufficient, and version both ends together when a custom packet is needed.
- Deliver large ideas as vertical slices with unit/protocol checks and live
  Hercules acceptance tests before expanding them across the product.
- Keep campaign and DM guidance data-driven so content does not become trapped
  inside client UI code.
- Update the roadmap and focused implementation spec as each slice lands.

In short: ambitious redesign is in scope; unbounded regressions are not.

## 8. Feature Roadmap

### Phase 0 — Baseline connectivity

Implementation plan: [plans/M0-connectivity.md](plans/M0-connectivity.md)

- [x] Rebuild Hercules with `PACKETVER=20220406` to resolve the protocol mismatch
      ([SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) §5).
- [x] **Disable packet obfuscation:** set `packet_obfuscation: 0` in
      `conf/import/battle.conf`
      ([SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) §5.1) — Korangar can't connect otherwise.
- [x] Symlink or copy `data.grf`, `rdata.grf`, `renewal2021.grf`, and
      `resources2021.grf` from `/mnt/h/RO/client/` to the Korangar client directory
      ([SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) §6.2). *(base GRFs still symlinked; 2021 GRFs copied to WSL)*
- [x] Update `korangar/archive/data/sclientinfo.xml` to point to `127.0.0.1` and ensure it can connect locally.
- [ ] (Optional for LAN) Populate `conf/import/char-server.conf` and `map-server.conf` on Hercules with `char_ip`/`map_ip`.
- [x] Login → char create → walk around Prontera. **This is the milestone demo.**

### Phase 1 — Playability parity

> **Every row below is checked, but Phase 1 is NOT done** — though the gap is narrower
> than the open rows in [plans/M1-p0-verification.md](plans/M1-p0-verification.md) §3
> make it look. Most of those rows (shop buy/sell, public chat, skill use, hotbar, stat
> allocation, identify, logout, char create/delete, NPC input) are **already green in the
> 106-scenario headless suite** — their wire protocol is proven; only the GUI pass is
> outstanding. What has genuinely **no** automated coverage is the UI-only set: buff bar
> render/expiry and the inventory weight footer. That plan's status is still
> *"In progress"*, and it — not this list — is the source of truth for GUI status.
>
> **Three different axes — do not conflate them.**
> 1. **Implemented?** → [PROJECT_PLAN.md](PROJECT_PLAN.md) §2's "Korangar" column (✅/🔶/❌).
> 2. **Wire protocol proven?** → the headless suite
>    ([tools/testing/headless_test_plan.md](../tools/testing/headless_test_plan.md)),
>    acceptance passed 2026-07-13.
> 3. **Proven through the GUI?** → [plans/M1-p0-verification.md](plans/M1-p0-verification.md).
>
> A feature can be ✅ implemented and headless-green and still have an unchecked GUI row.
> That combination means "the packets are right, the window hasn't been driven by hand" —
> it does **not** mean untested. Headless cannot reach the `korangar/src/` UI/state layer
> by construction.

- [x] Verify combat, NPC dialogue, warps, inventory, storage against Hercules.
      *All five live-verified on macOS 2026-07-10/11 — see
      [plans/M1-p0-verification.md](plans/M1-p0-verification.md) §3 for the per-row
      evidence: basic melee + damage numbers (Poring), NPC dialog `mes`/`next`/`close`
      + menu choices (Kafra), Kafra teleport warp, inventory open/use/equip/drop/split,
      and Kafra storage open/store/retrieve/close. Narrower gaps remain inside these
      areas and are tracked as their own rows in that checklist: skill damage numbers,
      NPC number/string `input` (E3.3), and shop buy/sell.*
- [x] Catalog Korangar's missing features (it is pre-alpha) that block play; file issues.
      *The catalog is [PROJECT_PLAN.md](PROJECT_PLAN.md) §2 (per-feature implementation
      status) plus the open rows in [plans/M1-p0-verification.md](plans/M1-p0-verification.md)
      §3. Defects are tracked in that plan's §5 defect log (M1-001…004, all closed), not
      as GitHub issues.*
- [x] **Promote noop packet handlers → real handlers** per the prioritized backlog in **§8.3** (start with the three MVP rows: status/buffs, skill-damage feedback, stats).
      *MVP rows promoted 2026-07-08/09: buffs (`StatusChangePacket`), skill damage
      (`DisplaySkillEffectAndDamagePacket` + player heal), stats (weight /
      `CriticalWeightUpdatePacket` / `ParameterChangePacket` / attack range /
      stat-up response). Remaining within those rows: cooldown bar UI,
      special-effect visuals, `StatusChangeSequencePacket`, zeny/exp display.*
- [x] Close the party/whisper framing risk before the first group session
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
  - **Mob skill-cast flavor lines:** no-damage skill casts from mobs/NPCs are
    currently invisible (the heal-number filter added 2026-07-13 intentionally
    suppresses them). `DisplaySkillEffectNoDamagePacket` carries caster + skill
    ID, so surface a flavor line ("The Whisper shimmers and fades away…") in
    chat or as overhead text — great DM-table atmosphere for AI casts like
    Cloaking, Hallucination, or buffs.
  - **Emote bubbles** *(implemented 2026-07-13)*: `DisplayEmotion` events play
    the matching `이팩트\emotion.spr`/`.act` action as a billboard at the
    entity (`world/emote.rs`, `AnimationData::render_action_frame`), playing
    once with the wire emote ID as the ACT action index. The chat line
    ("Sage Worm: hmph!") is retained as a log. Remaining polish: verify the
    ID→action alignment across the full emote range (dice, flags), and
    per-entity-height anchoring if tall bosses overlap their bubble.

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

- [x] **Player trade (protocol MVP, 2026-07-10)** *(in scope — distinct from brokering)*:
  - Packets + request/accept windows, lock/commit/cancel, `/trade` commands shipped.
  - **Still open:** drag-item grid, zeny field UI polish, last-second change highlight
    (P13 trade-scam guard), two-client live validation.

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
    *Base minimap (2026-07-10): **Alt+M** / Map button / Game Settings toggle
    (persisted `show_minimap`), resizable, top-right default, live player blip
    (`minimap\player_1.bmp`), Towninfo facility POIs (shops/kafra/guides via
    `System/Towninfo_EN.lub`). Hotbar shows **F1–F10** under slots.
    **Still deferred:** quest/compass/party markers — see
    [specs/navigation-quest-guiding.md](specs/navigation-quest-guiding.md)
    § "Follow-up — Minimap markers".*

- [ ] **Input & accessibility:**
  - **WASD character movement (P1 modernization):** support camera-relative
    keyboard movement alongside the existing click-to-move controls. Ship the
    keyboard-navigation MVP independently of the later third-person Action
    Camera so it can be tested and used sooner.
    - Modes: `Click`, `WASD`, or `Both`; default to `Both` for this project.
    - Disable character movement while chat, NPC input, or another text field
      has keyboard focus.
    - Support held movement and W+A/W+D/S+A/S+D diagonals using the existing
      walkable-tile pathfinder and server-authoritative movement packets.
    - Rotate directions relative to camera yaw; click-to-move remains intact.
    - Throttle/reuse destinations so held keys do not flood the map server.
    - Keep debug/free-camera WASD isolated to debug-camera mode.
    - Live acceptance: obstacles and corners, rapid reversals, diagonals, map
      changes, NPC interaction, combat chasing, latency, and a 30-minute mixed
      WASD/click session without rubber-banding or disconnects.
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

- [ ] **Aspirational Lighting & Atmosphere Modernization**
  ([GRAPHICS_PIPELINE.md](GRAPHICS_PIPELINE.md)):
  - **Artistic map profiles:** non-destructive overrides layered over RSW
    ambient/directional data for brightness, ambient balance, sun color, fog,
    saturation, and shadow softness.
  - **Campaign scene lighting:** Hercules/DM Beats can trigger interpolated
    dimming, seal colors, hazard pulses, boss blackouts, local lights, and
    victory restoration. Map change, cleanup, and reconnect always restore or
    reconstruct authoritative scene state.
  - **Lighting regression suite:** capture representative outdoor, forest,
    interior, dungeon, night, sprite-heavy, and many-light scenes before deeper
    renderer changes.
  - **HDR scene pipeline:** move world rendering to a linear floating-point
    target, remove premature light saturation, then add exposure and tone
    mapping. Keep UI compositing color-stable after scene presentation.
  - **Emissive + restrained bloom:** support opt-in emissive overlays for
    torches, portals, spells, runes, seals, hazards, and custom assets. Bloom is
    sourced from HDR/emissive intensity, never indiscriminately from UI.
  - **Contact-depth experiment:** evaluate subtle SSAO/contact shadows across
    RO sprites and low-poly maps; ship only if it improves depth without dirty
    halos or flattening painted art.
  - **Unified quality presets:** Low/Medium/High/Ultra cover existing shadow
    mode/detail/resolution plus shadowed-light budget and any future AO/bloom;
    choose backend-aware defaults.
  - **Optional custom material overlays:** legacy GRF content remains diffuse;
    custom campaign assets may opt into emissive/normal/roughness data through
    a documented packaging convention.
  - **Later generalization:** weather and time-of-day reuse the proven scene
    interpolation system after campaign lighting ships.
  - **Evidence gates:** the existing sRGB pipeline gets regression-tested, not
    presumed broken; shadow-priority changes require observed popping (basic
    hysteresis already exists); clustered lighting, full PBR, or IBL require a
    measured scene need before implementation.

- [ ] **Aspirational Modern Graphics Program**
  ([GRAPHICS_PIPELINE.md](GRAPHICS_PIPELINE.md),
  [WORLD_MAPS_ENTITIES.md](WORLD_MAPS_ENTITIES.md)):
  - **Compatibility rule:** every enhancement extends the current wgpu forward
    renderer and remains compatible with legacy RSW/GND/RSM/SPR/ACT/GRF data.
    New metadata and textures are optional overlays; missing overlays preserve
    current rendering.
  - **Atmosphere and depth:** finish `mapskydata.lub` sky/fog behavior, add
    height/distance fog and optional local volumes, and let campaign/weather
    profiles interpolate them through existing global/pass uniforms.
  - **Water modernization:** complete newer GND water support, then add
    depth-aware color, shoreline foam, reflection/refraction approximations,
    and quality-scaled distortion using the existing water forward drawer.
  - **Particles and effects:** improve batching, soft-particle depth fades,
    optional lighting/emissive response, deterministic quality budgets, and
    campaign-authored effect presets without replacing ACT/STR compatibility.
  - **Anti-aliasing and temporal stability:** retain MSAA/SSAA/FXAA; investigate
    SMAA and a carefully scoped temporal option only after motion-vector and
    sprite-ghosting tests. Add sharpening/upscaling as optional post-process
    passes, not as mandatory rendering paths.
  - **Outlines and selection readability:** depth/ID-buffer-aware hover,
    target, ally, hostile, interactable, and DM-selection outlines. Never rely
    on outline color alone; reuse the existing picker and forward data.
  - **Sprite/entity lighting modernization:** preserve `SPR`/`ACT` art while
    offering Classic, Soft/wrapped, and Enhanced lighting modes; validate light
    direction, luminance preservation, category-specific curvature, readable
    ambient floors, contact grounding, optional emissive overlays, and
    coverage-aware shadows. See `GRAPHICS_PIPELINE.md` “Sprite And Entity
    Lighting” for the current shader model and staged test plan.
  - **Decals and telegraphs:** projected or ground-aligned rings, cones, paths,
    footprints, spell marks, and hazard areas using bounded batched geometry or
    a focused decal pass. Gameplay remains server-authoritative.
  - **Camera presentation:** smooth optional follow, obstruction handling,
    cinematic rails/blends, depth-of-field only for deliberate cutscenes, and
    reduced-motion alternatives. Preserve the classic player camera.
  - **Animation presentation:** interpolate server-authoritative movement and
    facing, improve frame pacing and effect attachment, and add optional
    secondary presentation without changing SPR/ACT timing semantics used by
    gameplay.
  - **Environment motion:** quality-scaled wind response for foliage/cloth-like
    custom assets, rain/snow/ash layers, lightning, and surface reactions. Use
    existing model/particle instructions plus small new uniforms/passes.
  - **Texture quality:** preserve pixel-art modes while offering anisotropic
    filtering, high-quality mip generation, optional upscale/restoration
    overlays, and streaming/cache budgets. Never silently AI-replace original
    art or require enlarged assets.
  - **Transparency and effects polish:** build on existing WBOIT; audit sorting,
    additive effects, particles, water, and translucent shadows before adding
    specialized paths.
  - **Performance and scalability:** GPU/CPU timings per pass, representative
    benchmark maps, dynamic quality only when predictable, memory budgets, LOD
    and distance culling for map objects/effects, and backend-aware presets.
  - **Photo/debug tools:** screenshot supersampling, clean-HUD capture, lighting
    and pass toggles, frame captures, comparison screenshots, and visual
    regression baselines.
  - **Accessibility:** reduced flashes, reduced particles, reduced camera motion,
    effect-opacity controls, readable telegraph alternatives, and photosensitivity
    limits apply to every new visual system.
  - **Engine-feasibility gate:** prefer new uniforms, resources, compute jobs,
    or focused render/post passes within the existing instruction/context/drawer
    architecture. Do not plan ray tracing, virtualized geometry, a mandatory
    deferred renderer, or an engine replacement unless a future measured need
    justifies a separate architectural decision.

- [ ] **Tabletop & Action Mechanics (Modernization)**:
  - **Integrated Skill Check Dialogue:** NPC dialogue options automatically detect skill checks (e.g., `[Charisma DC 15]`) and trigger the dice-roll UI inline, rather than requiring separate chat commands.
  - **Active Dodge Roll / Dash (Long-term Extension):** Pushing the engine toward a true Action RPG. A dedicated evasion keybind (`Spacebar`) providing a brief movement burst and i-frames to actively avoid hazard telegraphs. *Note: Requires heavy custom C-plugin work on the Hercules server to handle coordinate snapping and i-frames without rubber-banding.*
  - **Campfire / Short Rest System:** A deployable physical campfire where the party can sit to rapidly recover HP/SP, serving as a roleplay anchor.
  - **Dynamic Bestiary Journal:** A monster manual that unlocks exact HP, weaknesses, and lore for a creature only after fighting it or passing a DM Lore check. See `specs/bestiary-journal.md`.
  - **Action Camera (WASD extension):** After the keyboard-navigation MVP is
    stable, add optional third-person mouse-look, cursor locking, center-screen
    targeting, and left-click attacks. This is an extension, not a prerequisite
    for basic WASD character movement.
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

**G — Accessibility, control, and comprehension:**
- **P15. Never encode meaning with color alone.** Pair color with an icon,
  label, shape, pattern, or motion-safe change. Test critical states in
  grayscale and with common color-vision simulations.
- **P16. Text and targets must remain usable.** Support scalable text/UI,
  readable contrast, line wrapping, and generous pointer targets. A 4K display,
  Retina scaling, a controller cursor, and a small laptop must not turn the same
  control into four different difficulty levels.
- **P17. Every primary flow has keyboard and pointer paths.** Focus order is
  visible and predictable; Escape backs out; Enter confirms only when safe;
  gameplay input never leaks through a focused text field or modal.
- **P18. Motion, audio, and effects reinforce rather than carry meaning.** Offer
  reduced motion/screen shake and independent UI/audio feedback controls.

**H — State, latency, and disclosure:**
- **P19. Make system state obvious.** Windows distinguish loading, empty,
  unavailable, stale, success, and error states. Never use a blank panel as an
  error message.
- **P20. Use progressive disclosure.** Show the common action first; place
  advanced options, raw IDs, recovery tools, and dangerous DM controls behind
  deliberate expansion. Complexity is available without being constantly
  visible.
- **P21. Acknowledge immediately, reconcile authoritatively.** Client actions
  may show pressed/pending feedback immediately, but Hercules confirmation owns
  the final state. Rejections explain what happened and how to recover; pending
  controls cannot be double-submitted accidentally.
- **P22. Preserve context.** Reopening a window restores useful tab, scroll,
  filter, size, and position state. Map changes and reconnects must not silently
  discard unfinished text or present stale server state as current.

**I — DM and campaign safety:**
- **P23. Separate preview, communication, and mutation.** Reading a Handbook
  cue is inert; sending narration is visible but does not advance state;
  committing a Beat previews the quests/flags/encounters it changes.
- **P24. Design for interruption and recovery.** A DM can inspect current state,
  identify the last committed beat, retry safe communication, and repair or
  roll back progression through explicit recovery tools.
- **P25. Protect player information.** GM-only notes, hidden rolls, unrevealed
  branches, server addresses, and private messages never appear in player
  surfaces, screenshots, tooltips, or public chat by accident.

**Review trick:** overpaint the entire UI in one ugly bright color and ask "is the
game still legible and playable?" Use during Phase 2 UI review to catch clutter.

#### UI definition of done

A new or substantially redesigned UI slice is not complete until it has:

- A named primary user, task, entry point, success state, and recovery path.
- Screens/states for normal, loading/pending, empty, disabled, rejected/error,
  overflow/worst-case, and reconnect/map-change behavior where applicable.
- Clear hierarchy at default scale and usable layout at the smallest supported
  window plus 1440p/4K scaling.
- Pointer and keyboard operation with correct focus ownership; controller
  operation when the feature claims controller support.
- No color-only meaning, clipped critical text, hidden destructive consequence,
  or gameplay input leaking through focused UI.
- Immediate local feedback followed by authoritative server reconciliation for
  network mutations.
- Reused design-system components/tokens rather than one-off spacing, colors,
  typography, or button behavior.
- A live test with representative content and at least one actual table member;
  record the finding in the relevant plan/spec.

### 8.3 Packet-handler backlog (noop → real) — the Phase 1 gap catalog

`version_20220406.rs` registers **51** map/char packets with `register_noop`
(§5.2/§5.3): parsed for correct framing, but their data is dropped. They are
**framing-safe today** — this backlog is which to promote to real handlers to
unlock Phase 2 features, *not* a correctness list (that would be *unregistered*
packets). Grouped by feature area; classifications are **by packet name** and
should be confirmed against `ragnarok-packets` before implementing.

| Priority | Feature area → unlocks | noop packets to promote |
|---|---|---|
| ~~**MVP**~~ **Done 2026-07-09** (core) | Buffs/debuffs (timed) → buff bars (§8 Combat) | `StatusChangePacket` promoted + `StatusEffects` state + MVP status bar window. Remaining: `StatusChangeSequencePacket`, real SC icons. See [specs/buff-bar-slice.md](specs/buff-bar-slice.md). **Note:** `StateChangePacket` is *not* a buff packet (option-flags, moved to World/map below). |
| ~~**MVP**~~ **Partial 2026-07-09** | Skill/damage feedback → floating combat text, cooldowns (§8 Combat) | **Done:** `DisplaySkillEffectAndDamagePacket` → floating numbers; `DisplayPlayerHealEffect` → heal numbers. **Still noop:** `DisplaySkillCooldownPacket` (hotbar polish), `DisplaySpecialEffectPacket`, `UseSkillSuccessPacket` (cast bars), `NotifyGroundSkillPacket`. Failure path of `ToUseSkillSuccessPacket` already promoted (server-feedback row). |
| ~~**MVP**~~ **Partial 2026-07-09** | Stats → character sheet, stat window (§8 Core windows) | **Done:** weight via `UpdateStat` + inventory footer (E3.5); `CriticalWeightUpdatePacket`; `ParameterChangePacket` → `UpdateStat`; `UpdateAttackRangePacket`; `RequestStatUpResponsePacket` failure → chat. **Still open:** zeny/exp HUD, full character-sheet stat breakdown. |
| **High** | Quests → campaign journal ([DM_INTERFACE.md](DM_INTERFACE.md)), quest tracker (§8 Nav) | `QuestListPacket`, `QuestNotificationPacket1`, `QuestRemovedPacket`, `HuntingQuestNotificationPacket`, `HuntingQuestUpdateObjectivePacket`, `NavigateToMonsterPacket`, `MarkMinimapPositionPacket` |
| **High** | Party/social → party frames (§8), friend list | Party roster/HP/position/chat/whisper packets are promoted **and live-validated two-client cross-map (2026-07-08)**; remaining work is party-frame UI and `FriendOnlineStatusPacket`. |
| ~~High~~ **Done 2026-07-08** (+ table 2026-07-10) | Server feedback → visible rejection messages | `MessageTablePacket` / `MessageTableColorPacket` → `NetworkEvent::MessageTable`; client resolves via `MsgStringTable` (`archive/data/msgstringtable.txt`, 0-based). Fixes boot id 3474 (`MSG_CHECK_ATTENDANCE_NOT_EVENT`). Skill-fail path still uses `skill_failed_text` in `version_20220406.rs`. |
| **High** | Progression toasts → notification system (§8 Info) | `AchievementListPacket`, `AchievementUpdatePacket`, `DisplayGainedExperiencePacket` |
| **Med** | Equipment view → character sheet, equip-swap (§8 Core windows) | `UpdateShowEquipPacket`, `EquippableSwitchItemListPacket`, `EquipAmmunitionPacket`, `AmmunitionActionPacket` |
| **Med** | Char-select screen polish | `CharacterListPacket`, `CharacterSlotPagePacket`, `CharacterBanListPacket`, `LoginPincodePacket` |
| **High** *(raised from Med 2026-07-15 — live evidence)* | World/map + entity option-state (sit/cloak/PK/effect visuals) | **`StateChangePacket` (option-flags, *not* buffs) — now the top row here, see [M1-007](plans/M1-p0-verification.md#5-defect-log).** Dropping it means `OPTION_HIDE`/`CLOAK`/`INVISIBLE` never reach the UI: no hide visuals at all, and hide-gated skills (`RG_RAID` requires `State: "Hiding"`) look broken with no way to diagnose. This makes Rogue effectively unplayable and misled a whole GUI session on 2026-07-15. Others: `ChangeMapCellPacket`, `MapTypePacket`, `EntityStopMovePacket`, `DisplayEmotionPacket`, `DisplayImagePacket` |
| **Low** | Clan/reputation (mostly N/A for a friends group) | `ClanInfoPacket`, `ClanOnlineCountPacket`, `ReputationPacket` |
| **Low** | Config/misc | `UpdateConfigurationPacket`, `ConnectionRefusedPacket` (`MessageTablePacket` promoted 2026-07-08, see server-feedback row) |
| **Out of scope** | Economy/mail (§8 scope note) — keep as noop | `OpenMarketPacket`, `NewMailStatusPacket` |
| **Keep noop** | Opaque, header-only framing placeholders | `Packet0b18` (registered in two handlers), `Packet8302` |

**Reading it:** the three MVP rows are the ones that make combat *legible* and are
the natural companions to the §8.2 MVP cut. `StatusChangePacket` in particular is a
prerequisite for buff/debuff bars — no buff timers without it.

---
