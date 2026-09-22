# Equipment eligibility and presentation — implementation plan

**Owners:** QW-047–049  
**Repositories:** Hercules generator and Korangar

## Goal

Generate complete equip restrictions from the authoritative Hercules item DB,
load them once in Korangar, and use one result everywhere an item is shown.

## Current state

- `world/library/equipment_eligibility.rs` contains a four-item fixture.
- `equip_presentation.rs` is only consumed by its tests.
- `item_stats.rs::item_tooltip_text` is used by inventory/vendor comparison.
- Inventory, buy, trade, storage, and floor surfaces have separate rendering
  paths and do not share the fixture eligibility result.

## Data contract

Create a versioned generated TSV or JSON with one row per equippable item:

`item_id, minimum_level, maximum_level, jobs, upper_mask, sex, locations,
weapon_level, slots`

Rules:

- Preserve Hercules meanings; do not translate job masks into guessed client
  classes in UI code.
- Represent unrestricted fields explicitly.
- Sort by numeric item id for reproducible output.
- Include schema version and source hash.
- Fail generation on unknown masks, duplicate ids, invalid locations, or items
  that cannot be round-tripped.

Implement the exporter under `Hercules/tools/`, drawing from the effective
renewal/import item database rather than copying a sample table. Add `--check`
and write the artifact under `korangar/src/world/library/`.

## Korangar integration

1. Replace `BUNDLED_ELIGIBILITY` with `include_str!` of the generated artifact.
2. Load `EligibilityTable` into `Library`; expose lookup through a method rather
   than re-parsing per tooltip.
3. Derive a `Wearer` from authoritative local character job, base level, sex,
   and the requested equipment location.
4. Return a typed result (`Allowed` or one ordered denial). Define precedence:
   location, sex, level, upper/job/class. The same item/wearer must return the
   same reason on every surface.
5. Extend the shared tooltip/presentation function to include stats, equipped
   comparison, blocked marker, tint, denial text, and whether Equip is enabled.
6. Call it from inventory, vendor buy, trade, storage/cart, and floor preview.
   Only inventory disables an Equip action; other surfaces remain inspectable.
7. Delete fixture-only exports and `#[allow(unused_imports)]` once production
   callers exist.

## Tests

- Generator fixtures: unrestricted, first/second/transcendent class, sex,
  upper, min/max level, multi-location, malformed mask, duplicate id.
- Rust parser: schema/hash mismatch and representative real items.
- Cross-surface golden table: identical denial and styling for all five
  surfaces; correct matching equipped slot and signed deltas.
- GUI: weapon, armor, accessory, costume, unusable item, long bonuses at laptop
  resolution and supported UI scales.

## Done

- The generated pack covers every effective equippable Hercules item.
- No eligibility rule or item id is hard-coded in Korangar presentation code.
- All item surfaces call the shared production path.
- Generator `--check`, unit tests, GUI matrix, and strict Clippy pass.

