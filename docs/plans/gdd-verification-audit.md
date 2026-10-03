# GDD verification audit

**Started 2026-10-03.** Owner request: check every GDD v0.2 task against what was claimed,
because reviews on 2026-10-03 kept finding "Built" rows that were partial, unwired, or broken.
This plan governs that audit; [gdd-improvement-plan.md](gdd-improvement-plan.md) (F01–F39),
[gdd-next-slices.md](gdd-next-slices.md) (slices 1–15, S1–S10) and GDD Appendix E are the claims
being checked. Update a row's verdict here, then correct the claim at its source.

## Why: what the 2026-10-03 reviews found

| Pattern | Example | Caught by |
|---|---|---|
| Claim names code that does not exist | F23 `show_chat_timestamps` toggle, `<GUIDE:id>` links | grep for cited symbols |
| Built, never wired to a player | F30 crafting formulas, F31 commission assign/complete, F16 recap fields | clippy dead-code |
| Data silently defeats the code | Pilot `ClearSkills` removed every MVP's minion summons | reading the data the code loads |
| A fix changes what tests observe | ViewData elite (class 1152) broke two elite scenarios and `@metrics` counts | running the scenarios |
| Work dropped between branches | 36 commits on closed PR #10, incl. 2026-09-18 live-confirmed fixes | `git cherry` + symbol check |
| CI never ran the real thing | Hercules builds skipped for weeks; client built only beside Hercules | reading CI logs |

The common cause: "Built" was written from the code author's side. Nothing checked that a player
can reach the feature, that the shipped data lets it work, or that a test would fail if it broke.

## Method: five checks per task

Each task gets a verdict from the strongest evidence reached. Use the CLAUDE.md evidence words.

1. **Claim inventory.** List every concrete claim in the row: symbols, files, numbers, behaviors.
2. **Source and wiring.** Each cited symbol exists, and something in the *production* path calls
   it: a window, menu entry, key binding, packet handler, NPC, or atcommand. Test-only callers do
   not count.
3. **Data and config.** The data the code needs is loaded in this server's configuration, and
   nothing loaded after it overrides it (ClearSkills, import configs, ViewData, map flags).
4. **Test meaning.** A test exists that would fail if the behavior broke (mutation check: break
   it, see red). Run it. Headless for server behavior; unit tests for client logic.
5. **Runtime.** Server behavior: a headless scenario against the dev server. Client UI: a line in
   the live GUI checklist ([gui-verification-pass.md](gui-verification-pass.md)), run with the
   owner. Nothing is "Verified" on source reading alone.

**Verdicts:** `Verified` (checks 1–5 pass) · `Verified (unseen)` (1–4 pass, GUI not yet watched)
· `Partial` (named gaps) · `Claim wrong` (the row overstates; correct it) · `Broken` (fix first).

## Tooling to build first (Batch 0)

- **Claim checker** `tools/audits/gdd_claims.py`: parse F01–F39 and S rows, extract every
  backticked identifier, report missing symbols and symbols with only test callers. One pass
  triages all rows and replaces guesswork about where to look.
- **Dead-code map:** list every `allow(dead_code)` / `cfg_attr(not(test), …)` and map it to its
  F row (the 2026-10-03 clippy pass marked F30, F31, F16 this way).
- **Scenario index:** which headless scenarios cover which rows, and which are absent from CI's
  paired list (the elite scenarios were never in CI).
- **Safe all-scripts load:** the CI "extratest" mirror must use a throwaway database; the
  2026-10-03 run against `herc_re_db` left orphan barter rows and inert `$` variables.

## Order: player impact first

| Batch | Scope | Why first | Main checks |
|---|---|---|---|
| 0 | Tooling above; decide PR #10 (port or record as dropped) | Triage for everything else | Claim checker run |
| 1 | Server gameplay: F14, F15, F16, F39, S3, S4, S8, S10, F21, F34 | Changes what players fight and earn; silent failures | Data/config, headless runtime |
| 2 | Combat and input: F09, F10, F11, F12, F13, slices 2, 3, 10, 13 | Every fight uses it | Wiring, unit tests, GUI checklist |
| 3 | Party and social: F20, F22, F23, F24, F25, slices 11, 12, 14 | Multiplayer paths, hard to test alone | Two-client headless, GUI checklist |
| 4 | Economy: F26–F31, F27 refine, trade (F28), slice 7 | Item loss/duplication risk | Server handlers, SQL paths, headless trade |
| 5 | Guide and data: F01–F08, F37, slices 5, 6, 8, 9 | Large but display-only | Claim checker, exporter `--check`, Guide tests |
| 6 | Polish and ops: F32, F33, F35, F36, F38, slices 1, 4, 15 | Lowest risk | Wiring, GUI checklist |

Each batch ends with: verdicts recorded below, claims corrected in the plan and Appendix E, fixes
filed as their own slices (not mixed into the audit), and the batch's headless scenarios added to
CI where missing.

## Already known going in

| Row | Finding | Status |
|---|---|---|
| F16 | Pilot MVPs never summoned escorts (`ClearSkills`) | Fixed 2026-10-03 (Hercules `35964619a`, scenario `mob-eddga-summons-escorts`) |
| F15 | ViewData elite broke two elite scenarios and `@metrics spawns` | Fixed 2026-10-03 (korangar `6ecd527b`, Hercules `35964619a`) |
| F14 | Aggressor flag lost on mode recalculation (latent) | Fixed 2026-10-03 |
| S8/F39 | Online hand-in could take equipped copies | Fixed 2026-10-03 |
| F23 | No chat timestamp toggle; no `<GUIDE:id>` links | Claim wrong — open |
| F30 | Formulas not shown in any window | Partial — open |
| F31 | No assign/complete action in the board window | Partial — open |
| F16 | Recap tracks damage taken/interrupts; toast shows only damage dealt | Partial — open |
| PR #10 | Live-confirmed 2026-09-18 fixes never reached `main` | Decision needed |

## Verdict log

| Row | Verdict | Evidence | Gaps / fixes filed | Date |
|---|---|---|---|---|
