# Inventory items & ranged attacks — gotchas

**Parent hub**: [docs/README.md](../README.md) · sibling notes:
[hercules-20220406.md](hercules-20220406.md),
[packet-length-fallbacks.md](packet-length-fallbacks.md).

Hard-won details from wiring bows/arrows (2026-07-22). The Rust packet defs in
`ragnarok-packets` are authoritative for layout; this note captures *modeling*
decisions a future agent would otherwise re-learn the hard way.

## Ammo (arrows) is stackable **and** equippable

Ammo (`item_type == IT_AMMO`, Hercules enum **10**) is the one item that is both
stackable (has an amount) **and** occupies an equip slot (`EquipPosition::AMMO`).
`InventoryItemDetails` has two variants — `Regular` (amount, no equip slot) and
`Equippable` (equip slot, and now an `amount`). Ammo **must be `Equippable`** or it
loses its equip option and its Ammo slot; but it must also carry a real `amount` or
the count never shows and stacks can't merge.

The server reports ammo differently across the three inventory sources, so classify
it centrally: build ammo via **`InventoryItemDetails::ammo()`**
(`korangar-networking/src/items.rs`) and route `item_type == IT_AMMO` through it in
**all three** handlers in `packet_versions/version_20220406.rs`:

- normal/stackable inventory list (login/relog) — otherwise built `Regular`;
- item pickup (`ItemPickupPacket`) — do **not** branch on `equip_position.is_empty()`
  for ammo (ammo has an equip position, so that path already made it `Equippable`,
  but inconsistently with the list);
- storage-add (`StorageItemAddedPacket`) — otherwise fell to `Regular`.

Inconsistent classification is exactly what caused "arrows lose their equip option
after a relog" and "no stack count".

`ItemOptions` is `Copy + Default` so `ammo()` can fill option slots without
packet-supplied data.

### Being `Equippable` means stack maths must handle *both* variants

`Inventory::remove_item` originally decremented the count only for `Regular` and
otherwise deleted the whole entry. Ammo is `Equippable`, so **firing a single
arrow deleted the entire stack** — Hercules sends a `delitem` for one arrow, the
match fell through, and 161 arrows vanished from the client at once.

It presents as a rendering or equip bug, never as an inventory bug: the Ammo slot
empties (the item holding the flag is gone), the arrows disappear from the
inventory grid mid-fight, and every later shot draws the *generic* arrow because
`equipped_ammunition()` no longer finds anything. It also self-heals on relog,
which makes it look intermittent.

Any code that matches on `InventoryItemDetails` to reach an `amount` must handle
both variants. Real gear is unaffected either way — it has `amount: 1`, so the
`amount > remove_amount` test fails and the item is removed outright.

## The stackable item list cannot tell you what is *worn*

`ZC_INVENTORY_ITEMLIST_NORMAL` has one slot field, and Hercules fills it from
**`id->equip`** — the *item database's* slot mask (`clif_item_normal`):

```c
p->WearState = id->equip;      // clif_item_normal  — where it CAN go
```

Compare the equippable list, which has **two** fields and a real worn state:

```c
p->location  = eqp_pos;        // clif_item_equip — where it CAN go
p->WearState = it->equip;      // clif_item_equip — where it IS worn
```

So for stackables the field is a *capability*, identical for every stack of a
given item and independent of the character. Every arrow stack reports `AMMO`
whether equipped or not. Reading it as the worn state made **all** arrow stacks
look equipped: each offered "Unequip" (which the server ignored, since they were
not equipped), and `equipped_ammunition()` — a `find_map` — picked whichever
stack came *first* rather than the real one.

The packet field is therefore named **`equippable_position`**, not
`equipped_position`, and both list branches start items unequipped. Only
`EquipAmmunitionPacket` can say what is actually worn.

### …and that packet arrives *before* the inventory it refers to

`clif->arrowequip` is sent from inside `clif_inventoryItems`, between the
stackable and equippable lists — i.e. **before** `clif->inventoryEnd`:

```
InventoryStart → normal list → arrowequip (0x013C) → equip list → InventoryEnd
```

The client only publishes the inventory at `InventoryEnd`, so emitting
`UpdateEquippedPosition` on arrival applies it to the *outgoing* inventory, which
the following `SetInventory` immediately replaces — equipped ammo would be lost
on every login while working fine when equipped by hand. The handler buffers the
index while a list is being accumulated and applies it at `InventoryEnd`
(`pending_equipped_ammunition`).

`Inventory::update_equipped_position` additionally clears `AMMO` from every other
stack, since there is one ammo slot. The server does unequip the previous stack
first, so this is belt-and-braces — but a lost ack would otherwise leave two
stacks flagged, and then the *first* silently wins.

## Equipping ammo uses its own packet — and the handler must **not** re-offset the index

Arrows do **not** equip via the normal `RequestEquipItemStatusPacket` ack. The server
sends **`EquipAmmunitionPacket` (header `0x013C`)** carrying only the inventory index.
It was previously `register_noop`'d, so the equip never reached the client (empty Ammo
slot). It now emits `NetworkEvent::UpdateEquippedPosition { index, AMMO }`.

**Index gotcha — the offset lives in the type, not the handler.** Use
`packet.inventory_index` as-is; do **not** subtract 2.

Both facts are true at once and that is what makes this trap work:

- Hercules really does send the `+2` — `clif_arrowequip` (`src/map/clif.c`) writes
  `WFIFOW(fd,2) = val + 2`, exactly like `clif_equipitemack`, `clif_delitem`,
  `clif_useitemack` and the rest.
- The field is typed `InventoryIndex`, and **that type's `FromBytes` already applies
  `wrapping_sub(2)`** (`ragnarok-packets/src/lib.rs`; `StorageIndex` does the same with
  `1`). By the time a handler sees the value, the offset is gone.

So reading Hercules alone "proves" a `- 2` is needed, and it is wrong — the subtraction
is a *double* subtraction that lands the `AMMO` flag two slots below the arrows. This
mistake has now been made twice. **Before adjusting any index, check the field's type
first:** `InventoryIndex`/`StorageIndex` are pre-adjusted, `RawIndex` is not. The
inventory *list* handlers subtract by hand only because their items carry `RawIndex`
— that lone `saturating_sub(2)` is not a convention to copy.

The failure is nasty because it is *silent in the UI you would check*. The equipment
window's Ammo box asks "which item carries `AMMO`", so a flag written to the wrong slot
just leaves the box empty — indistinguishable from "the equip never happened". It also
mis-selects the projectile sprite, because `equipped_ammunition()` reads the same flag.
Symptoms seen live with the double subtraction: the Ammo slot stayed empty, equipped
arrows appeared to vanish, unequip did nothing, and an earlier test put a *sword* in
the Ammo slot.

**Diagnose it against the database, not the client** — the client is the thing that is
wrong. `SELECT id, nameid, amount, equip FROM inventory WHERE char_id = …` shows the
true equipped ammo (`equip = 32768`, i.e. `EQP_AMMO`); compare that against the
client's `[ranged-attack] ammo_item=…` log line under `KORANGAR_PACKET_LOG`.

`Inventory::update_equipped_position` tolerates an unknown index (returns instead of
`unwrap()`-panicking) because an equip broadcast can arrive before an inventory reload.

## Normal ranged attack draws a flying arrow

A normal (non-skill) ranged attack has no projectile from the server — the client
draws it. On a `DamageEffect` with `skill_id == None`, `spawn_ranged_attack_projectile`
(lib.rs) fires `SkillProjectile::arrow` from shooter → target when the attacker's
weapon view is ranged.

**The projectile is the *ammunition* item's sprite, not a per-weapon constant**
(2026-07-26). Resolution order in `spawn_ranged_attack_projectile`:

1. **The local player's equipped ammo** — `Inventory::equipped_ammunition()` reads the
   `EquipPosition::AMMO` slot, and `ItemResource` turns the item id into its sprite
   base. Only the local player is covered: the server never reports another
   character's ammo.

   **`iteminfo` collapses most ammo onto the generic arrow.** Item 1770 (Iron Arrow)
   resolves to `화살`, not `철화살`, even though `아이템\철화살.spr` ships in
   `data.grf` — so per-arrow-type variety does *not* fall out of the client data by
   itself. Confirmed by the client's own loaded table; `item_info.rs` reads
   `identifiedResourceName` straight through, and the bundled English overlay only
   rewrites display names (a missing row would yield `사과`/Apple instead).

   `elemental_ammunition_resource` (`world/entity/mod.rs`) therefore fills that gap
   for the **nine elemental arrows that ship a distinct sprite**, and only where
   `iteminfo` returned `GENERIC_ARROW_RESOURCE` — so the client's own mapping still
   wins wherever it names something specific. Ids and elements come from each item's
   `bonus bAtkEle,Ele_*` script in `db/re/item_db.conf` (mechanical, not name
   matching); sprite presence was verified against `data.grf`'s file table with
   `tools/grf_list.py`. The item↔sprite *pairing* is a translation of the Korean
   resource names and is the part to doubt first if one looks wrong in flight.

   Frozen Arrow (1759), Arrow of Counter Evil (1766) and Holy Arrow (1772) are
   elemental but ship **no** distinct sprite, so they are deliberately absent rather
   than pointed at a guess. This is a small, deliberate divergence from the original
   client, which drives the projectile purely off `iteminfo`.

   Note `iteminfo.lub` is a **32-bit** compiled Lua chunk, so 64-bit Lua rejects its
   header — it cannot be dumped offline with a plain `mlua` harness. The running
   client is the practical way to read what the table actually contains.

   **Sprites alone are not enough to tell arrows apart.** Confirmed live: the stock
   elemental sprites are small recolours of the plain arrow, so at projectile size
   and speed a Fire Arrow still just reads as "an arrow". `ammunition_element`
   (`world/entity/mod.rs`) therefore gives elemental ammo a coloured halo and a
   point light — `SkillProjectile::with_elemental_glow`. The halo re-draws the
   arrow's *own* texture ~1.9× larger and additively in the element colour, so it
   needs no new art and always matches whichever sprite was selected; additive
   blending means it only brightens, keeping the arrow's shape.

   Elements come from each item's `bonus bAtkEle,Ele_*` script, so Rusty Arrow
   (Poison) and Silver Arrow (Holy) are right where name-matching would be wrong.
   **The glow table is deliberately wider than the sprite table**: it covers the
   three arrows above that ship no distinct sprite, which the sprite path cannot
   reach at all. Keep the two in step — a test asserts every sprite-table arrow
   also has an element, and that neutral ammo has none so it never glows.
1b. **Another player's ammunition**, via the fork's `LOOK_AMMO` broadcast
   (2026-07-27). Official Ragnarok reports ammunition for nobody but yourself, so
   remote archers previously always drew the generic arrow. The server keeps the
   item id in `view_data.ammo`, broadcasts it on equip/unequip, seeds it at login
   and re-sends it when the unit enters view; the client stores it on the entity
   (`Common::ammunition`). It carries the **item id**, so bullets and shuriken are
   covered too. `0` means unknown and falls through to the class default below —
   which is also what an older server yields. See `LOOK_AMMO` in the server's
   `map/map.h`, and [../plans/observer-view-verification.md](../plans/observer-view-verification.md)
   for the three traps this had to solve.
2. **The weapon class's canonical ammo item** for everyone else —
   `ranged_attack_default_ammunition` (Hercules `item_db.conf` ids: Arrow **1750**,
   Bullet **13200**, Shuriken **13250**).
3. **A hardcoded per-view sprite** if the item tables cannot name the resource —
   `ranged_attack_projectile_sprite`: bow (11) → `아이템\화살.spr`, firearms (17-21) →
   `아이템\탄약통.spr`, huuma shuriken (22) → `아이템\수리검.spr`. All three verified
   present in the configured GRFs; a test pins the two tables to the same set of views
   so they cannot disagree about which weapons are ranged.

- The arrow sprite is the **item icon** — a single 24×24 frame (RO draws flying arrows
  from the item sprite). It is **not** directional, so it must be **rotated**, not
  frame-picked: `SkillProjectile.angle_offset` = **-135°** onto the screen-space travel
  direction (dialed in live; the isometric camera tilts a horizontal shot), longest
  side scaled to ~40 world units.
- **Open sub-follow-up:** the grenade launcher (view 21) falls back to the Bullet
  sprite — the GRFs ship no grenade item sprite, so there is nothing better to draw
  until one is found. Everything else in this row is closed; **none of it has been
  live-verified yet.**
