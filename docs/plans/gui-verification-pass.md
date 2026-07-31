# GUI verification pass — everything that has never been on screen

| | |
|---|---|
| **Status** | **NOT STARTED.** Written 2026-07-31 as a queue for a later session |
| **Branch** | `agent/platform-connectivity-controls` (korangar), `agent/map-teleport-safety` (Hercules) |
| **Needs** | The graphical client. Some rows need two seats — both characters already exist, see §Two seats |
| **Blocks** | Nothing. Everything here is verification of work already shipped |

## Why this exists

Two full sessions of work — the 26 July feature batch and the 29–31 July suite
programme — have shipped **without the graphical client being launched once**.
Everything is wire-verified: the correct bytes arrive and become the correct
`NetworkEvent`s. Whether the client *draws* any of it is unknown.

That is not a small gap. State crosses five boundaries between one player's
action and another player's screen:

| # | Boundary | Confidence |
|---|---|---|
| 1 | Actor's client → server | Good |
| 2 | Server-side state (`sd->vd`, units, status) | Good |
| 3 | Server → observer (broadcast / spawn / enter-view) | Good |
| 4 | Observer's wire → `NetworkEvent` | Good — 38 unit assertions, 0 packet failures |
| 5 | **Event → pixel** | **None** |

This document is boundary 5. The project's own rule states it plainly:
headless-green means the wire data is correct and says nothing about the UI
layer, so *do not report a headless pass as "verified working in the client"*.

## Stack bring-up

```sh
brew services run mariadb                        # `run`, NEVER `start`
cd Hercules && ./dev.sh start && ./dev.sh wait
cd korangar/korangar && cargo run --release --bin korangar
```

**Snapshot the database first if the suite will also be run** —
`./dev.sh snapshot`, and `./dev.sh restore` afterwards. Verify the stack is
actually down when stopping (`pgrep -f 'map-server|char-server|login-server'`,
`lsof -nP -iTCP:5121 -sTCP:LISTEN`); `dev.sh stop` has reported success while
leaving `map-server` alive.

## Two seats

Both characters already exist — **no provisioning is needed**, and an earlier
note claiming a new account was required was wrong (sex is per-character here,
not per-account):

| Character | Account | Sex | Level | GM |
|---|---|---|---|---|
| `test` | `korangar` / `korangar` | M | 99 | 99 |
| `HeadlessTwo` | `headless2` / `headless2pw` | **F** | 99 | 99 |

Separate accounts, so both can be logged in simultaneously.

**Their job and gear drift — set them explicitly, never assume.** The suite
leaves whatever the last scenario used. At the time of writing `test` was a
**Taekwon (4046)** and `HeadlessTwo` a **Dancer (20)**, neither of which is what
any row below wants. Every row states its own `@jobchange`; run it even if you
think the job is already right. An earlier note in this repo asserted "`test`
has the Archer skills" — true when written, false a day later.

---

## The queue, cheapest first

### 1. `0x0189` map-zone refusal message — the only new item from 31 July

| | |
|---|---|
| **How** | On any non-PvP map, as a Dancer or Gypsy, cast `DC_UGLYDANCE`. Also try `@warp` into a no-teleport area, or `/memo` where saving is disallowed |
| **Watch for** | A red chat line: *"This skill cannot be used in this area."* Before the fix this printed **nothing at all** — the skill simply did nothing |
| **Why it matters** | Four distinct messages ride this packet (teleport / save point / skill / item). A pass on one is decent evidence for all four, since they share one handler |
| **Result** | |

### 2. Observer rows 10-11 — skill effect and status values from the far seat

| | |
|---|---|
| **Setup** | Two seats, and **`@jobchange 3` + `@allskill` on the acting seat** — these are Archer-line skills and the shared character will not be an Archer. Ids verified against `docs/skills.json`: `AC_CONCENTRATION` **45**, `AC_DOUBLE` **46**, `AC_SHOWER` **47** |
| **Watch for** | The *observer* sees the effect and the status icon/values — not the caster |
| **Note** | The last two open rows of [observer-view-verification.md](observer-view-verification.md) |
| **Result** | |

### 3. Support walk-into-range — **give this real attention**

| | |
|---|---|
| **How** | Heal or Blessing an ally ~15 cells away (Heal range is 9) |
| **Watch for** | Walks into range, then casts. **Self-buffs must still fire instantly** (self is distance 0) |
| **Why** | This path *changed behaviour* in the 26 July batch. Wants two seats: the client path is entity-agnostic, so a mob exercises the walk — but a mob closes the distance you are trying to measure |
| **Setup** | Neither test char has Heal; `@jobchange 8` + `@allskill` |
| **Result** | |

### 4. Ground-skill aiming footprint — shape or slab?

| | |
|---|---|
| **How** | Arm Storm Gust (81 cells), then Land Protector Lv10 (**225 cells**) |
| **Watch for** | Does a large area read as a *shape*, or as a solid slab? Out-of-range should tint red |
| **Geometry** | Verified against `skill_db` 2026-07-31: Storm Gust `Layout: 4` → 9×9 = **81**; Land Protector Lv10 splash **7** → 15×15 = **225** |
| **Status** | **PARTIAL** — it draws and the red tint works. The 225-cell question is the open one |
| **Result** | |

### 4b. Cast circles — never looked at, expect a rebuild

| | |
|---|---|
| **How** | Cast anything with a cast bar and watch the ground at the caster's feet. `Lockon` plus six `Beginspell` recipes exist in the recipe tables |
| **Watch for** | Whether they read as the original client's cast circles at all |
| **Expectation** | These are **procedural placeholders over generic ring textures**. CLAUDE.md's own note says to expect a rebuild, not a tick — so a "fail" here is the expected outcome and the useful output is a description of what is wrong |
| **Result** | |

### 5. Moonlit / Hermode — blocked longest, and the alpha calibration sample

| | |
|---|---|
| **How** | Two seats, **Clown + Gypsy**, partied. `@jobchange 4020` (Clown, `test`) and `@jobchange 4021` (Gypsy, `HeadlessTwo`), `@allskill` both, an instrument and a whip, same party |
| **Watch for** | Moonlit = flat salmon tile per cell, 9×9. **Hermode is sound-only by design** — hearing `헤르모드의 지팡이.wav` and seeing nothing is a **PASS** |
| **Why it gates other work** | Moonlit's tile is at **α 0.6**; the recovered roBrowser table for the whole song/Gospel/Fog-Wall family uses **α 0.05**, calibrated to a different renderer. **Moonlit is the calibration sample for that entire family** — note how it reads before anyone ports the rest |
| **Do this last** | It replaces the bow gear rows 2-3 need |
| **Ensemble rules** | Both skills are `Ensemble: true` and GM 99 does **not** bypass the partner check. The partner must be opposite sex, job-mask to `MAPID_BARDDANCER`, know the same skill, wield an instrument or whip, be in the same party, not already dancing, not sitting |
| **Result** | |

### 6. Rest of the 26 July batch — shipped, never seen

Rows 1, 2, 3 and 5 of the old checklist already PASSed
([RESUME-HERE.md](../RESUME-HERE.md)). Remaining: item names in trade-window and
weapon-refine rows (the skill-fail path passed; these two share
`resolve_item_name`, including its `NOTFOUND` sentinel filter).

---

## Known-unrendered — do not report these as bugs

Verified *stored* but deliberately **not drawn** yet, so a GUI pass will see
nothing and should not log a finding:

- **Headgear, robe, clothes/hair colour, body style** — `EntityData` carries all
  seven appearance fields since phase 1, but sprite composition is still
  body + head + weapon + shield. Rendering them is **phase 4**, blocked on
  accessory sprite paths and palette files this tree does not have.
- **Facing changes and stop-move** now produce real events instead of no-ops.
  Whether they *render* is untested — worth a look, but a miss here is a phase-4
  gap, not a regression.
- **Grenade launcher projectile** falls back to Bullet: no grenade sprite was
  found in the GRF survey. A known deviation.

## Traps for whoever drives this

- macOS F-keys and Home-to-sit: see [../MACOS_WORKFLOW.md](../MACOS_WORKFLOW.md).
- Hercules reports **several unrelated failures as "Skill level is not high
  enough"** — cause 0 is overloaded. Do not chase it as a client bug.
- Elemental fields refuse to spawn on top of *anything*, including the caster
  (`UF_NOFOOTSET`) — aim bare ground 4–5 cells away.
- The `test` character may not be the job you expect; `@jobchange 9` restores
  Wizard with the E1 hotbar intact.
- **Do not run `rustfmt` in this repo.** The committed tree is not
  rustfmt-clean, and pointing it at `lib.rs` rewrites 20+ unrelated files.
- The **hotbar is server-side** (Hercules hotkey table), so it cannot be edited
  from the client — change it offline.
