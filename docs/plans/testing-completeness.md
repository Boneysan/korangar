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
- The sweep's matcher **stopped at the first event it recognised**, and
  `SkillCast` was checked first — so `cast` (12% of observations) meant *a cast
  bar started and we stopped looking*, not that the skill did anything.
  **Closed 2026-08-10** by §1b step 1; the measurements above are from before it.
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
| client unit tests (277 run, 17 ignored) | world/state logic | how events reach that state | decent — but `cargo test -p korangar` did not COMPILE until 2026-08-10 |
| skill sweep | **the wire is alive**, and now *what* each cast produced | whether that is the *right* thing (§1b step 2) | was weak; the window is in, the assertion is not |
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

1. ~~**Collect a window of events instead of stopping at the first match.**~~
   **DONE 2026-08-10.** `observe_window` in `scenarios/skills.rs`: wait as before
   for the first recognised event, then keep classifying for the settle time the
   sweep was *already sleeping through* (`cast_ms + 400`, or 400ms). **The window
   is free** — measured against the same jobs, `skills-mage` ran 52s where it ran
   54s before. The result reported is no longer the first match but the strongest
   one (`evidence_rank`), and the per-skill table prints the whole window:
   `damage   [cast -> post-delay -> damage]`.

   **What it changed on the first run, which is the point:** every Mage bolt used
   to report `cast`. They now report `damage`. Across a job, 9 of 27 answered
   Priest casts had something after the first event — those are results the old
   model stated wrongly, not results it was silent about.

   **`cast` and `post-delay` now rank LAST**, below an explicit refusal. A cast
   bar starting is the weakest thing the sweep can see, and it used to outrank
   everything by arriving first.

   **The bug this immediately found — and it would have made step 2 pass for
   everything.** Hercules sends `SI_POSTDELAY` (icon 46) to the caster on *every*
   skill use (`skill.c:6616` + six siblings, gated only on
   `display_status_timers`, on by default). The sweep counted that as `buff`, so
   "the caster gained a status" was true of every skill in the game — including
   Cold Bolt, which grants nothing. It stayed invisible because `SkillCast` won
   the race for anything with a bar. It is now its own weakest label and excluded
   from `buff`.
2. **Enforce the derived expectations.** `tools/generate_skill_expectations.py`
   reads `skill_db.conf` and derives what each skill must be seen to do — **770**
   of 976 player skills: `Unit:` → a unit is placed, `StatusChange:` on a
   self/friend skill → the caster gains it, `NoDamage: true` → a no-damage
   effect, enemy-targeted with a damaging `Hit:` → damage. Skills whose entry
   says nothing keep the loose standard, which is honest.

   **Measured, not enforced, 2026-08-10.** The table is generated into
   `scenarios/skill_expectations.rs`, the sweep compares every cast against it,
   and the run prints the verdict split — `met` / `refused` / `blocked` /
   `unmet`. Nothing fails. The point of the measurement is to find out whether
   enforcing is honest yet, and the answer is decided by the `unmet` count, not
   by wanting it to be true.

   **Three derivation bugs the first real measurement found**, each of which
   would have reddened working skills:
   - **`Hit:` does not mean damage; `DamageType: { NoDamage: true }` does.**
     Decrease AGI, Lex Divina and Lex Aeterna are enemy-targeted with
     `Hit: "BDT_SKILL"` and deal exactly zero damage. 63 skills moved out of
     `Damage` into the new `Effect` kind, whose observable is `ZC_USE_SKILL`.
   - **A status with no `Icon:` is invisible to every client.**
     `ZC_MSG_STATE_CHANGE` carries the `SI_` icon, not the `SC_`, and 147 of
     Hercules' 694 statuses declare no icon — `SC_SIGHT` among them. 46 skills
     promised a status that is never sent; the generator now drops them back to
     the loose standard and names them.
   - **A modal choice is not a refusal and not a failure.** `AL_WARP` promises a
     unit and delivers a destination picker; the portal does not exist until
     something answers. Its own verdict (`blocked`).

   **What is left, and it is not derivable.** `AL_CRUCIS` is `Self`-typed with a
   `StatusChange:` that Hercules applies to the *enemies* in its splash, not the
   caster. `SplashArea` does not separate it — 114 skills carry that flag and
   most of them (Angelus, Impositio, Adrenaline) really do buff the caster. It is
   a fact about `skill_castend_nodamage_id`, not about the database, so the
   generator cannot see it. Left as a reported `unmet` row rather than guessed
   at.

**Validated before building, and this is why it is staged:** enforcing the
expectations against the *pre-window* observation model would have reddened
**217 working skills**. The assertion must accept an explicit refusal
(`fail-feedback`, `fail-missing-item`) as a legitimate alternative, because the
sweep genuinely cannot meet every precondition — no gemstones, no arrows, no
combo state. It does now, and `blocked` was added for the same reason.

**Measured across a full green run (2026-08-10, 135/0/1): 235 met, 372 refused,
7 blocked, 19 unmet over 633 casts.** Against the pre-window projection of *217
would redden*, the real number is **19** — and those 19 are **6 distinct skills**
repeated across jobs.

The refusal count is the honest ceiling on what this layer can prove without
provisioning every precondition, and it is the majority of all casts.

**The 7, and why enforcing is still not right.** None is a product bug and only
one is a derivation bug:

| Skill | What happened |
|---|---|
| `MC_IDENTIFY`, `RG_CLEANER`, `SA_SPELLBREAKER`, `SG_FEEL` | resolved against **nothing to act on** — no unidentified item, no graffiti, no cast to break, no star to feel. The server accepts, does the work, and has nothing to report |
| `PR_TURNUNDEAD` | the sweep's target is a Pupa, which is not undead |
| `AL_CRUCIS` | the underivable one, above |

**`SA_AUTOSPELL` was a seventh, and it was a MISCLASSIFICATION, not a missing
precondition.** `NetworkEvent::AutoSpellList` already existed — the
`auto-spell-list` scenario waits on it — but the sweep's classifier had no arm,
so the picker arrived and was *ignored* while `no-damage-effect` matched instead.
The sweep was looking straight at the list and not seeing it. One arm later it
reads `[cast -> post-delay -> no-damage-effect -> spell-list]` and its verdict is
`blocked`. Worth separating from the other six: a wrong label and an absent
precondition are not the same finding.

So the residue is now one honest category: *"the precondition was absent and the
server said nothing"* — the same as `refused`, minus the courtesy of a message.
Enforcing today would redden six working skills, which is the one outcome this
staging exists to prevent. **The next step is not the assertion, it is a stated
exemption per skill with its reason**, the same shape as the allowlist.

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

**Shipped 2026-08-10 — and the observation window immediately corrected the
claim above.** The retry does seat a partner and all 15 answer, but with the
window reading past the first event, **14 of the 15 answer `fail-feedback`**:
`cast (partner) -> post-delay (partner) -> fail-feedback (partner)`. Only
`SL_BARDDANCER` produced a real `buff`, and only because the shared partner
happened to be holding that job. Before the window they reported
`cast (partner)`, which read as success.

So the original allowlist reason was **right** and this section was half wrong:
the Soul Link family really does require a target of a specific class, and
seating *a* partner is not the same as seating the *right* one. What it bought is
honest: 15 silences became 14 visible refusals and 1 real cast. Covering the
family properly means setting the partner's job per skill (`@job` on the second
seat, 15 job changes), which is a real cost against a family of skills that does
nothing but grant a class-gated buff. **Left open deliberately, and stated rather
than allowlisted** — a refusal is a classified outcome, which is the rule.

## Tier 2

### 2a. Per-scenario duration baseline

`skills-mage` ran **4x** its usual time and pass/fail said nothing at all; it was
caught by eye, which is not a mechanism. Record durations, compare against a
committed baseline, flag large deviations. Cheap.

**Mostly covered already, by `tools/audits/flaky.py --slow-factor`** — it takes
the median across the run logs you hand it and flags any scenario whose worst run
exceeds it. That is *better* than a committed baseline, which goes stale against
machine and server changes and then gets ignored. **What is actually missing is
the logs**: nothing archives a full run, so the audit only works if whoever ran
the suite kept the output. That is the piece to build, and it is a directory and
a redirect, not a feature.

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

**A fifth instance, found 2026-08-10, and `event-routing.py` was green while it
sat there.** `spirit_spheres` has exactly **one write and zero reads** in the
whole tree: `ZC_SPIRITS` is modelled, registered, evented, routed, and stored on
the entity, and **nothing draws it** — Monk spheres and Gunslinger coins have
never appeared on screen. The `spirit-spheres` headless scenario is green,
because it asserts the wire.

It was not found by either testing layer. It came out of
`observer-parity.sh`'s **A2** check, which was pointing at something else
entirely — that the field has no `EntityData` slot and is therefore wiped on
`AddEntity` rebuild (the ammunition shape). Grepping for readers while triaging
that is what turned up the larger problem. **Two audits aimed at different
things, and the one that hit was aimed elsewhere.** That is an argument for 2c,
not a substitute for it: a `NetworkEvent` → state assertion would have caught
this directly instead of by accident.

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

## A layer that was counted and could not run

`cargo test -p korangar` **did not compile** — the trait solver overflowed on
`StorageAccess: Sync` through wgpu-core and naga. `cargo build --release` was
unaffected, so the client built, ran and shipped while the test profile failed,
and the table above counted 306 tests that nobody could execute. Fixed
2026-08-10 with the `#![recursion_limit = "256"]` the diagnostic itself asks for;
the real numbers are 294 collected, **277 passed, 0 failed, 17 ignored**.

**The general lesson, and it is the same one as everything else on this page:**
every number in the table above is a claim, and a claim nobody re-derives decays
into decoration. This one had been wrong long enough that its own count was
wrong too. The audits below exist so that the *other* claims cannot rot the same
way — and the thing that caught this was simply running it.

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
