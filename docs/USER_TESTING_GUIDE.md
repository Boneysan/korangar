# Ragnarok Online Modernization: Comprehensive Live Playtest Master Manual & Verification Runbook

**Document Version:** 2.0 (Exhaustive Live Runbook)  
**Target Architecture:** Korangar Client + Hercules Renewal Server (`PACKETVER=20220406`)  
**Scope:** Complete functional, tactical, mathematical, and visual verification across all 4 modernization waves (Waves 0–4: S1–S10, F01–F39).  
**Core Contracts Enforced:** Hercules Server Authority; Decision D7 (Strictly non-custodial peer requests); Exact Renewal Integer Arithmetic; Non-teleporting Navigation; Zero Teleport-on-Hit Boss Encounters.

---

## Table of Contents
1. [Test Infrastructure & Server Bring-Up](#1-test-infrastructure--server-bring-up)
2. [Client Launch, Resolution & Multi-Boxing Configuration](#2-client-launch-resolution--multi-boxing-configuration)
3. [Test Personas, Pre-conditions & Account Provisioning](#3-test-personas-pre-conditions--account-provisioning)
4. [Section 1: Movement, Input & Camera Navigation](#section-1-movement-input--camera-navigation)
5. [Section 2: Combat Targeting & Spell Casting Mechanics](#section-2-combat-targeting--spell-casting-mechanics)
6. [Section 3: Monster AI Archetypes, Mob Families & Elite Encounters](#section-3-monster-ai-archetypes-mob-families--elite-encounters)
7. [Section 4: Telegraphed MVP Boss Encounters & Defeat Recap (All 11 MVPs)](#section-4-telegraphed-mvp-boss-encounters--defeat-recap-all-11-mvps)
8. [Section 5: Character Progression, Stats Modes & Interactive Build Planner](#section-5-character-progression-stats-modes--interactive-build-planner)
9. [Section 6: In-Game Encyclopedia & Adventure Guide](#section-6-in-game-encyclopedia--adventure-guide)
10. [Section 7: World Atlas, Region Navigation & Route Finder](#section-7-world-atlas-region-navigation--route-finder)
11. [Section 8: Crafting, Renewal Cooking & Peer Commission Board](#section-8-crafting-renewal-cooking--peer-commission-board)
12. [Section 9: Direct Player Trade & Anti-Scam Protection](#section-9-direct-player-trade--anti-scam-protection)
13. [Section 10: Party Systems, Tactical Markers & Tonight's Goals](#section-10-party-systems-tactical-markers--tonights-goals)
14. [Section 11: UI Ergonomics, HUD Profiles, Fading & Viewport Constraints](#section-11-ui-ergonomics-hud-profiles-fading--viewport-constraints)
15. [Section 12: Loot Filtering, Rare Wishlist & Ground Audio Cues](#section-12-loot-filtering-rare-wishlist--ground-audio-cues)
16. [Section 13: Recoverable Death Penalty & Danger Guidance](#section-13-recoverable-death-penalty--danger-guidance)
17. [Section 14: GM Observability, Telemetry & Baseline Diffing](#section-14-gm-observability-telemetry--baseline-diffing)
18. [Section 15: Master Playtest Pass/Fail Verification Matrix](#section-15-master-playtest-passfail-verification-matrix)
19. [Section 16: Defect Logging, Diagnostic Evidence & Triage Guidelines](#section-16-defect-logging-diagnostic-evidence--triage-guidelines)

---

## 1. Test Infrastructure & Server Bring-Up

### 1.1 Server Pre-Flight Verification
Before launching the server processes, verify configuration integrity:
1. Navigate to the Hercules directory:
   ```bash
   cd /Volumes/T7/GitHub/Ragnarok_Online/Hercules
   ```
2. Verify that `conf/import/battle.conf` includes:
   ```conf
   mob_pilot_version: 1
   ```
   *(When `mob_pilot_version >= 1`, the modernized AI profiles, cancelable boss telegraphs, and elite spawns are active).*
3. Verify database connectivity and server clean boot:
   ```bash
   ./dev.sh restart
   ./dev.sh wait
   ```
4. Verify server process status:
   ```bash
   ./dev.sh status
   ```
   **Expected Result:**
   - `login-server`: RUNNING on port 6900.
   - `char-server`: RUNNING on port 6121.
   - `map-server`: RUNNING on port 5121.
   - Zero fatal errors in `log/login-server.log`, `log/char-server.log`, or `log/map-server.log`.
   - Log contains confirmation: `Loaded 29 mob AI profiles` and `Loaded 11 modernized pilot MVP bosses`.

---

## 2. Client Launch, Resolution & Multi-Boxing Configuration

### 2.1 Primary Client (GM / Leader)
Launch the primary client in release mode:
```bash
cd /Volumes/T7/GitHub/Ragnarok_Online/korangar/korangar
cargo run --release --bin korangar
```

### 2.2 Secondary Client (Player / Peer)
For peer-to-peer verification (Direct Trade, Commission Board, Party Markers, Healer Targeting), launch a secondary independent client instance in a separate terminal:
```bash
cd /Volumes/T7/GitHub/Ragnarok_Online/korangar/korangar
cargo run --release --bin korangar
```

### 2.3 Window Layout & Display Calibration
1. Set Primary Client window to the left half of the display (`1280x720` or `1920x1080`).
2. Set Secondary Client window to the right half of the display.
3. Test UI Scaling levels in Game Settings (`Ctrl + I`):
   - `1.0x` (Standard 1080p).
   - `1.25x` (1440p standard).
   - `1.5x` / `2.0x` (4K UHD).
   - *Expected:* Fonts, windows, hotbars, and text boxes scale crisply without text clipping or overlapping elements.

---

## 3. Test Personas, Pre-conditions & Account Provisioning

Provision four distinct test characters across two accounts to support all test cases:

| Account / Char | Account Level | Class / Level | Primary Testing Role |
|---|---|---|---|
| **Account A:** `tester_gm`<br>Char: `GM_Tester` | Level 99 (GM) | Lord Knight / Lv 99 / Job 50 | Mob spawning, telemetry observation, test provisioning. |
| **Account A:** `tester_novice`<br>Char: `Rookie_Novice` | Level 0 (Player) | Novice / Lv 1 / Job 1 | Early-game onboarding, death penalty, danger warnings. |
| **Account B:** `tester_player1`<br>Char: `Player_Knight` | Level 0 (Player) | Knight / Lv 85 / Job 50 | Combat front-liner, commission poster, party initiator. |
| **Account B:** `tester_player2`<br>Char: `Player_Priest` | Level 0 (Player) | Priest / Lv 80 / Job 50 | Support healer, trade partner, commission responder. |

### 3.1 GM Provisioning Script
Log into `GM_Tester` and execute the following GM command sequence in chat:
```text
@zeny 50000000
@baselvl 99
@joblvl 50
@allstats 99
@item 501 100       // Red Potion
@item 505 50        // White Potion
@item 506 50        // Blue Potion
@item 984 20        // Oridecon
@item 985 20        // Elunium
@item 1010 50       // Phracon
@item 1011 50       // Emveretarcon
@item 1201 5        // Knife [3]
@item 1219 2        // Damascus [1]
@item 1222 2        // Fire Damascus
@item 1117 2        // Flamberge
@item 2314 2        // Plate Mail [1]
@item 4001 5        // Poring Card
@item 4011 5        // Andre Card
@item 4033 5        // Hydra Card
@item 4116 5        // Raydric Card
@item 12112 5       // Outdoor Cooking Kit
@item 12115 5       // Royal Cooking Kit
```

---

## Section 1: Movement, Input & Camera Navigation

### Test Case 1.1: Responsive WASD Movement
- **Objective:** Verify 8-directional WASD movement, 200ms packet throttle, and collision clipping avoidance.
- **Preconditions:** Logged in on `Player_Knight` in Prontera (`@warp prontera 150 150`).
- **Steps:**
  1. Open Game Settings (`Ctrl + I`) -> verify **Keyboard Movement** is toggled ON.
  2. Press and hold `W` (north), then tap `D` (north-east diagonal).
  3. Release and press `S` (south), then `A` (south-west).
  4. Move character towards a fountain wall or building border.
- **Expected Results:**
  - Character moves smoothly relative to current camera orientation.
  - Movement packets respect a strict 200ms throttle interval (no packet flooding).
  - Character smoothly slides along walls without clipping through geometry or triggering server position desyncs.

### Test Case 1.2: Chat & Input Focus Movement Lock
- **Objective:** Verify keyboard inputs do not cause character movement while a text input box is active.
- **Preconditions:** Keyboard movement toggled ON.
- **Steps:**
  1. Press `Enter` or click inside the chat bar.
  2. Type: `WASD wasd www aaa sss ddd`.
  3. Click into the Search Bar of the Adventure Guide (`Alt + G`).
  4. Type: `sword`.
- **Expected Results:**
  - Typed letters appear inside the text fields.
  - Character remains completely stationary.

### Test Case 1.3: Hold-Mouse Continuous Pathing
- **Objective:** Verify continuous mouse drag pathing without packet spam.
- **Preconditions:** Character in an open area (`prt_fild08`).
- **Steps:**
  1. Hold left-click down on the ground and drag the cursor across the screen in circles.
  2. Continue dragging for 10 seconds.
- **Expected Results:**
  - Character follows the moving cursor smoothly.
  - Path reissue executes strictly on 200ms interval boundaries (`should_reissue_hold_mouse_move`).
  - No client frame hitching or server "packet dropped / flooded" warnings.

### Test Case 1.4: Click-to-Move Pathing & Obstacle Avoidance
- **Objective:** Verify A* navigation around obstacles.
- **Preconditions:** Character in Prontera near stairs or decorative trees.
- **Steps:**
  1. Left-click a valid walkable tile situated on the opposite side of a fence or building corner.
- **Expected Results:**
  - Path line computes around the obstruction.
  - Character traverses around the perimeter to the exact clicked destination cell.

### Test Case 1.5: Camera Orbit, Zoom & Rest Stance
- **Objective:** Verify camera manipulation and sit/stand toggles.
- **Steps:**
  1. Hold right-mouse button and drag horizontally and vertically.
  2. Scroll mouse wheel backward and forward.
  3. Press `Insert` or type `/sit`.
- **Expected Results:**
  - Camera orbits 360 degrees smoothly and pitches within calibrated safety bounds.
  - Camera zooms between minimum close-up and maximum strategic overview heights.
  - Character transitions between sitting (HP/SP recovery rate x2) and standing stances.

---

## Section 2: Combat Targeting & Spell Casting Mechanics

### Test Case 2.1: Smart Tab Target Cycling
- **Objective:** Verify distance-sorted hostile target cycling and dead/hidden entity skipping.
- **Preconditions:** Warp to `prt_fild08 150 150` with multiple Porings, Fabres, and Lunatics nearby.
- **Steps:**
  1. Press **Tab** repeatedly.
  2. Press **Shift + Tab** repeatedly.
  3. Defeat the currently targeted monster.
  4. Press **Tab** immediately while the defeated corpse animation is fading.
- **Expected Results:**
  - Tab cycles through living hostile monsters ordered by distance and screen angle.
  - Shift + Tab cycles through the target list in reverse order.
  - Dead monsters are skipped immediately upon lethal damage.

### Test Case 2.2: Chebyshev Range Rings (Single-Target Skills)
- **Objective:** Verify tactical range rings for targeted ranged skills.
- **Preconditions:** Logged in on Archer/Mage with *Double Strafe* or *Fire Bolt* on the hotbar.
- **Steps:**
  1. Hover cursor over the skill icon on the hotbar.
  2. Move cursor near a distant monster within skill range.
  3. Move cursor beyond maximum skill range.
- **Expected Results:**
  - A bright Chebyshev circle ring centers on the character showing maximum reach.
  - Within range: Target indicator highlights green.
  - Out of range: Ring highlights red; cursor transitions to circle-slash invalid state.

### Test Case 2.3: Ground-Targeted Skill Footprints
- **Objective:** Verify geometric cell footprints for ground-targeted skills.
- **Preconditions:** Logged in on Mage/Wizard or Priest.
- **Steps:**
  1. Arm *Fire Wall*: Observe ground grid highlight.
  2. Arm *Storm Gust*: Observe ground grid highlight.
  3. Arm *Sanctuary*: Observe ground grid highlight.
  4. Arm *Safety Wall* or *Pneuma*: Observe ground grid highlight.
  5. Move reticle onto invalid/blocked terrain (e.g. water, cliff, or off-map).
- **Expected Results:**
  - *Fire Wall*: Highlights exact 3-cell line perpendicular to caster facing.
  - *Storm Gust*: Highlights exact 9x9 square grid.
  - *Sanctuary*: Highlights cut-corner diamond formation.
  - *Safety Wall* / *Pneuma*: Highlights precise 1x1 cell.
  - Blocked terrain: Footprint stipples/hatches with invalid indicator.

### Test Case 2.4: Three Cast Modes & Per-Skill Overrides
- **Objective:** Verify Classic, Quickcast, and Hold-Aim-Release cast behaviors with per-skill configuration.
- **Preconditions:** In Game Settings (`Ctrl + I`) -> Combat / Cast Style.
- **Steps:**
  1. Set Global Cast Mode to **Classic**: Press skill hotkey -> Reticle appears -> Left-click cell to cast.
  2. Set Global Cast Mode to **Quickcast**: Place mouse over target cell -> Press skill hotkey -> Spell casts immediately at cursor.
  3. Set Global Cast Mode to **Hold-Aim-Release**: Press and hold skill hotkey -> Aim reticle with mouse -> Release same hotkey to cast.
  4. In Game Settings, configure **Per-Skill Ground Overrides**:
     - *Fire Wall*: Explicitly set to **Hold-Aim-Release**.
     - *Fire Bolt*: Explicitly set to **Quickcast**.
     - *Storm Gust*: Set to **Inherit Global**.
  5. Cast each spell sequentially.
- **Expected Results:**
  - Each skill executes its configured override behavior independently of the global setting.

### Test Case 2.5: Cast Cancellation (`CZ_CANCEL_CAST`)
- **Objective:** Verify manual abort of long cast times.
- **Preconditions:** Cast *Level 10 Thunderstorm* or *Meteor Storm*.
- **Steps:**
  1. Begin casting the high-level spell (visible cast bar begins filling).
  2. At 50% cast completion, press **Escape** or **Right-Click**.
- **Expected Results:**
  - Cast bar immediately aborts and disappears.
  - Character instantly snaps back to idle stance.
  - No SP is consumed; no spell footprint appears; no cooldown is triggered.

### Test Case 2.6: Hold-Aim-Release Cancellation
- **Objective:** Verify safe cancellation while aiming a Hold-Aim-Release spell.
- **Steps:**
  1. Hold down the hotkey for *Fire Wall* (reticle appears and tracks cursor).
  2. Before releasing the hotkey, press **Escape** or **Right-Click**.
  3. Release the hotkey.
- **Expected Results:**
  - Aiming reticle disappears.
  - No skill packet is sent to the server.
  - A subtle client cancellation notification is displayed.

---

## Section 3: Monster AI Archetypes, Mob Families & Elite Encounters

### Test Case 3.1: Coward Archetype (Poring)
- **Objective:** Verify low-HP fleeing behavior.
- **Preconditions:** `@warp prt_fild08 150 150`.
- **Steps:**
  1. Engage a Poring using weak normal attacks (unequip weapon if needed).
  2. Reduce Poring HP below 20% without killing it.
- **Expected Results:**
  - Poring immediately ceases attacking.
  - Poring turns 180 degrees and pathfinds away from the player in active retreat.

### Test Case 3.2: Aggressor Archetype (Fabre / Thief Bug)
- **Objective:** Verify aggressive hostile pursuit.
- **Preconditions:** `@warp prt_fild08 200 200`.
- **Steps:**
  1. Walk within 5 cells of a Fabre.
- **Expected Results:**
  - Fabre detects the player within line of sight, turns aggressive, and paths toward the player to initiate melee combat.

### Test Case 3.3: Skirmisher Archetype (Orc Skeleton)
- **Objective:** Verify 3-hit tactical disengagement behavior.
- **Preconditions:** `@warp orcsdun01 100 100`.
- **Steps:**
  1. Engage a standard Orc Skeleton (Mob ID `1152`).
  2. Land 3 consecutive melee hits.
- **Expected Results:**
  - After the 3rd hit, Orc Skeleton performs a tactical disengagement, stepping 2 cells directly backward before resuming its attack.

### Test Case 3.4: RangedKeeper Archetype (Orc Archer)
- **Objective:** Verify distance preservation and hazard avoidance.
- **Preconditions:** `@warp orcsdun02 100 100`.
- **Steps:**
  1. Engage an Orc Archer (Mob ID `1189`).
  2. Move within 2 cells of the archer.
  3. Cast a *Fire Wall* between yourself and the Orc Archer.
- **Expected Results:**
  - Orc Archer attempts to backstep to maintain a 5–7 cell firing range.
  - When Fire Wall is deployed, Orc Archer paths around the perimeter of the fire cells rather than walking through them.

### Test Case 3.5: Raydric Chivalry Family Mechanics
- **Objective:** Verify cooperative family behavior on Glast Heim pilot maps.
- **Preconditions:** `@warp gl_knt01 150 150`.
- **Steps:**
  1. Engage a Raydric (Mob ID `1163`) in the presence of a Raydric Archer (Mob ID `1276`).
  2. Observe AI state triggers: `MSS_BERSERK` and `MSS_RUSH`.
- **Expected Results:**
  - Raydric rushes to protect the archer.
  - Monsters execute consistent threat switching via `NPC_DARKNESSATTACK` without erratic erratic hopping.

### Test Case 3.6: Elite Monster Variant (`[Elite] Orc Skeleton`)
- **Objective:** Verify Elite monster attributes, telegraphs, and drop rates.
- **Preconditions:** `@warp orcsdun01 100 100` (`mob_pilot_version: 1`).
- **Steps:**
  1. Locate `[Elite] Orc Skeleton` (Mob ID `20901`).
  2. Check monster nameplate and target frame.
  3. Engage in combat; wait for its telegraphed skill.
  4. Defeat the Elite monster.
- **Expected Results:**
  - Nameplate displays `[Elite] Orc Skeleton`.
  - Monster has 3x normal Orc Skeleton HP (approx 9,000 HP).
  - Monster casts *Bash* (Lv 10) with a visible 1.2s cast bar.
  - Grants 3x Base and Job EXP upon defeat.
  - Drops *Orcish Cuspid* and *Skel-Bone* at elevated rates.

### Test Case 3.7: AI Pilot Rollback Switch Verification
- **Objective:** Verify server restores stock AI behavior when `mob_pilot_version` is set to 0.
- **Preconditions:** GM access to server configuration.
- **Steps:**
  1. In Hercules `conf/import/battle.conf`, set `mob_pilot_version: 0`.
  2. Execute `./dev.sh restart && ./dev.sh wait`.
  3. Re-engage Orc Skeleton on `orcsdun01`.
- **Expected Results:**
  - Orc Skeleton uses standard stock Renewal AI (no 3-hit skirmish retreat).
  - No `[Elite] Orc Skeleton` entities spawn.
  - Re-enable `mob_pilot_version: 1` and restart server to resume modernized testing.

---

## Section 4: Telegraphed MVP Boss Encounters & Defeat Recap (All 11 MVPs)

### 4.1 Encounter Core Architecture
Every modernized MVP encounter enforces the 6-component design pattern:
1. **Signature Tell:** 2.0s–3.0s cancelable cast bar with audio warning (`warning.wav`).
2. **Pressure Phase:** Minion escorts and steady melee threat.
3. **Movement Check:** Ground hazard forcing repositioning.
4. **Class Opportunity Window:** Signature move is fully cancelable via high-ASPD, Stun, Silence, or Knockback.
5. **Escalation / Enrage:** At $<30\%$ HP, boss triggers power/speed buffs with a visual roar.
6. **Recovery Window:** $\ge 15.0$ seconds of quiet recovery (standard auto-attacks only) between signature moves.
7. **Zero Teleport-on-Hit:** Boss NEVER teleports away when attacked at range or surrounded.
8. **BossTargetWindow:** Dedicated 360x200 window displaying HP bar, Level, Element chip, and Target-of-Target (`Target: You` or `Target: <PartyMember>`).
9. **Encounter Recap Toast:** Tracking damage dealt/taken, interrupted casts, duration, and defeat announcement.

---

### 4.2 Individual Boss Verification Runbooks

#### Boss 1: Eddga (Mob ID 1115)
- **Map:** `@warp pay_fild10 150 150` -> `@monster 1115 1`.
- **Minions:** Bigfoot x4 (`NPC_CALLSLAVE`).
- **Signature Move:** `SM_MAGNUM` (2.0s cast, 15s delay, cancelable).
- **Movement Check:** `WZ_METEOR` (3.0s cast, 20s delay, cancelable).
- **Enrage ($<30\%$ HP):** `NPC_POWERUP` + `NPC_SPEEDUP`.
- **Steps & Verification:**
  1. Target Eddga: Confirm `BossTargetWindow` displays Level 65, Fire 1 element, Target-of-Target.
  2. Attack with ranged weapon: Confirm Eddga NEVER teleports away.
  3. When Eddga casts *Magnum Break* (2.0s cast), land *Bash* or hit with high ASPD: Confirm cast bar breaks, `_stun.wav` plays, and 15s recovery begins.
  4. Bring Eddga below 30% HP: Confirm enrage activation.
  5. Kill Eddga: Confirm defeat toast appears: `"Defeated Eddga! Dealt <Dmg> dmg in <Duration>s"`.

#### Boss 2: Moonlight Flower (Mob ID 1150)
- **Map:** `@warp pay_dun04 150 150` -> `@monster 1150 1`.
- **Minions:** Nine Tail x3 (`NPC_CALLSLAVE`).
- **Signature Move:** `NPC_PULSESTRIKE` (2.0s cast, 15s delay, cancelable).
- **Movement Check:** `MG_FIREWALL` (1.5s cast, 18s delay, cancelable).
- **Pressure:** `MC_MAMMONITE` (Lv 10).
- **Enrage ($<30\%$ HP):** `NPC_POWERUP` + `NPC_SPEEDUP`.
- **Steps & Verification:**
  1. Confirm `BossTargetWindow` displays Level 68, Fire 3 element.
  2. Observe *Pulse Strike* 2.0s cast bar; interrupt with attack or skill.
  3. Confirm 15s recovery window without teleportation.
  4. Kill boss; verify defeat toast.

#### Boss 3: Golden Thief Bug (Mob ID 1086)
- **Map:** `@warp prt_sewb4 100 100` -> `@monster 1086 1`.
- **Minions:** Male Thief Bug x5 (`NPC_CALLSLAVE`).
- **Signature Move:** `SM_MAGNUM` (2.0s cast, 15s delay, cancelable).
- **Movement Check:** `MG_FIREBALL` (1.5s cast, 12s delay, cancelable).
- **Defense Buff:** `CR_REFLECTSHIELD` (Lv 10).
- **Enrage ($<30\%$ HP):** `NPC_POWERUP`.
- **Steps & Verification:**
  1. Confirm `BossTargetWindow` displays Level 70, Fire 2 element.
  2. Interrupt *Magnum Break* cast bar.
  3. Verify minion coordination and absence of teleportation on hit.
  4. Kill boss; verify defeat toast.

#### Boss 4: Orc Hero (Mob ID 1087)
- **Map:** `@warp gef_fild14 150 150` -> `@monster 1087 1`.
- **Minions:** High Orc x4 (`NPC_CALLSLAVE`).
- **Signature Move:** `LK_SPIRALPIERCE` (2.0s cast, 15s delay, cancelable).
- **Movement Check:** `MG_THUNDERSTORM` (2.5s cast, 18s delay, cancelable).
- **Enrage ($<30\%$ HP):** `NPC_POWERUP` + `KN_TWOHANDQUICKEN`.
- **Steps & Verification:**
  1. Confirm `BossTargetWindow` displays Level 71, Earth 2 element.
  2. Observe *Spiral Pierce* 2.0s tell; interrupt with Stun/Bash.
  3. Verify 15s recovery window.
  4. Kill boss; verify defeat toast.

#### Boss 5: Maya (Mob ID 1147)
- **Map:** `@warp anthell02 150 150` -> `@monster 1147 1`.
- **Minions:** Argiope x4 (`NPC_CALLSLAVE`).
- **Signature Move:** `KN_BRANDISHSPEAR` (2.0s cast, 15s delay, cancelable).
- **Movement Check:** `WZ_HEAVENDRIVE` (2.0s cast, 18s delay, cancelable).
- **Defense Buff:** `CR_AUTOGUARD` (Lv 10).
- **Enrage ($<30\%$ HP):** `NPC_POWERUP`.
- **Steps & Verification:**
  1. Confirm `BossTargetWindow` displays Level 75, Earth 4 element.
  2. Interrupt *Brandish Spear* cast bar.
  3. Verify no teleportation occurs.
  4. Kill boss; verify defeat toast.

#### Boss 6: Baphomet (Mob ID 1139)
- **Map:** `@warp prt_maze03 150 150` -> `@monster 1139 1`.
- **Minions:** Baphomet Jr. x5 (`NPC_CALLSLAVE`).
- **Signature Move:** `NPC_HELLJUDGEMENT` (2.5s cast, 18s delay, cancelable).
- **Movement Check:** `WZ_VERMILION` (3.0s cast, 20s delay, cancelable).
- **Lethal Threat:** `NPC_EARTHQUAKE` (Lv 5 at $<30\%$ HP).
- **Steps & Verification:**
  1. Confirm `BossTargetWindow` displays Level 81, Shadow 3 element.
  2. Observe *Lord of Vermilion* 3.0s cast bar; interrupt with rapid strikes.
  3. Confirm minimum 18s recovery window between signature casts.
  4. Kill boss; verify defeat toast.

#### Boss 7: Phreeoni (Mob ID 1159)
- **Map:** `@warp moc_fild17 150 150` -> `@monster 1159 1`.
- **Minions:** Hode x3 (`NPC_CALLSLAVE`).
- **Signature Move:** `WZ_HEAVENDRIVE` (2.0s cast, 15s delay, cancelable).
- **Ailment Check:** `NPC_PETRIFYATTACK` (1.5s cast, 12s delay, cancelable) + `NPC_WIDESTONE`.
- **Enrage ($<30\%$ HP):** `NPC_POWERUP`.
- **Steps & Verification:**
  1. Confirm `BossTargetWindow` displays Level 75, Neutral 3 element.
  2. Interrupt *Heaven's Drive* cast bar.
  3. Verify Hode minions spawn; confirm Phreeoni never teleports away.
  4. Kill boss; verify defeat toast.

#### Boss 8: Mistress (Mob ID 1059)
- **Map:** `@warp mjolnir_04 150 150` -> `@monster 1059 1`.
- **Minions:** Giant Hornet x4 (`NPC_CALLSLAVE`).
- **Signature Move:** `WZ_JUPITEL` (2.0s cast, 15s delay, cancelable).
- **Movement Check:** `NPC_WIDESILENCE` (1.5s cast, 15s delay, cancelable).
- **Enrage ($<30\%$ HP):** `NPC_POWERUP` + `NPC_AGIUP`.
- **Steps & Verification:**
  1. Confirm `BossTargetWindow` displays Level 74, Wind 4 element.
  2. Interrupt *Jupitel Thunder* 2.0s cast bar.
  3. Verify 15s quiet recovery window.
  4. Kill boss; verify defeat toast.

#### Boss 9: Drake (Mob ID 1112)
- **Map:** `@warp treasure02 150 150` -> `@monster 1112 1`.
- **Minions:** Wraith x3 (`NPC_CALLSLAVE`).
- **Signature Move:** `WZ_WATERBALL` (2.5s cast, 18s delay, cancelable).
- **Ailment Check:** `NPC_DRAGONFEAR` (1.5s cast, 15s delay, cancelable).
- **Buff:** `BS_MAXIMIZE` (Lv 5) + `NPC_AGIUP`.
- **Steps & Verification:**
  1. Confirm `BossTargetWindow` displays Level 73, Undead 1 element.
  2. Interrupt *Waterball* 2.5s cast bar.
  3. Verify 18s quiet recovery window.
  4. Kill boss; verify defeat toast.

#### Boss 10: Doppelganger (Mob ID 1046)
- **Map:** `@warp gef_dun02 150 150` -> `@monster 1046 1`.
- **Minions:** Nightmare x3 (`NPC_CALLSLAVE`).
- **Signature Move:** `LK_SPIRALPIERCE` (2.0s cast, 15s delay, cancelable).
- **Crowd Control:** `BS_HAMMERFALL` (1.5s cast, 12s delay, cancelable).
- **Enrage ($<30\%$ HP):** `NPC_POWERUP` + `KN_TWOHANDQUICKEN`.
- **Steps & Verification:**
  1. Confirm `BossTargetWindow` displays Level 71, Shadow 3 element.
  2. Interrupt *Spiral Pierce* 2.0s cast bar.
  3. Verify high melee pressure without teleportation.
  4. Kill boss; verify defeat toast.

#### Boss 11: Osiris (Mob ID 1038)
- **Map:** `@warp moc_pryd04 150 150` -> `@monster 1038 1`.
- **Minions:** Mummy x4 (`NPC_CALLSLAVE`).
- **Signature Move:** `ASC_METEORASSAULT` (2.0s cast, 15s delay, cancelable).
- **Hazard Check:** `AS_VENOMDUST` (1.5s cast, 12s delay, cancelable) + `WZ_QUAGMIRE`.
- **Enrage ($<30\%$ HP):** `NPC_POWERUP`.
- **Steps & Verification:**
  1. Confirm `BossTargetWindow` displays Level 78, Undead 4 element.
  2. Interrupt *Meteor Assault* 2.0s cast bar.
  3. Verify 15s quiet recovery window.
  4. Kill boss; verify defeat toast.

---

## Section 5: Character Progression, Stats Modes & Interactive Build Planner

### Test Case 5.1: Character Stats Modes & Hover Previews
- **Objective:** Verify Simple, Detailed, and Advanced stat modes and next-point deltas.
- **Preconditions:** Open Character Stats window (`Alt + A`).
- **Steps:**
  1. Click **Simple Mode**:
     - *Expected:* Plain-language descriptions (e.g. *"STR increases physical attack power and weight capacity"*).
  2. Click **Detailed Mode**:
     - *Expected:* Displays derived combat ratings, next-point stat deltas, and breakpoint distance counters.
  3. Click **Advanced Mode**:
     - *Expected:* Displays exact Renewal mathematical formulas (Status ATK, Soft DEF, Soft MDEF, Cast Reduction) labeled with `(exact)`.
  4. Hover over the **+** button for each stat:
     - Hover STR: `+1 STR -> +1 Status ATK, +30 Max Weight`.
     - Hover DEX: `+1 DEX -> +1 HIT, +1 Status ATK, +0.2% Cast Reduction`.
     - Hover AGI: `+1 AGI -> +1 FLEE, +ASPD contribution`.
     - Hover INT: `+1 INT -> +1.5 Status MATK, +1 Soft MDEF, +0.4% Cast Reduction`.
     - Hover VIT: `+1 VIT -> +1% Max HP, +1 Soft DEF, +0.8 Soft MDEF`.
     - Hover LUK: `+1 LUK -> +0.3 Crit, +0.1 Perfect Dodge, +0.3 Status ATK/MATK`.

### Test Case 5.2: Interactive Build Planner Level & Stat Staging
- **Objective:** Verify client-side simulation engine, level sliders, and scaling point costs.
- **Preconditions:** Open Stats window (`Alt + A`) -> Click **Build Planner**.
- **Steps:**
  1. Drag **Base Level** slider from 1 to 99:
     - *Expected:* Available stat points update to 1,225 total points (`STAT_POINTS_TABLE[98]`).
  2. Drag **Job Level** slider from 1 to 50:
     - *Expected:* Skill points update to 49 total points.
  3. Raise STR incrementally from 1 to 99:
     - *Expected:* Point costs scale up at threshold boundaries:
       - 1 to 10: 2 points per tick.
       - 11 to 20: 3 points per tick.
       - 21 to 30: 4 points per tick.
       - ...
       - 90 to 99: 16 points per tick.
       - Exactly 628 cumulative points spent at STR 99.
  4. Attempt to raise STR beyond 99:
     - *Expected:* Refused; cap notification: `"Stat cannot exceed 99 (max_parameter)"`.

### Test Case 5.3: Skill Tree Staging & Prerequisite Enforcement
- **Objective:** Verify skill prerequisites in the planner.
- **Preconditions:** In `BuildPlannerWindow` on a Swordsman/Knight character.
- **Steps:**
  1. Locate *Two-Hand Quicken* in the skill tree.
  2. Attempt to allocate a point to *Two-Hand Quicken* while *Two-Hand Sword Mastery* is at 0.
     - *Expected:* Refused; prerequisite blocker displayed: `Requires Two-Hand Sword Mastery Lv 1`.
  3. Allocate 1 point to *Two-Hand Sword Mastery*.
  4. Allocate points to *Two-Hand Quicken*.
     - *Expected:* Allocation succeeds; skill point budget decrements cleanly.

### Test Case 5.4: Plan Persistence (Disk Save / Load / Reset)
- **Objective:** Verify roundtrip disk persistence of staged build plans.
- **Steps:**
  1. Stage a custom build (e.g. STR 90, AGI 70, DEX 50).
  2. Click **Save 1**.
     - *Expected:* Stored locally at `client/build_plans/job-<id>-slot-1.json`.
  3. Click **Reset Plan**.
     - *Expected:* All values return to current live character baseline.
  4. Click **Load 1**.
     - *Expected:* Staged build plan restores identically; zero network packets sent.
  5. Close client completely, relaunch, open Build Planner, click **Load 1**.
     - *Expected:* Build plan restores cleanly from disk file.

### Test Case 5.5: Free Respec Commands
- **Objective:** Verify player-level free stat and skill reset commands.
- **Preconditions:** Normal player account (`Player_Knight`).
- **Steps:**
  1. Type `@streset`.
     - *Expected:* All stat points refunded to unspent pool with zero zeny cost.
  2. Type `@skreset`.
     - *Expected:* All skill points refunded with zero zeny cost.
  3. Learn *Two-Hand Sword Mastery Lv 1* and *Two-Hand Quicken Lv 1*.
  4. Type `@refundskill 60` (*Two-Hand Sword Mastery* ID).
     - *Expected:* Refused with error: `"Cannot refund skill: dependent skills exist"`.

---

## Section 6: In-Game Encyclopedia & Adventure Guide

### Test Case 6.1: Category Navigation & Coverage
- **Objective:** Verify complete in-game reference coverage across all 11 categories.
- **Preconditions:** Press **Alt + G** to open Adventure Guide.
- **Steps:**
  1. Click through all 11 category tabs:
     - `Monsters`, `Items`, `Cards`, `Skills`, `Statuses`, `Mechanics`, `Maps`, `Jobs`, `Quests`, `NPCs`, `Services`.
- **Expected Results:**
  - Every category displays verified game records without missing stubs.
  - Header displays authoritative data stamp: `Exported from Hercules <rev> (mode)`.

### Test Case 6.2: Search Alias Resolution
- **Objective:** Verify search aliases for ailments, nicknames, and jobs.
- **Steps:**
  1. Type `"freeze"` in the search bar -> Resolves to *Frozen* status effect.
  2. Type `"blind"` -> Resolves to *Blind* status effect.
  3. Type `"sin"` -> Resolves to *Assassin* job class.
  4. Type `"bs"` -> Resolves to *Blacksmith* job class.
  5. Type `"thara"` -> Resolves to *Thara Frog Card*.
  6. Type `"hydra"` -> Resolves to *Hydra Card*.
- **Expected Results:**
  - Search results instantly match colloquial names and aliases.

### Test Case 6.3: Second Job Progression Guide (§8.6)
- **Objective:** Verify comprehensive second-job transition documentation.
- **Steps:**
  1. In Adventure Guide, search for `"second job"`.
  2. Open **Second Job Progression Guide**.
- **Expected Results:**
  - Details all 12 Second Job branches (*Knight, Crusader, Wizard, Sage, Hunter, Bard, Dancer, Assassin, Rogue, Priest, Monk, Blacksmith, Alchemist*).
  - Documents Job 40 minimum requirement and Job 50 recommendation (+10 bonus stat points).
  - Explicitly states that skill points are permanently bound to their job tier.
  - Documents Transcendent progression path (High Novice -> Transcendent Second -> Third Class).

### Test Case 6.4: Card Socketing & Compounding Rule
- **Objective:** Verify card damage stacking documentation.
- **Steps:**
  1. Search for `"socketing"` in Adventure Guide.
- **Expected Results:**
  - Compounding into equipment slots is documented as permanent.
  - Explains **Additive stacking** for same category (two +20% Racial cards = +40%).
  - Explains **Multiplicative stacking** for different categories (one +20% Racial card x one +20% Size card = +44% total damage).

### Test Case 6.5: Refinement Odds & Permanent Destruction Warning
- **Objective:** Verify exact weapon/armor refinement odds.
- **Steps:**
  1. In Adventure Guide, navigate to **Mechanics** -> **Weapon & Armor Refinement**.
- **Expected Results:**
  - Lists exact success chances for Weapon Levels 1, 2, 3, 4 and Armor from +1 to +10.
  - Safe limits documented: Lv 1 (+7), Lv 2 (+6), Lv 3 (+5), Lv 4 (+4), Armor (+4).
  - Prominent red warning banner: `"Failure beyond safe limit results in permanent item and card destruction"`.

---

## Section 7: World Atlas, Region Navigation & Route Finder

### Test Case 7.1: World Atlas & Road Classification
- **Objective:** Verify 6 geographic regions and walk vs transport connections.
- **Preconditions:** Open World Map (`Alt + W`).
- **Steps:**
  1. Click through the 6 region tabs:
     - *Rune-Midgarts*, *Schwarzwald*, *Arunafeltz*, *Global Project*, *Dimensional Gorge*, *Dungeons & Landmarks*.
  2. Inspect connection lines between cities and dungeons.
- **Expected Results:**
  - `RoadKind::Walk` connections render in neutral walking road styling.
  - `RoadKind::Transport` connections (ferries, cat fleet, airships, dimensional rifts) render in distinct transport coloration.

### Test Case 7.2: Multi-Hop Route Finder (Non-Teleporting)
- **Objective:** Verify route calculation without server teleport bypass.
- **Steps:**
  1. On `Player_Knight` in Prontera, open World Map.
  2. Select **Byalan Island** (`iz_dun00`).
  3. Click **Plot Route**.
- **Expected Results:**
  - Multi-hop route calculates: `Walk to Izlude -> Sailor NPC Ferry (150 zeny) -> izlu2dun -> Enter Portal -> iz_dun00`.
  - Sets a client-side navigation breadcrumb on the minimap and in-world floor.
  - Zero warp packets or teleport requests sent to the server.

### Test Case 7.3: Map Danger Warnings
- **Objective:** Verify level gap warning upon entering dangerous maps.
- **Preconditions:** Logged in on `Rookie_Novice` (Base Level 1).
- **Steps:**
  1. Warp or walk to Glast Heim Castle (`@warp gl_knt01 150 150`, mean monster level ~110).
  2. Observe screen upon map entry.
  3. Open Game Settings (`Ctrl + I`) -> toggle OFF **Warn on dangerous maps**.
  4. Exit and re-enter the map.
- **Expected Results:**
  - On entry at Lv 1: Warning toast appears: `"Entering high danger area (Mean monster level 110)"`.
  - World Atlas node for `gl_knt01` displays high-contrast red outline and `!` icon.
  - Entry is NOT blocked (free world exploration preserved).
  - When setting is toggled OFF: Re-entering the map triggers zero warning toasts.

### Test Case 7.4: Minimap Waypoints, Layers & Zoom
- **Objective:** Verify personal waypoints, scroll zoom, and layer toggles.
- **Preconditions:** Expand minimap with **Ctrl + Tab**.
- **Steps:**
  1. Left-click any cell on the minimap.
     - *Expected:* Bright personal waypoint pin drops at that coordinate on the minimap and in-world.
  2. Right-click the minimap.
     - *Expected:* Personal waypoint pin clears cleanly.
  3. Scroll mouse wheel over the minimap.
     - *Expected:* Minimap zooms smoothly between 50% and 200%.
  4. In Game Settings, toggle layer visibility for **Portals**, **Facilities**, and **Party Members**.
     - *Expected:* Layer icons show/hide dynamically.
  5. Inspect a tracked route portal exit.
     - *Expected:* Portal blip renders with bright gold accent: `→ Route portal: <map_name>`.

---

## Section 8: Crafting, Renewal Cooking & Peer Commission Board

### Test Case 8.1: Dedicated Commission Board GUI Window
- **Objective:** Verify Commission Board GUI inputs, posting, and listing.
- **Preconditions:** Logged in on `Player_Knight`.
- **Steps:**
  1. Open menu -> Click **Commission Board** (or type `/commission board`).
  2. Inspect window components:
     - Window Title: `Crafting Commission Board`.
     - Non-custodial risk disclosure banner: *"Non-custodial: Materials & payment must be traded directly. No automated escrow. Crafting failure risk borne by requester unless negotiated."*
     - Input field 1: `Item Name:`
     - Input field 2: `Offered Fee (Zeny):`
     - Buttons: **Post Request** and **View All Active**.
  3. Enter Item: `+7 Fire Damascus`.
  4. Enter Fee: `100000`.
  5. Click **Post Request**.
- **Expected Results:**
  - Input fields clear.
  - Chat confirms: `Commission posted: +7 Fire Damascus (Fee: 100000 zeny)`.
  - Request is assigned an auto-incremented ID (e.g. `#1`).

### Test Case 8.2: Commission Board Listing & Anti-Tamper Safeguards
- **Objective:** Verify board synchronization and unauthorized cancellation rejection.
- **Preconditions:** Client A (`Player_Knight`) posted Commission #1. Client B (`Player_Priest`) logged in.
- **Steps:**
  1. On Client B, open Commission Board and click **View All Active** (or type `/commission list`).
     - *Expected:* Commission #1 is visible with requester name `Player_Knight`, item `+7 Fire Damascus`, fee `100000z`.
  2. On Client B, attempt to cancel Commission #1: Type `/commission cancel 1`.
     - *Expected:* Cancellation REFUSED: `"Only the original requester can cancel this commission."`
  3. On Client A (`Player_Knight`), type `/commission cancel 1`.
     - *Expected:* Cancellation ACCEPTED; confirmation toast: `"Commission #1 cancelled."`
  4. On Client B, refresh the board.
     - *Expected:* Commission #1 is removed from the active list.

### Test Case 8.3: Renewal Cooking Formulas & Success Calculation
- **Objective:** Verify Renewal cooking formulas and kit bonus calculations.
- **Preconditions:** Open Adventure Guide -> **Crafting** -> **Renewal Cooking**.
- **Steps:**
  1. Verify the exact cooking formula:
     $$\text{Rate (bp)} = (\text{Job} \times 20) + (\text{DEX} \times 20) + (\text{LUK} \times 10) + \text{KitBonus} - (\text{DishLevel} \times 500)$$
  2. Verify Cooking Kit bonuses:
     - Outdoor Cooking Kit: $+11.00\%$ (+1100 bp).
     - Home Cooking Kit: $+12.00\%$ (+1200 bp).
     - Professional Cooking Kit: $+13.00\%$ (+1300 bp).
     - Royal Cooking Kit: $+14.00\%$ (+1400 bp).
     - Fantastic / Legendary Cooking Kit: $+15.00\%$ (+1500 bp).
  3. Verify batch rating formula for 10-dish sets (`mix_cooking_rating`).

---

## Section 9: Direct Player Trade & Anti-Scam Protection

### Test Case 9.1: Partner Zeny Display Accuracy
- **Objective:** Verify partner's zeny displays in the dedicated zeny box without the "Item #0" bug.
- **Preconditions:** Client A (`Player_Knight`) and Client B (`Player_Priest`) standing together in Prontera.
- **Steps:**
  1. Initiate trade between Client A and Client B.
  2. On Client B, enter `250,000` in the Zeny offer box.
  3. Inspect Client A's trade window.
- **Expected Results:**
  - Client A displays `Their zeny: 250000` at the top of the partner slot grid.
  - Zero bogus "Item #0" rows appear in the partner's item slots.

### Test Case 9.2: Last-Second-Change Warning Banner
- **Objective:** Verify warning banner triggers when partner modifies their offer after player locks.
- **Steps:**
  1. Client A places `Knife [3]` in trade slot and clicks **Lock**.
     - *Expected:* Client A's side locks with green checkmark.
  2. Client B (unlocked) alters their offer: changes Zeny from `250000` to `25000`.
  3. Observe Client A's trade window immediately.
- **Expected Results:**
  - Client A's window displays high-contrast bold warning banner:
    `"!! Partner CHANGED their offer after you locked"`

### Test Case 9.3: 1.5-Second Confirmation Refusal Cooldown
- **Objective:** Verify rapid accidental confirmation is blocked after offer modification.
- **Steps:**
  1. Immediately after Client B modifies their offer (< 1.5 seconds), Client A clicks **Confirm Trade**.
- **Expected Results:**
  - Trade confirmation is REFUSED.
  - Client displays refusal toast: `"Trade offer was modified. Please review changes before confirming."`
  - Trade remains open and locked for review.

### Test Case 9.4: Slash Command Bypass Prevention
- **Objective:** Verify chat command `/trade commit` cannot bypass the anti-scam cooldown.
- **Steps:**
  1. Client B modifies an offer item.
  2. Client A immediately types `/trade commit` in chat.
- **Expected Results:**
  - Command is intercepted and refused by the trade guard.
  - Zero trade completion packet sent to the server.

### Test Case 9.5: Final-State Summary & Legitimate Trade Completion
- **Objective:** Verify final transaction state summary and clean asset transfer.
- **Steps:**
  1. Client A waits $> 1.5$ seconds, reviews modified offer, and clicks **Confirm Trade**.
  2. Client B locks and clicks **Confirm Trade**.
- **Expected Results:**
  - Both clients display final-state summary: `"You give: ... / You receive: ..."`.
  - Trade completes cleanly.
  - Client A receives 25,000 Zeny; Client B receives `Knife [3]`.
  - Zero item duplication or zeny loss.

---

## Section 10: Party Systems, Tactical Markers & Tonight's Goals

### Test Case 10.1: Party Distance & Cross-Map Tracking
- **Objective:** Verify real-time party member distance tracking and cross-map styling.
- **Preconditions:** Client A and Client B in a party (`Alt + Z`).
- **Steps:**
  1. Stand 8 cells apart in Prontera:
     - *Expected:* Party frame displays `8 tiles away` in neutral text.
  2. Client B warps to Geffen (`@warp geffen 120 100`):
     - *Expected:* Client A's party frame updates to `other map: geffen` in contrasting red text.

### Test Case 10.2: Healer Layout & Click-to-Target
- **Objective:** Verify dedicated healer party frames.
- **Steps:**
  1. In party window settings, toggle ON **Healer Layout**.
  2. Click a party member's health bar frame.
  3. Click **Navigate** on the party frame.
- **Expected Results:**
  - Party frames expand into enlarged high-contrast HP/SP bars.
  - Clicking the frame immediately targets the party member for healing/buffs.
  - Clicking Navigate sets a world-map breadcrumb to the party member's coordinates.

### Test Case 10.3: Tactical Target Markers
- **Objective:** Verify entity target markers, outlines, and auto-expiry.
- **Steps:**
  1. Target a monster on Client A; type `/mark focus`.
     - *Expected:* Crosshair focus icon and yellow outline appear on the monster on BOTH clients.
  2. Test other marker commands:
     - `/mark attack` (Sword icon / red outline).
     - `/mark cc` (Ice freeze icon / blue outline).
     - `/mark assist` (Shield icon / green outline).
  3. Wait 30 seconds without attacking the monster.
     - *Expected:* Marker automatically expires and clears from both clients.
  4. Type `/mark clear`.
     - *Expected:* Active markers immediately clear.

### Test Case 10.4: Tonight's Goals System
- **Objective:** Verify party goal coordination pinned to the HUD.
- **Steps:**
  1. On Client A, type `/goal add Hunt 10 Orc Skeletons`.
     - *Expected:* Goal appears pinned under `Tonight:` header on both clients' screens.
  2. Type `/goal add Clear Orc Dungeon 2F`.
     - *Expected:* Second goal pins below the first (up to 5 goals max).
  3. Type `/goal done 1`.
     - *Expected:* Goal 1 is struck through/archived with a completion chime.
  4. Type `/goal clear`.
     - *Expected:* Goals list clears from the party HUD.

### Test Case 10.5: Party EXP Share & Level Spread Boundary
- **Objective:** Verify 30-level spread boundary and 25% even-share bonus.
- **Preconditions:** Client A (Lv 80 Knight), Client B (Lv 51 Priest) -> Spread = 29 levels.
- **Steps:**
  1. Open Party Options -> Enable **Even Share**.
     - *Expected:* Even share enables successfully with 25% party bonus.
  2. Level up Client A to Lv 82 (Spread = 31 levels).
     - *Expected:* Even share is disabled automatically; chat warns of level gap $> 30$.

---

## Section 11: UI Ergonomics, HUD Profiles, Fading & Viewport Constraints

### Test Case 11.1: Screen-Edge Window Drag Clamping
- **Objective:** Verify windows cannot be dragged off-screen.
- **Steps:**
  1. Click and drag the Inventory window toward the top screen border.
  2. Drag toward bottom, left, and right screen boundaries.
- **Expected Results:**
  - Window movement clamps at screen edges.
  - Window titlebar, close button, and borders remain 100% visible and accessible.

### Test Case 11.2: Tooltip Viewport Clamping
- **Objective:** Verify long hover tooltips do not clip outside the screen.
- **Steps:**
  1. Drag Inventory window to the extreme bottom-right corner of the screen.
  2. Hover over an item with a lengthy description (e.g. multi-card socketed weapon).
- **Expected Results:**
  - Tooltip detects screen edge boundaries and flips upward/leftward.
  - 100% of tooltip text remains fully readable inside the viewport.

### Test Case 11.3: Escape Key Focus Release
- **Objective:** Verify Escape key unfocuses text fields before triggering window closures.
- **Steps:**
  1. Click into Chat bar and type: `Testing escape key`.
  2. Press **Escape** once.
  3. Press **Escape** a second time.
- **Expected Results:**
  - First Escape press: Text field loses cursor focus; character movement is restored; game menu does NOT open.
  - Second Escape press: Closes topmost open window or brings up system menu.

### Test Case 11.4: Window Grid Snapping
- **Objective:** Verify pixel grid snapping intervals.
- **Steps:**
  1. In Game Settings (`Ctrl + I`), set Window Grid Snapping to `16px`.
  2. Drag an interface window across the screen.
  3. Test intervals: `Off`, `8px`, `16px`, `32px`.
- **Expected Results:**
  - Window positions snap precisely to configured pixel grid increments.

### Test Case 11.5: 7 HUD Profiles & Combat Fading
- **Objective:** Verify HUD layout presets and out-of-combat opacity fading.
- **Steps:**
  1. In Game Settings, cycle through the 7 HUD profiles:
     - `Classic`, `Modern`, `Exploration`, `Dungeon`, `Healer`, `Farming`, `Minimal`.
     - *Expected:* Windows rearrange instantly to profile layout presets.
  2. Select `Modern` profile; enable **Combat Fading**.
  3. Stand out of combat for 5 seconds without moving or casting.
     - *Expected:* Hotbar, party frames, and target windows fade to 35% opacity.
  4. Cast a spell or take damage.
     - *Expected:* All interface windows immediately restore to 100% full opacity.

### Test Case 11.6: Chat Tabs, Timestamps & Link Security
- **Objective:** Verify chat filtering, timestamp toggles, and safe link parsing.
- **Steps:**
  1. In Game Settings, toggle ON **Chat Timestamps**.
  2. Cycle through the 5 chat tabs: `All`, `Party`, `Whisper`, `System`, `Loot`.
  3. Send chat message containing item link `<ITEM:1201>` and guide link `<GUIDE:jobs>`.
  4. Click the embedded links.
  5. Attempt to forge a malformed token: `<ITEM:99999999>`.
- **Expected Results:**
  - Messages display HH:MM:SS timestamps.
  - Tabs filter messages accurately according to category.
  - Valid links open the respective item tooltip or Guide page.
  - Malformed tokens are defanged and rendered as plain text.

---

## Section 12: Loot Filtering, Rare Wishlist & Ground Audio Cues

### Test Case 12.1: Visual Ground Loot Filtering
- **Objective:** Verify 3D ground item filtering modes.
- **Preconditions:** Spawn test drops on the ground:
  `@item 909 5` (Jellopy), `@item 1201 1` (Knife), `@item 4001 1` (Poring Card).
- **Steps:**
  1. Type `/loot gear` in chat.
     - *Expected:* Common items (Jellopies) hide from 3D ground render; weapons and armor stay visible.
  2. Type `/loot cards`.
     - *Expected:* Only cards remain visible on the ground.
  3. Type `/loot all`.
     - *Expected:* All ground drops reappear.

### Test Case 12.2: Wishlist Guaranteed Visibility
- **Objective:** Verify wishlisted items bypass loot filters.
- **Steps:**
  1. With `/loot cards` active (hiding common drops), add Jellopy to wishlist: `/wishlist 909`.
  2. Drop a Jellopy on the ground (`@item 909 1`).
- **Expected Results:**
  - Jellopy remains 100% visible on the ground with an alert highlight, bypassing the active filter.

### Test Case 12.3: Five Dedicated Ground Audio Cues
- **Objective:** Verify all 5 audio cues play their respective sounds with volume limiter.
- **Steps & Verification:**

| Event | How to Trigger | Target Audio File | Expected Behavior |
|---|---|---|---|
| **Dangerous Cast** | MVP boss begins signature cast | `data\wav\effect\warning.wav` | Warning chime plays (4s cooldown). |
| **Cast Interruption** | Stun/interrupt enemy cast bar | `data\wav\_stun.wav` | Impact stun cue plays (1.5s cooldown). |
| **Card / Wishlist Drop** | Card or wishlisted item drops | `data\wav\effect\p_success.wav` | High-priority fanfare plays (2s cooldown). |
| **Party Ping** | Party member sends `/ping` | `data\wav\party_alarm.wav` | Crisp alarm notification plays (3s cooldown). |
| **Quest Completion** | Complete quest or hunting goal | `data\wav\effect\complete.wav` | Quest completion chime plays (3s cooldown). |

*Audio Limiter Verification:* Trigger 10 drops in 1 second. Confirm audio limiter caps playback to 4 cues per 5-second window, preventing audio distortion.

### Test Case 12.4: Inventory Weight Threshold Warnings
- **Objective:** Verify 50% and 90% weight warnings.
- **Steps:**
  1. Load character with heavy items until weight reaches 50%.
     - *Expected:* HUD displays yellow weight warning: `"Weight > 50%: Natural HP/SP recovery halted"`.
  2. Continue loading character until weight reaches 90%.
     - *Expected:* HUD displays red weight warning: `"Weight > 90%: Cannot attack or cast spells"`.

---

## Section 13: Recoverable Death Penalty & Danger Guidance

### Test Case 13.1: Recoverable EXP Penalty Workflow
- **Objective:** Verify 1% EXP death penalty and 50% recovery upon 10 same-map kills.
- **Preconditions:** Character has $> 5\%$ Base and Job EXP on `prt_fild08`. Note exact EXP values.
- **Steps:**
  1. Allow monsters to kill character (or `@kill`).
     - *Expected:* Character loses 1% Base EXP and 1% Job EXP.
  2. Respawn at save point.
     - *Expected:* Status buff displayed: `"Recovering after respawn"`.
  3. Return to `prt_fild08` and defeat 10 monsters.
- **Expected Results:**
  - After the 10th monster kill, recovery script triggers.
  - 50% of the lost EXP (0.5% Base / 0.5% Job) is refunded.
  - Chat notification confirms: `"Combat recovery complete: 50% of lost EXP restored."`

---

## Section 14: GM Observability, Telemetry & Baseline Diffing

### Test Case 14.1: `@metrics` Tool Workflow
- **Objective:** Verify GM telemetry counters and privacy preservation.
- **Preconditions:** Logged in on `GM_Tester`.
- **Steps:**
  1. Type `@metrics status`.
     - *Expected:* Confirms metrics recording is **OFF** by default (opt-in privacy).
  2. Type `@metrics on`.
     - *Expected:* Metrics subsystem activates.
  3. Type `@metrics baseline save test_run`.
     - *Expected:* Baseline snapshot `test_run` stored.
  4. Perform 5 monster kills and complete a trade.
  5. Type `@metrics baseline diff test_run`.
- **Expected Results:**
  - Generates delta report showing total monster kills (+5) and trade events.
  - Zero player character names, account IDs, or private chat logged.

### Test Case 14.2: Non-GM Permission Enforcement
- **Objective:** Verify regular players cannot access GM observability tools.
- **Preconditions:** Logged in on `Player_Knight` (Account Level 0).
- **Steps:**
  1. Type `@metrics`.
  2. Type `@metrics status`.
- **Expected Results:**
  - Server refuses command with standard permission error: `"Unknown command or insufficient permissions."`

---

## Section 15: Master Playtest Pass/Fail Verification Matrix

| Test ID | Subsystem / Feature | Pass Criteria | Tester | Status |
|---|---|---|---|---|
| **TC-01** | Server Bring-Up | Clean start on ports 6900/6121/5121; zero fatal errors. | | [ ] Pass / [ ] Fail |
| **TC-02** | Client Launch | Korangar compiles & boots in release mode without crash. | | [ ] Pass / [ ] Fail |
| **TC-03** | UI Scaling | Crisp rendering at 1.0x, 1.25x, 1.5x, 2.0x scales. | | [ ] Pass / [ ] Fail |
| **TC-04** | WASD Movement | 8-directional smooth move, 200ms throttle, no wall clipping. | | [ ] Pass / [ ] Fail |
| **TC-05** | Input Focus Lock | Typing in chat/search does not trigger character movement. | | [ ] Pass / [ ] Fail |
| **TC-06** | Mouse Drag Pathing | Continuous hold-mouse navigation without packet flooding. | | [ ] Pass / [ ] Fail |
| **TC-07** | Obstacle Pathing | Natural A* pathfinding around corners and fences. | | [ ] Pass / [ ] Fail |
| **TC-08** | Camera & Sit/Stand | Smooth orbit, pitch bounds, zoom, and `/sit` toggle. | | [ ] Pass / [ ] Fail |
| **TC-09** | Tab Target Cycling | Nearest hostile cycling; Shift+Tab reverse; dead mobs skipped. | | [ ] Pass / [ ] Fail |
| **TC-10** | Range Rings | Chebyshev circle on hover; green in-range, red out-of-range. | | [ ] Pass / [ ] Fail |
| **TC-11** | Ground Footprints | Fire Wall (3x1), Storm Gust (9x9), Sanctuary (diamond). | | [ ] Pass / [ ] Fail |
| **TC-12** | Cast Modes | Classic, Quickcast, and Hold-Aim-Release execute cleanly. | | [ ] Pass / [ ] Fail |
| **TC-13** | Per-Skill Overrides | Per-skill cast mode overrides take priority over global setting. | | [ ] Pass / [ ] Fail |
| **TC-14** | Cast Cancellation | Esc/Right-click aborts cast (`CZ_CANCEL_CAST`); SP preserved. | | [ ] Pass / [ ] Fail |
| **TC-15** | Hold-Aim Abort | Esc before release cancels Hold-Aim cast; zero packet sent. | | [ ] Pass / [ ] Fail |
| **TC-16** | Coward AI | Poring flees at $<20\%$ HP. | | [ ] Pass / [ ] Fail |
| **TC-17** | Aggressor AI | Fabre pursues player within line of sight. | | [ ] Pass / [ ] Fail |
| **TC-18** | Skirmisher AI | Orc Skeleton steps back 2 cells after every 3 hits. | | [ ] Pass / [ ] Fail |
| **TC-19** | RangedKeeper AI | Orc Archer maintains range and paths around Fire Wall. | | [ ] Pass / [ ] Fail |
| **TC-20** | Raydric Chivalry | Chivalry family rushes to protect archers; no erratic hopping. | | [ ] Pass / [ ] Fail |
| **TC-21** | Elite Orc Skeleton | ID 20901: 3x HP/EXP, `[Elite]` nameplate, 1.2s interruptible Bash. | | [ ] Pass / [ ] Fail |
| **TC-22** | AI Rollback Switch | `mob_pilot_version: 0` restores stock AI and clears elites. | | [ ] Pass / [ ] Fail |
| **TC-23** | Boss Target Window | Dedicated 360x200 window: HP bar, Level, Element, Target-of-Target. | | [ ] Pass / [ ] Fail |
| **TC-24** | Boss Anti-Teleport | All 11 bosses have 0% random teleport-on-hit. | | [ ] Pass / [ ] Fail |
| **TC-25** | Boss Cast Interrupt | High ASPD/Stun breaks signature cast; stun chime plays. | | [ ] Pass / [ ] Fail |
| **TC-26** | Boss Recovery Window | $\ge 15$s quiet auto-attack window after signature moves. | | [ ] Pass / [ ] Fail |
| **TC-27** | Boss Enrage Phase | $<30\%$ HP enrage activates with visual roar. | | [ ] Pass / [ ] Fail |
| **TC-28** | Encounter Recap | Defeat toast: `"Defeated <Boss>! Dealt <Dmg> in <Duration>s"`. | | [ ] Pass / [ ] Fail |
| **TC-29** | Eddga Pilot | Magnum Break (2.0s), Meteor (3.0s), Bigfoot x4 verified. | | [ ] Pass / [ ] Fail |
| **TC-30** | Moonlight Flower | Pulse Strike (2.0s), Fire Wall (1.5s), Nine Tail x3 verified. | | [ ] Pass / [ ] Fail |
| **TC-31** | Golden Thief Bug | Magnum (2.0s), Fireball (1.5s), Reflect Shield verified. | | [ ] Pass / [ ] Fail |
| **TC-32** | Orc Hero | Spiral Pierce (2.0s), Thunderstorm (2.5s), 2HQ verified. | | [ ] Pass / [ ] Fail |
| **TC-33** | Maya | Brandish (2.0s), Heaven's Drive (2.0s), Auto Guard verified. | | [ ] Pass / [ ] Fail |
| **TC-34** | Baphomet | Hell Judgement (2.5s), LoV (3.0s), Earthquake verified. | | [ ] Pass / [ ] Fail |
| **TC-35** | Phreeoni | Heaven's Drive (2.0s), Petrify (1.5s), Hode x3 verified. | | [ ] Pass / [ ] Fail |
| **TC-36** | Mistress | Jupitel (2.0s), Wide Silence (1.5s), Hornet x4 verified. | | [ ] Pass / [ ] Fail |
| **TC-37** | Drake | Waterball (2.5s), Dragon Fear (1.5s), Wraith x3 verified. | | [ ] Pass / [ ] Fail |
| **TC-38** | Doppelganger | Spiral Pierce (2.0s), Hammer Fall (1.5s), 2HQ verified. | | [ ] Pass / [ ] Fail |
| **TC-39** | Osiris | Meteor Assault (2.0s), Venom Dust (1.5s), Mummy x4 verified. | | [ ] Pass / [ ] Fail |
| **TC-40** | Stats Modes | Simple, Detailed, Advanced modes with exact Renewal math. | | [ ] Pass / [ ] Fail |
| **TC-41** | Build Planner Sliders | Base Lv (1-99) -> 1,225 pts; Job Lv (1-50) -> 49 skill pts. | | [ ] Pass / [ ] Fail |
| **TC-42** | Stat Point Costs | Scaled costs (2 to 16 pts); 99 cap enforced. | | [ ] Pass / [ ] Fail |
| **TC-43** | Skill Prerequisites | Blocker message when prerequisite skill is missing. | | [ ] Pass / [ ] Fail |
| **TC-44** | Build Plan Save/Load | 3 disk slots roundtrip correctly; persists across client reboot. | | [ ] Pass / [ ] Fail |
| **TC-45** | Free Respecs | `@streset`, `@skreset`, `@refundskill` work without zeny cost. | | [ ] Pass / [ ] Fail |
| **TC-46** | Adventure Guide | 11 category tabs complete; provenance header stamp shown. | | [ ] Pass / [ ] Fail |
| **TC-47** | Search Aliases | Nicknames (sin, bs, freeze, hydra) resolve to correct entries. | | [ ] Pass / [ ] Fail |
| **TC-48** | Second Job Guide | All 12 second jobs, Job 40 vs 50 rules, transcendent path. | | [ ] Pass / [ ] Fail |
| **TC-49** | Card Stacking Rules | Additive within same category, multiplicative across categories. | | [ ] Pass / [ ] Fail |
| **TC-50** | Refinement Odds | Weapons Lv 1-4, Armor odds, safe limits, destruction warning. | | [ ] Pass / [ ] Fail |
| **TC-51** | World Atlas Regions | 6 geographic regions, Walk vs Transport distinct road styling. | | [ ] Pass / [ ] Fail |
| **TC-52** | Route Finder | Multi-hop breadcrumb route without teleport bypass. | | [ ] Pass / [ ] Fail |
| **TC-53** | Danger Warning | Toast on maps $\ge$ player level + 15; opt-out toggle works. | | [ ] Pass / [ ] Fail |
| **TC-54** | Minimap Controls | Waypoint left-click drop / right-click clear; scroll zoom. | | [ ] Pass / [ ] Fail |
| **TC-55** | Commission Board UI | Window opens, risk disclosure banner shown, post & list work. | | [ ] Pass / [ ] Fail |
| **TC-56** | Commission Security | Only requester can cancel request; other players refused. | | [ ] Pass / [ ] Fail |
| **TC-57** | Cooking Formulas | Kit bonuses (Outdoor to Legendary) and dish formulas verified. | | [ ] Pass / [ ] Fail |
| **TC-58** | Trade Partner Zeny | Zeny displays accurately in zeny box; zero "Item #0" bug. | | [ ] Pass / [ ] Fail |
| **TC-59** | Trade Scam Warning | Bold warning banner when offer changes after player locks. | | [ ] Pass / [ ] Fail |
| **TC-60** | Trade Refusal Guard | Confirm clicked $< 1.5$s after change is refused with toast. | | [ ] Pass / [ ] Fail |
| **TC-61** | Trade Command Guard | `/trade commit` cannot bypass the 1.5s anti-scam cooldown. | | [ ] Pass / [ ] Fail |
| **TC-62** | Trade Completion | Final state summary displayed; items and zeny transfer cleanly. | | [ ] Pass / [ ] Fail |
| **TC-63** | Party Tracking | Tile distance on same map; red map name on cross-map. | | [ ] Pass / [ ] Fail |
| **TC-64** | Healer Layout | Enlarged HP/SP bars; click-to-target party member. | | [ ] Pass / [ ] Fail |
| **TC-65** | Tactical Markers | `/mark focus/attack/cc/assist`; 30s auto-expiry; owner clear. | | [ ] Pass / [ ] Fail |
| **TC-66** | Tonight's Goals | `/goal add/done/clear`; HUD pinning under `Tonight:`. | | [ ] Pass / [ ] Fail |
| **TC-67** | Party EXP Spread | 30-level spread boundary; 25% even-share bonus verified. | | [ ] Pass / [ ] Fail |
| **TC-68** | Window Edge Clamp | Dragging clamped so headers/close buttons remain reachable. | | [ ] Pass / [ ] Fail |
| **TC-69** | Tooltip Clamping | Viewport edge clamping prevents text cut-off on screen borders. | | [ ] Pass / [ ] Fail |
| **TC-70** | Esc Focus Release | First Esc releases text box focus without closing window/menu. | | [ ] Pass / [ ] Fail |
| **TC-71** | Grid Snapping | Snap intervals (Off, 8px, 16px, 32px) position windows cleanly. | | [ ] Pass / [ ] Fail |
| **TC-72** | HUD Profiles | 7 presets switch window positions instantly. | | [ ] Pass / [ ] Fail |
| **TC-73** | Combat Fading | 35% alpha out of combat after 5s; instant 100% on combat/damage. | | [ ] Pass / [ ] Fail |
| **TC-74** | Ground Loot Filter | `/loot all/gear/cards` visually filters ground drops. | | [ ] Pass / [ ] Fail |
| **TC-75** | Wishlist Bypass | `/wishlist <id>` ensures dropped item stays visible in all modes. | | [ ] Pass / [ ] Fail |
| **TC-76** | 5 Audio Cues | Warning, Stun, Complete, Alarm, Fanfare play; limiter holds. | | [ ] Pass / [ ] Fail |
| **TC-77** | Weight Warnings | 50% recovery stop and 90% attack/cast block warnings. | | [ ] Pass / [ ] Fail |
| **TC-78** | Death Recovery | 1% EXP loss; 50% refund after 10 kills on the same map. | | [ ] Pass / [ ] Fail |
| **TC-79** | GM Telemetry | `@metrics` opt-in, baseline save/diff; non-GM players refused. | | [ ] Pass / [ ] Fail |

---

## Section 16: Defect Logging, Diagnostic Evidence & Triage Guidelines

When logging an issue or anomaly during live playtesting, please adhere to the defect capture protocol below:

### 16.1 Issue Report Template
```markdown
### Playtest Issue Report

- **Test Case ID:** (e.g. TC-59 Trade Scam Warning)
- **Subsystem:** (e.g. Section 9 Direct Player Trade)
- **Character Name & Job:** (e.g. Player_Knight, Lv 85 Knight)
- **Client OS & Resolution:** (e.g. macOS 14.5, 2560x1440, UI Scale 1.25x)
- **Server Revision:** (Git commit hash, e.g. 593d20ef)
- **Map & Coordinates:** (`/where`, e.g. prontera 155, 150)
- **Preconditions:** (e.g. Both clients in trade window; Client A locked with Knife [3])
- **Steps to Reproduce:**
  1. Client B altered zeny from 250,000 to 25,000.
  2. Client A observed trade window.
  3. Client A clicked Confirm Trade within 0.5s.
- **Expected Outcome:**
  Warning banner displays `!! Partner CHANGED their offer after you locked`; Confirm Trade is refused with toast.
- **Observed Behavior:**
  Warning banner displayed, but Confirm Trade was accepted immediately without refusal toast.
- **Severity Level:** (Blocker / Critical / Major / Minor / Cosmetic)
- **Attached Diagnostics:**
  - Client screenshot / video clip
  - Client stdout log: `korangar.log`
  - Map-server log: `Hercules/log/map-server.log`
```

### 16.2 Severity Definitions
- **Blocker:** Server crash, memory leak, infinite loop, packet desync disconnecting clients, or loss of character data/items.
- **Critical:** Failure of core contract (e.g. unauthorized commission cancellation, trade anti-scam bypass, MVP teleporting on hit).
- **Major:** Feature broken (e.g. Build planner fails to load save slot, minimap waypoints don't appear, audio cues don't play).
- **Minor:** Visual glitch, layout misalignment, tooltip clipping, or incorrect text formatting.
- **Cosmetic:** Minor color discrepancy, font kerning, or typo in Guide text.
