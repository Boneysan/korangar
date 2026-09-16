# Party colors and UI sounds — implementation plan

**Owners:** QW-082–084

## Goal

Apply stable, accessible party colors consistently and play restrained UI sounds
once per action transition, with local persistent preferences.

## Party colors

1. Key colors by stable `CharacterId` (fall back to `AccountId` only when the
   character id is unavailable), not vector index alone.
2. Assign defaults deterministically from authoritative roster order plus id;
   existing members retain assignments when a member leaves/rejoins or packet
   order changes.
3. Store local overrides in `GameSettings`, scoped by server and character id.
   Overrides never produce network traffic.
4. Expand `party_colors.rs` into a production `PartyColorState` owned by
   `ClientState`; validate alpha and contrast against both party-window and
   minimap backgrounds.
5. Use the same resolved color in `PartyWindow` member rows and current-map
   party markers in `MinimapState`. Offline/different-map state remains visible
   through text/icon, not a misleading map marker.
6. Add an accessible picker, Reset, and deterministic palette fallback.

## UI sounds

1. Inventory shipped assets and choose one activation and one rejection sound;
   record paths and licenses/source in the plan evidence.
2. Keep edge detection in `UiSoundGate`, but own gates by logical control/action
   rather than one global boolean.
3. Route playback through the existing `AudioEngine<GameFileLoader>` in
   `lib.rs`; load/cache keys once like `main_menu_click_sound_effect`.
4. Trigger on accepted input transitions and explicit rejection results, never
   per held frame or render frame.
5. Add enable and volume controls to `AudioSettings`; persist them through the
   existing settings mechanism and respect master/effects volume.

## Tests

- Party: reorder, reconnect, leave/rejoin, removal, collision beyond six members,
  override round-trip/reset, bad alpha/contrast correction, two local clients.
- Minimap/party window use exactly the same resolved RGB value.
- Sound: click/hold/release/reclick, rejected action, disabled and zero volume,
  settings round-trip, playback count through a mock/sink.
- GUI/audio live pass at two UI scales and with rapid keyboard repeat.

## Done

- `party_colors.rs` and `ui_sounds.rs` have production owners and call sites.
- Color overrides remain local and accessible.
- One action produces one sound; holding does not repeat.
- Tests and strict Clippy pass without dead-code allowances.

