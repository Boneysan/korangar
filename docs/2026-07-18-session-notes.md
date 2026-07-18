# 2026-07-18 — Phase C closeout + Phase D code (weapon visuals)

## Phase C (closed live earlier this day)

Runtime layer composition (C1–C5): multi-layer compose, attach, shield draw
order, partial swaps, shield layer. Documented and committed as
`cef7fff4`.

## Phase D code (this session)

Weapon visual completeness on branch `agent/platform-connectivity-controls`:

1. **Item→view + Assassin L/R** — inventory keeps nameids;
   `effective_weapon_view` combines dual-wield to views 25..=30.
2. **Per-item sprites** — probe `{job}_{sex}_{itemId}` then class suffix.
3. **`_검광` trails** — native allowlist from Ragexe `0x976EC0` (views 1–7,
   16–18, 25–30); per-item trail probe retained; `_발광` not wired.
4. Multi weapon-family layer swap; audit tool lists per-item + trails.

Native RE on `../../RO/client/2019-06-05fRagexe_patched.exe` (SHA matches
docs): path builders `0x7C4F90` / `0x7C4B30`.

**Tests:** `cargo test -p korangar --lib` → 178 passed; ignored GRF weapon
roster probe green (includes Mjolnir 1530).

## NEXT (for Claude / Codex / next session)

**Do not start Phase E until live D is signed off.**

→ **[docs/plans/phase-d-live-verification.md](plans/phase-d-live-verification.md)**

In-game: Knight sword/spear/trail, Mjolnir 1530, sword+Guard, SinX dual
daggers, bow/mace no class trail. Record pass/fail with date+observer in that
file and in `animation-fidelity.md` §5.
