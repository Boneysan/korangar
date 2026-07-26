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
brew services run mariadb              # `run`, NEVER `start` — `start` re-registers autostart
cd Hercules && ./athena-start start    # several minutes; loads 1156 maps
cd korangar/korangar && cargo run --release --bin korangar
```

Verified 2026-07-26 that the stack boots clean **with the new Hercules delta**:
`Successfully 'connected' to Database 'ragnarok'`, `Successfully loaded '1156' maps`,
map-server listening on 5121, char-server handshake OK. The map-server binary was
rebuilt after the delta, so **no `make` is needed** — but a server restart is.

Server stdout goes to **`Hercules/log/athena-start.out`** (appended, so stale
shutdown errors linger — read the *end*, not a `head`). `log/map.log` stays empty.
That file is untracked noise; leave it uncommitted.

## The checklist, cheapest first

| # | What | How to see it | Watch for |
|---|---|---|---|
| 1 | **Item names in messages** | Cast Land Protector with no Yellow Gemstone | "You need a Yellow Gemstone to use this skill.", never `#715`. Also check a trade-window row and a weapon-refine result — all three go through `resolve_item_name`. |
| 2 | **Ammo-item projectiles** | Bow attack with plain Arrow, then Iron/Fire Arrow | The flying sprite should *change with the arrow type*. Firearms (views 17-21) and huuma shuriken (22) now fire too. Grenade launcher falls back to Bullet **by design**. |
| 3 | **Ground-cast walk-into-range** | Arm a ground skill, click a cell well outside its range | Should walk into range then cast, instead of nothing happening. If nothing walkable is close enough, expect a chat line rather than silence. |
| 4 | **Support walk-into-range** | Heal/Blessing an ally ~15 cells away (Heal range is 9) | Same walk-then-cast. **This path changed behaviour**, so give it real attention — self-buffs must still fire instantly (self is distance 0). |
| 5 | **Cast cancel** | Start a long cast, press **right-click**; repeat with **Escape** | Cast bar clears and the skill does NOT go off. Also: right-click with a skill *armed* still clears the reticle first; right-click on nothing still rotates the camera; Escape on nothing still opens the menu. **Moving must NOT cancel** — casting roots, and that is authentic. |
| 6 | **Ground-skill aiming footprint** | Arm Storm Gust (81 cells), then Land Protector Lv10 (225) | The real question: does a large area read as a *shape* or a solid slab? Colour/alpha are guesses (`IN_RANGE` / `OUT_OF_RANGE` in `render_skill_aiming_footprint`). Out-of-range should tint red. |
| 7 | **Moonlit / Hermode** | Clown/Gypsy, or grant the skills | Moonlit = flat salmon tile per cell, 9×9. **Hermode is sound-only by design** — hearing `헤르모드의 지팡이.wav` and seeing nothing is a **PASS**, not a bug. |

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
