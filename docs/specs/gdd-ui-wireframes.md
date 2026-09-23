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

For each slice's PR, attach a screenshot at the smallest supported viewport, default size, and 1440p/4K scale; check keyboard focus, empty/error, reconnect/map-change, and combat overlap against these contracts.
