# GUI verification pass — everything that has never been on screen

| | |
|---|---|
| **Status** | **IN PROGRESS.** Written 2026-07-31; first rows walked the same day — **row 1 PASS**, row 11's probe corrected as invalid, rows 10/3/4/4b/5/6 open |
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
| **Result** | **PASS** 2026-07-31 — `DC_UGLYDANCE` as a Dancer in Prontera printed *"This skill cannot be used in this area."* Covers `info_type` 2; the other three types differ only by a string literal in the same `match`, so the packet + handler + chat path are all proven |

**Two order facts worth keeping**, both checked while setting this row up:

- **A whip is not needed to see the message.** `DC_UGLYDANCE` requires
  `WeaponTypes: { Whips: true }` and the seat was holding a bow, but it is a
  `Self:` skill, so `unit_skilluse_id2` runs `status->check_skilluse`
  (`unit.c:1540`, the map-zone check) *before*
  `skill->check_condition_castbegin` (`unit.c:1601`, the weapon check). Seeing
  *"You can't use this skill with that weapon"* instead would mean the zone
  check was skipped — a finding, not a setup error.
- **GM 99 does not bypass it.** The enforcement lives in
  `status_check_skilluse_mapzone` (`src/map/status.c`), which has no permission
  check at all — unlike `skillnotok`, which honours `PC_PERM_SKILL_UNCONDITIONAL`.
  It sends `clif->skill_mapinfomessage(sd, 2)` on any `PACKETVER >= 20080311`.
- `@useskill <id> <lv> self` is a valid trigger — it routes through
  `unit->skilluse_id` with no bypass (`pc->autocast_clear` first, so the
  `AUTOCAST_ITEM` escape in `skillnotok` cannot apply).

### 2. Observer rows 10-11 — skill effect and status values from the far seat

| | |
|---|---|
| **Setup** | Two seats, and **`@jobchange 3` + `@allskill` on the acting seat** — these are Archer-line skills and the shared character will not be an Archer. Ids verified against `docs/skills.json`: `AC_CONCENTRATION` **45**, `AC_DOUBLE` **46**, `AC_SHOWER` **47** |
| **Watch for** | The *observer* sees the effect and the status icon/values — not the caster |
| **Note** | The last two open rows of [observer-view-verification.md](observer-view-verification.md) |
| **Result** | **Row 10 PASS and row 11 PASS**, 2026-08-02. `test` as a Sage (`@jobchange 16` + `@allskill`) on prt_fild08: Fire Bolt's effect reached `HeadlessTwo` identically (row 10), and `SA_VOLCANO` clicked on bare ground was visible from the observer seat (row 11). The 2026-07-31 row-11 attempt used an INVALID probe, see below, and was never a result either way. **This closes the observer-view checklist** — but row 10 exposed a new gap it was not testing for: see *The observer never sees a cast* below. |

**One job change covers four rows.** `@jobchange 16` + `@allskill` gives Fire Bolt
(row 10), the three Sage fields (row 11), Land Protector Lv10 = the 225-cell case
(row 4), Fire Wall (the direction-dependent wall) and a cast bar (row 4b). Sage
does *not* have Storm Gust — that needs `@jobchange 9`.

**Row 11 cannot be fired with `@useskill`.** For a ground skill `@useskill` casts
at the *target player's cell* (`atcommand.c:5960`, `unit->skilluse_pos(bl,
pl_sd->bl.x, pl_sd->bl.y, ...)`), and the Sage fields carry `UF_NOFOOTSET`, so
both `self` and a named partner aim at an occupied cell. It must be armed and
clicked on bare ground.

**Use a field map, not a town.** Row 1's PASS *was* a town map-zone rejection, so
the same zone rules could refuse a probe in Prontera and it would read as a
client bug. `prt_fild08` + `@killmonster`, with the observer brought over by
`@jumpto test`.

#### Row 11: do NOT use `AC_CONCENTRATION` — corrected 2026-07-31

This row's substitution of `AC_CONCENTRATION` for the original *"Sage field"*
**cannot test what the row is for**, and produced a false negative (observer saw
nothing, which is correct behaviour) before the substitution was checked.

`SC_CONCENTRATION` (`db/re/sc_config.conf:156`) is:

```
Flags:     { Buff: true }
CalcFlags: { Agi: true, Dex: true }
Icon:      "SI_CONCENTRATION"
```

**No `Opt1`/`Opt2`/`Opt3` flag.** Korangar attaches an entity visual purely from
opt1/opt2 — `status_effect_asset` (`korangar/src/world/entity/mod.rs:93`) maps
only `OPT1_STUN`→`stun.str`, `OPT1_SLEEP`→`sleep.str`, `OPT2_POISON`/
`OPT2_DEADLY_POISON`→`poison.str`, `OPT2_SILENCE`→`silence.str`. So Improve
Concentration has **no entity visual for anybody** — not the observer, not even
the caster. Its only representation is a buff-bar icon, and the buff bar is
self-only.

The packet *does* reach the observer, so this is not a broadcast gap:
`clif_status_change_sub` (`src/map/clif.c`) ends in
`clif->send(..., bl, (sd && sd->status.option&OPTION_INVISIBLE) ? SELF : AREA)`
— **`AREA`**, i.e. every observer gets it. The value simply has nothing to draw.

**Use a status that has both values and a visual** — the original *"Sage field"*
note meant `SA_VOLCANO` **285** / `SA_DELUGE` **286** / `SA_VIOLENTGALE` **287**
(`@jobchange 16` + `@allskill`). Those are exactly the three the Hercules delta
patched `status_get_val_flag()` for so `val1`/`val2` render instead of "+0"
(CLAUDE.md §3b), and they draw as ground units every seat can see. Aim **bare
ground 4-5 cells away** — they carry `UF_NOFOOTSET` and refuse to spawn on top
of anything, including the caster.

**Lesson, general:** before substituting a skill into a verification row, check
that it still carries the property the row tests. A status with no opt-state has
no visual, so "observer sees nothing" is unfalsifiable — the row can neither
pass nor fail.

### 2b. Party window and party-member bars — **built and live-verified 2026-08-02**

Not on the original queue; it came out of row 10. Verified on screen: the party
window's controls, and **HP, SP and cast bars over party members**.

| | |
|---|---|
| **Result** | **PASS.** Cast bar, SP and HP all visible over a party member from the far seat, no hover needed |

What it took, and why the two halves were not equal:

- **The cast bar was client-only.** The observer's state was already correct —
  see the method note in [observer-view-verification.md](observer-view-verification.md).
  Remote players are `Entity::Npc`, whose `render_status` takes no `client_tick`,
  so the bar was unreachable; `render_ally_status` now takes HP/SP/cast and draws
  each independently, skipping any whose data is unknown.
- **SP needed a server delta.** Main-branch Hercules never reports a party
  member's SP — only Zero got the wide 22-byte `ZC_NOTIFY_HP_TO_GROUPM` (0x0BAB).
  Widened the guard rather than inventing a packet, because **0x0bab is already
  `packetLen(0x0bab, 22)`** in Hercules' main table *and* the client's generated
  `lengths_20220406.rs`. Full note, including the `case SP_SP:` trigger that is
  easy to miss and the battleground guard that must **not** be widened, is in
  korangar `CLAUDE.md` §3b.
- **The window** gained a status line (party-less / invite received, with the
  inviting party's name / invite sent and awaiting an answer / member count) and
  buttons for every `/party` command, each disabling itself with a
  `disabled_tooltip` when it cannot apply.

**Party membership survives logout** — confirmed in passing, and it is *correct*.
Parties live in the `party` table server-side, so rejoining is not needed; a
character keeps its party across sessions exactly as in official RO.

### 2c. Social windows — the whole family has one GUI row between them

**Why this block exists.** The Social section of
[M1-p0-verification.md](M1-p0-verification.md) contains exactly one row —
*"Public chat send + receive"*. Whisper, friends and party were never listed
anywhere, so every one of those windows shipped without being looked at. That is
the same gap that hid the observer cast bar for two sessions: the wire layer was
green the whole time (`friend-lifecycle`, `friend-reject`, and five party
scenarios all pass), and green wire says nothing about whether a window draws.

**Data is not the problem here.** Unlike party SP, every field these rows need
already reaches the client. What is untested is `event → pixel`, and in
particular *re-render on change*, which is precisely where the cast bar failed:
state was correct and simply never reached a renderer.

Two seats. `test` (Alt+H friend list, Alt+Z party) and `HeadlessTwo`.

#### Party — what today's session did and did not prove

Verified live 2026-08-02 (see §2b): Create, Invite, Accept, the roster filling
in, HP/SP/cast bars, and membership surviving a logout. That is the happy path
only. Still unverified:

| # | Check | Watch for | Result |
|---|---|---|---|
| P1 | **Reject** button on the invited seat | Inviter gets the rejection line; the invited seat's status line returns to "Not in a party" | ☐ |
| P2 | **Leave** button | Roster empties on *both* seats, status line resets, bars over the ex-member disappear | ☐ |
| P3 | Status line during an outgoing invite | Inviter shows *"Invited X; waiting for an answer…"*, and it **clears** when the answer lands — either answer | ☐ |
| P4 | Disabled states | Create greys out once in a party, Invite until in one, Accept/Reject with no invite, Leave when party-less — each with its tooltip | ☐ |
| P5 | A member going **offline** | Roster line flips to `(offline)`; bars over them stop drawing | ☐ |
| P6 | Party chat | Sent from one seat, shown in the other's chat window | ☐ |

#### Friends — nothing has ever been on screen

| # | Check | Watch for | Result |
|---|---|---|---|
| F1 | Add by name from the friend list text box | `FriendRequestWindow` **pops automatically** on the other seat (`lib.rs:4797`) | ☐ |
| F2 | Accept | Both lists gain the friend with **no relog** | ☐ |
| F3 | Reject | Request window closes; requester gets the rejection line | ☐ |
| F4 | Remove | Per-friend Remove button empties the row on both sides | ☐ |
| F5 | **Online glyph flips live** | Friend logs out → `○`, back in → `●`, without reopening the window | ☐ |
| F6 | List survives relog | Friends still listed after logging out and back in | ☐ |

**F5 and F6 are the two to watch.** `FriendEntry::set_online` rewrites
`display_label` and is wired to `FriendOnlineStatus` (`lib.rs:4817`) with a unit
test for the glyph itself — so if the glyph does not flip on screen, the state is
right and the **re-render** is missing, exactly like the cast bar. F6 depends on
`SetFriendList` at login (`lib.rs:5056`), which arrives *during connect*; that
same timing is why `party-persists-relog` had to assert functionally rather than
wait for a packet.

### 2d. Everything built 2026-08-02 — rows, grouped by setup

**Why grouped:** the pass has stalled before on setup churn. Each block below is
one configuration; walk a whole block before changing jobs or gear. Blocks are
ordered so nothing later undoes something earlier — **E replaces the gear, so it
is last**.

#### Block A — two seats, no gear needed *(most of the day's work is here)*

Seat A `test`, seat B `HeadlessTwo`, adjacent on a field map. Setup:

```
A: @warp prt_fild08 320 185      B: @jumpto test
A: @killmonster
```

| # | Check | Watch for | Result |
|---|---|---|---|
| N1 | **Left-click seat B's character** | Target frame opens: name, class ("High Wizard", not an id), and Whisper / Invite / Trade / Add friend / Ignore | ☐ |
| N2 | Left-click **your own** sprite | **Nothing opens** — self-targeting is excluded deliberately | ☐ |
| N3 | Chat: press **Party**, type a line | Goes to party chat, and the line is a **different colour** from system text | ☐ |
| N4 | Chat: press **Whisper**, fill the target field, send | Arrives privately, in its own colour | ☐ |
| N5 | Whisper with the target field **empty** | Prints the `/w` usage hint. **Must not** appear in public chat — that would leak a private message | ☐ |
| N6 | Type `/party leave` while the channel is **Party** | Runs as a *command*, not sent as party chat | ☐ |
| N7 | Seat B whispers A, then A presses **Reply** | Channel switches to Whisper with B pre-filled. Receiving a whisper must **not** switch the channel on its own | ☐ |
| N8 | A invites B to a party | B's popup says **"test invites you to join …"** — the name comes from fork packet `0x0EFF`; a bare party name means the Hercules delta is missing | ☐ |
| N9 | Invite popup **Whisper** button | Aims chat at the inviter | ☐ |
| N10 | Party window as **leader**: Kick / Promote | Enabled; kicking removes the row on both seats, promoting moves the ★ | ☐ |
| N11 | Party window as **non-leader** | Kick / Promote greyed with "Only the party leader can do this" | ☐ |
| N12 | Share EXP / pickup / loot toggles | Flip and **stay** flipped — they reflect the server's answer, not the click | ☐ |
| N13 | Roster rows | Show **class name** and, when a member dies (`@die` on B), read `DEAD` | ☐ |
| N14 | A requests a trade with B | B's popup names **A and their level**, and offers Whisper | ☐ |
| N15 | In the trade, **right-click an inventory item → Add to trade** | The item appears on **B's** side of the window. This is the path the 0x0B42 fix restored | ☐ |
| N16 | Trade zeny field → **Add zeny** | Amount appears in the offer; non-numeric input is ignored, not an error | ☐ |
| N17 | A sends B a friend request | B's popup offers **Whisper** alongside Accept / Reject | ☐ |
| N18 | Friend list rows | Whisper / Invite / Trade / Remove; **online friends sort first** | ☐ |
| N19 | B logs out, then back in | Friend glyph flips `●`→`○`→`●` **without reopening the window**, and the list survives the relog | ☐ |

#### Block B — Sage / Wizard seat

`test` is already a Sage with `@allskill`. Setup for the Ice Wall row: `@jobchange 9` + `@allskill`.

| # | Check | Watch for | Result |
|---|---|---|---|
| N20 | Cast **Auto Spell** (`SA_AUTOSPELL`) | A window lists the offered spells **by name**; picking one closes it and the server accepts | ☐ |
| N21 | As a Wizard, cast **Ice Wall**, then try to walk through it | The client **refuses to path through** it, and the cells free up when it expires. **No headless test can cover this** — the suite does not link the `korangar` crate | ☐ |
| N22 | Row 4 — Land Protector Lv10 armed | 225 cells: shape or solid slab? Then Fire Wall facing different directions | ☐ |
| N23 | Row 4b — cast circle | **Expected FAIL**, root-caused: nothing triggers one. Confirm rather than investigate | ☐ |

#### Block C — instance and NPC refine

| # | Check | Watch for | Result |
|---|---|---|---|
| N24 | In a party, `@dminstance start prontera 156 191` | Instance window opens naming it; `@dminstance end` closes it | ☐ |
| N25 | Refine equipment at a **blacksmith NPC** | Result line names the item and the new level. This is `0x0188`, a *different* path from the Weapon Refine skill | ☐ |

#### Block D — Priest

`@jobchange 8` + `@allskill`.

| # | Check | Watch for | Result |
|---|---|---|---|
| N26 | Row 3 — Heal an ally ~15 cells away | Walks into range then casts; **self-buffs still fire instantly** | ☐ |

#### Block E — last, replaces gear

Row 5 (Moonlit / Hermode, Clown + Gypsy) — see §5.

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
| **Result** | **FAIL as expected, but for a sharper reason than "the placeholder looks wrong": nothing triggers a cast circle at all.** 2026-08-02, both seats, Sage Fire Bolt. The **cast bar draws correctly** on the caster — so `SkillCast` arrives and is handled — and the ground at the caster's feet stays empty on both clients |

#### Why there is no circle — the trigger does not exist

Two independent halves are missing, and the recipes are *not* the problem:

- **Cast start spawns no visual.** `NetworkEvent::SkillCast` → `entity.start_cast()`
  (`korangar/src/lib.rs:5126`), and `start_cast` (`world/entity/mod.rs:1384`) only
  arms an `ActorCast { ends_at, total_ms }` for the cast **bar**. Its `_skill_id`
  is underscore-prefixed — the skill is deliberately unused, so nothing could key
  a per-element circle off it even if a circle existed.
- **The `Beginspell` / `Lockon` recipes are unreachable from a cast.**
  `special_effect.rs:210-225` maps them to real `Burst` recipes, but they are
  driven by `EffectId`s **the map server never sends while casting**: grepping
  `EF_BEGINSPELL|EF_LOCKON` over Hercules `src/` hits `db/constants.conf` and
  nothing else. The only emitters in the whole tree are quest scripts calling
  `specialeffect` (`bard_quest.txt`, `okolnir.txt`, `eye_of_hellion.txt`, …), so
  those recipes do get used — just never for a cast circle.

**In the original client the cast circle is a client-side visual**, drawn from the
cast-start packet and coloured by the skill's element. Building it here is a new
client-side feature hung off `start_cast` (which already carries the skill id and
the duration), *not* a fidelity fix to the existing recipes. Cost is real but
bounded; the element→circle mapping should come from roBrowserLegacy's tables per
the effect-fidelity rule, never guessed.

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
