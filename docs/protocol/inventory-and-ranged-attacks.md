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

## Equipping ammo uses its own packet — and the index has **no offset**

Arrows do **not** equip via the normal `RequestEquipItemStatusPacket` ack. The server
sends **`EquipAmmunitionPacket` (header `0x013C`)** carrying only the inventory index.
It was previously `register_noop`'d, so the equip never reached the client (empty Ammo
slot). It now emits `NetworkEvent::UpdateEquippedPosition { index, AMMO }`.

**Index gotcha:** the initial inventory *list* packets store items at `raw_index - 2`
(see the `saturating_sub(2)` in the list handlers), but the `EquipAmmunitionPacket`
index is **already the stored index — do NOT subtract 2.** A `-2` here writes the
`AMMO` slot onto the *wrong* item (verified live: it put a sword in the Ammo slot).
When in doubt, log `index` vs the stored `(index, item_id)` list before adjusting.

`Inventory::update_equipped_position` tolerates an unknown index (returns instead of
`unwrap()`-panicking) because an equip broadcast can arrive before an inventory reload.

## Normal ranged attack draws a flying arrow

A normal (non-skill) ranged attack has no projectile from the server — the client
draws it. On a `DamageEffect` with `skill_id == None`, `spawn_ranged_attack_projectile`
(lib.rs) fires `SkillProjectile::arrow` from shooter → target when the attacker's
weapon view is ranged. `ranged_attack_projectile_sprite` (world/entity) maps the view
to a sprite; only **bow (view 11 → `아이템\화살.spr`)** is wired.

- The arrow sprite is the **item icon** — a single 24×24 frame (RO draws flying arrows
  from the item sprite). It is **not** directional, so it must be **rotated**, not
  frame-picked: `SkillProjectile.angle_offset` = **-135°** onto the screen-space travel
  direction (dialed in live; the isometric camera tilts a horizontal shot), longest
  side scaled to ~40 world units.
- **Open sub-follow-ups:** gun-bullet (views 17-21) and shuriken (22) projectiles;
  per-arrow-type sprites (fire/poison arrows).
