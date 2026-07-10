# Implementation Spec — Buff/Debuff Bar (first slice)

| | |
|---|---|
| **Status** | Core implemented (2026-07-09) — text bar + timers; icons still deferred |
| **Parent** | [FEATURE_ROADMAP.md](../FEATURE_ROADMAP.md) §8 Combat feedback · §8.2 MVP cut · §8.3 MVP row (status/buffs) |
| **Why first** | Smallest end-to-end slice that touches every layer — packet → event → state → widget → per-frame tick. It's the **template** for every other §8.3 noop→handler promotion. |
| **Verified against** | `korangar-networking`, `ragnarok-packets`, `korangar/src/{state,interface,lib.rs}` (2026-07-05) |

## 1. Scope

**In:** a HUD bar showing the **local player's** active status effects, each as a
tile with a countdown, added/removed live from the server.

**Out (deferred):** buffs on *other* entities (nameplates own that later), real
status **icon sprites** (none mapped yet — MVP uses placeholder tiles),
right-click-to-cancel, sorting/filtering. These are follow-ups, not blockers.

**Data source:** `StatusChangePacket` (+ `StatusChangeSequencePacket` as a
follow-up). **Not** `StateChangePacket` — that's option-flags (sit/cloak/PK), not
timed buffs; the §8.3 MVP row lumped all three, but only the first two are buffs.

`StatusChangePacket` already carries everything needed:
```
index: u16                      // status/SC id → which tile (+ future icon)
entity_id: EntityId             // whose — filter to the local player for this slice
state: u8                       // 1 = gained, 0 = lost
duration_in_milliseconds: u32   // full duration (0 / u32::MAX ⇒ no timer, infinite)
remaining_in_milliseconds: u32  // seed the countdown
value: [u32; 3]                 // ignore for MVP
```

## 2. Data flow (end to end)

```
map server ──StatusChangePacket──▶ packet handler (promote from noop)
   └▶ NetworkEvent::StatusChange ──▶ lib.rs event loop
        └▶ mutate ClientState.status_effects (add / refresh / remove, player only)
             └▶ StatusBarWindow reads status_effects path (reactive re-render)
        per frame ▶ tick: recompute remaining from expiry; drop expired
```

## 3. Steps

### Step 1 — Promote the packet handler (noop → real)
`korangar-networking/src/packet_versions/version_20220406.rs`
- Replace `register_noop::<StatusChangePacket>()` with a `register(|packet: StatusChangePacket| …)`
  that returns a new event.
- Add the event to the `NetworkEvent` enum (`korangar-networking/src/event.rs`):
  ```rust
  StatusChange {
      entity_id: EntityId,
      index: u16,
      gained: bool,          // packet.state == 1
      duration_ms: u32,
      remaining_ms: u32,
  }
  ```
  (Map `state` → `gained` at registration; keep the enum semantic, not raw.)

### Step 2 — State module
New `korangar/src/state/status.rs`, mirroring `state/hotbar.rs`:
```rust
#[derive(Default, RustState, StateElement)]
pub struct StatusEffects {
    effects: Vec<StatusEffect>,
}
#[derive(Clone, RustState, StateElement)]
pub struct StatusEffect {
    index: u16,
    expires_at: Option<Instant>,   // None = infinite (duration 0 / sentinel)
    duration_ms: u32,              // for the depletion ratio
}
```
Methods: `apply(index, duration_ms, remaining_ms)` (insert or refresh by `index`),
`remove(index)`, and `tick(now)` (drop where `expires_at <= now`).
- Add `status_effects: StatusEffects` to `ClientState` (`state/mod.rs`), and expose
  a path via the existing `ClientStatePathExt` pattern → `client_state().status_effects()`.

### Step 3 — Handle the event (mutate state)
`korangar/src/lib.rs`, in the `NetworkEvent` match (model on the
`UpdateEntityHealth` arm ~1528 and `UpdateStat` which uses `this_player()`):
```rust
NetworkEvent::StatusChange { entity_id, index, gained, duration_ms, remaining_ms } => {
    // Slice = local player only. Compare to the player entity id (this_player()).
    if is_local_player(entity_id) {
        let effects = self.client_state.follow_mut(client_state().status_effects());
        match gained {
            true  => effects.apply(index, duration_ms, remaining_ms),
            false => effects.remove(index),
        }
    }
}
```
`expires_at = (duration_ms==0 || sentinel).then_none() else Instant::now() + remaining_ms`.

### Step 4 — Per-frame tick
Where the frame update already runs (same place the client advances per-frame
state in `lib.rs`), call `status_effects.tick(Instant::now())` to expire tiles.
The reactive `StateElement` derive means the window re-renders when the Vec changes.

### Step 5 — The widget
New `korangar/src/interface/windows/status_bar.rs`, modeled directly on
`interface/windows/hotbar.rs`:
```rust
impl CustomWindow<ClientState> for StatusBarWindow<P>
where P: Path<ClientState, Vec<StatusEffect>> {
    fn window_class() -> Option<WindowClass> { Some(WindowClass::StatusBar) } // add variant
    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;
        window! {
            title: /* localized */, class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            elements: ( split! {
                gaps: theme().window().gaps(),
                children: /* map each StatusEffect → a tile element:
                            placeholder colored box + index label + mm:ss remaining */,
            }, )
        }
    }
}
```
**MVP tile** = colored box + `index` as text + `mm:ss` countdown (exactly the
placeholder in the design mockup). Real icon sprites are Step 5-follow-up once a
`SC index → sprite` map exists (none today).

### Step 6 — Open it in-game
Open `StatusBarWindow::new(client_state().status_effects())` where the other in-game
HUD windows are opened (same site as the hotbar), gated on being in the map/in-game
state. Position defaults near the hotbar; HUD edit mode (§8) will move it later.

## 4. Decisions & gotchas
- **Self-only** this slice — keeps it to one entity-id compare, no per-nameplate work.
- **Infinite buffs:** `duration_ms == 0` (or the RO "permanent" sentinel) ⇒ no
  countdown, tile persists until a `state==0` removal.
- **Refresh vs stack:** RO re-applies a buff by re-sending `StatusChange`; `apply()`
  updates the existing tile by `index` (no duplicates).
- **Relog / map change:** the server re-sends active statuses on map load; clearing
  `effects` on `ChangeMap`/disconnect avoids stale tiles.
- **Icons deferred:** placeholder tiles are acceptable for the friends-group MVP and
  match the mockup; don't block the slice on sprite mapping.

## 5. Verify (per the `verify` skill / §10)
1. In-game, cast a self-buff (e.g. Blessing / Increase AGI, or `@useskill`) →
   a tile appears with a counting-down timer.
2. Let it expire → tile disappears at 0 (Step 4 tick).
3. Re-cast before expiry → timer refreshes, no duplicate tile.
4. Change map → tiles clear and repopulate from the server's resend.

## 6. Effort & payoff
~1 packet promotion + 1 small state module + 1 event arm + 1 tick call + 1 window.
**Low.** Its value beyond the feature: it exercises the whole
packet→event→state→widget→tick loop once, so every other §8.3 promotion
(skill-damage feedback, stats, quests) becomes copy-the-shape work.
