# Observer parity — the property, the gaps, and a harness that finds them

| | |
|---|---|
| **Status** | Designed 2026-07-29. Supersedes the "what to build next" half of [observer-parity-audits.md](observer-parity-audits.md) |
| **Goal** | Every client that can see a character converges on the same appearance, animation and effects — the guarantee a modern client gives and RO never did |
| **Prereq** | Two headless sessions (already possible), later two GUI clients (`client2/`, provisioned) |

## 1. The property, stated so it can be tested

Six bugs in one session all had the same shape, and "handlers drop things" is too
narrow a lesson. The property we actually want is **convergence**:

> For every observable attribute `S` of character `A`, every client that can see
> `A` reaches the same value of `S` within a bounded time — **regardless of when
> that client arrived, what order the packets landed in, or what happened while
> it was away.**

Three quantifiers do all the work, and each maps to a test axis:

| Quantifier | Test axis | The bug it catches |
|---|---|---|
| *every client* | two sessions, assert on the **observer**, never the actor | state only the actor is told about |
| *when it arrived* | change-then-arrive, arrive-then-change, relog | missing enter-view recovery |
| *what order* | packet before spawn, spawn after packet, rebuild | silent drops, rebuild wipes |

`LOOK_AMMO` broke in four different ways because it failed all three. Nothing
about it was special; it was simply the first attribute we looked at this hard.

## 2. Four boundaries, and where the six bugs sat

State crosses four boundaries between the server's memory and a pixel. Each is
independently auditable and — crucially — **independently automatable**.

```
   server state          B1              B2                B3                B4
  (sd->vd, sc, …) ──broadcast──▶ wire ──packet→event──▶ NetworkEvent ──handler──▶ ClientState ──compose──▶ sprite
```

| | Boundary | Failure mode | Of the six bugs |
|---|---|---|---|
| **B1** | server → wire | not broadcast; wrong target; sent before `map->addblock`; no enter-view recovery; guard can't transmit "none"; source zeroed server-side | **3** (pre-addblock login seed, disguise `memcpy`, enter-view guard) |
| **B2** | wire → `NetworkEvent` | packet unmodeled, `register_noop`, or a match arm falling to `_ => None` | 0 *(so far — §3 shows this is the widest boundary)* |
| **B3** | event → `ClientState` | `if let Some(entity) = find(…)` with no else; entity rebuild wipes it; stale value never invalidated | **2** (pre-spawn drop, `AddEntity` wipe, stale ammo) |
| **B4** | state → sprite | field stored somewhere the renderer doesn't read; state on `Player` when observers are `Npc` | **1** (`set_hair` gated on `Entity::Player`, `Common` passing `head: None`) |

The headless suite as it exists today sees **B1 and B2 only**. That is not a
defect to fix by extending it — it is the correct scope for a wire tester. B3
and B4 need different instruments, described in §5.

## 3. What the audit found today

Run cold against the current tree, before writing any harness.

### 3.1 The headline: the wire already carries everything, and we throw it away
*(FIXED 2026-07-29 for the seven appearance fields — the diagnosis below stands,
and the guild/c_level fields are still dropped deliberately.)*

`clif_set_unit_idle` fills the spawn packet with the **complete** appearance —
`hair_style`, `hair_color`, `cloth_color`, `head_top`, `head_mid`,
`head_bottom`, `robe`, `weapon`, `shield`, `body_style`, `sex`, `clevel`, guild
emblem (`clif.c` ~1885–1935). `EntityAppearPacket` / `EntityAppear2Packet` /
`MovingEntityAppearPacket` model all of it faithfully.

Then `EntityData` (`korangar-networking/src/entity.rs:4`) keeps **17 fields and
drops 10**: `accessory`, `accessory2`, `accessory3`, `head_palette`,
`body_palette`, `robe`, `guild_id`, `emblem_version`, `c_level`, `body`.

So hair colour, clothes colour, all three headgear slots, robes, body style and
the guild emblem are missing for **every entity, local and remote** — and it is
not a broadcast gap or a handler gap. It is one struct, with four construction
sites (`from_character` plus three `From` impls).

**Why this is the highest-leverage fix in the whole programme:** the spawn packet
is recovery mechanism #1. Widening `EntityData` gives every one of those fields
spawn, respawn, rebuild, enter-view *and* login recovery **simultaneously and for
free**, because `clif_getareachar_unit` calls `set_unit_idle` unconditionally on
every arrival. Ammunition needed four separate fixes precisely because it is the
one attribute the spawn packet does *not* carry.

### 3.2 B2 is the widest boundary, and it is invisible to code review
*(FIXED 2026-07-29 except hat effects. The match is now exhaustive, so the
compiler — not an audit — catches the next look type; verified by adding a probe
variant and getting `E0004`.)*

**Nine of fourteen `SpriteChangeType` variants fall to `_ => None`**
(`version_20220406.rs:401`): `HeadBottom`, `HeadTop`, `HeadMiddle`,
`HairCollor`, `ClothesColor`, `Shoes`, `Body`, `Robe`, `Body2`. The server
broadcasts them; the client silently discards them. This is Gap 1 of
[observer-view-verification.md](observer-view-verification.md), now counted.

**`ChangeDirectionPacket` (`0x009C`) is `register_noop`** — and
`clif_changed_dir` broadcasts `AREA_WOS` from the parse handler (`clif.c:12158`)
and `AREA` from `unit.c:982`. **A remote player turning in place is invisible to
every observer.** Their facing only ever updates as a side effect of movement.
This is a live animation-parity gap, found by reading the no-op list, and it is
new — no checklist row covers it.

**`EntityStopMovePacket` (`0x0088`) is `register_noop`**, so a remote entity that
stops early keeps animating toward its old destination until the client's own
timer expires.

**`EquipmentEffectPacket` is `register_noop`** — that is `clif_hat_effect`, which
the server bothers to re-send on enter-view (`clif.c:5058`). Hat/garment effects
never render. Low priority for a classic-era fork, but it belongs on the list.

There are **32 `register_noop` registrations** in total. The no-op list is a
mechanical, zero-cost audit that nobody had run: *every entry is a thing the
server says and the client ignores.* Most are correctly ignored. Three are not.

### 3.3 Corrections to [observer-parity-audits.md](observer-parity-audits.md)

Two claims in A5 are wrong, and the second one would have caused real work to be
done for no reason:

- **"`LOOK_BODY2` is unconditional and is the correct model" — false.** It is
  guarded exactly like the others: `if (vd->body_style)` at `clif.c:5031`, and
  again at 1604 and 1928. `LOOK_CLOTHES_COLOR` is guarded at 5029/1602/1926 and
  the audit does not mention it at all. There are **three** guards of this shape,
  not one.
- **"`LOOK_ROBE` … ships broken the day robes render" — false, and it should not
  be fixed.** At enter-view the guard is preceded, unconditionally, by
  `set_unit_idle`, which carries `robe` in the spawn packet. The guard can only
  lose an attribute the spawn packet does **not** carry — which was ammunition,
  and only ammunition. Once §3.1 lands, robes are covered by mechanism #1. Leave
  the server alone and add a comment saying why.

The generalisation worth keeping: **a "cannot transmit none" guard is harmless
for any attribute the spawn packet carries.** That single sentence retires A5 as
a bug class and reclassifies it as a comment task.

### 3.4 A seventh bug, found by the audit suite an hour after it existed

**Armed remote players probably keep the battle stance on town maps.**
`refresh_neutral_stance` runs on the local-player path (`lib.rs:7226`, right
after `set_in_safe_zone`) and never on the `AddEntity` path (`lib.rs:3691`), and
it is not derived at construction: `Common::new` hardcodes ready-fight at
`world/entity/mod.rs:1106` while `in_safe_zone` is still `false`. It does not
self-heal, because `return_to_neutral` pins ReadyFight when `previous_state` is
`2|4|8`. The local player was fixed for this in `abb519f1`; the remote path was
not. **Code-read only — confirm on two clients.** Audit A9.

This is the same two-paths shape as bug 5 (hair on `Player`, observers are
`Npc`), and it is precisely what **L2 eliminates structurally**: one appearance
map keyed by account id means there is no second path to drift.

### 3.5 No enter-view recovery for an in-flight cast

`clif_useskill` is `AREA` only and nothing in `clif_getareachar_unit` re-sends
it, so an observer who arrives mid-cast sees no cast bar and no cast circle. This
is category 3 (no recovery) — but it is also **authentic**: the original client
behaves the same way. Recorded as a decision, not a bug. Revisit only if the DM
campaign wants it.

## 4. The fix order, and why B2 must come first

The harness reads `NetworkEvent`s. **It is structurally blind to anything B2
drops** — a two-session test for headgear parity would pass a broken client
today, because neither session would ever be told. So:

> **Fix B2 before building the harness, or the harness certifies its own blind
> spot.**

Concretely, that is one change with three parts:

1. **Widen `EntityData`** with the ten dropped fields (§3.1). Four construction
   sites; mechanical.
2. **Make the sprite-change match exhaustive.** Replace the nine `_ => None`
   arms with a single generic
   `NetworkEvent::ChangeLook { account_id, look_type, value }`, keeping the
   existing typed events for the five that already have handlers. An exhaustive
   `match` on `SpriteChangeType` means the compiler flags the next look type
   somebody adds — this is S2 from the verification doc, and it is what makes the
   harness able to *see* headgear/dye at all.
3. **Model `ChangeDirectionPacket` and `EntityStopMovePacket`** as real events
   (§3.2). Both are small and both are live gaps.

## 5. The harness, in three layers

Matched to the boundaries. Each layer is cheap *because* the layer below it
narrowed what it has to prove.

### L1 — wire parity, two headless sessions *(covers B1 + B2)*

**Most of this already exists.** `connect_pair` in `scenarios/social.rs:22`
already stands up a GM primary and a non-GM partner on the same map and reports
whether each sees the other. Promote it into `context.rs` and it becomes the
harness primitive.

Three pieces of actual work:

1. **Fold appearance into `TestContext::track`.** Today it tracks
   `AddEntity`/`EntityMove`/`EntitySlide`/`RemoveEntity` and `entities` holds
   *spawn data only* (`context.rs:366`). It must apply every look change to the
   tracked `EntityData`, so the map means "what this session currently believes",
   not "what it was told once".
2. **Per-session ledgers.** One `Ledger` is shared by every session
   (`config.ledger.clone()`), so packet attribution is merged. Observer parity
   needs to know *which* session received a packet. Give each `TestContext` its
   own, and keep a merged view for the existing coverage report.
3. **One assertion primitive.**
   `observer.assert_converges(subject_account, field, expected, timeout)` —
   polls, then fails with the recent-event log that `wait_for` already produces.

Then the matrix. For each attribute, five timings — this is the shape that caught
all four ammunition bugs, generalised:

| | Timing | Catches |
|---|---|---|
| **T1** | change while observed | broadcast missing or wrong target |
| **T2** | change while out of view, then walk into view | no enter-view recovery |
| **T3** | change, then the observer logs in fresh | login seed ordering (`addblock`) |
| **T4** | change, then the subject warps out and back | rebuild wipe |
| **T5** | change **to none/zero** | guards that cannot transmit "none" |

Attributes: hair, hair colour, clothes colour, head top/mid/bottom, robe, weapon,
shield, ammunition, body style, job, sex, direction, sit/stand, death/resurrect.
≈15 × 5 ≈ **75 assertions, all scripted**, running as `--scenario observer`.

Add a sixth timing for anything the fork adds to `view_data`:
**T6 — disguise, undisguise, re-assert.** That is A8, mechanised. It is the only
one of the six bugs that no amount of client-side testing could ever have found.

### L2 — client-state parity *(covers B3)*, headless, via a bounded extraction

The audits doc offers "extract the handlers" and calls it a real refactor. It is
— all 131 `NetworkEvent` arms are entangled with `async_loader`, `library` and
the graphics engine. **But we do not need all 131.** Every B3 bug lived in the
appearance arms, and those can be lifted out on their own:

> Introduce `ClientState::entity_appearance: HashMap<AccountId, EntityAppearance>`
> — the generalisation of `remote_ammunition` (`state/mod.rs:224`), which A2 of
> the audits already names as the reference implementation for this shape —
> holding every observer-visible attribute. Seed it from
> `AddEntity` (possible only after §4.1), update it from every look change, clear
> on **map change**, never on entity removal. Sprite composition reads from it.

That makes `apply_look_change(&mut map, event) -> Option<SpriteReload>` a **pure
function over plain data**: no `Map`, no `PathFinder`, no async loader, no GPU.
Every B3 bug becomes a three-line unit test:

| Test | Bug it pins |
|---|---|
| apply before any `AddEntity`, then spawn → value present | pre-spawn silent drop |
| `AddEntity` twice → value survives | rebuild wipe |
| change weapon → ammunition cleared | stale cross-weapon value |
| observer path and local path read the same map | `Player`-vs-`Common` divergence |

The last one is the point: **the `Entity::Player` / `Entity::Npc` split stops
being able to cause this class**, because there is one map and it is keyed by
account id, not by entity variant. Today `Entity` construction needs a `Map` and
a `PathFinder`, which is why no test in the tree builds one; the appearance map
sidesteps that entirely.

This is smaller than route 2 and lands the same coverage where it matters.

### L3 — render parity *(covers B4)*, two GUI clients, human-free comparison

Cannot be headless — it needs sprites and a GPU. But it can stop needing a human
to *compare*:

**`KORANGAR_OBSERVER_DUMP=<path>`** — env-gated, writes a JSON snapshot of every
visible entity's composed appearance (the resolved part-file list, palettes,
current action, animation frame) on a keypress. Run two clients, dump both,
`diff`. This is S3 from the verification doc, and it is the permanent form of the
ad-hoc `[entity-diag]` / `[hair-diag]` prints that solved the last session and
were then deleted.

The human step shrinks from "do these two screens agree?" to "does this one
screen look right?" — which is the only question a human is actually better at.

## 6. Sequence and cost

| Phase | Work | Stack? | Cost | Unlocks |
|---|---|---|---|---|
| **0** | ~~Correct §3.3; commit the static audits; comment the A4/A5 sites~~ **DONE 2026-07-29** — `tools/audits/observer-parity.sh` + baseline + [runbook](../../tools/audits/README.md), and `clif.c` now explains at both sites why they are safe | no | done | stops re-derivation; every finding classified and diffed on every run |
| **1** | ~~B2: widen `EntityData`, exhaustive look match + `ChangeLook`, model direction + stop-move~~ **DONE 2026-07-29** — workspace green, 5 new wire tests, audit rebaselined (open items 32 → 21) | no | done | **the harness's eyesight**, and every appearance attribute gains spawn-packet recovery |
| **2** | L1 harness — **DONE + LIVE-VERIFIED 2026-07-29, 6/6 double-run.** `TestContext::connect_pair`, appearance folded into `track`, `assert_converges` / `assert_in_view` / `observed`, and six `phase11` rows covering T1–T6 | yes | done | B1/B2 mechanical, forever |
| **3** | ~~L2 appearance map~~ **PREMISE CHANGED — see below** | no | — | — |
| **4** | B4: render headgear, robe, dye (S4) | yes | feature | the actual visible payoff — now landing on a structure that cannot repeat the class |
| **5** | L3 observer dump | yes | small | B4 comparison without a second pair of eyes |

**Phase 3's premise changed, and it should not be built as specced.** L2 proposed
generalising `remote_ammunition` into an off-entity appearance map. Phase 1 went
the other way — appearance lives on `Common`, seeded from `EntityData` — and that
is *more* correct by this document's own §3.1 reasoning: the spawn packet is the
strongest recovery mechanism there is, so an attribute it carries has a
rebuild-safe home on the entity and needs no map. Ammunition needs one precisely
because it is the exception.

What remains of L2 is the part that was always the expensive half: making
`NetworkEvent → ClientState` callable outside the render loop, so the handlers
can be unit-tested. That is still blocked, and now measurably: `Common::new`
takes a `&Library`, and `Library::new` needs a `GameFileLoader` — real GRF
access — so no test in this tree can construct an entity at all. Route 2 (extract
the handlers) is the only way through, and the original advice stands: do it only
if the class recurs.

Phases 0 and 1 needed **no running stack**, and phase 2 was written without one.

**Phase 4 is the one judgment call.** It is the only phase that is a feature
rather than infrastructure, and it is the reason anyone would care about the rest.
Phases 1–3 make it safe; if the campaign needs headgear on screen sooner, do 1
then 4, and pick up 2/3 after — but not 4 before 1, or it inherits every gap in
§3.

## 7. What lands in CI

The automation ladder, in the order it becomes available:

1. **Now, shipped** — `tools/audits/observer-parity.sh` ([runbook](../../tools/audits/README.md)).
   Nine checks, 76 findings, each classified with a rationale in
   `observer-parity.baseline`. Exit 0 nothing new · 1 unclassified hit · 2 stale
   baseline entry. Findings that are classified *and still bugs* are marked
   `# OPEN:` and reprinted on every run, so a green exit can never be misread as
   "nothing wrong". Two seconds, no server, no build.
2. **After phase 1** — an exhaustive-match compile error is the CI check. No
   script needed; the compiler is the audit.
3. **After phase 2** — `headless-tester --scenario observer` beside the existing
   107. Same gate, same exit code, same ledger.
4. **After phase 3** — `cargo test -p korangar`, no server required. The fastest
   feedback in the programme and the one most likely to catch a regression at
   commit time.
5. **Manual** — L3 dump diff, before shipping anything in B4.

## 8. Exit criteria

- **B1** — every attribute has a named recovery mechanism, or a test proving it
  does not need one. Every fork field on `view_data` has a T6 row.
- **B2** — the `SpriteChangeType` match is exhaustive; every remaining
  `register_noop` has a comment saying why ignoring it is correct.
- **B3** — no appearance state reachable only through `Entity::Player`; every
  attribute in the appearance map has the four unit tests from §5.
- **B4** — two dumps of the same scene diff clean.

## 9. Traps, carried forward

- **Instrument both ends before choosing a fix.** Four plausible causes survived
  code review last session and were killed by measurement.
- **Grep under-reports and sounds confident** — `ACMD_DEF2` aliases, GRF listings,
  and (this session) `clif_*` functions that take `send_target` as a *parameter*
  rather than a literal. Read the function before believing the grep.
- **Choose probe values a fallback cannot produce** (A6). Silver Bullet `13201`,
  not `13200`.
- **Audit documents can be wrong** — Gap 2 was, A5 was (§3.3). Verify a
  documented gap before building on it. This document is not exempt.

Related: [observer-parity-audits.md](observer-parity-audits.md),
[observer-view-verification.md](observer-view-verification.md),
[../RESUME-HERE.md](../RESUME-HERE.md), `../../tools/testing/headless_test_plan.md`.
