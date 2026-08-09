# Testing completeness — what each layer proves, and what nothing proves

| | |
|---|---|
| **Status** | Written 2026-08-09, after a day spent measuring what the suite actually establishes. Tier 1 and Tier 2 are the build-out |
| **Why** | The suite was green and the numbers were being read as stronger claims than they support. Nine of the day's findings were bugs *in the tests*, not the product — and a test that lies is worse than no test |
| **The rule** | **No layer may fail silently.** Not coverage percentage — every gap must be either covered or *stated*, and every audit passes only when each hit is classified |

## The measurement that started this

Green did not mean what it looked like. Measured across a full run:

- **983 skill casts**: 36% were the server *refusing* the skill, 26% were passive
  skills never cast at all, 6% were accepted silence. Only **25 of the 403**
  skills the sweep touches were checked against an outcome specific to them.
- The sweep's matcher **stops at the first event it recognises**, and `SkillCast`
  is checked first — so `cast` (12% of observations) means *a cast bar started
  and we stopped looking*, not that the skill did anything.
- **43 of 81 allowlist entries were dead**, each one a loaded gun: silence in
  that skill would be absorbed without a word. One of them
  (`MG_NAPALMBEAT`) was actively masking an intermittent.
- The `SG_` blanket prefix covered **18 skills where only 8 are ever silent**.
- **103 campaign story beats** are catalogued and never executed.
- **3 client event arms** discard the event outright, and they are the quest
  system the campaign depends on.

None of that was visible from a pass/fail count, which is the point.

## What each layer proves

| Layer | Proves | Blind to | State |
|---|---|---|---|
| headless suite (135 scenarios) | the wire data is correct | everything in `korangar/src` | strong |
| fork-delta guards (11/11) | an upstream merge did not silently drop a server patch | — | strong |
| `packetver-variants.py` | the client registers the header active at this PACKETVER | — | strong |
| `event-routing.py` | no server event is discarded outright | values stored but never read; draw order | new |
| client unit tests (306) | world/state logic | how events reach that state | decent |
| skill sweep | **the wire is alive** — not that skills work | skill behaviour | weak, now labelled |
| — | — | **campaign content** | absent |
| GUI pass | pixels and behaviour | — | manual, mostly unrun |

**The claim to never make:** "the suite is green, so the game works." The tester
links `ragnarok-packets` and `korangar-networking` and **not** `korangar/src`.

## Tier 1 — build first

### 1a. Campaign content smoke test

`dm-beat-table` verifies 19/19 arc menus and 59 warp beats, and **catalogues 103
story/encounter beats without executing them** — deliberately, because they spawn
bosses and mutate campaign flags. So the suite proves the campaign's *menus*
open and has never run a beat.

For a fork whose stated purpose is the Seal Cascade campaign (CLAUDE.md rule 1),
that is the largest untested surface in the tree — 30 files, 10,389 lines, 66
scripted NPCs.

The scaffolding already exists: `@dm reset confirm` wipes campaign state and
`dm-beat-table` already walks every arc. What is needed is executing each story
beat, asserting it completes rather than erroring, and resetting between arcs.

**Trap to respect:** these mutate shared campaign state and spawn mobs. The
scenario must reset on *every* path including failure, or it hands the next
scenario a half-run arc — the shared-state failure this suite has been bitten by
repeatedly (see `observer.rs`'s header, and the 165-second Land Protector field
that silently killed `AL_PNEUMA` two minutes downstream).

### 1b. Skill correctness — observation window, then derived expectations

Two changes, in order. The second is worthless without the first.

1. **Collect a window of events instead of stopping at the first match.** Today
   `SkillCast` arrives first for anything with a cast bar and the sweep stops
   there, so it cannot see what the skill did.
2. **Enforce the derived expectations.** `tools/generate_skill_expectations.py`
   reads `skill_db.conf` and derives what each skill must be seen to do — 664 of
   976 player skills: `Unit:` → a unit is placed, `StatusChange:` on a
   self/friend skill → the caster gains it, enemy-targeted with a damaging
   `Hit:` → damage. Skills whose entry says nothing keep the loose standard,
   which is honest.

**Validated before building, and this is why it is staged:** enforcing the
expectations against today's observation model would redden **217 working
skills**. The assertion must accept an explicit refusal (`fail-feedback`,
`fail-missing-item`) as a legitimate alternative, because the sweep genuinely
cannot meet every precondition — no gemstones, no arrows, no combo state.

**Two derivation bugs caught by validating against real runs**, both of which
would have produced a confidently wrong table:
- `skill_db` spells the attack flag **`Enemy`**, not `Attack` — the first version
  derived *zero* damage expectations and looked plausible doing it.
- **`StatusChange:` on an enemy-targeted skill lands on the TARGET**, not the
  caster. Frost Diver freezes the monster; the sweep watches the caster.

Generating a table removes drift and *nothing else*. It still encodes whatever
assumption you wrote.

### 1c. Aim `Support`-typed skills at the partner

`Friend`-targeted skills arrive on the wire as `SkillType::Support`, and the
sweep casts Support **at the caster**. A Soul Linker cannot link itself, the
target filter rejects it, and Hercules drops the request with a bare `return 0`
— the same silent path as an out-of-range ground cast.

That is 15 allowlist entries whose stated reason ("requires target player of
specific class") describes a server condition when the truth is a harness
limitation. `connect_pair` already provides a partner. Aiming at it converts 15
silences into real coverage.

## Tier 2

### 2a. Per-scenario duration baseline

`skills-mage` ran **4x** its usual time and pass/fail said nothing at all; it was
caught by eye, which is not a mechanism. Record durations, compare against a
committed baseline, flag large deviations. Cheap.

### 2b. The intermittent-silence cluster

Five skills across four jobs go silent intermittently: `MG_NAPALMBEAT`,
`MG_SAFETYWALL`, `HT_SHOCKWAVE`, `HP_BASILICA`, `SL_SMA`. Most are
unit-creating or targeted. One shared cause is far more likely than five
coincidences — cell occupancy, or timing against the 12-second unit wait.

**This cluster only became visible once allowlist entries had to justify
themselves.** Before that, `MG_NAPALMBEAT`'s entry absorbed one of them and the
4x anomaly with it. Investigate as a group, not one at a time.

### 2c. Event-routing depth

`event-routing.py` catches an arm that is literally `=> {}`. It cannot catch an
arm that stores a value nobody reads — stage three of the recurring failure
("the data arrives and nothing displays it": wire → handler → surface lifetime →
draw order). Closing it means extracting routing from the ~4,000-line consume
method so `NetworkEvent` → state can be asserted directly. High cost, real
payoff: three of the four recorded instances of that bug were in this layer.

### 2d. A harness that links `korangar`

Would unlock pathing, `Map`, `Traversable` — e.g. Ice Wall's *blocking* half,
where only the wire half is testable today. High cost; sequence it after 2c,
which shares the refactor.

## Tier 3 — deliberately not building

- **3rd-class skills** (573 never swept — Warlock, Sorcerer, Rune Knight, Arch
  Bishop …). A scope boundary for a campaign fork, not a gap. Stated so the
  number is not misread.
- **`korangar-interface` tests** (0 today). Shared upstream code; cuts against
  the rebaseability rule (CLAUDE.md §4).
- **Parallel suite infrastructure.** The repo's own recorded decision: buys speed
  and isolation, not trust, and trades a known-good serial suite for new failure
  modes.
- **GUI automation.** No viable sprite-verification path here. What *can* be
  automated is the setup, so a pass is 20 minutes of looking rather than an hour
  of provisioning.

## The mechanism that keeps this honest

Three audits now enforce *classify, don't silence*, and each caught something
real on the day it was written:

- **stale allowlist entries** — reported at the end of every run, so the list
  cannot rot again. Known intermittents are excluded, because **a warning that
  fires every run is one nobody reads** — the reporter nearly became the thing it
  was built to prevent.
- **`event-routing.py`** — an event the client discards must be classified with
  a reason.
- **`packetver-variants.py`** — re-run after any PACKETVER change or Hercules
  merge; a stale variant is otherwise invisible.

And the run now ends by printing what it observed, including the sentence *"a
green sweep means the wire is alive, NOT that the skills work"* — because the
whole failure mode here is a number being read as a stronger claim than it
supports.

## Related

- [work-backlog.md](work-backlog.md) — the standing inventory
- [gui-verification-pass.md](gui-verification-pass.md) — the manual layer
- [../../tools/audits/README.md](../../tools/audits/README.md) — audit runbook
- [../../tools/testing/headless_findings.md](../../tools/testing/headless_findings.md) — bug log
