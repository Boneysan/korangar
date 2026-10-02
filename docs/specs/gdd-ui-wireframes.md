# GDD next-slice UI wireframes

**Parent:** [GDD §§5, 8–10, 15](../GDD.md) and [next-slices plan](../plans/gdd-next-slices.md). These are small-screen content and interaction contracts for the numbered UI slices, not final pixel art. Use existing theme tokens, window controls, and UI scaling. A designer may refine spacing and imagery without changing the information hierarchy.

## Persistent in-world HUD (1, 3, 6, 12, 14)

```text
+--------------------------------------------------------------+
| [Quest tracker: 2 pinned]              [Minimap]             |
|  □ Rat Tails  3/5                     [next exit →]           |
|  □ Talk to Sun-Hwa  [Guide]             [party ping •]        |
|                                                              |
| [Monster: Orc Archer] [HP] [cast 1.2 s]                      |
| [Race] [Size] [Element]  [Guide entry]                        |
|                                                              |
| [party]                                      [toast x 2 max]  |
| [HP/SP]   [hotbars]                           [chat]          |
+--------------------------------------------------------------+
```

- The monster surface appears when a hostile entity is selected by click or Tab. It closes on despawn, death, logout, or explicit clear; a hidden knowledge field is labeled “Discover to reveal” in detail view rather than drawn as a misleading value. The cast bar remains legible if race/size/element are unknown.
- The quest tracker shows only pinned quests and client hunting goals, with distinct icons/text. A quest without a known location opens its log instead of displaying an invented route. The tracker never covers the minimap at the smallest supported viewport.
- Next-exit marker names the destination map on focus/hover. Unavailable route shows “No verified route from this map” beside the target; it leaves normal map/quest information visible.
- Toasts stack at most two high, with a short summary and optional Open action. A third waits or coalesces. They do not cover the target cast bar, NPC dialog, or chat input. The event remains in chat/log history.
- Edit HUD overlays clear handles and lock state. Locked windows cannot move. Combat fade never hides cast warnings, party danger, or a focused interactive control.

## Inventory/storage and destructive actions (4, 7)

```text
Inventory                         Storage
[Search items________] [Type ▼]   [Search items________] [Type ▼]
[Sort: Name ▼] [Show locked ✓]    [Sort: Name ▼]
[item grid, original slot IDs]    [item grid, original slot IDs]
Selected: Oridecon x3  [Lock] [Drop…] [Use/Equip]
```

- Search and sort change view order only; server inventory indices remain attached to each rendered stack. Storage uses the same query component, without pretending its server actions are inventory actions.
- Lock is per character and **item ID**: every copy of that item type shares the lock. A locked item gives an explanation before Drop or NPC Sell. The visible Drop opens quantity selection, then confirmation for locked/rare items.
- Character card shows Play and Delete… as separate visible actions. Delete opens a plate with the exact character name to type, a Cancel button, and a disabled final button until the name matches. The existing server delete request is sent only by that final action.
- Empty search says “No matching items” with a Clear search action. Missing item metadata remains visible under “Other”, not silently filtered out.

## Adventure Guide (8–9)

```text
Adventure Guide  [Search Hydra____________]
[All] [Monsters] [Items] [Cards] [Skills] [Maps] [Quests] ...
Results              | Hydra — Monster (known identity)
Hydra • Monster      | Lv / race / size / element; discovery badge
Hydra Card • Card    | Drops / rates / effects (when verified)
Byalan • Map         | Found in: Byalan Island [Navigate]
                     | Related: Hydra Card →    Server data revision
```

- Search is global, including aliases/effect terms; category tabs filter results without changing the query. The selected result shows source revision and incomplete-data notes when needed. Verified build facts are visible before discovery; an unknown field says “Not documented yet”.
- Cross-links preserve the search and scroll position so a player can travel item → monster → map and back. Navigate only appears for a verified destination from the map graph. The player view never includes DM Spawn or reveal-all controls.
- Keyboard: focus search on open, arrows move results, Enter opens, Back returns to prior entry, Escape closes or leaves the field according to the shared window convention. Empty, loading, stale-revision, and no-route states have text labels.

## Ready check, pings, bindings, and accessibility (11, 13, 15)

- A ping is visible on the minimap and in-world with kind **and** text, sender, map, and remaining lifetime. A ping on another map shows in the party notice without a false local-world marker. Ready check shows member names and Ready/Waiting/Declined, a deadline, and a dismiss action; duplicate replies do not create duplicate rows.
- Remap screen groups movement, combat, windows, and communication actions. Each row shows current chord, Change, and Reset. On conflict it names the existing action and asks Replace or Cancel; reserved OS/text-entry keys cannot be bound to gameplay. Defaults remain usable without opening this screen.
- Accessibility settings preview high-contrast and color-vision themes, reduced motion, combat-text density/size, and effect density. A danger cast at minimum density remains readable by shape and text, not only hue. Alternate ground-target confirmation has an on-screen description and can be tried without spending a skill.

## Integrated Build Planner (F03, §§7.4, 7.6)

```text
+-----------------------------------------------------------------------------------+
| Build Planner (Simulation)                                                [X]     |
| [Target Base Lv: < 15 > ==|==================== 99 ]  Stat Points: 42 rem / 1273  |
| [Target Job  Lv: < 40 > =======|=============== 50 ]  Skill Points: 8 rem / 49    |
| Class: Knight (Job ID 7) · [Load Plan ▼] [Save Plan] [Reset Plan]                  |
+-----------------------------------------------------------------------------------+
| Planned Stats & Derived Projections                                               |
|  STR:  45 (+ 8)  [-1] [+1] (Cost: 6)   |  HP:   4,210 (+1,850)   HIT:   215 (+42)  |
|  AGI:  30 (+ 2)  [-1] [+1] (Cost: 4)   |  SP:     340 (+  120)   FLEE:  185 (+30)  |
|  VIT:  35 (+ 4)  [-1] [+1] (Cost: 5)   |  ATK:    142 (+   58)   ASPD:  164 (+ 8)  |
|  INT:  12 (+ 0)  [-1] [+1] (Cost: 3)   |  MATK:    28 (+    9)   Hard DEF: 45 (+12)|
|  DEX:  32 (+ 3)  [-1] [+1] (Cost: 5)   |  Cast: -18.2% (estimate)Soft DEF: 28 (+10)|
|  LUK:   9 (+ 2)  [-1] [+1] (Cost: 2)   |  Soft MDEF: 16 (+ 4)    Hard MDEF: 0 (+ 0)|
+-----------------------------------------------------------------------------------+
| Planned Skill Tree [First Job] [Second Job]                                       |
|  [Icon] Two-Hand Sword Mastery  Lv 10/10  [-1] [+1]                               |
|  [Icon] Two-Hand Quicken         Lv  7/10  [-1] [+1] (Req: 2H Sword Mastery Lv 1)  |
|  [Icon] Bowling Bash             Lv  5/10  [-1] [+1] (Req: Bash Lv 10, MB Lv 3...) |
|  [Icon] Brandish Spear           Lv  0/10  [-1] [+1] (Requires: Spear Mastery Lv 1)|
+-----------------------------------------------------------------------------------+
| NOTE: Simulated planner state only. No network packets are sent to the server.    |
| Live stat/skill point allocations remain separate in the Character Overview window.|
+-----------------------------------------------------------------------------------+
```

- Target Base Level and Job Level sliders project total available points from verified server tables (`db/re/statpoint.txt` and `db/re/job_db.conf`), clamped to the character's class maximums.
- Stat point costs strictly track Renewal threshold costs (`2 + (v - 1)/10` below 100, `16 + 4*((v - 100)/5)` at 100+); cost indicators update dynamically.
- Derived combat values (HP, SP, HIT, FLEE, ASPD, DEF, MDEF, ATK, MATK, Cast reduction) compute live deltas against the active character using verified Renewal engine formulas; all client-side estimates are clearly marked.
- Skill allocations enforce prerequisite tree dependencies using `ReferenceJobSkillTree`; unfulfilled dependencies reject incrementing until prereqs are met.
- Plans can be named, serialized to local storage, loaded, or reset back to the character's current live baseline.
- **Safety guarantee:** The Build Planner never sends `StatUp` or `LevelUpSkills` network packets. Live point spending remains exclusively in the Stats/SkillTree window Apply flows.

## Stats Interface Modes & Equipment Comparison (F04, §§7.6, 10.8, 10.10)

### Stats Window Modes (Simple / Detailed / Advanced)

```text
+-----------------------------------------------------------------------+
| Character Stats                                               [X]     |
| [Simple]  [*Detailed*]  [Advanced]           Available Points: 14     |
+-----------------------------------------------------------------------+
| STR:  45 (+ 8)  [+1] (Cost: 6)  | Next STR: +1 Status ATK, +30 Weight |
|                                 | Skills: Bash, Bowling Bash, Pierce  |
| AGI:  30 (+ 2)  [+1] (Cost: 4)  | Next AGI: +1 FLEE                   |
|                                 | Next Soft DEF breakpoint in 2 AGI   |
| VIT:  35 (+ 4)  [+1] (Cost: 5)  | Next VIT: +1% Max HP                |
|                                 | Next Soft DEF breakpoint in 1 VIT   |
| INT:  12 (+ 0)  [+1] (Cost: 3)  | Next INT: +1.5 MATK, +1 Soft MDEF   |
|                                 | -0.19% Variable Cast (estimate)     |
| DEX:  32 (+ 3)  [+1] (Cost: 5)  | Next DEX: +1 HIT, -0.38% Cast (est) |
|                                 | Skills: Double Strafe, all casts    |
| LUK:   9 (+ 2)  [+1] (Cost: 2)  | Next LUK: +0.3 CRIT, +0.1 P.Dodge   |
|                                 | Next CRIT point in 1 LUK (exact)    |
+-----------------------------------------------------------------------+
| Combat Projections & Breakpoints                                      |
|  HP: 4,210  SP: 340   HIT: 215 (exact)   FLEE: 185 (exact)            |
|  ATK: 142 (Status: 62 + Weapon: 80)     MATK: 28 (exact)              |
|  Soft DEF: 28 (exact) Hard DEF: 45      Soft MDEF: 16 Hard MDEF: 0    |
|  Variable Cast Time: -18.2% (estimate: stat contribution)             |
+-----------------------------------------------------------------------+
| [Advanced Mode Preview]:                                              |
|  Status ATK = floor(Lv/4) + STR + floor(DEX/5) + floor(LUK/3) (exact) |
|  Soft DEF   = floor((BaseLv + VIT)/2) + floor(AGI/5) (exact)          |
|  Soft MDEF  = INT + floor(VIT/5) + floor(DEX/5) + floor(Lv/4) (exact) |
|  HIT        = 175 + BaseLv + DEX + floor(LUK/3) (exact)               |
|  FLEE       = 100 + BaseLv + AGI + floor(LUK/5) (exact)               |
+-----------------------------------------------------------------------+
```

### Equipment & Card Comparison Tooltip with Script Bonuses & Target Context

```text
+-----------------------------------------------------------------------+
| +7 Flamberge [2]                                                      |
| One-Handed Sword · Weapon                                             |
| ATK 150  Req. Lv 48  Weight 120.0  Slots 2                            |
|                                                                       |
| — Card Sockets —                                                      |
| [1] Hydra Card: +20% physical damage vs Demi-Human                    |
| [2] Skeleton Worker Card: +15% physical damage vs Medium, ATK +5      |
|                                                                       |
| — Special Effects —                                                   |
| • +20% physical damage vs Demi-Human                                  |
| • +15% physical damage vs Medium                                      |
| • ATK +5                                                              |
|                                                                       |
| — vs equipped: +5 Broadsword [1] (Andre Card) —                       |
| ATK 150 (+25)   Slots 2 (+1)   Refine +7 (eq +5)                      |
| Damage vs Demi-Human: +20% (new)                                      |
| Damage vs Medium: +15% (new)                                          |
| Base ATK bonus: +5 (eq +20, delta -15)                                |
|                                                                       |
| — vs selected monster: Orc Archer (Demi-Human, Medium, Earth 1) —     |
| Race Bonus:   +20% vs Demi-Human (Hydra Card)                         |
| Size Bonus:   +15% vs Medium (Skeleton Worker Card)                   |
| Total vs Target: +35% physical damage multiplier (estimate)           |
| Net advantage vs equipped weapon: +18.4% effective DPS (estimate)     |
| Weapon Element: Neutral vs Earth 1 → 100% effectiveness (exact)       |
+-----------------------------------------------------------------------+
```

- **Stats interface modes (§10.8):**
  - **Simple:** Concise role description for each stat, highlighting primary offensive/defensive functions and weight capacity.
  - **Detailed:** Current base and bonus values, point upgrade cost, exact next-point derived deltas, and affected active/passive skills.
  - **Advanced:** Complete mathematical formulas, component breakdowns (base / job / equip), exact Renewal defense and attack formulas, breakpoint tracking, and cast time estimates.
- **Plain-language equipment bonuses (§10.10):**
  - Scripts are translated into unambiguous player-facing bonuses (`bonus bStr, 3` → `STR +3`; `bonus2 bAddRace, RC_DemiPlayer, 20` → `+20% physical damage vs Demi-Human`).
  - Unsupported or unmodeled script commands (e.g. complex autospell logic, unreviewed scripts) are explicitly flagged as `[Unsupported script effect: ... (unmodeled)]` rather than invented or silently omitted.
- **Versus-monster contextual comparison (§10.10, §5.13):**
  - When a monster is selected, hovered equipment and cards evaluate race, size, element, and defense against the target's verified facts.
  - Elemental effectiveness multipliers look up authoritative rates from `attr_fix.conf` (`exact`).
  - Combined damage multipliers against target are labelled `(estimate)`.

## F05 · World Map & Route Finder (§9.2, §9.8)

### World Map Atlas Overview & Destination Details Wireframe

```text
+---------------------------------------------------------------------------------------------------+
| World Map & Route Finder                                                                      [X] |
+---------------------------------------------------------------------------------------------------+
|  [Rune-Midgarts]  [Schwarzwald]  [Arunafeltz]  [Global Project]  [Dimensional Gorge]              |
|                                                                                                   |
|           [Juno]=================(Airship)=================[Einbroch]---(Train)---[Einbech]       |
|             |                                                  |                     |            |
|             |                                              (Airship)                 |            |
|        [Al De Baran]---------[Lutie] (Sleigh)                  |                [Lighthalzen]     |
|             |      \                                           |                                  |
|         (Mjolnir)   \                                      [Alberta]========(Voyage)======[Amatsu]|
|             |        \                                      /    \                         |      |
|          [Geffen]----[Prontera]                           /      (Cat Fleet)               |      |
|             |       /    |     \                         /          \                      v      |
|             |  (Royal) (Sograt) (Highway)               /         [Malangdo]          [Amatsu     |
|             |   /        |         \                   /                                   Cave]  |
|             v  v         v          v                 /                                           |
|          [Payon]      [Morroc]     [Izlude]====(Ferry)====[Byalan Island (iz_dun00)]              |
|                          |            |                                                           |
|                          |         (Honeymoon)                                                    |
|                          v            v                                                           |
|                       [Umbala]-----[Jawaii]                                                       |
|                          |                                                                        |
|                       [Comodo]                                                                    |
+---------------------------------------------------------------------------------------------------+
| Destination: iz_dun00 • visited • Dungeons & Landmarks                                            |
| Suggested level: ~42 (static-spawn mean)                                                          |
| Static population: 145 spawn records • 8 species                                                  |
| Party here: Alice, Bob                                                                            |
| Facilities: Kafra Employee, Tool Dealer, Ferry Sailor                                             |
| Connections: 2 travel legs (1 walk, 1 transport) • 2 connections • next: service at izlude        |
|              (197, 205) — choose Byalan Island (conditional) • Costs 150 zeny. → izlu2dun         |
+---------------------------------------------------------------------------------------------------+
| Route Status: Gold: route  •  Cyan: overland  •  Blue: in-world transport  •  Cyan dots: trail   |
| [ Clear Route ]                                                                                   |
+---------------------------------------------------------------------------------------------------+
```

- **World Map Atlas & Navigation Graph (§9.2):**
  - Displays authoritative locations mapped onto the verified warp graph across 6 major regions: Rune-Midgarts Kingdom, Republic of Schwarzwald, Arunafeltz States, Global Project, Dimensional Gorge, and Dungeons & Landmarks.
  - Overland walk roads are distinguished from in-world transport lines (airships, passenger ferries, cat fleet expeditions, Santa Claus flying sleighs, dimensional rifts).
  - Selected routes are highlighted as minimum-hop paths in gold, with breadcrumbs drawn on the active minimap.
- **In-World Transport Edges & NPC Services (§9.8):**
  - Distinct transport edges (e.g. Izlude ↔ Byalan Island ferry, Izlude/Alberta ↔ Malangdo cat fleet, Juno ↔ Einbroch airship, Al De Baran ↔ Lutie sleigh) describe departure coordinates, exact dialog action, availability (`always` vs `conditional`), and fare or quest requirements.
  - In-world transport edges are explicit travel legs, never mislabeled as walk-warp portal entities.
- **Account-Wide Visited Flags & Discovery Sync (§9.6):**
  - Visited locations display `✓` for visited, `·` for unvisited, and `!` for danger-level disparity (mean spawn level ≥ player level + 15).
  - Visited history is preserved account-wide: switching characters on the same account retains all visited maps without re-synchronizing.
  - Logging in from a different account resets visited history, enforcing strict account isolation.
- **Non-Teleporting Route Safety Contract (§9.2, §9.8):**
  - Clicking any atlas location or facility queues `InputEvent::SetNavigationDestination`.
  - The client provides guidance and minimap breadcrumbs only; it **never** emits teleportation packets, admin warps, or server-side position updates.
  - If a destination is unreachable from the current map, the UI displays "No verified route" and emits a toast notice without modifying navigation state.

## F06 · Map Information & Danger Guidance (§9.6, §9.8, §5.13)

### Map Information & Danger Guidance Panel Wireframe

```text
+---------------------------------------------------------------------------------------------------+
| Map Information — Orc Dungeon 2F (orcsdun02)                                                  [X] |
+---------------------------------------------------------------------------------------------------+
| Region: Dungeons & Landmarks                                       Discovery: Visited (Account)   |
| Status: ! DANGEROUS LEVEL DISPARITY (Average Level: 35 vs Your Level: 20)                         |
+---------------------------------------------------------------------------------------------------+
| Level Guidance:                                                                                   |
|   Suggested level: ~35 (range: 24–45) (static-spawn mean)                                         |
|                                                                                                   |
| Population Summary:                                                                               |
|   Static population: 180 spawn records • 6 species                                                |
|   Key Spawns: Orc Archer (Lv 24), Zenorc (Lv 34), Orc Skeleton (Lv 24), High Orc (Lv 45)         |
|                                                                                                   |
| In-Map Facilities & Services:                                                                     |
|   Towninfo facilities: none listed for this map                                                   |
|                                                                                                   |
| Connections & Travel:                                                                             |
|   Connections: 1 portal exit (→ orcsdun01)                                                        |
|   Path from current: 3 travel legs (walk warp)                                                    |
|                                                                                                   |
| Party Presence:                                                                                   |
|   Party here: Alice (Archer Lv 22), Bob (Priest Lv 38)                                            |
+---------------------------------------------------------------------------------------------------+
| [!] Non-Blocking Danger Warning Toast:                                                            |
|     "Caution: orcsdun02 averages level 35, 15+ above your level (20).                             |
|      Warning only; travel is unrestricted."                                                       |
+---------------------------------------------------------------------------------------------------+
| [✓] Warn on entering dangerous maps (Settings toggle)       [ Set Route ]   [ View on Atlas ]     |
+---------------------------------------------------------------------------------------------------+
```

- **Spawn-Derived Suggested Level Range (§9.6, §5.13):**
  - Computes the mean spawn level and exact min–max range from verified static spawn records: `Suggested level: ~{mean} (range: {min}–{max}) (static-spawn mean)`.
  - Non-combat maps (towns, indoor houses) or maps lacking verified static spawn definitions explicitly report: `Suggested level: unavailable (low-coverage or non-combat map)`. The system never fabricates or guesses level ranges.
- **Authoritative Static Population Summary (§9.6):**
  - Summarizes static population records and species diversity: `Static population: {records} spawn records • {species} species`.
  - When static records are unavailable, explicitly reports `Static population: no verified spawn records (low coverage)`.
- **Connections & NPC Facilities (§9.8):**
  - Details outgoing walk portal exits and available in-world transport connections.
  - Lists localized town facilities and service NPCs (Kafra, Tool Dealer, Weapon Dealer, Stylist, Inn) with coordinate-verified navigation routes.
- **Party Presence (§9.6):**
  - Real-time enumeration of online party members located on the selected map: `Party here: {names}`.
  - When no online party members are present, cleanly indicates: `Party here: no online members reporting this map`.
- **Non-Blocking Opt-Out Danger Guidance Contract (§9.6):**
  - Trigger threshold: `mean_spawn_level >= player_level + 15`.
  - Non-blocking guarantee: Danger warnings are non-blocking toasts; map entry, character movement, warp portal traversal, and navigation destination setting are **never locked or prevented**.
  - Opt-out toggle: `warn_dangerous_maps` client setting allows players to completely suppress high-level warnings.
  - Coverage integrity: Low-coverage or missing spawn maps return `None` and do not produce false alarms.

## F07 · Monster Population Regions, Portal Destination Labels & Route Accents (§9.4, §9.9, §10.12)

### Minimap Overlays & Portal Destination Accents Wireframe

```text
+-------------------------------------------------------------------------------+
| Map — Prontera Field 08 (prt_fild08)                                      [X] |
+-------------------------------------------------------------------------------+
| [ N: prontera ]                                                               |
|         +===================================================+                 |
|         | [★] → Route portal: prontera (gold accent)        |                 |
|         |                                                   |                 |
|         |     +---------------------------------------+     |                 |
|         |     |                                       |     |                 |
| [ W:    | [■] |   BROAD POPULATION REGION OVERLAY     | [■] | [ E: izlude ]   |
| prt_    | Por-|   • Poring (Map-wide • High population)  | Por-| Portal to       |
| fild07] | tal |   • Lunatic (Map-wide • Medium)       | tal | izlude          |
| Portal  | to  |                                       | to  | (soft violet)   |
| to      | prt_|     [You] (Player blip)               | iz- |                 |
| prt_    | fild|                                       | lude|                 |
| fild07  | 07  +---------------------------------------+     |                 |
|         |                                                   |                 |
|         | [■] Portal to moc_fild01 (soft violet)            |                 |
|         +===================================================+                 |
| [ S: moc_fild01 ]                                                             |
+-------------------------------------------------------------------------------+
| Prontera Field 08 (182, 240)    [ - ] [ + ]                                   |
| [ Location ] [ Assist ] [ Danger ] [ Retreat ] [ Ready ] [ On my way ]        |
| [ Share current route ]                                                       |
+-------------------------------------------------------------------------------+
| Layer Settings:                                                               |
|   [✓] Show portal destination markers on minimap                              |
|   [✓] Show monster population regions on minimap                              |
|   [✓] Show towninfo facility markers on minimap                               |
|   [✓] Show party member blips on minimap                                      |
|   [✓] Show quest marks on minimap                                             |
+-------------------------------------------------------------------------------+
```

- **Broad Population Rectangles & Qualitative Density (§9.4):**
  - Exports 3,380 broad spawn rectangles across 368 maps (`docs/spawn-rectangles.v1.json`) derived from static placements.
  - Qualitative density classification:
    - **High population:** amber/gold tinted overlay (`PopulationDensity::High`, amount ≥ 20 for map-wide or ≥ 15 for localized).
    - **Medium population:** cyan/teal tinted overlay (`PopulationDensity::Medium`, amount ≥ 6 for map-wide or ≥ 5 for localized).
    - **Low population:** soft slate blue tinted overlay (`PopulationDensity::Low`, amount < 6 for map-wide or < 5 for localized).
  - Exploration integrity guarantee: **Never reveals exact randomized runtime spawn cells or respawn timers.**
  - Selected monster filter: highlights the player's tracked hunting goal or active monster selection while keeping broad search boundaries.
- **Verified Portal Destination Labels (§9.9):**
  - Hovering any verified walk-warp on the minimap or 3D world identifies its exact destination map name: `"Portal to {to_map}"`.
  - Inactive, disabled, or unindexed warps retain classic unidentified behavior and are never exposed.
  - NPC services (passenger ferries, cat fleet expeditions, airships) are distinct transit steps and are **never mislabeled as walk-warp portal entities**.
- **Tracked-Route Portal Accents (§9.9, §10.12):**
  - When a navigation destination is active, the next verified exit leading along the route is accented:
    - Minimap: bright gold tint (`255, 210, 70`) and 1.25x size scale.
    - Hover tooltip: `"→ Route portal: {to_map}"`.
    - In-world hover text: `"→ Route portal: {to_map}"`.
  - Non-route portal exits remain soft violet (`190, 110, 245`) with `"Portal to {to_map}"`.
- **Three Live Pilot Maps Verification (§9.4, §9.9):**
  - **`prt_fild08` (Outdoor open field):** Map-wide broad spawn regions for Poring, Lunatic, Pupa. Verified walk warps to `prontera`, `prt_fild07`, `izlude`, `moc_fild01`. When routing to Prontera, the north portal to `prontera` receives tracked route accent `→ Route portal: prontera`.
  - **`prt_maze01` (Dungeon maze with localized spreads):** Localized bounding boxes (Poring center 179,20 spread 21x21 -> box `158..=201, 0..=41`; Fabre center 99,20 spread 21x21 -> box `78..=120, 0..=41`). Verified bounding boxes without exact spawn cells or timers.
  - **`izlude` (Town hub):** Non-combat map (0 combat spawns, only single Wild Rose cleaner). Verified walk-warp to `prt_fild08` labeled as portal exit. In-world NPC services (Byalan ferry at 197, 205; Malangdo cat fleet at 182, 218) accurately excluded from walk portal blips.

For each slice's PR, attach a screenshot at the smallest supported viewport, default size, and 1440p/4K scale; check keyboard focus, empty/error, reconnect/map-change, and combat overlap against these contracts.


