# Resume here — E3.1 GUI pass (paused 2026-07-16)

Quick-start for picking this back up. Full detail is in
[2026-07-16-session-notes.md](2026-07-16-session-notes.md).

## One-line status

**RESOLVED 2026-07-22.** M1 P0 checklist at **34/34 verified** — rejection
messages closed via unit fixtures + headless `skill-fail-rejection` (Fire Bolt
at 0 SP → chat). Earlier: 7 client bugs fixed; remaining open §5 defects are
P1/P2 polish (see below). Phase D live-green 2026-07-21; **next engine work is
Phase E** (skill/status recipes). History below is kept for context.

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

Reconciled 2026-07-22 against the live §5 table:

| ID | Pri | Status | What |
|---|---|---|---|
| M1-006 | P0 | ✅ Fixed 2026-07-16 | Skill-targeting mode (`PendingSkill`) live-verified |
| M1-008 | P1 | 🟡 Partial | Wizard kit effects live-verified 2026-07-17; catalog-wide coverage + sounds remain (Phase E adjacent) |
| M1-009 | P1 | 🔴 Open | No gear stats/comparison (data in binary; extract shared source outside `src/dm/`) |
| M1-010 | P1 | 🟡 Names done | Icons still need artwork |
| M1-014 | P2 | 🔴 Open | Char delete is right-click-only *and* unconfirmed |
| M1-015 | P2 | 🔴 Open | Stuck at server select after failed login — observed once, not reproduced |

**Also fixed during the GUI sitting (not open):** M1-005 dialog sizing, M1-007 hide/cloak, M1-011 zero-duration stick, M1-012 status-end packet, M1-013 3-char list wipe, M1-016 emotes.

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
