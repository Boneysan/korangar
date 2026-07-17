# Resume here — E3.1 GUI pass (paused 2026-07-16)

Quick-start for picking this back up. Full detail is in
[2026-07-16-session-notes.md](2026-07-16-session-notes.md).

## One-line status

**RESOLVED 2026-07-16.** M1 P0 checklist at **33/34 verified** (only
"rejection messages" left, arguably already met); 7 client bugs fixed, 7 filed.
Both gating items cleared: `skills-monk` retested 3/3 green (flake confirmed),
clean logout live-verified. Both repos pushed to origin. History below is kept
for context.

## The two things left before pushing

1. **Confirm `skills-monk`.** The final full-suite re-run (2026-07-16, after the
   quest_db fix) came back **105/106** — `dm-quest-lifecycle` now passes, but a
   *different* scenario failed: `skills-monk` (one of the 39 job sweeps). A
   different failure each run is the flake signature, and skill sweeps have known
   transient issues (monster congestion, Kaizel, observation windows). **It was
   NOT retested — that's the next action.** Run:
   ```sh
   cd korangar
   for i in 1 2 3; do cargo run --release --example headless-tester -p korangar-networking -- --scenario skills-monk 2>&1 | grep -aE "Summary:"; done
   ```
   - If it passes on retest → flake, gate is effectively green, proceed to push.
   - If it fails consistently → real issue. Check whether today's `0x0229` /
     `0x0196` status changes interact with Monk's spirit-sphere / Explosion
     Spirits status effects. Isolate the same way the quest bug was: does the
     failing subsystem touch the diff?

2. **Clean logout** — the one remaining checklist row that needs a human. Log out
   of the GUI properly and confirm no hang/stuck state. Then E3.1's exit bar is
   met (every P0 row ✅ or a filed defect).

## Then push

Two repos, both **committed but unpushed**:

- **korangar** (`agent/platform-connectivity-controls`): **6 commits** ahead of
  origin — `a85767bc` (M1-005/007), `49aaa933` (buff names + M1-011), `ee195fcd`
  (M1-012), `2e94441c` (M1-013), `8d33abc0` (GUI-pass docs), `c6cb2680`
  (regression-run docs).
- **Hercules** (`agent/map-teleport-safety`): **2 commits** ahead —
  `aa40e2053` (quest_db `//` syntax fix), `5569764ba` (Identify Test NPC).

Push both to `origin` (the fork; do NOT push to `upstream`).

## Bugs fixed this pass (all live-verified, committed)

| ID | What |
|---|---|
| M1-005 | Dialogue box collapsed + persisted collapsed to `window_cache.ron` |
| M1-007 | `StateChangePacket` `0x0229` noop → no hide/cloak visuals |
| M1-010 | Buff bar showed raw indices → 699 English names from `db/constants.conf` |
| M1-011 | `duration_ms == 0` treated as infinite → statuses stuck forever |
| M1-012 | `StatusChangeSequencePacket` `0x0196` (status *end*) noop → cancelled buffs never cleared |
| M1-013 | Empty terminator `0x0B72` wiped the char list at exactly 3 characters |
| (Hercules) | quest_db `//`-comment syntax error broke all campaign quests |

## Bugs filed, still open (in `plans/M1-p0-verification.md` §5)

| ID | Pri | What |
|---|---|---|
| M1-006 | P0 | No skill-targeting mode — **needs a design call** (see below) |
| M1-008 | P1 | Skills play no animation (`DisplaySpecialEffectPacket` noop; 1124-entry EffectId table needed) |
| M1-009 | P1 | No gear stats/comparison (data is in the binary; blocked on `src/dm/` rebaseability) |
| M1-010 | P1 | Kept open for the **icon** half (names done; icons need artwork) |
| M1-014 | P2 | Char delete is right-click-only *and* unconfirmed |
| M1-015 | P2 | Stuck at server select after failed login — **observed once, not reproduced** |

## Decision blocking M1-006

Before building the skill-targeting mode, need the user's call:
- Cancel a pending target with **right-click**, **Escape**, or both?
- Pressing a **second** skill while one is pending — swap the target, or cancel?
- Clicking **empty ground** — cancel, or fizzle?

## Environment notes (or you'll lose time)

- **MariaDB does not autostart** — `brew services run mariadb` (NOT `start`).
- Bring the server up: `cd Hercules && ./athena-start start` (login 6900, char
  6121, map 5121, api 7121). **The live map-server log is
  `log/athena-start.out`, NOT `log/run-map.out`** (the latter is stale from
  Jul 12 and cost time this session).
- The `test` character (account `korangar`, slot 0) was rebuilt into a clean
  **Knight** (99/50, full Knight skill tree, coherent F1–F9 hotbar, a Spear +
  gear + 3 unidentified items in inventory, 30 stat / 10 skill points unspent,
  5M zeny). Equip the Spear in-game to enable the melee skills. This does **not**
  affect the suite — it reshapes the job per scenario.
- **The GUI and the headless suite share the `korangar` account** — close the
  client before running the suite, or they kick each other.
- More traps (macOS F-keys need Fn, Home-not-Insert to sit, `trader` not `shop`
  NPCs) are in the memory file `ro-test-environment-traps.md`.
