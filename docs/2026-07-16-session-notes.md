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

## Regression run + a caught server bug

Re-ran all 106 scenarios after the fixes — the first full run since the 07-13
gate. One failure: `dm-quest-lifecycle`, timing out on `QuestAdded`.

**It was not a client regression.** The three promoted packets are unrelated to
quests, and no quest file changed in the client commits. The diagnostic path:

1. `QuestAdded` comes from `QuestNotificationPacket1/4` — untouched today.
2. `git diff` over the day's commits: zero quest files.
3. The DM command *executed and reported success* ("Quest 20001 started for 1
   party member(s)") but the quest never landed in the char's table → the fault
   is server-side quest-add, not client packet handling.
4. The map-server log: `db/quest_db.conf:16164 - syntax error`. Quest_db failed
   to load, so `setquest()` silently no-oped for every campaign quest.

Root cause: Hercules `d28ffb666` (post-gate off-by-one MobId fixes) injected a
`//` comment into a **single-line** quest entry —
`Targets: ( { MobId: 1366 // Lava Golem /* … */ Count: 20 }, )`. `//` runs to
end of line, eating `Count: 20 }` and the closing `)`; libconfig then reported
the error at the next structure it reached (line 16164, which looked innocent).
Fixed in Hercules `aa40e2053`; server now loads all 3172 quest entries and the
scenario passes.

**Lesson worth keeping:** a single failure in the first run after a gap is not
automatically a flake *or* a regression from the current change. Ask first
whether the failing subsystem is even touched by the diff. Here it plainly was
not, which pointed straight at the intervening server commits rather than
today's client work.

## Still open

- **Clean logout** — the last checklist row needing a human.
- The `test` character was rebuilt into a clean, well-equipped **Knight** for
  future manual passes (see the character-rebuild note below); it does not affect
  the suite, which reshapes the job per scenario.

---

# M1-006 skill-targeting — built and live-verified (later 2026-07-16)

After the E3.1 pass closed and both repos were pushed, this follow-on session
built the **skill-targeting mode** filed as M1-006, live-verified it, and filed
two new findings. Uncommitted at time of writing; see the M1-006 row in
`plans/M1-p0-verification.md` for the canonical detail.

## What it does

Pressing a targeted hotbar skill with **no valid target under the cursor** now
*arms* it: the cursor becomes the attack reticle and the next left-click picks
the target. Per skill type:

- **Attack** — hover-instant fast path if already over a target, else arm.
- **Ground/Trap** — **always** arm, so you aim and place the AoE with the click.
- **Support** — hovered entity, else self (kept the original fallback).
- **SelfCast** — unchanged instant self-cast.

Cancel = right-click (primary) or Escape (Escape clears the target *before* it
opens the menu). Pressing another skill re-arms (swap); a chat line names the
armed skill. Core decision logic is `resolve_pending_cast`, a pure function with
6 unit tests; the armed state clears on map disconnect so it can't leak across a
relogin. All in `korangar/src/lib.rs` (rule-4 note: this is core input, not DM).

## The design conventions (user-approved)

Cancel via right-click **and** Escape; a second skill **swaps**; empty ground
**fizzles** for entity-targeted (stays armed, never walks) and **casts at the
cell** for ground-targeted. These are the genre-standard RO conventions.

## What live testing taught — three rounds, three real lessons

The feature "worked" in code immediately; the value was in what live iteration
surfaced that headless never could:

1. **Support arming broke self-cast.** First cut armed Support skills too. But you
   can't reliably click your *own* sprite (the picker returns the tile under your
   feet), so Heal became uncastable. Reverted Support to hovered-entity-else-self.
   Lesson: self-target support skills must not require a click-target.

2. **Ground instant-cast gave no way to aim.** The hybrid "instant-cast if over a
   valid target" is wrong for ground skills — the cursor is *always* over a valid
   tile, so it dropped the AoE at wherever the cursor sat with no aiming step. This
   read to the tester as "Storm Gust isn't selectable." Fix: ground skills *always*
   arm.

3. **The big one — test-character skill pruning.** Storm Gust wouldn't cast or even
   announce. Root cause was **not** the client: Hercules' `pc_calc_skilltree`
   **prunes on login** any skill whose tree prerequisites aren't met (and any skill
   outside the job's tree entirely). Granting a curated cross-tier / cross-job skill
   list via SQL left only the first-tier, prereq-less skills (Fire Bolt, Cold Bolt)
   — everything else was silently stripped, so the client correctly showed them as
   unlearned/dimmed and refused to cast. **You must grant a complete, valid job
   tree.** The `test` char (150000) is now a Wizard with the full Mage+Wizard tree.
   This cost the most time and is the reusable trap — recorded in the
   `ro-test-environment-traps` memory.

## M1-008 made all of this hard to see

Skills draw **no cast animation** (M1-008), so a correctly-sent cast looks
identical to one that did nothing — which is why "spells don't cast on ground"
looked like a targeting bug when it wasn't. Added temporary `eprintln` + a
"Casting X" chat line purely to prove the packet was sent (it was), then removed
both once confirmed. Kept only the **"Aiming X"** arm line, which is genuinely
useful (identifies the armed skill, makes swaps visible) and low-frequency.
Damage numbers already confirm offensive casts.

## New findings filed

- **M1-016 (P2)** — emotes render only as a chat line, no animated emoticon over
  the entity. Machinery exists (`DisplayEmotion` → `EmoteBubbles::show` + lazy
  sprite-sheet load); the sheet likely never resolves. See §5.
- **Cast interruption (feature request, not a bug)** — the tester repeatedly
  wanted right-click/Esc to abort an *in-progress* cast. RO cancels a cast by
  *moving*; there is no cancel packet. This is a separate feature from targeting
  (M1-006's cancel only clears an *armed*, pre-cast skill) — build it on its own.
