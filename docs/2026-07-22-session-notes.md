# 2026-07-22 / 2026-07-23 — E1 live close, M1-017, Soul Strike sprites

Branch: `agent/platform-connectivity-controls`  
Pushed: `ec206219` (M1-017 + Soul Strike travel + E1/M1 plan docs).  
Uncommitted after that: filename-guess rollback for F1/F3–F7 + further plan docs
(this file + classic-effect updates).

## 1. Phase E1 live verification — CLOSED

Walked all 7 Wizard skills on char `test` (F1–F7) against Barricade on
`prt_fild07`. **Mechanism pass** (spawn place/time/texture), not classic fidelity.

| F# | Skill | Result |
|---|---|---|
| F1 | Napalm Beat | PASS mechanism |
| F2 | Soul Strike | PASS mechanism (later upgraded to classic sprites) |
| F3 | Frost Diver | PASS mechanism |
| F4 | Fire Ball | PASS mechanism |
| F5 | Jupitel | PASS mechanism |
| F6 | Earth Spike | PASS mechanism |
| F7 | Heaven's Drive | PASS mechanism |

Details: [plans/phase-e1-live-verification.md](plans/phase-e1-live-verification.md).

### GUI driving traps (for next session)

- Synthetic F-keys via AppleScript are eaten by macOS; use a real CGEvent helper
  or user keyboard. Non-F keys work through cliclick.
- Barricade is **not** magic-immune (Def 0 / Mdef 0); missed casts were hotbar
  F-keys never firing.

## 2. M1-017 logout crash — CLOSED live

**Symptom:** Menu → Log out with Skill Tree open killed the client
(`State::get` unwrap / `ManuallyAsserted`).

**Experiments (user-driven):**

| Open at logout | Result |
|---|---|
| Stats only | Clean → character select |
| Skill Tree only | Crash every time |

**Root causes (two layers):**

1. `NetworkEvent::LoggedOut` called `skill_tree().clear()` **while Skill Tree
   was still open** — `tabs().index(n).skills().manually_asserted()` then
   panicked on the next layout (MapServerDisconnected may close windows a frame
   later).
2. Skill Tree also stored `this_player().manually_asserted().skill_points()`;
   after `entities().clear()`, optional player is `None`.

**Fix:**

- Close all windows on `LoggedOut` **before** clearing skill_tree / hotbar.
- Optional paths for skill points and tab skills (`Path<…, false>`, `try_get`,
  empty layout if missing).
- Files: `lib.rs`, `interface/windows/skill_tree/{mod,tabs,slot,state}.rs`.

**Live:** Skill Tree open → Log out → character select. Status in
[plans/M1-p0-verification.md](plans/M1-p0-verification.md).

## 3. Classic sprite path — Soul Strike live-OK

Backend (`SpriteEffects`, `EffectAsset::Sprite`, `spawn_resolved_effect`) was
already on the branch (`44927a32`).

### Iteration

| Attempt | Behavior | User feedback |
|---|---|---|
| Hit-only `soule` at impact | Cast anim → ~1s empty → blast on target | Wrong — felt like delayed explosion |
| Travel `soule` caster→target | Ghosts fly along cast path | **OK — parked** |

### Implementation

- `ProjectileRecipe::SpriteTravel { path, multi_hit }`
- `SpriteEffects::spawn_travel` (lerp + stagger for multi-hit)
- Skill 13: `path = "이팩트\\soule"`, `multi_hit = true`
- No separate hit sheet for Soul Strike (arrival is the hit)

## 4. Filename-guess mapping for F1/F3–F7 — FAILED live, reverted

Tried GRF root-name matches for the remaining E1 skills. User: all six **need
work**. Sheets were mostly **other skills'** effects:

| Guess | Why wrong |
|---|---|
| `블래스트` | Gunslinger-style blast, not Napalm |
| `프리징스피어` / `라이트닝스피어` | Other spear effects |
| `페트롤로지` | Genetic petrology |
| `어스퀘이크` | Unrelated quake sheet |
| `fireball.spr` as travel | Name match; still did not read right live |

**Reverted** F1 / F3–F7 to E1 procedural / STR recipes. Only Soul Strike keeps
`SpriteTravel`.

**Lesson:** do **not** map classic skills by `이팩트\*.spr` filename. Need
reverse-engineered effect IDs or side-by-side with the official client.

Current presentation table:
[plans/classic-effect-fidelity.md](plans/classic-effect-fidelity.md).

## 5. iRO Wiki visual brief (for next polish)

Reference: [Wizard (iRO Wiki)](https://irowiki.org/wiki/Wizard) + skill pages
(Napalm Beat, Fire Ball, Frost Diver, Jupitel Thunder, Earth Spike, Heaven's Drive).

| Skill | Wiki visual intent |
|---|---|
| Napalm Beat | Psychokinetic hit on target; Ghost property; AoE damage split |
| Frost Diver | Stream of frigid ice → target; freeze chance |
| Fire Ball | Fireball travel; splash AoE at impact (edge 75%) |
| Jupitel | Crackling lightning **ball**; multi-hit (animation multi, damage often one bundle) + knockback |
| Earth Spike | Ground under **one** target rises into spikes; multi-hit by level |
| Heaven's Drive | Ground rises in **5×5** area; multi-hit by level |
| Soul Strike | Ghost orbs travel — **done** |

Wiki note (Earth Spike / Heaven's Drive / Jupitel): *despite the animation, all
damage is connected in one bundle* — multi-hit is mostly visual.

Use this as acceptance language when retuning procedural geometry; do not treat
wiki as a GRF asset table (it has none).

## 6. Code / API notes for agents

| Piece | Location |
|---|---|
| Sprite effect holder | `korangar/src/world/sprite_effect.rs` |
| Recipes | `korangar/src/world/skill_recipe.rs` (`classic_sprites::SOULE`, `SpriteTravel`) |
| Spawn travel | `Client::add_sprite_travel_projectile` in `lib.rs` |
| Fixed sprite + STR dispatch | `Client::spawn_resolved_effect` |
| Debug log | `KORANGAR_SPRITE_EFFECT_DEBUG=1` → `[sprite-effect] travel|spawn|loaded` |
| Test character | `test` / char_id 150000, Wizard, F1–F7 hotbar server-side |

## 7. Commits this window

| SHA | Summary |
|---|---|
| `ec206219` | fix: close M1-017 logout crash and fly Soul Strike sprites (docs + code) |

Post-commit working tree (if not yet committed): rollback of bad F1/F3–F7 sprite
maps to procedural, plan pointer updates, this session note.

## 8. NEXT AGENT

1. **Optional:** commit remaining docs + recipe rollback if still dirty.
2. **Classic fidelity:** reverse-engineer skill→effect (official client / RE),
   or polish procedural F1/F3–F7 against the wiki brief in §5 — **no more
   filename guesses**.
3. **Phase E2** (persistent skill units) is unblocked whenever wanted.
4. GUI leftovers: M1-009 vs-equipped confirm, M1-014 delete UX, bow arrow
   eyeball, hotbar “skill level not high enough” anomaly.

Start: [plans/classic-effect-fidelity.md](plans/classic-effect-fidelity.md) and
[plans/animation-fidelity.md](plans/animation-fidelity.md) `NEXT AGENT` field.
