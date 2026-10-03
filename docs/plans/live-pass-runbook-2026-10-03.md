# Live Pass Runbook — Modernized GDD Features (2026-10-03)

This runbook is the definitive human-testing checklist for all features built and automated-tested in the GDD modernization programme.

Tick **[x]** only after observing the behavior live in a running client. Note any discrepancies directly.

---

## 1. Setup & Environment Bring-Up

- **Hercules Server:**
  ```bash
  cd Hercules && ./dev.sh restart && ./dev.sh wait
  ```
- **Korangar Client:**
  ```bash
  cd korangar/korangar && cargo run --release --bin korangar
  ```
- **Accounts Required:**
  - **Account A (GM):** For `@monster`, `@item`, `@job` convenience.
  - **Account B (Player):** Standard group 0 player account for peer trade, commissions, and party checks.

---

## 2. Dedicated Commission Board & Crafting (F30, F31 / Decision D7)

| Action | Expected Live Result | Observed |
|---|---|---|
| Open via menu button or `/commission board` | Dedicated `CommissionBoardWindow` opens with "Crafting Commission Board" title | [ ] |
| Inspect window header | Non-custodial risk disclosure is clearly visible: *"Non-custodial peer requests. Materials & payment must be traded directly."* | [ ] |
| Enter item `"Fire Damascus"`, fee `"50000"`, click **Post Request** | Request is queued; chat displays confirmation: `Commission posted: Fire Damascus (Fee: 50000 zeny)` | [ ] |
| Click **View All Active** or type `/commission list` | Window/chat displays active peer commissions | [ ] |
| Type `/commission cancel <id>` from Account B (non-owner) | Cancellation is refused with authorization error toast | [ ] |
| Type `/commission cancel <id>` from Account A (owner) | Commission is removed from active listings with confirmation | [ ] |
| Craft stat cooking dishes / check Guide | Renewal cooking formulas, kits (Outdoor through Legendary), and stat dish rates are documented in Guide Mechanics | [ ] |

---

## 3. Build Planner & Stat Preview (F01, F03, F04)

| Action | Expected Live Result | Observed |
|---|---|---|
| Open Character Stats (`Alt+A` / `Alt+S`) | Simple / Detailed / Advanced modes toggle with plain-language derived stat descriptions | [ ] |
| Hover any stat row (+ button) | Tooltip displays next-point stat delta (HP, SP, Hit, Flee, Soft DEF/MDEF) and breakpoint distance | [ ] |
| Click **Build Planner** button in Stats window | `BuildPlannerWindow` opens; defaults to current character's live level, stats, and job tree | [ ] |
| Move Base Level and Job Level sliders | Stat point and skill point budgets update according to Hercules Renewal progression curves | [ ] |
| Stage stat points up to 99 cap | Point costs increase at exact threshold bands (1–99 costs 628 points total); refused if budget exceeded | [ ] |
| Stage skills on second job branch | Prerequisite blockers prevent staging high-tier skills until dependencies are met; cannot stage above available budget | [ ] |
| Click **Save 1**, reset plan, click **Load 1** | Staged build restores identically from client disk (`client/build_plans/`); no network packets sent | [ ] |

---

## 4. Telegraphed Boss Encounters & Encounter Recap (F15, F16 / §6.9)

| Action | Expected Live Result | Observed |
|---|---|---|
| Spawn Phreeoni: `@monster 1159 1` | Phreeoni targets player; regular hits occur without random teleport-on-hit | [ ] |
| Observe Phreeoni signature cast | Visible cast bar appears for `NPC_POWERUP` / `NPC_CRITICALSLASH` with a 2.0s tell | [ ] |
| Interrupt Phreeoni during cast | High ASPD or stun/bash cancels cast; audible interrupt cue plays; cast bar disappears | [ ] |
| Observe recovery window | Phreeoni enters $\ge 15$s basic auto-attack recovery window before next telegraphed skill | [ ] |
| Test second MVP: `@monster 1059 1` (Mistress) | Casts `WZ_STORMGUST` with multi-cell tell; summons Giant Hornet escorts; $\ge 25$s recovery | [ ] |
| Target MVP boss | Large dedicated `BossTargetWindow` opens showing boss HP, level, element, and target-of-target | [ ] |
| Defeat boss | Client displays defeat toast: `"Defeated <Boss>! Dealt <Damage> dmg in <Duration>s"` | [ ] |

---

## 5. Class Progression & Mechanics Teaching (F18 / §8.6)

| Action | Expected Live Result | Observed |
|---|---|---|
| Open Adventure Guide (`Alt+G` / Menu) | 11 categories available: Monsters, Items, Cards, Skills, Statuses, Mechanics, Maps, Jobs, Quests, NPCs, Services | [ ] |
| Search `"second job"` or navigate to Server Rules | **Second Job Progression Guide** opens | [ ] |
| Read Second Job Guide details | All 12 branches documented (Knight, Crusader, Wizard, Sage, Hunter, Bard/Dancer, Assassin, Rogue, Priest, Monk, Blacksmith, Alchemist); explains Job 40 minimum vs Job 50 recommendation and permanent skill point allocation | [ ] |
| Search `"socketing"` in Adventure Guide | **Card Socketing & Compounding Rule** opens; explains permanent socket binding and damage scaling rules | [ ] |
| Search `"weapon refine"` in Adventure Guide | Displays exact success rates, safe limits, ore requirements, and destruction risks for Weapon Levels 1–4 | [ ] |

---

## 6. Direct Trade Last-Second-Change Guard (F28 / §12.1)

| Action | Expected Live Result | Observed |
|---|---|---|
| Initiate trade between Client A and Client B | Trade window opens on both clients | [ ] |
| Client B offers 50,000 Zeny | Client A's window displays `"Their zeny: 50000"` (not a bugged item #0 slot) | [ ] |
| Client A locks trade (`Lock` button) | Client A's trade is locked; Client B has not locked | [ ] |
| Client B changes offer (e.g. reduces Zeny to 1) | Client A's window immediately flashes warning banner: `"!! Partner CHANGED their offer after you locked"` | [ ] |
| Client A clicks `Confirm Trade` immediately | Action is refused with toast warning; trade does not complete | [ ] |
| Client A waits >1.5s and clicks `Confirm Trade` | Confirmation proceeds; final state shows `"You give: ... / You receive: ..."` | [ ] |
| Both confirm | Trade completes cleanly; zeny and items transfer accurately with no duplication | [ ] |

---

## 7. UI Scaling, Viewport Safety & Window Snapping (F33 / §10.16)

| Action | Expected Live Result | Observed |
|---|---|---|
| Drag any window towards edge of screen | Window cannot be dragged off-screen; titlebar and close controls remain safely reachable | [ ] |
| Hover buttons/items at extreme bottom/right of screen | Tooltip clamps inside screen boundaries; never overflows off-screen | [ ] |
| Focus a text box (chat, commission, search) and press `Escape` | Text box unfocuses cleanly without opening unintended game menus | [ ] |
| In Game Settings, set HUD grid snapping to 16px | Dragged windows snap their top-left coordinates to 16px boundaries | [ ] |
| Resize game window (or change UI scale in `Ctrl+I`) | All UI elements, fonts, and frames scale smoothly without clipping text | [ ] |

---

## 8. Party Coordination, Pings & Tonight's Goals (F20, F22 / §13)

| Action | Expected Live Result | Observed |
|---|---|---|
| Target a monster and type `/mark focus` | Target receives focused marker icon and outline visible to all party members; expires after 30s | [ ] |
| Type `/goal add Farm 100 Jellopy` | Goal appears pinned under `"Tonight:"` in the party HUD for both clients | [ ] |
| Type `/goal done 1` | Goal is marked completed and archived | [ ] |
| Click party member frame in party window | Targets member; displays Chebyshev distance (`"4 tiles away"` or `"other map: <name>"`) | [ ] |
| Toggle Healer Profile in party settings | Party frames expand into high-readability enlarged HP/SP bars | [ ] |

---

## 9. Audio Cues Verification (F35 / §16.2)

| Trigger | Expected Audio Cue | Heard |
|---|---|---|
| Enemy begins long dangerous cast | Warning chime plays (`effect\warning.wav`) | [ ] |
| Enemy cast is interrupted | Stun strike cue plays (`_stun.wav`) | [ ] |
| Monster drops a Card or Wishlist item | Rare success fanfare plays (`effect\p_success.wav`) | [ ] |
| Party member sends a Ping | Distinct party alarm alert plays (`party_alarm.wav`) | [ ] |
| Quest objective or contract completed | Completion chime plays (`effect\complete.wav`) | [ ] |

---

## 10. Reporting Discrepancies

If any step deviates from the expected result:
1. Record the exact action, character class/level, and map coordinate (`/where`).
2. Capture screenshot (`F12` or screenshot shortcut).
3. Check `Hercules/log/map-server.log` and client console for packet or script warnings.
4. File the finding against the corresponding GDD Feature ID (e.g. `F16`, `F31`).
