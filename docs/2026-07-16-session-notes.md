# Session notes — 2026-07-15/16 (E3.1 GUI pass)

The M1 P0 checklist driven by hand against a live server, start to finish. It
took the pass from 17 verified rows to **32 of 34**, fixed **6 client bugs**,
and filed **6 more** with root causes.

Every fix had the same shape: **the packets were already correct and
headless-green; the client's handling of them was wrong.** That is the blind
spot the 106-scenario suite has by construction, and this session turned it from
an argument into six data points.

## Bugs fixed (all live-verified the same session)

| ID | Symptom | Root cause |
|---|---|---|
| M1-005 | Dialogue box read as empty | `window!` for Dialog declared no size floor, so it collapsed — **and the collapsed size persisted to `window_cache.ron`**, so it reopened broken every session |
| M1-007 | No hide/cloak visuals at all | `StateChangePacket` (`0x0229`) was `register_noop`; `OPTION_HIDE` never reached the UI. Knock-on: hide-gated skills (`RG_RAID`) looked broken with no way to diagnose |
| M1-010 | Buff bar showed `10:218s` | No index → name mapping. Now resolves from Hercules' own `db/constants.conf` (699 names) |
| M1-011 | "Skill Delay" stuck forever | `apply()` treated `duration_ms == 0` as *infinite*. Zero means **already over**; Hercules signals permanent with `INFINITE_DURATION = -1` → `u32::MAX`, already handled |
| M1-012 | Cancelled buffs never cleared | Statuses **start** on `0x0983` but **end** on `0x0196` — and `0x0196` was `register_noop` |
| M1-013 | Character list empty at 3 characters | Hercules sends a second, **empty** `0x0B72` at exactly 3 chars as an end-of-pagination marker; `set_characters` cleared all slots before re-adding, so the terminator wiped the list |

## Bugs filed (open)

| ID | Pri | Note |
|---|---|---|
| M1-006 | P0 | No skill-targeting mode — hover-then-press is the only way to aim. Needs a design call (cancel semantics), not just code |
| M1-008 | P1 | Skills play no animation. `DisplaySpecialEffectPacket` is noop; playback works, but `EffectId` has **1124** variants vs the 10 `VisualEffect` hand-maps — needs a data table, mind the No-Upstream-IP rule |
| M1-009 | P1 | No gear stats or comparison. **Data is already in the binary** (`items.json` has Atk/Def/Slots/EquipLv; `DmItem` embeds it) — but it lives under `src/dm/`, which rule 4 isolates for rebaseability |
| M1-010 | P1 | Kept open for the **icon** half (needs artwork); names are done |
| M1-014 | P2 | Character delete is right-click-only *and* unconfirmed — fixing discoverability alone would make accidental deletion easier |
| M1-015 | P2 | Stuck at server select after a failed login. **Observed once, not reproduced** — root cause is a lead, not a finding |

## Why headless missed all of it

Worth internalising, because it is structural rather than an oversight:

- **Noop packets emit no `NetworkEvent`**, so there is nothing for a scenario to
  assert on. M1-007 and M1-012 were both invisible for this reason while every
  skill sweep passed green.
- **No scenario asserts on buff-bar contents** — M1-010/M1-011 sat in plain sight.
- **Fixtures clean up after themselves.** `character-create-delete` never holds 3
  characters at once, so M1-013 could not occur. It needed an account that had
  *accumulated* characters — what a person does and a fixture doesn't.
- **Fresh context per scenario.** `bad-password` never carries a prior successful
  login into a failure, which is exactly the state M1-015 needs.

## Two bugs stacked on one feature

M1-011 was **masking** M1-012. While every status was wrongly treated as
infinite, nothing expired, so a missing end packet made no observable
difference. Fixing expiry is what exposed it. Only sequential live testing
surfaces that ordering — a one-shot audit would have found the first and
declared victory.

## Test-environment lessons

- **Do not hand a tester a polluted character.** Aborting the headless suite
  mid-run left `test` as a level-99 Rogue with a hotbar full of skills from
  other jobs (Blacksmith, Acolyte, Whitesmith). Every key did nothing —
  correctly — and cost real time chasing a bug that did not exist.
- **macOS eats F1–F9** by default (`com.apple.keyboard.fnState` unset). Use
  **Fn+F1**, or enable standard function keys. Not a client bug.
- **Insert does not sit on this Mac** — laptop keyboards lack the key. **Home**
  is the verified path.
- **`@item` cannot make unidentified items.** `Hercules/npc/custom/identify_test.txt`
  adds an **Identify Test NPC at `prontera,164,200`** using `getitem2`.
  Deliberately separate from the Dialogue Test NPC — headless `dialogue-choice`
  asserts that NPC's menu exactly and picks by index.
- **Stock shops are `trader` NPCs, not `shop`.** Searching for `shop` finds only
  `npc/custom/itemmall.txt`, which is commented out. Sell was verified at the
  Payon Weapon Dealer (`payon_in01,15,119`).

## Docs corrected

The pass kept surfacing stale claims, each of which had already misled someone:

- `FEATURE_ROADMAP.md` §8.3 listed `StatusChangeSequencePacket` as pending
  *after* it was promoted, and `DisplaySkillCooldownPacket` / `UseSkillSuccessPacket`
  as "still noop" when both were already handled.
- The Phase 1 note claimed buff bar and weight footer had no coverage; both are
  now hand-verified.
- `M1-p0-verification.md` §4 told you to write an `input` test NPC that already
  existed, and its environment block had WSL-only paths for a macOS pass.

## Still open

- **Clean logout** — the last row needing a human.
- **Reset the `test` character** (still a Rogue with a hand-written hotbar,
  `skill_point = 0`) and **re-run all 106 headless scenarios**: three packets
  were promoted in shared crates today (`0x0229`, `0x0196`, plus the status-name
  table), so the suite needs to prove no regression.
