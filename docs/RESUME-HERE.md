# Resume here — live pass of the 2026-07-26 batch

Everything below is **code + tests green, pushed, and NOT live-verified**. The next
session is a **GUI live pass**, not new feature work.

(The previous occupant of this file was the E3.1 GUI handoff, resolved 2026-07-22;
its history lives in [2026-07-16-session-notes.md](2026-07-16-session-notes.md).)

## One-line status

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
| 2 | **Ammo-item projectiles** | Bow attack with plain Arrow, then Iron/Fire Arrow | The flying sprite should *change with the arrow type*. Firearms (views 17-21) and huuma shuriken (22) now fire too. Grenade launcher falls back to Bullet **by design**. | open — unblocked: `@item Iron Arrow 500` and `@item Iron 20` both confirmed working after the fix below |
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
