# Headless Client Tester — Findings, Investigations & Resolutions

This document logs historical findings, protocol discoveries, and resolutions encountered while running the Korangar headless client integration tests (`korangar-networking/examples/headless-tester`).

All findings are classified by layer:
* **Shared Crate**: Changes to `ragnarok-packets` or `korangar-networking` (directly affects the graphical client).
* **Client Test Harness**: Changes to test-runner logic (`scenarios/`, `context.rs`, etc.).
* **Server Emulator**: Changes to Hercules configuration, scripts, or databases.

---

## Summary of Recent Resolutions (July 2026)

All 73 integration tests—including all 39 job class skill sweeps and phase 6/7 item/dialogue loops—now pass with a **100% green status**.

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
