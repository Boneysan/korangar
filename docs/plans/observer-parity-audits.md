# Observer-parity audits — finding the next Fire Arrow before a player does

| | |
|---|---|
| **Status** | Designed 2026-07-29, derived from five live bugs found the same day |
| **Origin** | [observer-view-verification.md](observer-view-verification.md) |
| **Prereq** | Two clients — `client2/`, already provisioned |

## The organising idea

Five bugs were found in one session. Every one was invisible to single-client
testing, and **four of five were invisible to code review as well** — plausible
explanations survived reading and were killed only by measurement.

The useful generalisation is not "handlers drop things". It is:

> **State that reaches the client out-of-band from the spawn packet needs a
> recovery mechanism. Ask which one it has. If the answer is "none", it is a bug
> waiting for the right timing.**

There are exactly three recovery mechanisms in this client:

1. **Carried by the spawn packet** — `EntityData` re-supplies it on every spawn
   and rebuild (`head`, `weapon`, `shield`, `body_state`, `health_state`,
   `option`, health).
2. **Lazily re-requested** — the client notices it is missing and asks again
   (entity names: `are_details_unavailable` → `set_details_requested`,
   `lib.rs:2949`).
3. **Nothing.** The value arrives once. Miss it and it is gone until something
   unrelated happens to resend it.

Ammunition was category 3, which is why it broke in four different ways.

## Audits

Six are static and cheap enough to run as CI checks. The seventh is the real
investment.

### A1 — entity-keyed handlers with no `else`

```bash
grep -n "find(|entity| entity.get_entity_id()" korangar/src/lib.rs
```

**Result 2026-07-29:** 40 sites across ~10 handlers. All share the shape that
silently dropped ammunition. Triage by recovery mechanism, not by the pattern:

| Handler | Recovery | Verdict |
|---|---|---|
| `ChangeHair`, `ChangeShield`, `ChangeWeapon` | spawn packet | safe |
| `PlayerSitDown/StandUp`, `ResurrectPlayer` | spawn `body_state` | safe |
| `UpdateEntityDetails` (name) | **lazy re-request** | safe — *checked* |
| `HealEffect`, `SkillCastCancelled`, `EntityMove` | transient | harmless |

Nothing outstanding today. **Re-run whenever a `NetworkEvent` is added** — the
audit's value is at the moment a category-3 field is introduced.

### A2 — what survives entity reconstruction

`AddEntity` deliberately removes and rebuilds an entity already on screen. The
replacement inherits **only** `inherit_fade_state` (plus animation data and
safe-zone, set explicitly afterwards). Everything else is re-derived from
`EntityData` or lost.

```bash
grep -n "inherit_\|npc.set_" korangar/src/lib.rs
```

Any `Common` field mutated by a `NetworkEvent` but absent from `EntityData` is a
finding. This is how ammunition was wiped after being correctly applied.

### A3 — appearance state gated on `Entity::Player`

```bash
grep -n "if let Self::Player" korangar/src/world/entity/mod.rs
```

**Remote players are built as `Entity::Npc` and use `Common`.** Anything stored
on `Player` alone is invisible to every observer, and the compiler will not say
so — `set_hair` was a silent no-op for remote players for exactly this reason.

**Result 2026-07-29:** clean, after the hair fix. **This is the audit to run
before shipping headgear, robe or dye** (S4), which will otherwise land in the
identical trap.

### A4 — server broadcasts issued before `map->addblock`

```bash
awk 'NR>=11180 && NR<=11321' src/map/clif.c | grep -E "clif->(changelook|sendlook)"
```

An `AREA` send walks the map's block list, so anything broadcast before the
character joins it reaches **nobody**. This is what made the ammunition login
seed useless.

**Result 2026-07-29:** `LOOK_WEAPON` (×2) and `LOOK_SHIELD` are also broadcast
before `addblock` in `clif_parse_LoadEndAck`. **Latent, not live** — the spawn
packet re-supplies both, so observers get the right values anyway. It becomes a
real bug the moment a look type *not* carried by the spawn packet is added
there. Worth a comment at the site rather than a fix.

### A5 — re-send guards that cannot transmit "none"

```bash
grep -nE "if \(.*!= 0\)" src/map/clif.c   # around clif_getareachar_unit
```

A guard of the form `if (x != 0) refreshlook(...)` means an arriving observer can
be told the value *is* something but never that it is **nothing** — so a removal
that happened while they were away is never corrected.

**Result 2026-07-29:** `LOOK_ROBE` still has this guard
(`if (tsd->status.look.robe != 0)`), the exact shape removed from `LOOK_AMMO`.
**Latent** — korangar does not render robes yet (Gap 1). Fix it in the same
change that starts rendering them, or it ships broken. `LOOK_BODY2` is
unconditional and is the correct model.

### A6 — fallback values that collide with real ones

Not a bug class; a **diagnosability** class, and it cost a full extra test round.

`ranged_attack_default_ammunition(17)` returns `13200`, which is also the id of a
real Bullet. So a log line reading `ammo_item=13200` could mean "resolved
correctly" or "resolved nothing and fell back", with no way to tell. The test
only became conclusive using Silver Bullet (`13201`), deliberately *not* the
fallback.

**Rule:** when testing a path with a fallback, choose a probe value the fallback
cannot produce. When writing one, log *which branch ran*, not just the result —
`used_fallback` exists for the sprite lookup and should exist for the ammo
lookup too.

### A7 — observer-parity harness, in two layers

**Correction to an earlier draft of this doc:** "extend the headless tester and it
catches all five" is **false**. The headless tester consumes `NetworkEvent`s
directly and never runs `korangar/src/lib.rs`, so it sees the wire, not the
client's use of it. Of the five bugs, it could have caught **one**.

| Bug | Layer | Headless? |
|---|---|---|
| Login broadcast before `map->addblock` | server | **yes** |
| Pre-spawn silent drop (`find(...)` no else) | client handler | no |
| Entity-rebuild wipe | client handler | no |
| Stale ammunition across weapon change | client handler | no |
| Hair on `Player`, `Common` passing `None` | client model | no |

Four of five lived in the layer the headless suite structurally cannot see —
the same axis note already in `CLAUDE.md`. So the harness has to be two things.

#### A7a — wire parity *(cheap, do first)*

Assert the **server** sends an observer what it should. Nearly free, because the
primitives exist:

- `TestContext::connect_as(config, username, password, character, ...)` is already
  parameterised by credentials — a second session is one call with `headless2`.
- `TestContext` is self-contained (own `NetworkingSystem`, buffer, identity), so
  two can coexist without refactoring.
- It already tracks `entities: HashMap<EntityId, EntityData>` — an observer's view
  of others.

The one gap: that map holds **spawn data**. Sprite-change events must be folded
into it, or the harness cannot see a look change at all. That is the actual work.

Assert: for each action by session A, session B's tracked view of A converges
within the timeout. Catches server-side ordering, missing broadcasts, and guards
that cannot transmit "none" — i.e. audits A4 and A5, mechanically.

#### A7b — client-state parity *(the real investment)*

Catching the other four needs the client's **state layer** driven without a
window: feed `NetworkEvent`s through the real handlers and assert on the
resulting `ClientState`/`Entity`. Today that logic lives in `lib.rs` inside the
event loop, so it cannot be reached from a test.

Two routes, in increasing cost:

1. **Formalise the diagnostics.** Today's `[entity-diag]` / `[hair-diag]` prints
   were ad hoc and deleted afterwards. A permanent, env-gated observer dump —
   S3 in the original audit — would make the *real* client scriptable: run two
   clients, dump both, diff. Cheap, and it captures the exact instrumentation
   that solved this session.
2. **Extract the handlers** so `NetworkEvent → ClientState` is callable outside
   the render loop. The correct fix, and a genuine refactor — the arms are
   entangled with `self.async_loader`, `self.library` and the graphics engine.

Route 1 is what I would do next; route 2 only if this class keeps recurring.

### A8 — server-side `view_data` lifetime *(new, and it found a live bug)*

The fork stores broadcast state in `sd->vd` (`vd->ammo`). That struct has a
lifetime nobody audits, so ask: **what resets it?**

```bash
grep -rn "memset(&sd->vd\|sd->vd = \|memcpy(&sd->vd" src/map/*.c
```

**Result 2026-07-29 — a real latent bug in our own feature. FIXED same day.**
`status_set_viewdata` (`status.c`) has two branches for `BL_PC`:

```c
if (pc->db_checkid(class_)) { ... assigns individual fields ... }  // ammo survives
else if (vd != NULL) { memcpy(&sd->vd, vd, sizeof(struct view_data)); }  // ammo ZEROED
```

The first branch assigns `class`, weapon, shield, headgear, hair and palettes one
by one and never touches `ammo`, so it survives a job change. The second —
**disguise into a mob or NPC class** — overwrites the whole struct, zeroing
`vd->ammo`. Un-disguising returns through the *first* branch, which never
re-assigns `ammo`, so it stays `0` **permanently**.

Symptom: `@disguise` → `@undisguise`, and every observer draws that player's
arrows as generic, forever, until they happen to re-equip ammunition. **This
matters here because DM mode uses disguises.**

**Fixed** by re-deriving `vd->ammo` in the player-class branch of
`status_set_viewdata`, beside the existing `clif->get_weapon_view` call — so it
is restored on un-disguise the same way weapon and shield already are. Note it
guards on `>= 0`, not upstream's `> 0` for the same index elsewhere in `pc.c`:
inventory slot 0 is valid, and that off-by-one is a live upstream bug.

**The general rule:** any field the fork adds to `view_data` inherits this
lifetime problem. Adding one means auditing every wholesale write to `sd->vd`.

## Priorities

Grounded in what the code actually looks like, not guessed:

| | Item | Cost | Why |
|---|---|---|---|
| 1 | ~~Fix A8's disguise hole~~ | **DONE 2026-07-29** | Was a live bug in shipped code; DM mode uses disguises |
| 2 | **A7a wire parity** | small — `connect_as` already takes credentials; the work is folding sprite-change events into `TestContext::entities` | Makes A4/A5 mechanical |
| 3 | **S3 observer dump** (A7b route 1) | small | Turns this session's ad-hoc diagnostics into a permanent tool |
| 4 | A4/A5 latent comments | ~10 min | Stops the next reader re-deriving them |
| 5 | A7b route 2 (extract handlers) | real refactor | Only if this class recurs again |

## Exit criteria

An audit "passes" when every hit is classified, not when it returns nothing:

- **A1/A2** — every hit has a named recovery mechanism (spawn packet, lazy
  re-request) or a fix
- **A3** — no appearance state on `Player` alone
- **A4/A5** — each remaining site has a comment saying why it is safe
- **A8** — every fork-added `view_data` field survives, or re-seeds after, every
  wholesale write to `sd->vd`

## Beyond the observer axis — other things worth auditing

Same question, different state:

- **Status effects with values.** `clif_status_change_sub` sends `AREA` with
  `val1..3`; the client applies them to any visible entity. Untested from the far
  seat (checklist row 11). Values are the interesting part — the fork already
  had to patch `status_get_val_flag` for `SC_VOLCANO`/`DELUGE`/`VIOLENTGALE`
  because the server sent zeroes.
- **Skill effects and cast bars.** Cleared by inspection in the original audit,
  never eyeballed from the far seat (row 10).
- **Ground units.** Land Protector and traps are drawn from unit packets, not
  from the caster's state — so they *should* be observer-safe, which is a
  prediction worth falsifying.
- **Death, respawn, resurrect.** `body_state` is in the spawn packet, so it
  should self-heal; the interesting case is dying while an observer is away.
- **Ammunition sprite coverage.** 19 arrow sprites ship, 9 are mapped, and
  `iteminfo` returns the generic resource for the rest — see the survey in
  [observer-view-verification.md](observer-view-verification.md). Needs the
  roBrowser-table method, **not** guessed filenames.

## Method notes worth keeping

- **Instrument both ends before choosing a fix.** Four plausible causes each
  survived code review and were killed by measurement: the `target = SELF`
  hidden-object override, an enum discriminant mismatch, an equip/unequip
  ordering race, and entity reconstruction (dismissed early on
  `replaces_existing=false` in the login burst — the replacement comes later).
- **A temporary `ShowInfo` plus two client prints** ("packet received", "entity
  found") splits server-from-client in a single run. That pair answered in
  minutes what hours of reading did not.
- **Audit documents can be wrong.** Gap 2 of the original audit asserted the
  local player is absent from `entities()`; diagnostics disproved it, and its
  proposed S1 fix was for a hole that does not exist. Verify a documented gap
  before building on it.
- **Grep can under-report and sound confident.** Extracting atcommand names with
  a pattern that only matched `ACMD_DEF(name)` silently skipped all 50
  `ACMD_DEF2("alias", fn)` entries — 17% of commands — and produced the false
  claim that `@hairstyle` did not exist. Same failure mode as
  `get_files_with_extension` under-reporting GRFs.
