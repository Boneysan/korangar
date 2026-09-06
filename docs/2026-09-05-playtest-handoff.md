# Playtest continuation handoff — 2026-09-05 (T4 measured, T13 shipped)

Read this first, then `docs/plans/playtest-2026-09-05.md`.

Nothing in this tree is committed. Both repos have a large uncommitted working
tree. Packs have **not** been rebuilt (T1).

## What changed this pass

T4 was instrumented to the end and **named nothing in the compositor**, so no
compose or draw code was touched. T13 (headgear) is implemented and mechanically
verified against the archive. Client suite 323 pass / 0 fail / 17 ignored;
`cargo clippy -- -Dwarnings` and `--all-features` clean; `cargo fmt --all
--check` clean.

## T4 — the answer the dump gave

**The compositor does not detach heads.** Three measurements, all reproducible:

| Instrument | Scope | Result |
|---|---|---|
| `playtest-sprite-audit composed` | male + female novice, hair 1, facings 0/2/4/6, every stage from raw clip to interface rectangle | Head overlaps body at all four facings. Composed offsets, action layout, and `animation_part_area` rectangles all agree. |
| `playtest-sprite-audit sweep` | **1,651,104** composed facings: 196 job bodies × 42 hairs × 2 sexes × 13 action groups × 8 facings | **Zero** detachments in the standing groups Idle (0) and ReadyFight (4). All 454 hits are Die / Hurt / Skill poses where the art leans away by 1–11 px. |
| `playtest-sprite-audit attach` | 597 player body + head ACTs, 270,545 drawing motions | Standing motions with no attach point exist in **5 files only** — all GM / non-player sprites (`운영자2`, `활용병`) — and they lack it at *all eight* facings, never a subset. |

Why that is conclusive for the compose path: `apply_child_attach` translates
every head part by `body_attach − head_attach`, so head and body are locked
together by construction. The only escape is its early return when an attach
point is missing, and the third sweep shows no playable job can hit that at
*some* facings and not others — which is the shape the report describes.
Downstream cannot separate them either: the shader's `mirror` flips UVs inside
the part's own quad, and `extra_depth_offset` moves depth, not screen position.

**What this does not prove.** The report was made against the pre-fix tree, and
this same uncommitted tree removed the loader's `attach_point_count == Some(1)`
gate. That gate had real assets to drop — 38 motions declare a count other than
one — but all of them are kagerou costume Skill motions, so it does not explain
a standing head either. T4 needs **one live look** before it is called fixed,
and it must be **in-world**: the select cards are south-only now, which masks
the original report rather than testing it.

Two small things the tables surfaced that were deliberately **not** changed,
because both are inside the "do not retune by eye" line:

- Mirrored even-width parts sit 1 px off. `-((size − 1) / 2)` centres a rect on
  its authored position; mirroring that interval wants `-(size / 2)`. Affects
  both the interface and world paths at facings 4–7. Sub-pixel-adjacent, and
  the ACT data does not settle the convention on its own.
- `finalize_frame_layout` discards `merge_frame`'s computed offset and size and
  recomputes both from the action layout. Intentional (feet stay planted across
  an action), noted so it is not rediscovered as a bug.

## T13 — headgear now renders

- New library table `AccessoryName` (`src/world/library/accessory_name.rs`)
  reads `accessoryid.lub` + `accname.lub`, then `accname_f.lub` over the top for
  the female sprites that differ. 4142 entries.
- `get_entity_part_files` emits the three slots **bottom → middle → top**
  (`accessory`, `accessory3`, `accessory2`), directly after the head so layer
  order paints hats over the head and under the weapon.
- **Hats parent to the body attach, exactly like the head.** Measured with
  `hat-attach`, not assumed: a headgear ACT carries the *same* body-relative
  attach point the head does for the same facing (619/1853 male and 1527/1864
  female hats match hair 1 exactly, the rest by a pixel). A hat is the head's
  **sibling**, not its child. The earlier note in this file saying hats must
  parent to the *head* attach was wrong — doing that would apply the head's own
  shift twice. Pinned by
  `headgear_parents_to_the_body_attach_like_the_head`.
- The `_` trap: `accname.lub` stores names with their own leading underscore
  (`_고글`), so `{sex}_{name}` built `남__고글` and **every hat resolved to
  nothing, silently** — a hat whose sprite is missing looks exactly like no hat.
  Caught only by running the lookup against the archive. Pinned by
  `accname_entries_keep_their_own_separator`.
- `hat-lookup` verification: **1706 of 1984** mapped view ids resolve to a real
  SPR+ACT pair. The 278 that do not are regional/promotional headgear these
  GRFs never shipped (Russia ribbon, Fanta cans); they add no layer, rather than
  a fallback silhouette.
- A `ZC_SPRITE_CHANGE` on any hat slot rebuilds the layers
  (`refresh_entity_headgear_layers`, a full part-list reload — the three slots
  are independent and each one that appears shifts the layers after it).
- Character-select cards show hats too: the character list already carries the
  three view ids.

Not yet seen on screen.

## Instruments (all headless, no GPU, no server)

```bash
cd korangar
cargo run --release --bin playtest-sprite-audit                # asset validity + attach dump (original)
cargo run --release --bin playtest-sprite-audit -- composed    # full per-facing geometry, one asset
cargo run --release --bin playtest-sprite-audit -- sweep       # every job × hair × facing, overlap invariant
cargo run --release --bin playtest-sprite-audit -- attach      # attach-point coverage, whole archive
cargo run --release --bin playtest-sprite-audit -- hat-attach  # headgear vs head attach points
cargo run --release --bin playtest-sprite-audit -- hat-lookup  # view id → sprite, resolution rate
cargo run --release --bin playtest-sprite-audit -- acc-tables  # which lua tables the archive ships
```

`decode_animation_layer_with_sizes` is what makes these run without a GPU: the
texture-size lookup is a parameter, so the audit drives the **real** decoder
rather than a second copy that would be free to disagree with it.

## Branches and state

| Repo | Branch | HEAD | Working tree |
|---|---|---|---|
| `korangar/` | `agent/platform-connectivity-controls` | `18f18283` | Dirty, plus untracked `korangar/src/playtest_audit.rs`, `korangar/tools/playtest_sprite_audit.rs`, `korangar/src/world/library/accessory_name.rs` |
| `Hercules/` | `agent/map-teleport-safety` | `8087690c3` | Dirty: party job packet, `@partyjump`, iz_ac02 cache + warps, `groups.conf` |

Cargo from `korangar/`, never the workspace root (no root `Cargo.toml`).

## Next, in order

1. **Play it.** T4 in-world at 9 / 12 / 3, and equip a hat. Those are the only
   two things left that a headless run cannot answer.
2. **T1 pack rebuild**, once the tree is meant to ship. Until then friends do
   not get quest-window, AMD white-screen, logging, WASD, hotbar, party jump or
   headgear.

## Pitfalls already paid for

- `cargo check` from the workspace root fails. Use `korangar/`.
- CI runs `cargo clippy -- -Dwarnings` **and** `--all-features`; both must pass.
- `Library::new` fails in a bare audit binary (`map_sky_data`: "bad header in
  precompiled chunk"). Load the one table you need via `Table::load` instead.
- `accname.lub` names are EUC-KR and carry their own leading `_`.
- `MinimapBlip` lost `Copy` after `name: String`.
- `apply_keyboard_move(&mut self)` during `input_event_buffer.drain()` — defer
  WASD like `toggle_sit`.
- Mapcache plugin ignores `--grf-path`; it reads `conf/grf-files.txt`.
- `@autoloot` is a **drop-rate threshold**, not "pick up everything". Friends
  are group 0: `#autoloot "Char Name" 100`.
- Quest log is Ctrl+Q; close is Ctrl+W. Alt+U is unbound. Party is Alt+Z/Alt+P.
- Do not "fix" the 1156 vs 1184 map-load count without a missing-map report.
