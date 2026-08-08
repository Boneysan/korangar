# Resume here — live pass status

> **2026-08-08 — the cause-0 work is finished, suite-verified, and STILL NOT ON
> SCREEN.** The row-5 failure ("skill level is not high enough" from a Clown with
> the skill at max — it meant *no ensemble partner*) turned into an audit of every
> channel by which the server reports a failure, and then into a rebuild of how
> that reporting works. See
> [protocol/server-error-channels.md](protocol/server-error-channels.md).
>
> **The finding that reframed it:** cause 0 is not laziness in Hercules, it is the
> protocol's fallback for conditions Gravity never numbered — 21 of the 33 states
> in `skill_check_condition_castbegin` report cause 0 and only **one** has a
> dedicated cause; `skill.c` alone has 174 cause-0 emissions. The official client
> says "Skill level is not high enough" to all of them too. So there is no "send
> the right cause" fix. What there is, is a split:
> - **static preconditions** (`State:` in skill_db — shield, falcon, cart, stance)
>   are on disk at both ends, so nothing needs sending;
>   `tools/generate_skill_states.py` carries them across (42 skills, 13 states);
> - **runtime outcomes** (roll missed, nobody in range, not enough experience, no
>   partner) only the server knows, and now ride **`ZC_SKILL_FAIL_REASON`
>   (fork packet 0x0EFE)** — see CLAUDE.md 3b for the five touch points.
>
> **Two bugs were found by writing the guard rather than by auditing.**
> `ZC_ADD_SKILL` (0x0111) was **never modelled**, so any skill *granted* rather
> than levelled was eaten by the length fallback — Plagiarism's copied skill, quest
> rewards, and anything a campaign script grants, all invisible until relog. And
> the reason field was first modelled as a `ByteConvertable` enum, which fails to
> deserialize on an unknown value and **discards the whole read buffer** — in a
> packet whose enum is documented *append only*, i.e. the intended way to extend it
> was the trigger. Both fixed and guarded.
>
> **Suite: 124 scenarios, green twice** — ordered, and shuffled (seed 20260808),
> 123 passed / 0 failed / 1 structural skip, 0 packet deserialization failures.
> The shuffled pass matters because all scenarios share one character and the
> double-run gate cannot see order dependence.
>
> **What the suite CANNOT tell you, and what to do first on screen.** The headless
> tester links `ragnarok-packets` and `korangar-networking` — **not** `korangar/src`.
> So `NetworkEvent::SkillAdded` → the Skill Tree window is new, unreached code.
> One action each, most valuable first:
> 1. **`@questskill 1014` as a Priest — does Redemptio appear in the skill tree?**
>    The only path no test can reach, and that window is where the M1-017 logout
>    crash lived.
> 2. **Use an Yggdrasil Berry twice inside five seconds** — the reuse-delay message
>    ("%d seconds left until you can use"), which read "Content has been saved in
>    [SaveData_ExMacro5]" before the Hercules gloss fix.
> 3. **Cast Redemptio with no party** — the 0x0EFE reason text as a chat line.
> 4. **`@kick` from a second seat** — the map-connection `SC_NOTIFY_BAN`.
>
> **Block E is still open and is still the actual next task**: Moonlit's α 0.6 is
> confirmed correct (which unblocks the whole song/Gospel/Fog-Wall family), but its
> tile then drew as a solid red square after a fix made during the walk, and **the
> instrumentation log that would settle it in one cast has never been read**. Rows
> 4b and 5 of [plans/gui-verification-pass.md](plans/gui-verification-pass.md).
> Do this in the same session — the client has to be launched for all of it.

<details>
<summary>2026-08-07 — the error-channel audit that started it (superseded above)</summary>

> **2026-08-07 — a whole batch of error-message work is pushed and NOT live-verified.**
> Blocks D and E of [plans/gui-verification-pass.md](plans/gui-verification-pass.md)
> ran, and the row-5 failure ("skill level is not high enough" from a Clown with
> the skill at max — it meant *no ensemble partner*) turned into a full audit of
> every channel by which the server reports a failure. Four channels were
> completely silent, 25 skill-fail causes printed "Skill failed (reason 73)", and
> the message table the client resolves ids through was **wrong often enough to
> invert a success into a failure**. See
> [protocol/server-error-channels.md](protocol/server-error-channels.md) for the
> channel map, the four sweeps needed to find them all, and the short list of
> live checks — `@kick` from a second seat is the most valuable single action.
>
> **Block E is still open**: Moonlit's α 0.6 is confirmed correct (which unblocks
> the whole song/Gospel/Fog-Wall family), but its tile then drew as a solid red
> square after a fix made during the walk, and **the instrumentation log that
> would settle it in one cast has never been read**.

</details>

**Session 2026-07-28/29 closed a lot.** The observer-view checklist
([plans/observer-view-verification.md](plans/observer-view-verification.md)) is
**rows 1-5, 7, 8, 9 PASS**, row 6 retired as unobservable, **rows 10-11 open**.
That pass found **six real bugs**, all invisible to single-client testing:

1. Login `LOOK_AMMO` broadcast issued before `map->addblock` — reached nobody
2. Client dropped the ammunition packet when it arrived pre-spawn
3. `AddEntity` rebuilt the entity and wiped the applied value
4. Cached ammunition survived a weapon change — arrows drawn from a revolver
5. **Every remote player rendered with hair style 1**, always, since `Common`
   passed `head: None` and `set_hair` was gated on `Entity::Player`
6. Disguise zeroed `vd->ammo` server-side and un-disguise never restored it

All six fixed, live-verified on two clients, and pushed. The bug *classes* are
generalised into runnable audits in
[plans/observer-parity-audits.md](plans/observer-parity-audits.md) — which
themselves found two latent issues (weapon/shield broadcast before `addblock`,
and a re-send guard that turned out to be harmless) and one live one (item 6).

**A programme plan now sits on top of those audits:**
[plans/observer-parity-harness.md](plans/observer-parity-harness.md) (2026-07-29).
It reframes the six bugs around the four boundaries state crosses between
`sd->vd` and a pixel, and it found three new live gaps by reading the no-op
list — **remote players' facing changes never reach observers** (`0x009C` is
`register_noop`), stop-move is ignored, and hat effects are dropped. Its headline:
the spawn packet already carries the **complete** appearance (hair colour,
clothes colour, all three headgear slots, robe, body style, emblem) and
`EntityData` drops all ten fields, which is why none of it renders. Read §4
before touching the harness — the ordering there is not optional.

**Harness phases 0 and 1 are DONE (2026-07-29), both written with the stack
down and NOT live-verified.** Phase 0 is the audit suite
(`tools/audits/observer-parity.sh` + [runbook](../tools/audits/README.md)).
Phase 1 closed the wire→event boundary: `EntityData` now carries the seven
appearance fields the spawn packet always sent, the `SpriteChangeType` match is
exhaustive behind a new `ChangeLook` event, and `ChangeDirection` / `StopMove`
produce real events instead of no-ops. Workspace tests green (253 + 21), five
new wire-level tests, audit open items 32 → 21.

**Those appearance fields are stored, not drawn** — sprite composition is still
body + head + weapon + shield. Rendering them is phase 4 and needs accessory
sprite paths and palette files this tree does not have.

**Phase 2 is DONE and LIVE-VERIFIED (2026-07-29): 6/6 PASS, double-run.**
Six scenarios assert on the *observer* across the five timings that generalise
the four `LOOK_AMMO` bugs (change while watched, while out of view, before the
observer logs in, across an entity rebuild, and a change to zero), plus a
disguise round-trip for audit A8.

```sh
cargo run --release --example headless-tester -p korangar-networking -- --scenario phase11
```

**It is `phase11`, not `--scenario observer`** — `--scenario` matches a scenario
*name* or `phaseN`, and "observer" is neither, so it silently selects nothing.

They were **proved able to fail**: deleting the `ChangeLook` tracking makes 4 of
6 fail. `observer-look-fresh-login` still passes without it, because it takes
everything from the spawn packet — which is exactly what that row is for.

## Test-suite validity work — shuffle pass DONE (2026-07-29/30)

The full suite is **114 scenarios**, not 107 or 125.

**A `--shuffle <seed>` detector is in the tester** (verified: same seed
reproduces, different seeds differ, no scenarios lost). All 114 share one
character, so order dependence is the suite's structural weak point, and the
existing double-run gate cannot see it — it runs the same order twice.

```sh
./target/release/examples/headless-tester --scenario all --shuffle 42
./target/release/examples/headless-tester --list --shuffle 42   # preview order only
```

**The shuffle pass is complete and it paid for itself: four order-dependence
bugs plus one real client bug.** Runs: seed 42 → 111/114, fixes → 112/114, fresh
seed 1337 → 113/114 (fixes generalised; one new bug), fixes → re-run.

**Correction to the old tally: the recorded "114/114 green" was never true.**
`skills-dancer` and `skills-gypsy` had been failing the whole time behind a green
count, because a skipped/mis-jobbed scenario reports as PASS (item 1 below).

The four order-dependence bugs, all fixed:

1. **`weapon-refine-missing-material`** — recorded as "root cause not yet found".
   It was never a refine bug: `skill-fail-rejection` drains SP to zero and never
   restores it, `WS_WEAPONREFINE` costs 5 SP, and `ensure_job` does not heal, so
   the drain survived the job change. Now restores SP on the way out.
2. **`observer-look-clear`** — `FAR_MAP` was the constant `"prontera"`, but
   `connect_pair` meets wherever the partner character was *last left*, and that
   position persists in the `char` table. Now `far_map_from(home_map)`. Three
   scenarios used `FAR_MAP`, so all three were latent.
3. **`skills-dancer` / `skills-gypsy`** — *not* order-dependent. The shared
   character is male, and **Hercules does not refuse a sex-mismatched job
   change**: `pc_mapid2jobid` (`pc.c:6465`) round-trips through the character's
   sex, so asking for Gypsy silently yields Clown and the server says "Your job
   has been changed." No failure message exists, so `sweep_job`'s
   gender-restriction skip guard could never fire. Both now route to the female
   partner character (`HeadlessTwo`, which also has GM rights).
4. **`incoming-damage`** — inherited whatever job the previous scenario left;
   after `skills-soul-linker` the provoked mob never retaliates. Passed in
   natural order only because `dm-warp-recall` precedes it there. Now normalises
   the job best-effort. **Two wrong hypotheses first** (a lethal provoking blow —
   the fallback never fired; then mob species — 1007 both passed and failed).
   Bisect settled it: failing state → normalise to 4008 → passes, nothing else
   changed. **Bisect, don't theorise.**

**The client bug the sweeps found once they could run:** `ZC_NOTIFY_MAPINFO`
(**0x0189**) was unmodeled, so `register_length_fallbacks` ate it and four
user-facing messages were dropped — cannot teleport here / save point cannot be
memorized / skill unusable here / item unusable here. Hercules sends this
*instead of* `clif->skill_fail` (`clif.c:6213` says so). Any skill in the map
zone's `disabled_skills` therefore did nothing at all, silently, on any non-PvP
map: `DC_UGLYDANCE`, `BD_ETERNALCHAOS`, `BD_ROKISWEIL`, `CG_HERMODE`,
`BA_DISSONANCE`, `DC_DONTFORGETME`. Fixed with a packet + handler + regression
test (`ee26fb23`). **Not yet seen in the graphical client** — the chat line is
wire-verified only.

**Also fixed later the same session** (details in
[tools/testing/headless_findings.md](../tools/testing/headless_findings.md)):

- **Skips no longer count as passes.** `Scenario` has a third outcome and the
  summary reads `N passed, N failed, N skipped, N known-fail`. `skills-novice` is
  the one legitimately permanent skip — the Novice tree is passive apart from
  quest-gated actives — which is why a skip must **not** fail the exit code.
- **The `BD_ETERNALCHAOS` / `BD_ROKISWEIL` allowlist entries are gone**; their
  stated reason was wrong (map-zone disabled, not ensemble).
- **Hercules' per-IP anti-flood was causing every "random disconnect" cluster.**
  `ip_rules` is now disabled in the Hercules tree — see CLAUDE.md §3b. **The
  diagnostic:** connection-flavoured failures *plus* `Packet coverage: … 0 failed`
  means environment, never protocol. Count `there is no char-server online`.
- **Ammunition accumulated until the character was overweight**, at which point
  `@item` fails and every item-dependent scenario breaks at once, far from the
  cause. `observer-ammo-disguise` now cleans up after itself. The existing
  backlog was purged by hand via SQL and **does not replay on a fresh database**.
- **Traps now prove they were placed** (`AddSkillUnit`), 12 assertions per run
  where there were previously zero.

**Suite hardening is COMPLETE (2026-07-31).** Four items, all validated by a
clean full run on an unseen seed:

1. **`./dev.sh snapshot` / `restore`** (Hercules tree) — rolls the database back
   after a run, so accumulation cannot build up again. Refuses while any
   character is online, and verifies the dump's completion trailer.
2. **`weapon-refine-success` uses real commands** — `@stat` is not a Hercules
   command, so two thirds of its documented flakiness fix never ran.
3. **`normalize()` at scenario start** — full HP/SP plus removal of stacking
   items, at the *start* so a failing scenario cannot skip it. It must **not**
   flush: the server delivers real state at login (`QuestList`), and flushing ate
   it.
4. **Unit-creating skills prove their unit** — 38 `ground-unit` assertions
   against zero before. `UNIT_CREATING_SKILLS` is a snapshot of `skill_db`'s
   `Unit:` blocks; **regenerate it when skill_db changes** (snippet in the const's
   doc comment).

**The standing inventory lives in
[plans/work-backlog.md](plans/work-backlog.md)** (assembled 2026-08-02) — live
verification debt, features that exist only as discarded data, suite depth, and
the server deltas a merge would silently drop. It also carries the four greps
that regenerate it, because a day of fixes came out of lists nobody was reading.

**Still open:**

- **`MG_FROSTDIVER` is a known intermittent** in `skills-super-novice`.
- **THE GUI PASS — STARTED 2026-07-31, one row closed.** Boundary 5 now has its
  first evidence. The queue, with setup, traps and a known-unrendered list (so
  phase-4 gaps are not logged as bugs), is
  **[plans/gui-verification-pass.md](plans/gui-verification-pass.md)**.
  - **Row 1 `0x0189` PASS** — `DC_UGLYDANCE` as a Dancer in Prontera printed
    *"This skill cannot be used in this area."* on screen. That closes the one
    genuinely new item from 31 July; the other three `info_type` values differ
    only by a string literal in the same `match`, so the packet, the handler and
    the chat path are all proven.
  - **Row 11's probe was invalid and has been corrected** — `AC_CONCENTRATION`
    has no opt1/opt2 state, so it has no entity visual for *anyone* and the row
    could neither pass nor fail. Use a Sage field (`SA_VOLCANO` 285 /
    `SA_DELUGE` 286 / `SA_VIOLENTGALE` 287). Full reasoning in both plan docs.
    **Generalise this: when substituting a skill into a verification row, check
    the substitute still carries the property the row tests.**
  - Rows 10, 3, 4, 4b, 5 and 6 remain open. Row 10 is cheapest — both seats are
    already provisioned and it needs no gear change.

- **Two client-launch facts worth not re-deriving.** The release binary goes
  stale silently: on 31 July it predated the `0x0189` fix, so the row would have
  reported a false FAIL against a build that never contained the code. **Check
  the binary's mtime against the commit you mean to test, and rebuild first.**
  Seat A is `cd korangar/korangar && ../target/release/korangar`; seat B is
  `cd client2 && <abs path>/target/release/korangar` (its `client/` uses absolute
  GRF paths and logs in as `headless2`).

- **`KORANGAR_PACKET_LOG=1` is not a full dump** — it prints only `impact`,
  `damage`, `add-entity`, `local equipped`, `look change` and unknown/failed
  incoming bytes. It logs **nothing for status changes or skill casts**, so its
  silence is not evidence that a cast did not happen. Confirm state against the
  `char`/`skill` tables instead.
- **Do not invest further in the suite.** Remaining ideas (parallel instances,
  per-scenario characters) buy speed and isolation, not trust, and would trade a
  known-good serial suite for new failure modes.

**Trap: never run a second tester instance while the suite is running either.**
All scenarios share one character *and* one partner character, so a concurrent
run corrupts both. Two headless instances in parallel would need two more GM
accounts plus their own characters, and the partner character name is currently
hardcoded (`context.rs`, `CHARACTER_NAME`), which is the actual blocker — RO
character names are globally unique.

**Trap that cost a whole 40-minute run:** do **not** run any `cargo` command
while the suite is executing. `cargo run` rebuilds
`target/release/examples/headless-tester` underneath the running process and
produces a cascade of bogus "disconnected" failures (14 of them), with a
perfectly healthy server. Invoke the built binary directly instead, as above.

**Still open, cheapest first, all needing the stack:** run `--scenario observer`
above; observer rows 10-11 (skill effect and status values from the far seat —
no setup needed, `test` has the Archer skills); confirm the A9 stance bug (two
seats in a town, one armed — code-read only so far); then the 2026-07-26 batch
leftovers below.

**Phase 3 should NOT be built as originally specced** — phase 1 changed its
premise. See the harness plan §6.

---

## The 2026-07-26 batch

Seven commits landed 2026-07-26 on `agent/platform-connectivity-controls`
(`3fc2c020` … `6b883a5f`) plus one Hercules commit (`ea036fac8` on
`agent/map-teleport-safety`). Both repos pushed, working trees clean.
**Nothing in this batch has been seen on screen or heard.**

## Bring the stack up (in this order)

```sh
brew services run mariadb                        # `run`, NEVER `start` — `start` re-registers autostart
cd Hercules && ./dev.sh start && ./dev.sh wait   # several minutes; loads 1156 maps
cd korangar/korangar && cargo run --release --bin korangar
```

`dev.sh` (added 2026-07-28) wraps `athena-start` and `make`; see
[MACOS_WORKFLOW.md](MACOS_WORKFLOW.md). Use `./dev.sh build` for any server-source
change — it fails loudly when `map-server` was not actually relinked, which is the
half of the `make map` trap a build log cannot show you.

Verified 2026-07-26 that the stack boots clean **with the new Hercules delta**:
`Successfully 'connected' to Database 'ragnarok'`, `Successfully loaded '1156' maps`,
map-server listening on 5121, char-server handshake OK. The map-server binary was
rebuilt after the delta, so **no `make` is needed** — but a server restart is.

Server stdout goes to **`Hercules/log/server-latest.log`** — a fresh file per run
now, so the whole file is this run. `log/map.log` stays empty. The older
`log/athena-start.out` was *appended* to across runs, which is why stale shutdown
errors from previous boots kept getting misread as live failures; it is untracked
leftover noise, so leave it uncommitted and ignore it.

## The checklist, cheapest first

Results as of the 2026-07-26 evening pass are in the **Result** column.

| # | What | How to see it | Watch for | Result |
|---|---|---|---|---|
| 1 | **Item names in messages** | Cast Land Protector with no Yellow Gemstone | "You need a Yellow Gemstone to use this skill.", never `#715`. Also check a trade-window row and a weapon-refine result — all three go through `resolve_item_name`. | **PASS** (skill-fail path; trade/refine rows not yet seen) |
| 2 | **Ammo-item projectiles** | Bow attack with plain Arrow, then Iron/Fire Arrow | The flying sprite should *change with the arrow type*. Firearms (views 17-21) and huuma shuriken (22) now fire too. Grenade launcher falls back to Bullet **by design**. | **PASS** 2026-07-29 — Fire Arrow, Iron Arrow, Bullet and Silver Bullet all resolve distinctly, on the shooter *and* on an observer. Grenade launcher's Bullet fallback is a **known deviation**, not by design: `battle_check_arrows` requires `A_GRENADE` for `W_GRENADE`, but no grenade projectile sprite was found in the GRF survey. |
| 3 | **Ground-cast walk-into-range** | Arm a ground skill, click a cell well outside its range | Should walk into range then cast, instead of nothing happening. If nothing walkable is close enough, expect a chat line rather than silence. | **PASS** — walked into range, then cast |
| 4 | **Support walk-into-range** | Heal/Blessing an ally ~15 cells away (Heal range is 9) | Same walk-then-cast. **This path changed behaviour**, so give it real attention — self-buffs must still fire instantly (self is distance 0). **Wants two seats:** the client path is entity-agnostic (`resolve_pending_cast`, `lib.rs:905`, treats `Attack`/`Support` alike), so a mob exercises the walk — but a mob closes the distance you are trying to measure. Neither test char has Heal; `@jobchange 8` + `@allskill`. | open |
| 5 | **Cast cancel** | Start a long cast, press **right-click**; repeat with **Escape** | Cast bar clears and the skill does NOT go off. Also: right-click with a skill *armed* still clears the reticle first; right-click on nothing still rotates the camera; Escape on nothing still opens the menu. **Moving must NOT cancel** — casting roots, and that is authentic. | **PASS** — both gestures cancel |
| 6 | **Ground-skill aiming footprint** | Arm Storm Gust (81 cells), then Land Protector Lv10 (225) | The real question: does a large area read as a *shape* or a solid slab? Colour/alpha are guesses (`IN_RANGE` / `OUT_OF_RANGE` in `render_skill_aiming_footprint`). Out-of-range should tint red. | **PARTIAL** — draws, and the red out-of-range tint works. The 225-cell shape-vs-slab question is still unanswered. |
| 7 | **Moonlit / Hermode** | **Two seats, Clown + Gypsy** — granting the skills is *not* enough, see below | Moonlit = flat salmon tile per cell, 9×9. **Hermode is sound-only by design** — hearing `헤르모드의 지팡이.wav` and seeing nothing is a **PASS**, not a bug. | open |

### Row 7 is an ensemble skill — it cannot be cast solo

Both `CG_MOONLIT` and `CG_HERMODE` are `Ensemble: true` (`db/re/skill_db.conf`),
`player_skill_partner_check: true` (`conf/map/battle/skill.conf:231`), and the
Admin group has **`skill_unconditional: false`** (`conf/groups.conf:308`) — so GM
99 does *not* bypass the check. `skill_check_condition_char_sub`
(`src/map/skill.c:15400`) requires a partner who is **all** of: opposite sex ·
job masks to `MAPID_BARDDANCER` · **knows the same skill** · wields an instrument
(`W_MUSICAL`) or whip (`W_WHIP`) · **in the same party** · not already dancing ·
not sitting. This is a renewal server, so the partner must independently pass the
skill's own cast requirements too.

Setup, once both seats are in: `@jobchange 4020` (Clown, seat A — male) and
`@jobchange 4021` (Gypsy, seat B — female), `@allskill` on both, an instrument
and a whip, and a party. Do this **last** — it replaces the bow gear the earlier
rows need.

### Found while driving the pass: `@item` could not take a multi-word name

`@item Iron Arrow 500` silently produced **one Iron** (id 998). Unquoted, `@item`
parsed `%99s %12d`, took `Iron` as the name, failed to read `Arrow` as the
quantity — and still returned ≥ 1, so it reported success. Only the *quoted*
form ever supported spaces.

Fixed server-side in `src/map/atcommand.c` (`atcommand_item_search` +
`atcommand_item_parse`, shared by `@item`, `@itembound`, `@item2`,
`@itembound2`). **The trap:** you cannot just peel the trailing integer off,
because **1797 items have a display name ending in a digit** (`Vesper Core 01`,
`Magic Bible Vol1`, `Vita500`). The parser therefore resolves *longest name
first* — it tries the whole argument string as a name, then peels one trailing
integer at a time until something resolves. Quoted names, bare IDs and
single-word names behave exactly as before.

**Longest-first has a trap of its own**, and it bit the first attempt: the ID
lookup was `itemdb->exists(atoi(name))`, and `atoi("1770 500")` is `1770`. So
the whole string resolved as an ID, the quantity was never peeled, and
`@item 1770 500` would have quietly handed over **one** item — a regression on
the most common DM usage. An ID is now only accepted when the string is numeric
end to end (`strtol` + endptr check). The compiler was happy either way; a
14-case throwaway C harness against a stub item table found it in one run.

Display-name lookup was already case-insensitive, so `iron arrow` works. Aegis
names (`Iron_Arrow`) stay case-sensitive because `case_sensitive_aegisnames:
true` in `conf/map/battle/misc.conf` — config left alone. For finding a name,
`@ii <partial>` already searched multi-word text correctly.

## Why item 7's alpha matters beyond item 7

Moonlit's tile is at **α 0.6**. The recovered roBrowser table for the whole
song/Gospel/Fog Wall family uses **α 0.05**, calibrated to *roBrowser's* renderer.
Korangar's ground-decal pass composites differently, and the batch-1 lesson was that
additive alphas needed ~0.5–0.8 against lit terrain. **Moonlit is the calibration
sample for that entire family** — note how it reads before anyone ports the rest.

## What comes after the live pass

Blocked on one engine decision, not on data: `UnitBody::GroundQuad` is a single flat
quad, but the authentic ground-tile recipe is *two* layers — a tinted tile plus a
texture bobbing between +0.2 and +0.6 cell with a per-cell phase offset. Adding a
`UnitBody` variant for that unlocks **`PA_GOSPEL` (`0xb3`)**, **`PF_FOGWALL`
(`0xb6`)** and **`NPC_EVILLAND` (`0xc7`)** immediately, and all 16 songs for free if
the campaign ever moves to pre-renewal. Full colour/texture table is in
[plans/classic-effect-fidelity.md](plans/classic-effect-fidelity.md).

## Test-environment traps — read before driving

- macOS F-keys and Home-to-sit: see [MACOS_WORKFLOW.md](MACOS_WORKFLOW.md).
- Hercules reports **several unrelated failures as "Skill level is not high enough"**
  (cause 0 is overloaded). Don't chase it as a client bug.
- Elemental fields silently refuse to spawn on top of *anything*, including the
  caster (`UF_NOFOOTSET`) — aim bare ground 4–5 cells away.
- The `test` character (150000) may be a Priest; `@jobchange 9` restores Wizard with
  the E1 hotbar intact.
- **Do not run `rustfmt` in this repo.** The committed tree is not rustfmt-clean, and
  pointing it at `lib.rs` (the crate root) rewrites 20+ unrelated files.
