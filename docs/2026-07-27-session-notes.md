# 2026-07-27 — equipped ammunition end to end, elemental arrow glow, observer-view audit

Branch: `agent/platform-connectivity-controls` (korangar) +
`agent/map-teleport-safety` (Hercules).
Previous: `87907b1d` (first live-pass results + the `@item` fix).

Started as "arrows won't equip", ended as three stacked client bugs, a new
server broadcast, and an audit of a whole bug class. Read
[plans/observer-view-verification.md](plans/observer-view-verification.md)
before touching any visual — it has the live checklist nobody has walked yet.

## 1. Equipped ammunition — three bugs, each hiding the next

Symptoms were "all arrows show as equipped", "unequip does nothing", "equipped
arrows vanish", "every arrow flies as a plain arrow". One root cause each, found
in this order only because each masked the one beneath it.

**a. The stackable item list cannot report worn state.** Hercules fills its one
slot field from `id->equip` — the *item database's* mask — not from the
character (`clif_item_normal`). Every arrow stack therefore claims `AMMO`.
Reading it as worn state marked them all equipped, offered "Unequip" on stacks
the server did not consider equipped, and made `equipped_ammunition()` pick
whichever stack came *first*. The packet field is now named
`equippable_position`, and both list branches start items unequipped.

**b. `clif->arrowequip` arrives mid-list.** It is sent from inside
`clif_inventoryItems`, before `inventoryEnd`, so the event was applied to an
inventory that the following `SetInventory` immediately replaced — equipped ammo
lost on every login, while equipping by hand worked. Now buffered and applied at
the end of the list.

**c. `remove_item` deleted whole stacks.** It decremented only `Regular`; ammo is
`Equippable`, so firing one arrow deleted all 161. This is what made the arrows
"disappear" and forced every later shot onto the generic sprite. Both variants
now decrement.

**A note on (a)/(b): the index was never the problem.** A previous session
concluded there was an off-by-two and added `saturating_sub(2)` to three
handlers. That was wrong and made things worse: `InventoryIndex::from_bytes`
already applies `wrapping_sub(2)`, so the handler subtracting again lands two
slots early. Hercules genuinely does send `n + 2`, which is what makes this
trap work — reading the server source alone "proves" a subtraction is needed.
**Check the field's type first:** `InventoryIndex`/`StorageIndex` are
pre-adjusted, `RawIndex` is not. Documented in
[protocol/inventory-and-ranged-attacks.md](protocol/inventory-and-ranged-attacks.md).

## 2. Elemental arrows get a glow

With selection working, the stock sprites turned out to be faint recolours — a
Fire Arrow still read as a plain arrow in flight. `ammunition_element`
(`world/entity/mod.rs`) now drives a coloured halo plus a point light, from each
item's `bonus bAtkEle,Ele_*` script rather than its name.

The halo re-draws the arrow's *own* texture ~1.9× larger and additively, so it
needs no new art and matches whichever sprite was chosen. Deliberately **wider
than the sprite table**: Frozen (1759), Counter Evil (1766) and Holy (1772) are
elemental but ship no distinct sprite, so a glow is the only way they can read
as elemental at all. A test pins both tables together.

Intensity (26 vs Fire Ball's 48), halo scale and pulse are one-line constants in
`SkillProjectile::with_elemental_glow`.

## 3. Remote players' ammunition — `LOOK_AMMO`

Official Ragnarok never reports anyone else's ammunition, so every remote
archer's arrows drew as the generic one. The fork now broadcasts it.

**Why it rides `LOOK_FLOOR`.** `0x0F00` is exactly `MAX_PACKET_DB` (see the
`CZ_CANCEL_CAST` commit), so there is no id left for a new packet without
resizing `packets->db`; and a new `enum look` member would raise `LOOK_MAX`,
which is `MAX_STYLIST_TYPE`. `LOOK_FLOOR` is one of two slots Hercules marks
"unknown purpose" and never sends. Korangar had the matching slot free too — it
was `ResetCostumes`, unused.

Three traps, all silent, all solved (details in the plan doc):
persist in `view_data` · seed at login · re-send on enter-view.

Because it carries the **item id**, gunslinger bullets and huuma shuriken get
this for free.

Live-verified with two clients: `local_player=false ammo_item=Some(ItemId(1752))
ammo_sprite=Some("아이템\불화살.spr") used_fallback=false`.

## 4. Observer-view audit — the bug class

The Fire Arrow bug was invisible to single-client testing: correct for the
shooter, generic for everyone else. The audit for others is in
[plans/observer-view-verification.md](plans/observer-view-verification.md).

Two gaps found, both **unfixed**:

- **Sprite-change broadcasts are dropped** (`version_20220406.rs:400`, `_ => None`)
  for headgear, robe, hair/clothes colour, body style. No live symptom — those
  parts are not rendered for anyone — but it is exactly where the next Fire
  Arrow lands.
- **Half the handlers ignore the local player.** `ChangeHair` and `ChangeWeapon`
  search only `entities()`; the local player lives at `this_entity()`. Hair is
  observably broken (your own haircut needs a relog); weapon is masked by the
  inventory path.

Fixes are written up as S1–S5 in the plan doc. **S1 + S2 are the recommended
pair** — roughly an hour, and they make both gaps structurally unable to recur.

## 5. Gotchas worth keeping

- **The map server target is `make map_sql`.** `make map` fails with *"No rule to
  make target"* — which contains no "error" and slips through a grepped build
  log. This produced two false "it still doesn't work" rounds against a
  two-hour-old binary. Check `ls -la Hercules/map-server` after every change.
- **Two clients:** second instance needs its own settings dir (absolute paths in
  `game_archives.ron`, a second account in `login_settings.ron`) or they fight
  over `window_cache.ron`. Recipe in the plan doc.
- The running map server may still be a debug build with two extra `ShowInfo`
  lines; source is clean, so any rebuild clears it.

## Next

1. Walk the 11-row observer checklist (rows 1–7 cover shipped work; row 8 is
   expected to FAIL).
2. S1 + S2 from the plan doc.
3. Then either S4 (headgear/robe/dye) or back to the paused E2 batch.
