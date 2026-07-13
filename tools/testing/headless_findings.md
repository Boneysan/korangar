# Headless Client Tester — Findings, Investigations & Resolutions

This document logs historical findings, protocol discoveries, and resolutions encountered while running the Korangar headless client integration tests (`korangar-networking/examples/headless-tester`).

All findings are classified by layer:
* **Shared Crate**: Changes to `ragnarok-packets` or `korangar-networking` (directly affects the graphical client).
* **Client Test Harness**: Changes to test-runner logic (`scenarios/`, `context.rs`, etc.).
* **Server Emulator**: Changes to Hercules configuration, scripts, or databases.

---

## Summary of Recent Resolutions (July 2026)

The original 73 integration tests—including all 39 job class skill sweeps and
phase 6/7 item/dialogue loops—passed as a complete run. The expanded suite now
contains 91 scenarios; its newly added lifecycle, skill-menu, multi-client, and DM cases
must be reported separately until they complete a clean full run.

## Open findings from the expanded suite (July 12, 2026)

| Finding / Issue | Affected Scenarios | Layer | Current evidence |
| :--- | :--- | :--- | :--- |
| Character-slot success requires entitlement | `character-slot-switch-rejected` | Test fixture / server configuration | The primary fixture has `slotchange=0`. The automated scenario now verifies the explicit rejection and cleans up its disposable character. A success/persistence case requires a separate entitled fixture. |
| Partner registration connection closes after creation | phase 8 scenarios on a fresh server | Client test harness | Hercules created the `_M` account but closed that registration connection. The harness now retries once using the stable username instead of repeatedly sending the registration suffix. |
| Interrupted partner session can remain online | phase 8 after Ctrl-C | Server emulator / fixture hygiene | An interrupted two-client run can leave the partner account unavailable until Hercules clears its online state. Normal `TestContext` drops perform acknowledged logout; forced termination cannot. |

## Expanded-suite graphical-client handoff

This is the port checklist for scenarios added after the original 73-scenario run.
“Shared automatically” means the graphical client calls the same
`korangar-networking` implementation and no headless code should be copied.

| Headless coverage | Implementation layer | Graphical-client handoff |
| :--- | :--- | :--- |
| `teleport-select` | Shared packet/event API plus graphical selection window | Already consumed by the Warp Selection window; retain live click verification. |
| `teleport-cancel` | Shared `NetworkingSystem::cancel_warp_selection` | Shared automatically. Cancellation is deliberately a no-op on the wire; never send an empty destination because Hercules treats it as random Teleport. |
| `weapon-refine-missing-material`, `weapon-refine-success` | Shared refine list/result/effect events plus graphical Weapon Refine window | Packet behavior is shared automatically. The graphical client must resolve `item_id` through item metadata for result text and render `bs_refinesuccess.str`; both were manually verified. |
| Character-slot rejection | Shared character-server event mapping | Shared automatically. A success/persistence graphical check still requires a character with `slotchange > 0`. |
| Whisper, emotion, friends, party, and trade | Shared events and requests; graphical state/window consumers | Protocol behavior is shared automatically. Graphically verify window opening, accept/reject controls, roster/state updates, and item presentation. |
| DM dice and read-only command contracts | Normal chat requests and `ChatMessage` responses | No protocol port. Dice/Commands windows must emit the documented strings and display server feedback. |
| Partner registration/retry and cleanup | Headless fixture harness only | Do not port; these helpers only make integration tests repeatable. |
| Skill icon audit | Graphical resource loader/library | Run `cargo run -p korangar --bin skill-asset-audit` with configured archives; zero missing player-visible assets is required. |
| `repair-weapon-cancel`, `repair-weapon-success` | Shared modern Repair Weapon packets/events/API plus graphical selection window | Ported; both scenarios pass live. Graphical core flow was verified on macOS 2026-07-12: the window offered Sword, selection repaired it, and named success feedback appeared. Window resize/move and graphical Cancel remain presentation checks. |

### Port completion rule

A headless result alone does not verify presentation. Integration is complete when:

1. serialization/deserialization and event mapping have unit coverage;
2. the live headless scenario passes without unknown packets;
3. the graphical event consumer updates the appropriate state/window;
4. a manual graphical pass verifies clicking, labels, icons, resizing where relevant,
   feedback text, and visual effects; and
5. results and fixture requirements are recorded here and in `testing_guide.md`.

Window resizing is a graphical framework guarantee rather than a protocol behavior.
It was live-verified across the client on macOS 2026-07-12; new windows inherit
two-axis resizing from the `window!` component default.

| Finding / Issue | Affected Scenarios | Layer | Root Cause & Resolution |
| :--- | :--- | :--- | :--- |
| **Kaizel Resurrection Hang** | `use-consumable` | Client Test Harness | **Root Cause**: Running `skills-soul-linker` beforehand left the `Kaizel` buff active on the player character. When `@die` was called in `use-consumable`, Kaizel instantly self-resurrected the character, swallowing the expected `RemoveEntity` event and causing a timeout.<br>**Resolution**: Replaced the non-existent GM `@dispel` command with double-death retry logic. The first `@die` consumes the Kaizel buff, and a second `@die` triggers character death. |
| **Adjacent Walkable Cell Exhaustion** | `skills-paladin`, `skills-creator`, etc. | Client Test Harness | **Root Cause**: Dead or alive dummy targets (Pupas) accumulated around the player character between skill casts because monsters were only cleared at the end of the scenario. Once all 8 adjacent cells were filled, `approach_target` failed to find a walkable cell.<br>**Resolution**: Added immediate `kill_all_monsters()` calls inside the sweep loop to clean up dummy targets after each skill. |
| **Dynamic Positioning & Walk Drift** | `skills-paladin`, `skills-stalker` | Client Test Harness | **Root Cause**: Fixed warp coordinates `(170, 180)` were occasionally blocked or adjusted by the server. Furthermore, approaching targets in sequence created a "random walk" that drifted the player into walls.<br>**Resolution**: Switched to `warp_random` to dynamically acquire a safe coordinate, and added a walk reset back to `start_position` before each skill cast. |
| **Movement-Binding & Channeling States** | `skills-paladin`, `skills-stalker` | Client Test Harness | **Root Cause**: Channeling or persistent stealth skills like `PA_GOSPEL` (Gospel) or `ST_CHASEWALK` (Chase Walk) bound the character's movement or status, causing subsequent active skills (like `PA_SHIELDCHAIN` or `TF_HIDING`) to fail.<br>**Resolution**: Added these skills to `stateful_skill_rank` so they are sorted to the end of the sweep, preventing their state from locking later skills. |
| **Weapon/Ammo & Wall Dependencies** | `skills-assassin`, `skills-stalker`, etc. | Client Test Harness | **Root Cause**: Several skills require special setups—such as `AS_CLOAKING` (requires standing next to a wall) or `ST_REJECTSWORD` (requires daggers/swords)—which fail silently on open grass fields.<br>**Resolution**: Added these skills (`AS_CLOAKING`, `ST_REJECTSWORD`, `ST_PRESERVE`, `ST_FULLSTRIP`, `RG_GRAFFITI`, `RG_CLEANER`) to the `ALLOWLIST` with explanation comments. |

---

## Detailed Investigations

### 1. The Kaizel Resurrection Buff
> [!IMPORTANT]
> The Soul Linker self-resurrection buff, **`Kaizel`**, is extremely persistent. Because there is no standard GM `@dispel` command in the default Hercules build, the only way to clear it programmatically without restarting the server or map session is to trigger a character death.
* **Finding**: `use-consumable` failed with a timeout waiting for player death.
* **Analysis**: When the client receives `@die`, the server processes character death but immediately resurrects the character due to Kaizel. The client never receives a `RemoveEntity` event for the player, leading to a test timeout.
* **Resolution**: The double-death retry loop was implemented in [items.rs](../korangar-networking/examples/headless-tester/scenarios/items.rs). It attempts character death, waits for 2 seconds, checks if the character resurrected, and issues a second `@die` command if needed.

### 2. Random-Walk Drift & Target Accumulation
> [!TIP]
> Live integration tests sweeping dozens of active skills sequentially are prone to positioning drift. If the test harness spawns a target, walks next to it, and does not clean up the target, the player character gradually crawls across the map (random walk) and gets boxed in by their own targets.
* **Finding**: Paladin and other martial classes failed with `no walkable adjacent cell found around target` after successfully sweeping 15-20 skills.
* **Analysis**:
  1. The player character was not reset to a starting coordinate between casts, causing them to wander away.
  2. The target dummies (Pupas) remained on the map, occupying adjacent cells.
* **Resolution**: 
  1. In [skills.rs](../korangar-networking/examples/headless-tester/scenarios/skills.rs), the starting tile is dynamically recorded via `warp_random` as `start_position`.
  2. Before each skill sweep, the character walks back to `start_position`.
  3. After each cast, the target dummy is removed via `kill_all_monsters()`.

### 3. Stateful & Disabling Skills
> [!CAUTION]
> Certain skills modify player movement and casting states persistently. In a headless test sweep, these must be executed last to avoid polluting the state of other independent skill tests.
* **Examples**:
  * `HP_BASILICA` (blocks all actions within a holy zone).
  * `PA_GOSPEL` (binds the player to the spot and channels a zone).
  * `ST_CHASEWALK` (enters persistent stealth and drains SP).
* **Resolution**: These are registered in `stateful_skill_rank` and sorted to the very end of the execution queue.
