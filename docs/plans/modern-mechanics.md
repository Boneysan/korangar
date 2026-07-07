# Implementation Plan: Tabletop & Action RPG Mechanics

This document provides the technical engineering specifications for integrating modern Action RPG and Tabletop mechanics into the Korangar Rust client and the Hercules C server.

## 1. Action Camera (WASD Movement)
**Architecture:** Client-Side Input Translation (`korangar/src/input/` & `korangar/src/camera/`)

- **State Management:** Add a `CameraMode::Action` variant to `korangar::graphics::Camera`. When active, `winit` cursor gets locked (`CursorGrabMode::Confined` / `CursorGrabMode::Locked`) and set to invisible.
- **Mouse Look:** Map `winit::event::DeviceEvent::MouseMotion` directly to `camera.yaw` and `camera.pitch`.
- **Movement Vector Calculation:** 
  - On `W/A/S/D` `KeyboardInput`, calculate the 2D local movement vector.
  - Rotate the 2D vector by the `camera.yaw` matrix to get absolute world map coordinates (X, Y).
  - Raycast to the nearest walkable grid tile from the `korangar_collision` mesh.
- **Packet Throttling:** 
  - Do not spam `RequestPlayerMovePacket`. Maintain a 200ms `TickTimer`. If WASD is held, send a `RequestPlayerMovePacket` aiming 2-3 grid tiles ahead of the current walking direction to ensure smooth server-side pathing without hitting the packet flood limit.
- **Combat Binding:** Map `MouseButton::Left` to cast a raycast forward from the center of the screen to select the nearest `EntityId`. If valid, immediately serialize and push `RequestActionPacket` (Type: Attack) to the `korangar_networking` action channel.

## 2. Integrated Skill Check Dialogue
**Architecture:** Client Parsing (`korangar/src/interface/windows/dialog.rs`)

- **Packet Interception:** Listen for `DialogMenuPacket` (which carries Hercules' `select()` strings). 
- **String Parsing:** Split the packet's string by `:`. For each menu item, apply `Regex::new(r"^\[(?P<stat>\w+)\s+DC\s+(?P<dc>\d+)\]\s+(?P<text>.*)$")`.
- **UI Swapping:** If the regex matches, instanciate a `SkillCheckButton` widget instead of a standard `MenuButton`.
- **Event Flow:**
  1. `onClick` fires `NetworkEvent::DiceRollRequested { stat: caps["stat"], dc: caps["dc"].parse() }`.
  2. The `DiceCardManager` renders the 3D physics dice roll.
  3. Upon animation completion, if the local state resolves to Success, the client transmits `NextButtonPacket` containing `menu_id: matched_index` (the 0-indexed choice).
  *(Note: This creates a hard contract for the DM's script: `select("[Charisma DC 15] Lie:Tell Truth")` where case 1 is success and case 2 is fail).*

## 3. Campfire / Short Rest System
**Architecture:** Hercules Script (`npc/custom/`) + Client Graphics/Audio

- **Server-Side (`camp_script.txt`):**
  ```c
  // Pseudo-script structure
  OnUse:
    set .@cid, getcharid(0);
    // Spawn hidden NPC at user coordinates with campfire sprite (e.g. 1913)
    movenpc "Campfire#"+.@cid, strcharinfo(3), strcharinfo(1), strcharinfo(2);
    initnpctimer "Campfire#"+.@cid;
    end;

  OnTimer5000: // Every 5 seconds
    // Get all units in 5x5 radius
    getunits(BL_PC, .@units, 0, strcharinfo(3), strcharinfo(1), strcharinfo(2), 5);
    for(.@i = 0; .@i < getarraysize(.@units); .@i++) {
      if (checksitting(.@units[.@i])) {
        sc_start SC_REGEN, 5000, 3, 10, 0, .@units[.@i]; // Apply custom regen status
      }
    }
  ```
- **Client-Side:** In `korangar::entity::Entity::update()`, if `self.sprite_id == CAMPFIRE_SPRITE_ID`:
  - Push a `PointLight { color: vec3(1.0, 0.5, 0.1), range: 6.0 }` to the wgpu render pass.
  - Register a `SpatialAudioSource` playing the `campfire_loop.ogg` buffer, updating its 3D position relative to the camera each frame.

## 4. Dynamic Bestiary (Lore Checks)
**Architecture:** Hercules JSON Echo + `korangar/src/dm/` State

- **Server-Side State:** Unlocks are tracked using account variables (`#bestiary_unlock_<mob_id>`).
- **Data Sync:** On login, a script loops over unlocked variables and emits a structured `dispbottom` string.
  - `dispbottom "[DMJ]{\"action\":\"bestiary_sync\", \"ids\":[1002, 1011]}"`
- **Client Parsing:** `korangar::dm::parser::parse_dm_json` intercepts `ServerMessagePacket` / `GlobalMessagePacket` starting with `[DMJ]`.
- **UI Binding:** `korangar_interface::windows::BestiaryWindow` maps the `ids` array to `System/monsterinfo.lua` data blocks. If `MobId` is not in the `ids` array, render the stats payload as obfuscated `???` text.

## 5. Active Dodge Roll / Dash
**Architecture:** Hercules C-Plugin + Client Prediction (High Complexity)

- **Client Prediction:** 
  - On `winit` `Spacebar` keypress, lock client movement input.
  - Call `Entity::play_animation(AnimationState::Dash)`.
  - Calculate `dest_x` and `dest_y` 4 grid cells in the `camera.yaw` forward vector.
  - Send `RequestDodgeRollPacket { dest_x: u16, dest_y: u16, dir: u8 }` to the server via the map channel.
  - Interpolate the local entity model to `dest_x, dest_y` over 300ms using a bezier curve (do not rely on standard grid walk).
- **Server Plugin (`src/map/clif.c` & `src/map/unit.c`):**
  - Inject a packet handler for `RequestDodgeRollPacket` (e.g. opcode `0x0F00`).
  - **Validation:** Call `path_search()` or `map_getcell()` to ensure `dest_x/y` is walkable and within max dash range.
  - **Execution:** 
    - Immediately warp the unit silently `unit_warp(bl, mapid, dest_x, dest_y, CLR_TELEPORT)`.
    - Apply a custom status change: `sc_start(bl, bl, SC_DODGE_IFRAME, 100, 0, 500)`.
  - **I-Frame Logic:** In `battle.c` `battle_calc_damage()`, return 0 or `BCT_MISS` if the target has `SC_DODGE_IFRAME`.
- **Broadcast:** The server broadcasts an `EntityDodgePacket` to nearby clients, bypassing the standard `EntityMovePacket`, so remote clients know to render the fast 300ms bezier dash rather than a sudden teleport.

## 6. True Dialogue Tree (CRPG Style Window)
**Architecture:** Client-Side Presentation Rewrite (`korangar/src/interface/windows/dialog.rs`)
**Complexity:** Medium (No server C-code needed)

Ragnarok Online's server scripting engine is strictly linear (`mes` -> `next` -> `select`), clearing the screen constantly. We can fake a modern, branching CRPG dialogue tree (like *Disco Elysium* or *Baldur's Gate 3*) purely through client-side UI trickery.

- **Continuous Text Buffer:** 
  - Instead of overwriting the window's text state every time a `NpcDialogPacket` (`mes`) is received, the client appends the text to a scrolling `Vec<String>` buffer.
- **Handling `NextButtonPacket` (`next`):** 
  - Do not clear the screen when the player clicks "Next". Simply render a small inline "Continue..." prompt. Send the packet to the server, and let the server's next `mes` append directly below it.
- **Handling `DialogMenuPacket` (`select`):** 
  - Render the branching options at the bottom of the accumulated text block.
  - When the player clicks an option, append the chosen text to the buffer (e.g., `<color=#AABBCC>[Player]: "Tell me about the town."</color>`) before sending the response index to the server.
- **Handling `CloseButtonPacket` (`close`):** 
  - Provide an "End Conversation" button. Only clear the `Vec<String>` buffer completely when the window is actually destroyed.
- **Visuals:** Add a character portrait panel to the left of the scrolling text (mapping the NPC's `sprite_id` to a high-res 2D portrait). The result completely hides the archaic scripting engine and feels like a modern branching narrative game.

## 7. UI Trickery (Client-Side Interception)
**Architecture:** Client UI Parsers (`korangar/src/interface/`)
**Complexity:** Low (Pure Client-Side)

To modernize the feel of the game without rewriting server-side logic, the client intercepts archaic server data packets and translates them into modern UI components:

- **Inventory Sorting (Bag Tabs):** 
  - *Data:* The server sends `RegularItemListPacket` (a single, unsorted array of items). 
  - *Trickery:* The client cross-references `iteminfo.lua`, filtering the array into categorized tabs (Weapons, Consumables, Quest Items) and rendering a modern tabbed bag system.
- **Loot Toast Notifications:** 
  - *Data:* The server sends `ItemPickupPacket`. 
  - *Trickery:* Instead of logging a standard chat line, the client suppresses the chat message and pushes a slide-in `ToastNotification` widget on the HUD with the item icon, name, and quantity.
- **Cast Bars:** 
  - *Data:* The server sends `UseSkillSuccessPacket` which includes a `delay_time` field (cast duration).
  - *Trickery:* The client attaches a dynamic `CastBarWidget` to the `Entity`'s nameplate, filling up over `delay_time` milliseconds to provide precise visual interrupt windows.
- **Chat-Parsed Quest Trackers:** 
  - *Data:* Server scripts often use `dispbottom` or `broadcast` for quest updates (e.g., "Goblins Defeated: 5/10").
  - *Trickery:* Apply a regex listener to `GlobalMessagePacket`. If a string matches an active quest objective format, suppress it from chat and update a persistent, transparent `QuestTrackerHUD` widget on the side of the screen.

## 8. Gamepad / Controller Integration
**Architecture:** Client Input Layer (`korangar/src/input/`) & UI Framework
**Complexity:** High (UI framework adjustments needed)

Making a top-down, click-heavy MMO playable on a controller is entirely possible by borrowing mechanics from games like *Final Fantasy XIV* and *Destiny*:

- **Movement & Combat (The "Cross Hotbar"):** 
  - Using the **Action Camera (WASD)** logic from Section 1, map the Left Thumbstick to character movement and Right Thumbstick to camera rotation.
  - Map face buttons (A, B, X, Y) to basic attacks or interactions.
  - Implement a "Cross Hotbar": Holding Left Trigger (LT) maps the 4 face buttons and 4 D-Pad directions to Action Bar slots 1-8. Holding Right Trigger (RT) maps them to slots 9-16.
- **UI Interaction (Virtual Cursor vs Focus):** 
  - *Virtual Cursor:* The easiest implementation. Pressing a specific button (e.g., Select/Share) unlocks the camera and turns the Left Thumbstick into a mouse cursor (with magnetism/friction when hovering over buttons), allowing the player to click on the skill tree or inventory using 'A'.
  - *D-Pad Focus Navigation:* A more native approach where pressing the D-Pad snaps "Focus" between adjacent UI elements. This requires extending `korangar_interface` widgets to support a `is_focused` state and rendering a highlight box around them.
- **Radial Quick-Wheels:** 
  - Holding a bumper (LB/RB) draws a radial circle on the screen. The Right Thumbstick selects a slice (e.g., Potions, Mounts, Emotes), and releasing the bumper executes the action.

## 9. Knowledge Gaps & Future Investigations
Before we commit code for these mechanics, we need to investigate how exactly Korangar handles a few underlying systems:
- **`korangar_interface` Extensibility:** Can we easily inject "Focus" states for controller D-Pad navigation without rewriting the core retained-mode UI loop?
- **Animation States:** For the Dodge Roll, does the official RO sprite format (`.spr`/`.act`) have unused animation frames we can hijack for a "dash", or do we need to speed up the walking animation dynamically?
- **Pathfinding Mesh:** For the Action Camera, we need to verify how fast `korangar_collision` can raycast from screen-center to the world grid to support rapid WASD-to-grid movement without tanking the framerate.

## 10. DM UI Command Pipeline
**Architecture:** DM UI (`korangar/src/interface/windows/dm/`) -> Chat Packet Transport
**Complexity:** Low (Zero server-protocol changes required)

The DM interface avoids the need for custom server-side C-plugins by using the existing Hercules `@command` structure as a headless transport layer (Phase A).

- **1. Command Generation (UI Layer):** 
  - The DM interacts with a custom `korangar_interface` window (e.g., the *Check Console* or *Encounter Panel*).
  - Example: The DM selects a target and clicks "Roll Charisma DC 15". The UI widget constructs a formatted string: `format!("@dm check {} Charisma 15", target_name)`.
- **2. Packet Transmission:** 
  - The UI dispatches a network action to send a `GlobalMessagePacket` (the standard chat packet) containing the constructed string. To the server, it looks exactly as if the DM typed the command into the chat box.
- **3. Server Processing (Hercules Script):** 
  - The Hercules server parses the `@dm` command and executes the corresponding logic in `npc/custom/dm_campaign/`. 
  - It generates the results and emits a structured JSON echo to the DM using `dispbottom` (e.g., `[DMJ]{"t":"check_result","success":true}`).
- **4. Client Interception & State Update:** 
  - The `korangar::dm::parser::parse_dm_json` intercepts any incoming `ServerMessagePacket` or `GlobalMessagePacket` that begins with `[DMJ]`. 
  - It **suppresses** the packet so it never renders in the DM's chat box.
  - It deserializes the JSON and updates the global `korangar::dm::DmState` (e.g., updating the Initiative Tracker list or marking a Campaign Beat as complete).
  - The DM's UI windows read from `DmState` and re-render instantly, creating the illusion of a native, hard-coded application.

## 11. DM State Tracking & Architecture
**Architecture:** `korangar/src/dm/state.rs`

To successfully run the "Seal Cascade" campaign, the `DmState` struct must track several distinct data models, populated exclusively by the `[DMJ]` packet intercepts:

- **Campaign Board & Decision Ledger:** 
  - *Data Model:* A tree of `Arc` -> `Beat`, and a `HashMap<FlagId, bool>` for decisions.
  - *Implementation:* The core campaign tree is compiled into the client statically (from `CAMPAIGN.md`). The server only sends `[DMJ]` payloads containing the IDs of completed beats and active flags (e.g. `[DMJ]{"t":"campaign_sync","completed_beats":[101,102],"flags":{"spared_goblin":true}}`).
- **Initiative Tracker:** 
  - *Data Model:* `Vec<InitiativeEntry>` (ordered).
  - *Implementation:* The DM clicks "Roll Initiative". The server script calculates AGI + d20 for the party and mobs, sorts them, and blasts the ordered array in a `[DMJ]` payload. The client renders this as a draggable, vertical overlay.
- **Encounter Panel (HP Tracking):** 
  - *Data Model:* `HashMap<EntityId, EncounterMob>`.
  - *Implementation:* The server broadcasts boss HP changes natively via `UpdateEntityHealthPointsPacket`. However, the DM interface tracks "Bloodied" thresholds and scaling percentages locally in this struct to visually alert the DM when a phase-transition should occur.
- **Hazard & Trap Board:** 
  - *Data Model:* `Vec<ActiveHazard> { id, x, y, radius, type }`.
  - *Implementation:* The UI provides a 2D map overlay. The DM clicks a coordinate, which fires `@dm hazard spawn x y 5 fire`. The server executes the hazard logic, and echoes the `[DMJ]` payload to all clients, allowing players to render the in-world spatial telegraphs (red circles) at `x, y` before the server actually deals damage.

## 12. Modern Quest Tracking & HUD
**Architecture:** `korangar/src/interface/windows/quest/` & HUD Layout
**Complexity:** Medium (UI + Server `[DMJ]` Sync)

Classic RO handles quests via convoluted item drops or invisible variables. This client completely replaces that with a modern, World of Warcraft or FFXIV style Quest Journal and HUD Tracker:

- **The Campaign Journal (Quest Log):** 
  - A beautiful, book-like UI window that categorizes quests into *Main Campaign*, *Side Quests*, and *Completed*. 
  - Data is driven locally by `tools/campaign_quest_merge.py` which pre-compiles the campaign quests into `OngoingQuestInfoList.lub`. 
  - The server sends standard `QuestListPacket` and `QuestNotificationPacket1` to update the active state.
- **Persistent HUD Tracker:** 
  - A transparent, click-through widget pinned to the right side of the screen displaying up to 5 tracked quests.
  - Updates in real-time (e.g., `[ ] Slay 10 Goblins (5/10)`). 
  - *Trickery:* For custom DM objectives that don't trigger native Quest Packets, the client regex-listens to the `GlobalMessagePacket` for specific strings or `[DMJ]` echoes to increment the progress bars silently.
- **3D World Integration:** 
  - The UI maps tracked quest coordinates to the minimap (rendering a golden tracking radius or waypoint).
  - Actively tracked quests will render glowing, in-world pathfinding ribbons leading the player toward the objective (as outlined in the Cross-Map Guiding roadmap).
