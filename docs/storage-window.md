# Kafra Storage Window (Korangar)

**Live path:** 2026-07-11 · `PACKETVER` 20220406

## Behavior

When the map server opens personal storage (`openstorage` / `@storage`):

1. Client receives inventory framing for `invType = STORAGE` (`0x0B08` start →
   item lists → `0x0B0B` end) and/or capacity `0x00F2`.
2. Client opens the **Storage** window (item grid) and ensures **Inventory** is
   open so items can be dragged.
3. Drag Inventory → Storage sends `CZ_MOVE_ITEM_FROM_BODY_TO_STORE2` **0x0364**.
4. Drag Storage → Inventory sends `CZ_MOVE_ITEM_FROM_STORE_TO_BODY2` **0x0365**.
5. Close button / window close sends `CZ_CLOSE_STORE` **0x00F7**.

Deposit notifications use **`0x0B44`** (`ZC_ADD_ITEM_TO_STORE` for main ≥ 20200916),
not the older `0x0A0A` layout.

## Stock Kafra: click Close first

Hercules Kafra scripts typically do:

```text
close2;        // show Close, pause script
openstorage;   // runs only after the player presses Close
```

So it is **normal** that storage appears only after you dismiss the dialog.
That is server script ordering, not a missing client open.

## Key files

| Area | Path |
|------|------|
| UI grid | `korangar/src/interface/windows/storage.rs` |
| Open on `SetStorage` / `StorageAmount` | `korangar/src/lib.rs` (`open_storage_window_if_needed`) |
| Drag move events | `InputEvent::MoveItem` Inventory↔Storage |
| Packets | `ragnarok-packets` — `StorageAmountPacket`, `StorageItemAddedPacket` (0x0B44), move/close CZ |
| Handlers | `korangar-networking/.../version_20220406.rs` |

## Regression checks

1. Kafra → Storage → **Close** dialog → Storage + Inventory open.
2. Drag a stackable item into storage; capacity updates; item leaves inventory.
3. Drag it back out.
4. **Close storage** — window closes; server flag cleared.
5. Empty storage still opens a grid (not only `/store` chat hints).
