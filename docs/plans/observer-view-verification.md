# NEXT — observer-view audit: what *other* players see

| | |
|---|---|
| **Status** | **Code audit done 2026-07-27 — live checklist NOT yet walked** |
| **Branch** | `agent/platform-connectivity-controls` |
| **Origin** | The Fire Arrow bug (2026-07-27), see [../protocol/inventory-and-ranged-attacks.md](../protocol/inventory-and-ranged-attacks.md) |
| **Needs** | Two clients — see §Running two clients |

## Why this exists

Every visual in this client was built and verified **from the caster's seat**.
The Fire Arrow work exposed what that misses: arrows drew correctly for the
player firing them and as a plain generic arrow for *everyone else*, because the
projectile was chosen from the local inventory and official Ragnarok never
reports anyone else's ammunition. It looked completely correct in single-client
testing, which is the only testing that had ever been done.

The bug class, stated generally:

> Any visual derived from `client_state().inventory()` or `this_entity()` is
> true for you and silently wrong for every observer. Any visual derived from a
> field the server only sends to the owner cannot be right for observers at all.

That grep **is** the audit:

```bash
grep -rn "client_state().inventory()\|this_entity()" korangar/src/ \
  | grep -v "^korangar/src/interface/"
```

Below is the result of running it (and the server-side equivalent) on
2026-07-27. Two real gaps, one fixed; the rest verified sound.

## Verified correct — no action

Checked and confirmed to resolve per-entity, so an observer sees the truth.
Listed so a future pass does not re-derive them.

| Visual | Why it is fine |
|---|---|
| Weapon, shield / left hand | `LOOK_WEAPON` packs weapon into `val` and shield into `val2`; both handled. Layer refresh is the same code for local and remote. |
| Base sprite / job / disguises | `LOOK_BASE` → `ChangeJob`, applied to any entity. Only the *skill-tree rebuild* is local-gated, which is correct (`lib.rs:4596`). |
| Status effects | `clif_status_change_sub` sends **`AREA`** with `val1`/`val2`/`val3`, and the client applies `update_animation_status` to any visible entity (`lib.rs:4188`). Only the HUD list is local-only, correctly. |
| Cast bar / casting pose | `start_cast` / `is_casting` live on `Entity`, not on the player. `cancel_own_cast` is local-only, which is right — you may only cancel your own. |
| Skill effects, damage effects, ground units | `spawn_successful_caster_skill_effect` / `spawn_damage_caster_skill_effect` resolve the caster from the entity list, falling back to the local player only when the id is `0`. |
| Effect sounds | No local-player gating anywhere in the sound paths. |
| **Ammunition** | **Fixed 2026-07-27** — `LOOK_AMMO` broadcast. Because it carries the *item id*, gunslinger bullets and huuma shuriken get this for free. |

## Gap 1 — sprite-change broadcasts are dropped on the floor

**`korangar-networking/src/packet_versions/version_20220406.rs:400`**

The `SpriteChangePacket` handler maps `Base`, `Hair`, `Weapon`, `Shield` and now
`Ammunition`, then ends in `_ => None`. That silently discards:

`HeadBottom` · `HeadTop` · `HeadMiddle` · `HairCollor` · `ClothesColor` ·
`Shoes` · `Body` · `Robe` · `Body2`

**Impact today is limited, and the reason matters:** `Common` has no fields for
headgear, robe, or palettes, and the composed part list is body + head + weapon
+ shield only — confirmed live, e.g.

```
parts=["인간족\몸통\남\스나이퍼_남", "인간족\머리통\남\2_남", "인간족\헌터\헌터_남_활"]
```

So these parts are not drawn for *anybody*, local or remote. This is a **feature
gap, not a live regression** — do not go hunting for a visible bug here.

It earns its place on this list because it is precisely where the next Fire
Arrow will happen. The day headgear, robes, or dye are rendered, the sprite will
build from the spawn packet and then never update, so a hat swapped in front of
you stays wrong until the observer walks away and back. Wiring the events is the
cheap half; entity fields, part composition, and palette support are the rest.

## Gap 2 — sprite-change handlers disagree about the local player

The local player lives at `this_entity()`, **not** in `entities()`. Every
`account_id`-keyed handler must therefore check both — and only half of them do:

| Handler | `this_entity()` fallback | Consequence |
|---|---|---|
| `ChangeJob` (`lib.rs:4590`) | yes | correct |
| `ChangeShield` (`lib.rs:4646`) | yes | correct |
| `ChangeHair` (`lib.rs:4631`) | **no** | **your own haircut does not apply until relog** |
| `ChangeWeapon` (`lib.rs:4617`) | **no** | latent — masked, see below |
| `ChangeAmmunition` (new) | no — **deliberate** | correct; see below |

`ChangeHair` is the observable one: hair style *is* rendered, and nothing else
sets it, so another player's haircut updates live while your own does not.

`ChangeWeapon` has the identical hole but is **masked**: the local weapon is also
set from the inventory on every equip/unequip (`lib.rs:4346`, `lib.rs:4572`, via
`equipped_weapon_look()`), so the broadcast being dropped never shows. It will
bite the first time a weapon look changes *without* an inventory event —
a disguise, a costume, or a script-driven `changelook`.

`ChangeAmmunition` skipping the local player is correct and should stay: the
local inventory is authoritative and exact, and the broadcast is only a hint for
observers. Keep the comment there saying so, or someone will "fix" it.

This is the mirror image of the Fire Arrow bug and worth stating plainly —
"verify from the other seat" cuts both ways. Single-client testing could never
have caught the ammunition bug; two-client testing would never have caught this
one.

## How to fix

Three tiers. **S1 and S2 are the recommended pair** — together they cost about
an hour and make both gaps structurally unable to recur. S3 onward is optional.

### S1 — one helper for "entity by account id" *(fixes Gap 2)*

The duplicated `find(...)`-else-`this_entity()` dance is why half the handlers
disagree. Replace it with a single helper so the fallback cannot be forgotten:

```rust
/// Resolve an entity by account id, including the local player.
///
/// The local player lives at `this_entity()`, not in `entities()`; a handler
/// that only searches `entities()` silently ignores changes to *yourself*.
fn with_entity_by_account_id<R>(
    &mut self,
    account_id: AccountId,
    apply: impl FnOnce(&mut Entity) -> R,
) -> Option<R> {
    if let Some(entity) = self
        .client_state
        .follow_mut(client_state().entities())
        .iter_mut()
        .find(|entity| entity.get_entity_id().0 == account_id.0)
    {
        return Some(apply(entity));
    }
    self.client_state
        .try_follow_mut(this_entity())
        .filter(|entity| entity.get_entity_id().0 == account_id.0)
        .map(apply)
}
```

Convert `ChangeHair`, `ChangeWeapon`, `ChangeShield` and `ChangeJob` to it.
Leave `ChangeAmmunition` on the entities-only path with its existing comment —
that exception is deliberate.

Watch the borrow: the existing arms call `Self::refresh_entity_*_layer(&self.async_loader, …)`
while holding `&mut entity`. Either pass the loaders into the closure or return
the parts and refresh afterwards; the arms differ enough that a mechanical
conversion will not compile first try.

### S2 — make the sprite-change match exhaustive *(fixes Gap 1's silence)*

Delete `_ => None` and list every variant. Then a new `SpriteChangeType` cannot
be added without the compiler pointing here, and the next reader sees at a
glance what is unhandled and why:

```rust
// Not rendered yet: `Common` has no fields for these and the composed part
// list is body + head + weapon + shield only. Listed explicitly rather than
// caught by `_` so adding a variant is a compile error, not a silent drop.
SpriteChangeType::HeadBottom
| SpriteChangeType::HeadTop
| SpriteChangeType::HeadMiddle
| SpriteChangeType::HairCollor
| SpriteChangeType::ClothesColor
| SpriteChangeType::Shoes      // Hercules: "No packet uses this"
| SpriteChangeType::Body       // Hercules: "unknown purpose"
| SpriteChangeType::Robe
| SpriteChangeType::Body2 => None,
```

This does not add a feature — it converts a silent drop into a visible, dated
decision. Apply the same rule to every enum **we** own at a protocol boundary.

### S3 — an observer-view debug dump

Today's debugging burned two rounds on "is it the client or the server?". A dump
of what the client believes about a *remote* entity answers that instantly.
Behind an env flag, alongside `KORANGAR_PACKET_LOG`:

```
[entity-debug] id=2000001 name=HeadlessTwo weapon=1701 shield=0 ammo=1752
               hair=2 job=11 statuses=[...]
```

Print on demand (a keybind or chat command) rather than per frame. The value is
that it states what the *observer* thinks, which is exactly the thing no
single-client test can show you.

### S4 — render headgear, robes and dye *(the actual Gap 1 feature)*

Only when the feature is wanted. In order: add the fields to `Common` → extend
`get_entity_part_files` → palette support for hair/clothes colour → wire the
`SpriteChangeType` arms S2 left returning `None`. Each step is independently
testable, and S2 guarantees the last step cannot be forgotten.

### S5 — process, so the class stops recurring

- **Two-seat rule.** Any change to a visual states how it was checked from the
  observer seat, or says explicitly that it cannot be observed. Rows 1–11 in the
  checklist below are the standing script.
- **Trust the timestamp, not the build log.** `make map` is not a target here; it
  fails with *"No rule to make target"*, which does not contain the word "error"
  and slips through a grepped build log. This cost two false negatives in one
  session, and is now enforced by `Hercules/dev.sh build` rather than by
  remembering to run `ls -la Hercules/map-server`.
- **Owner-only state has a recipe.** Anything broadcast from private state hits
  the three traps below. Follow `LOOK_AMMO` rather than improvising.

## Latent — nothing depends on these yet

The server tells observers nothing about **refine level, cards, item options, or
shadow gear**; none appear in the spawn packet. No visual currently keys on
them, so there is nothing to fix. If one ever does — a glow on high-refine
weapons is the obvious candidate — it needs the same treatment `LOOK_AMMO` got,
and the ordering trap below applies.

## The three traps `LOOK_AMMO` had to solve

Any future broadcast of owner-only state hits all three. They are cheap to get
wrong and each fails *silently*.

1. **Persist it in `view_data`.** A look change reaches only whoever is already
   watching. Without server-side state there is nothing to re-send.
2. **Seed it at login.** `pc_equipitem` runs only on a manual equip, so a
   character who logs in already equipped would broadcast nothing. Seeded in
   `clif_inventoryItems` (`clif.c`).
3. **Re-send on enter-view.** `clif_getareachar_unit` (`clif.c`), next to the
   `LOOK_ROBE` re-send. Without it, anyone standing there before you arrived
   shows the default forever.

Trap 3 is the one that looks fine in casual testing: walk up to a stationary
archer and you would see nothing wrong until you happened to watch them equip.

## Running two clients

**Already set up on this box (2026-07-28)** — the second instance lives at
`/Volumes/T7/GitHub/Ragnarok_Online/client2/`, outside both git repos so it needs
no gitignore entry and survives across sessions:

```bash
cd /Volumes/T7/GitHub/Ragnarok_Online/client2
KORANGAR_PACKET_LOG=1 /Volumes/T7/GitHub/Ragnarok_Online/korangar/target/release/korangar
```

Its `client/game_archives.ron` uses **absolute** paths (relative ones resolve
against the *new* cwd, not `korangar/korangar/`) and `client/login_settings.ron`
logs in as `headless2`. `window_cache.ron` was deliberately not copied — two
instances sharing one settings dir fight over it.

**`bgm/` must also be linked into the second cwd** — it is symlinked there
already. Found live: the second seat had sound effects but **no music**, which
looks like a broken audio device and is not. Effects go through
`game_file_loader` and so follow the absolute GRF paths, but music does not touch
the archive layer at all: `find_file_path` (`korangar-audio/src/lib.rs:889`)
builds a plain relative `bgm/<track>.mp3` and resolves it against the **cwd**. A
second instance therefore plays every sound *except* music. The same applies to
map BGM — only the `mp3NameTable` lookup is in the GRF, the file itself is not.

Everything else is cwd-safe, checked at the same time: `msgstringtable` tries the
GRF *before* its relative fallbacks (`msgstringtable.rs:57`), themes live in the
copied `client/` dir, and the only other relative read is in test code.

Accounts: `korangar` (2000000) and **`headless2` (2000001) are BOTH group 99** —
an earlier version of this note said headless2 was GM 0, which is wrong and cost a
detour. Each seat can therefore `@`-command itself; no cross-client gearing.
`headless3` (2000002) is group 0. Plaintext passwords in `login.user_pass`.

Characters, both already bow-geared from the 2026-07-27 session:

| Seat | Account | Character | Class | Sex | Gear |
|---|---|---|---|---|---|
| A | `korangar` | `test` (150000) | Sniper (4012) | M | Bow + Fire Arrow ×193, several other arrow types |
| B | `headless2` | `HeadlessTwo` (150018) | Hunter (11) | **F** | Bow + Fire Arrow ×500, Arrow ×500 |

`HeadlessTwo` was switched to **female** on 2026-07-28 so the pair can become
Clown + Gypsy for row 7 of the RESUME-HERE checklist — ensemble skills require
opposite sex (see below). Done as a `char`.sex SQL update, which is honoured
directly because PACKETVER 20220406 ≥ 20141016 (`char.c:1033 char_mmo_gender`).
The in-game `@changecharsex` is **self-only** and also resets skills; that reset
exists for sex-locked Bard/Dancer skills, so skipping it was safe for a Hunter.
Revert with `@changecharsex` from that seat, or flip the column back.

`KORANGAR_PACKET_LOG=1` prints the line that settles ammunition questions:

```
[ranged-attack] view=11 local_player=false ammo_item=Some(ItemId(1752))
                ammo_sprite=Some("아이템\불화살.spr") used_fallback=false
```

`local_player=false` with `used_fallback=false` is the proof an observer resolved
someone else's real ammunition. `used_fallback=true` means it fell back to the
weapon-class default.

> **Build gotcha that cost two false negatives.** The map server target is
> **`make map_sql`**, not `make map` — the latter fails with *"No rule to make
> target"*. **Fixed 2026-07-28: use `Hercules/dev.sh build`**, which runs the
> right target and then fails loudly if `map-server` was not relinked while a
> source file is newer than it. If you build by hand anyway, confirm
> `ls -la Hercules/map-server` is newer than your edit first.

## Live checklist — NOT yet walked

Two clients, both with bows, in view of each other.

**Rows 1, 2 and 6 were mis-specified** and are corrected below. They said "A sees
the glow appear when B *equips*", which describes something the code never does:
the glow is attached to `SkillProjectile::arrow(...)`, so it only exists on an
arrow **in flight**. Equipping ammunition is invisible to everyone — RO does not
render arrows on a standing character. As originally written these rows would
have failed for a reason that is not a bug.

| # | Check | Expected | Result |
|---|---|---|---|
| 1 | B equips Fire Arrow, then **fires** | A sees the arrow fly with a red glow | **PASS** (2026-07-28, after both fixes) |
| 2 | B swaps to Iron Arrow and **fires** | plain arrow, **no** glow — 1770 is non-elemental | **PASS** |
| 3 | A walks out of range and back | glow still correct (enter-view re-send) | **PASS** — verified by an actual walk |
| 4 | B relogs with arrows already equipped | A sees them correctly, no re-equip (login seed) | **PASS** — was the original failure, see Bug 1 |
| 5 | B fires; check A's log | `local_player=false`, `used_fallback=false` | **PASS** |
| 6 | B unequips ammo and **fires** | A's arrows revert to plain | ☐ — needs ammo in a second stack to fire at all |
| 7 | Repeat 1–3 with a gunslinger or shuriken | same behaviour — the broadcast is item-id based | ☐ |
| 8 | A changes **own** hair | updates without relog — **expected to FAIL**, Gap 2 | ☐ |
| 9 | B changes hair while A watches | A sees it update live | ☐ |
| 10 | B casts a levelled skill (e.g. Fire Bolt) | effect matches what B sees | ☐ |
| 11 | B gains a status with values (Sage field) | A sees the same visual | ☐ |

**Still unobserved: the mid-session equip path.** A diagnostic showed
`pc_equipitem`'s ammo branch ran **zero** times across a whole session — the test
characters log in with ammunition already equipped from the `inventory` table, so
no manual equip ever reaches the server. Every pass so far exercised the login
seed and the enter-view re-send, never `pc_equipitem`. Test it explicitly.

## What rows 1–5 found: two bugs, both fixed 2026-07-28

Neither was in the checklist, and neither is visible from one client.

### Bug 1 — the login broadcast reached nobody (server)

`clif_parse_LoadEndAck` sent the `LOOK_AMMO` seed from `clif->inventoryList(sd)`
~95 lines **before** `map->addblock(&sd->bl)`. `clif_changelook` defaults to
`target = AREA`, and an AREA send walks the map's block list — which the character
has not joined yet, so the broadcast went to nobody. It still set `vd->ammo`
locally, which is exactly why the *arriving-observer* path worked and masked this:
`clif_getareachar_unit` re-sends correctly, so walking away and back passed while
watching someone log in failed. `clif->spawn` cannot cover the gap either, because
`LOOK_AMMO` rides `LOOK_FLOOR`, which the spawn packet never carries.

Fixed by re-broadcasting straight after `clif->spawn`, where the character is on
the map and has an audience.

### Bug 2 — the client threw the value away twice (client)

Even with bug 1 fixed, observers still drew the generic arrow. Instrumenting both
ends showed the packet was sent correctly, arrived correctly, and had the right
enum discriminant (`LOOK_FLOOR` = `Ammunition` = 11 on both sides) — the client
was discarding it, in **two independent ways**:

```
change-ammunition account=2000001 entity_found=FALSE   ← arrives pre-spawn, dropped
add-entity        id=EntityId(2000001)                 ← entity born here
change-ammunition account=2000001 entity_found=TRUE    ← applied
add-entity        id=2000001 replaces_existing=TRUE old_ammo=Some(1752)  ← wiped
```

1. **Pre-spawn race** — `ChangeAmmunition` did `if let Some(entity) = …find(…)`,
   so a value arriving before the entity existed was silently dropped.
2. **Entity replacement** — `AddEntity` deliberately removes and rebuilds an
   entity that is already on screen ("so the entity doesn't exist twice"). The
   replacement inherits only `inherit_fade_state`; ammunition was reset to `0`.

Either alone breaks it, which is why fixing one at a time kept "not working".

Fixed by moving the value **off the entity** into
`ClientState::remote_ammunition: HashMap<AccountId, ItemId>`. Entity lifetime can
no longer lose it. `Common::ammunition` and its accessors were deleted rather than
left as a second source of truth. Cleared on map change only — the server re-sends
on enter-view so a stale entry is overwritten, whereas evicting on entity removal
would reopen the same hole.

**The generalisable lesson.** This is the third time this shape has bitten the
project (`clif->arrowequip` arriving mid-inventory-list on 07-27, then both halves
above). *Any* `account_id`-keyed event applied with `if let Some(entity) = find(…)`
is a silent drop waiting to happen, and the S2 rule — make the match exhaustive so
the compiler objects — does not help here because the drop is in the *handler*,
not the mapping. When S4 renders headgear, robes or dye, route them through the
same off-entity map rather than the entity.

**Method note worth keeping.** Four plausible causes were killed by measurement
after surviving code review: the `target = SELF` hidden-object override, an enum
discriminant mismatch, an equip/unequip ordering race, and entity reconstruction
(dismissed too early on `replaces_existing=false` in the login burst — it does
happen, just later). Instrument both ends before choosing a fix.

Rows 1–7 cover the shipped ammunition work. Row 8 is Gap 2. Rows 10–11 are
regression cover for the two categories the audit cleared on inspection but
which have never actually been eyeballed from the far seat.
