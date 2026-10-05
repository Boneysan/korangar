# Player interface once-over: audit and plan

Status: **plan, 2026-10-05.** Requested after the quest log was called "a confusing mess", widened by the owner to every searchable view and then the whole player interface. Nothing here is built yet. The quest log design below was agreed before this plan existed.

## Why the interface feels messy

About 60 windows were added feature by feature over several months, and no shared layout rules were ever written down. The result is the same problems in many places:

- **Everything expanded at once.**
  - The quest log shows every quest's guidance, every required item, and a route button for every map an item drops on, all in one flat scroll.
  - Collapsible sections appear only in Party, Friends and two debug windows.
- **No grouping.**
  - Only the skill tree uses tabs.
  - Game Settings has 45 buttons and Render Options 34, with no sections.
  - The Adventure Guide has 15 category buttons in two rows plus a separate Search button.
- **Action floods.** One button per map, per source, per item, instead of one action per thing.
- **Numbers and developer text shown to players.**
  - Story quests are named "Quest 20004".
  - The Guide formats ids into player text in about 30 places, for example "(ID 501)", "Job ID", or "Monster ID … has no matching bundled bestiary entry".
  - The Guide's header line is provenance jargon: "data revision …, discovery badges sync per account; mechanics remain open".
- **List and details stacked.** The Guide puts results and details in two scroll views one above the other, so each competes for height.

Evidence: a structural scan of `korangar/src/interface/windows/` (2026-10-05) for tabs, collapsible sections, scroll views, search boxes, buttons, and id formatting. The scan is a starting point, not a review; rows marked *needs a look* below have not been read closely.

## Constraints

- **I cannot see the screen.** Code that compiles and tests that pass do not prove a layout reads well. Every slice ends with the owner looking at it, ideally with a screenshot, before the next slice starts.
- **Toolkit.** Use only what `korangar-interface` already provides: `tabs!`, `collapsible!` (with `initially_expanded`, state kept across frames), `drop_down!`, `scroll_view!`, `text_box!`, `split!`, `button!`, `state_button!`. No new widget work unless a slice proves it is needed.
- **Rebaseability.** Fork-only windows stay in `interface/windows/dm/` and their own files. Upstream windows (inventory, equipment, chat, skill tree) get the lightest touch.
- **Testability.** Each redesigned window builds its rows from a pure "row plan" function. One source then drives both the row count and the rows, so they cannot drift (the quest log currently counts and builds separately), and tests can assert the structure: what is collapsed, what is grouped, and that no raw id reaches the text.

## Design rules (apply to every slice)

1. **Summary first, details on demand.**
   - Lists show one line per thing: name, kind, and a short state such as `2/3`, `✓` or `tracked ★`.
   - Details sit in a collapsible section, or in a detail pane for searchable views.
   - Collapsed by default, except the item the player is working on (for example, tracked quests).
2. **Group distinct kinds with tabs.** Use tabs when a window holds two or more kinds of content, such as quests, hunts, goals and clues, or settings areas.
3. **One primary action per thing.** Secondary actions live in the details. Never a button per map or per source; use one "Route" button plus a collapsed "Where to find (N)" section.
4. **One layout for every searchable view.**
   - A search box on top, searching as you type or on Enter, with no separate Search button.
   - Category as a dropdown or a single row of tabs.
   - Results as one-line rows (kind icon or tag, name, one-line summary), with details beside or below.
   - Clicking a cross-link opens it in the same view, with a Back button.
5. **No raw numbers or developer text for players.**
   - Ids, schema revisions and provenance notes move to tooltips or a developer toggle.
   - Unknown names say so in words ("Unnamed quest"), never as a number.
6. **Consistent status language.** Green `✓` means done, grey means open, and amber means needs attention. Progress always reads `have / need`.
7. **Settings in sections.** Use tabs or collapsible groups, each with a short heading, instead of long button walls.

## Audit: player-facing windows

Priority: **P1** means named by the owner or a clear mess; **P2** means visible clutter by the rules above; **P3** means checking only. Debug and inspector windows (frame, packet, theme, profiler, render options) are out of scope.

| Window | Size | What is wrong (by the rules) | Priority |
|---|---|---|---|
| Quest Log | 501 lines | All quests expanded; route-button floods; story quests named by number; clues appended at the end; counting and building are separate code | **P1, slice 1** |
| Adventure Guide | ~5,000 lines | 15 category buttons plus a Search button; developer header line; ids in player text (about 30 sites); results and details stacked | **P1, slice 2** |
| World Map & Route Finder | 1,426 lines | One custom atlas view plus Clear Route; whatever is wrong is visual (*needs a look and a screenshot*) | **P1, slice 3** |
| Minimap | 794 lines | 9 buttons on a map overlay (*needs a look*) | P2 |
| Game Settings | 443 lines | 45 buttons, no sections | P2 |
| Menu | small | 21 buttons in one list; group them (character / social / world / settings) | P2 |
| Party | 495 lines | 22 buttons; has collapsible members (*needs a look*) | P2 |
| Commission Board | small | Form, list and actions in one column; four text boxes | P2 |
| Character Overview, Stats, Equipment, Inventory | small | Stats grew a "View" switch, Crafting odds and Build planner buttons (*needs a look*) | P3 |
| Shop (Buy, Sell, Cart) | small | *needs a look* | P3 |
| Trade, Storage, Chat, Hotbar, Skill Tree | — | Recently worked on or upstream; check only | P3 |
| Character select and create, Login | — | *needs a look* | P3 |
| DM windows (Bestiary, Loot, GM Commands with 66 buttons) | — | DM-only; GM Commands would benefit from sections | P3 |

## Slices

Each slice follows the same steps:
1. Redesign behind a row-plan function, with structure tests.
2. Add a GUI-pass row.
3. The owner looks, plus a screenshot if possible.
4. Adjust, then merge.

Each slice is one PR.

1. **Quest Log (design agreed 2026-10-05).**
   - **Tabs:** Quests, Hunts, My goals, Clues.
   - **Rows:** each quest is one collapsible line, `★ name   have/need ✓`, collapsed unless tracked.
   - **Inside a quest:** guidance, objectives as `✓/✗ item  have / need  [Guide]`, one `Route` button, and drop sources collapsed under "Where to find (N maps)".
   - **Quests vs Hunts:** a quest is a *Hunt* when it has item turn-ins (only the campaign hunting-contract table supplies them, `resolve_quest_entry`) or kill counts.
   - **Story quests** get their names from the Guide's reference quest data instead of "Quest 20004".
2. **Adventure Guide.**
   - A category dropdown instead of 15 buttons; search on Enter.
   - Results as one-line rows with a kind tag.
   - Details in their own pane, with Back for cross-links.
   - The developer header and ids move to tooltips.
   - Split the 5,000-line file along category lines as part of the work.
3. **World Map.** Start from owner screenshots and notes on what is confusing, then apply rules 1, 3 and 5 to its panels and labels.
4. **Settings and Menu.** Game Settings in tabbed sections; Menu grouped.
5. **Minimap, Party, Commission Board.**
6. **P3 pass.** Short look at each remaining window with the owner, fixing only what fails a rule.

## Open questions for the owner

- **Screenshots:** can you capture the windows as they look today, especially the World Map and the Guide? That gives each slice a before picture and gets the map slice started.
- **Icons:** should the kind tags in lists be text (`[Item]`) or icons? Icons need sprite work.
- **Window size:** is a wider default window acceptable for the Guide's details pane, or should details stay below the results?
