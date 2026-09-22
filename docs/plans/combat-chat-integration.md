# Combat chat integration — implementation plan

**Owners:** QW-080–081

## Goal

Create a real Combat channel fed from typed authoritative events, with bounded
history and persistent filters. Do not parse rendered public-chat strings.

## Event sources

Audit `korangar-networking/src/event.rs` and the main event match in
`korangar/src/lib.rs`; add typed data only where the existing event loses a
required field. Map exactly once:

- normal/skill damage dealt and received;
- healing;
- status gain/loss;
- skill refusal with resolved reason;
- base/job EXP gain;
- item/zeny loot.

Each packet occurrence creates at most one `CombatEntry`. Preserve source,
target, skill/item id, amount, local direction, and timestamp as structured
fields; format names only at the presentation boundary using `Library`.

## State

Replace the free functions in `state/combat_chat.rs` with:

- `CombatLogState` containing a bounded `VecDeque` (default 500);
- category filter flags;
- unread count and selected-channel state;
- coalescing metadata for explicitly approved spam only.

Store filter preferences in `GameSettings`. Clear character-specific history on
character switch/logout; filters persist. Disabled categories may still update
game state but must not create visible combat rows.

## UI integration

Extend the existing `interface/windows/chat.rs`, `ChatWindowState`, and
`ChatHistory` instead of opening a parallel chat implementation:

- Add a Combat viewing channel distinct from outgoing Say/Party/Whisper modes.
- Hide the outgoing text box or keep the last send channel when Combat is
  selected; never accidentally send combat text publicly.
- Add category toggles, unread indicator, clear action, follow-scroll behavior,
  and accessible non-color labels.
- Bound rendering to visible/history limits.

## Tests

- One typed fixture for every category and inbound/outbound direction.
- Exactly-once mapping where multiple packets describe one skill result.
- Filters, persistence, unread selection behavior, bounded eviction, clear.
- Unknown ids degrade to stable labels rather than panic.
- Live crowded fight: readable ordering, no duplicate damage/heal, acceptable
  allocation/frame behavior.

## Done

- `CombatEntry` is constructed only from real networking events in production.
- The Combat channel and filters are usable in the existing chat window.
- No display-string parser exists.
- Unit/live tests and strict Clippy pass without dead-code allowances.

