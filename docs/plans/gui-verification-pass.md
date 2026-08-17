# GUI verification pass — everything that has never been on screen

## Open-only (read this first — 2026-08-12)

**Source of truth for live GUI work.** Headless suite acceptance is closed
([headless-next-steps.md](../../tools/testing/headless-next-steps.md)); this file
is boundary 5 (event → pixel). Do not re-walk closed Blocks A–D unless
regression-smoking.

| Priority | Item | Notes |
|---|---|---|
| ~~1~~ | ~~**Block E — Hermode**~~ | **PASS 2026-08-16 — reached for the first time, and the block is now closed.** Sound heard on **both** seats, 49 units across the 7×7 of `Layout: 3`, all `body=none str=None`. **Three things that looked like failures were correct:** the empty status window (`SC_HERMODE` has no `Icon:` in `sc_config.conf`, so Hercules never sends the state change), the rooted caster (`SC_DANCING` blocks `unit_can_move`, ~31 s at Lv5, and Hermode is named as a case `SC_LONGING` cannot free), and the invisible field. **Why it had never been reached:** the runsheet sent people to `prt_fild08`, where the skill is refused twice over — upstream bans `CG_HERMODE` in the **Normal** zone, *and* `skill.c` demands a warp portal within 1 cell. Both gates are now fork deltas (see CLAUDE.md §3b) |
| **2** | **Block E — Moonlit confirm** | Redesign **CLOSED 2026-08-08** after red-square/light bugs; re-confirm after any skill-unit change |
| **3** | **N20 — Auto Spell window** | Still **☐** — list spells by name, pick one, server accepts |
| **4** | **N24 — Instance window** | **FAIL** mostly Hercules map-name truncation; retry short name (`izlude`) for window-only check |
| **5** | **Block A re-walk — party roster + trade rows only** | The one block still stale after the 2026-08-12 triage. `a552bc57` changed the roster when *we* leave; `a617834a` changed traded items leaving the giver's inventory. Both landed after the block closed. Two rows |
| **6** | **Gospel / Fog Wall / Evil Land ground fields** | **Never on screen** — steps in **§5b**. Added by `b6148b5c`; hover sizes and opacities are *estimates*. **Gospel is coded at α 0.05, the value §5 measured as not porting to this renderer** — walk it first, expect it invisible |
| **7** | **The four refuse/remove paths** | P1 reject a party invite · P5 a member going offline · F3 reject a friend request · F4 remove a friend. Recovered by the 2026-08-12 reconciliation below — Block A walked every *accept* path and no *refuse* path |
| **8** | **M1-009 and M1-014 live confirms** | Both shipped 2026-07-22 marked "code complete — live GUI confirm recommended" and never confirmed: gear stat tooltips with vs-equipped deltas, and the two-step character delete |
| **—** | **N23 — Cast circles** | **Expected FAIL** until feature is built (cast *bar* works). Not a live-pass grind item |
| **—** | Known-unrendered | Headgear/robe/colour, **spirit spheres**, quest UI — do **not** file as bugs; feature first |

**Closed for the live queue:** Blocks A–B (N1–N19 social), most of B–D (Ice Wall
pathing, ground footprint, support walk-into-range, NPC refine / item names),
rows 1–4 of the cheap queue, observer 10–11, Moonlit redesign (2026-08-08).

> [!WARNING]
> **"Closed" here means *verified once, against one revision*.** All seven
> closed blocks were flagged stale on 2026-08-12 by
> `tools/audits/gui-pass-staleness.py`; **six were then read and cleared, and
> one survives.**
>
> **Block A is the one still stale, and it needs two rows, not nineteen.** Of
> the eleven commits on its files, the eight dated 2026-08-04 are the pass's
> *own* fixes — this document records each as "PASS 2026-08-04 (after a fix,
> re-verified live)". Two landed on 2026-08-05, after it closed, and each
> changes a behaviour a row asserts: **`a552bc57`** (the party roster when *we*
> are the one who left) and **`a617834a`** (traded items leaving the giver's own
> inventory). **Re-walk the party-roster and trade rows.**
>
> The other six carry a `reviewed_through` with its reason in the tool. Two
> results worth knowing, because they generalise: **`d5eb977a` is rustfmt
> everywhere it appears** — checked statement by statement, since `-w` does not
> collapse a multi-line join — and **`d2682580` / `b6148b5c` are not changes
> *since* Blocks D and E were verified, they are the commits that verified
> them**, landing the same day and saying so in their own messages. A date
> anchor cannot express "same day, afterwards"; that is what `reviewed_through`
> is for.
>
> This is not a criticism of the pass; it is the one property a manual pass
> structurally lacks. Run the tool before a GUI session so the queue starts
> from what is *currently* unverified, and clear each stale block honestly:
> re-walk it, or read the commits and record why they cannot reach it.
> Do not simply carry the PASS forward.
>
> **The tool itself was under-reporting until 2026-08-12.** It anchored on
> `git log --since`, which is a traversal *cutoff* rather than a filter: git
> stops walking once it meets a commit older than the date, so anything behind
> that is dropped silently. On Block A it reported **5** commits where a full
> walk finds **11** — hiding `3226a5f2`, `57308acd` and `6a60d062`, three of the
> exact behaviours its rows assert — and the count moved between two runs on the
> same branch, which is the tell. It now walks the whole history and compares
> dates in Python.

**Improvements / bugs found during this pass** are recorded inline under each
block’s Result cells and “Findings from the … walk” sections (empty job table,
whisper sender echo, trade inventory, support skill visuals, etc.) — not a
separate changelog.

### Give every finding a mechanical guard where one is possible

Reviewing the pass's own findings (2026-08-12): **most of the bugs it caught
were state-layer bugs, not pixel bugs** — the class label reading "Adventurer"
for every job (a Lua table looked up under the wrong global, with the failure
swallowed by an `if let Ok`), the sender's own whisper never appearing, a bare
`/W` broadcasting a private message to the whole map, the party roster surviving
your own departure. None of those needed a GPU to detect. They needed somebody
to *call the function*.

Two of them already got a unit test as part of the fix — `job_name.rs` pins the
rebirth classes **and asserts the table is not empty**, which is what actually
went wrong, and `slash_command_usage` is covered. That is the pattern; it is
just not yet the habit.

| The finding was… | Guard it with | Why |
|---|---|---|
| A value computed or looked up wrongly | a unit test on that function | Cheapest, and it re-runs forever |
| A window's state machine going wrong (busy player, stale roster, wrong target) | a unit test on the `*WindowState` | `ChatWindowState` and `DialogWindowState` show the shape: `default()`, call methods, assert |
| An event that never reaches state | a headless scenario + `event-routing.py` | Already covered by the suite |
| State stored and never drawn | `unread-state.py` | Catches exactly this (it found `spirit_spheres`) |
| Draw order, colour, a missing sprite, a font glyph | **eyes** — nothing else can | This is the residue that justifies the manual pass |

The gap worth closing: `korangar/src/interface` is **66 files / ~13,000 lines
with two test modules**, and it holds fourteen `*WindowState` structs whose
methods are ordinary Rust. `korangar/src/state` is in far better shape (10 of
17 tested). Adding a test alongside each future GUI fix costs minutes and is
the only thing that stops a re-walk being the only way to know.

---

| | |
|---|---|
| **Status** | **2026-08-12 doc reconcile:** open-only table above. **2026-08-08 (evening):** Moonlit CLOSED (redesigned), Hermode STILL OPEN, six bugs fixed off-checklist. Four of the six were invisible to any automated test and two needed a second seat: Gypsy login panic, Moonlit white bloom, `@kick` no explanation, "Rejected from Server" behind char select, no numeric HP/SP, Base EXP bare at max level. Separately **passed**: `ZC_ADD_SKILL`, `ZC_SKILL_FAIL_REASON`, item reuse-delay. Field static on `prt_fild08` is RSW ambience, not a bug. **SETUP TRAP: ensemble rows BEFORE cause-0 checks** — `@jobchange 8` destroys the Gypsy seat Hermode needs. | 
| **Status (previous)** | **IN PROGRESS.** Written 2026-07-31. Observer rows closed 2026-08-02. **Blocks A and B COMPLETE 2026-08-04** — A: 19/19 and 12 bugs; B: 4/4 with N23 a root-caused FAIL. **Block C COMPLETE 2026-08-05**: N15 PASS, N19 PASS, N25 PASS (**which closes row 6**), and N24 a root-caused FAIL that is **not a client bug** — Hercules truncates instanced map names past seven characters. Four bugs fixed and live-verified that day, three of them found *off* the checklist while setting up for it. **Block D COMPLETE 2026-08-06**: N26 PASS on both halves, which closes row 3 — and it surfaced a bug it was not looking for, **eleven support skills landing with no visual at all** (fixed, live-verified). **Block E STARTED 2026-08-06 and left open** — Moonlit's 9×9 field draws and **α 0.6 is confirmed correct**, which answers the calibration question the whole song family was waiting on; its sound played 81 times (once per cell) and is fixed; the tile's borrowed texture showed *Land Protector's pattern* and the fix for that is shipped but **unverified after it drew as a solid red square**. Hermode not reached. **Open: rows 4b and 5** |
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
(row 10), the three Sage fields (row 11), Land Protector — **max level 5, 121
cells, not the "225" row 4 used to claim** — Fire Wall (the direction-dependent
wall) and a cast bar (row 4b). Sage does *not* have Storm Gust; that needs
`@jobchange 9`, which also brings Lord of Vermilion, the other 121-cell skill.

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

> [!NOTE]
> **Reconciled 2026-08-12.** These twelve rows were written on 2026-08-02 and
> then largely re-walked under different numbers as Block A's `N` rows two days
> later, but nobody carried the results back — so twelve `☐` sat here reading as
> untested work when **seven were done and one had been closed off-checklist**.
> Each row below now names the row that superseded it, or says it is still open.
>
> **The pattern in what survived is worth more than the count: every one of the
> four is a *refuse* or *remove* path.** P1 rejecting an invite, F3 rejecting a
> friend request, F4 removing a friend, P5 a member going offline. Block A walked
> accept, add and join throughout and never once walked the negative — which is
> exactly the asymmetry the headless suite had to be corrected for twice.

#### Party — what today's session did and did not prove

Verified live 2026-08-02 (see §2b): Create, Invite, Accept, the roster filling
in, HP/SP/cast bars, and membership surviving a logout. That is the happy path
only. Still unverified:

| # | Check | Watch for | Result |
|---|---|---|---|
| P1 | **Reject** button on the invited seat | Inviter gets the rejection line; the invited seat's status line returns to "Not in a party" | ☐ **STILL OPEN** — N8 proved the popup appears, never that Reject answers |
| P2 | **Leave** button | Roster empties on *both* seats, status line resets, bars over the ex-member disappear | ☐ **STILL OPEN**, and reopened by `a552bc57` — this is the Block A re-walk row |
| P3 | Status line during an outgoing invite | **CLOSED 2026-08-05** off-checklist — the line was printed off a successful *socket write*, and Hercules refuses an invite from outside a party silently (`party.c:382`). Fixed in `6ff109ac`. Original: inviter shows *"Invited X; waiting for an answer…"*, and it **clears** when the answer lands — either answer | ☐ |
| P4 | Disabled states | Create greys out once in a party, Invite until in one, Accept/Reject with no invite, Leave when party-less — each with its tooltip | **SUPERSEDED by N11** (2026-08-04) — non-leader controls greyed with their message |
| P5 | A member going **offline** | Roster line flips to `(offline)`; bars over them stop drawing | ☐ **STILL OPEN** — N13 walked `DEAD`, never offline |
| P6 | Party chat | Sent from one seat, shown in the other's chat window | **SUPERSEDED by N3** (2026-08-04) |

#### Friends — nothing has ever been on screen

| # | Check | Watch for | Result |
|---|---|---|---|
| F1 | Add by name from the friend list text box | `FriendRequestWindow` **pops automatically** on the other seat (`lib.rs:4797`) | **SUPERSEDED by N17** (2026-08-04) |
| F2 | Accept | Both lists gain the friend with **no relog** | **SUPERSEDED by N18** (2026-08-04) — its sorting finding could only have been seen by accepting one live |
| F3 | Reject | Request window closes; requester gets the rejection line | ☐ **STILL OPEN** — same gap as P1: the accept path was walked, the refuse path never |
| F4 | Remove | Per-friend Remove button empties the row on both sides | ☐ **STILL OPEN** — N18 confirmed the button *exists*, never that pressing it removes on both sides |
| F5 | **Online glyph flips live** | Friend logs out → `○`, back in → `●`, without reopening the window | **SUPERSEDED by N19** (2026-08-05) |
| F6 | List survives relog | Friends still listed after logging out and back in | **SUPERSEDED by N19** (2026-08-05) |

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
| N1 | **Left-click seat B's character** | Target frame opens: name, class ("High Wizard", not an id), and Whisper / Invite / Trade / Add friend / Ignore | **PASS 2026-08-04** (after a fix, re-verified live: reads "HeadlessTwo / Gypsy"). Originally FAILed — frame, name and all five buttons were correct but class read "Adventurer". Not "JobName missed the job" as predicted below: **the table was empty for every id in the game**, see §Findings |
| N2 | Left-click **your own** sprite | **Nothing opens** — self-targeting is excluded deliberately | **PASS 2026-08-04** |
| N3 | Chat: press **Party**, type a line | Goes to party chat, and the line is a **different colour** from system text | **PASS 2026-08-04** |
| N4 | Chat: press **Whisper**, fill the target field, send | Arrives privately, in its own colour | **PASS 2026-08-04** (after a fix, re-verified live: the sender's own line now shows). Originally FAILed — the recipient side was correct all along, but **the sender's own client showed nothing at all**, see §Findings |
| N5 | Whisper with the target field **empty** | Prints the `/w` usage hint. **Must not** appear in public chat — that would leak a private message | **PASS 2026-08-04** (after a fix). Bare `/w` prints the whisper usage; `/wisper` prints "Unknown command"; neither reaches public chat. Note the row as written never exercised the path that was actually broken — see the slash-command finding |
| N6 | Type `/party leave` while the channel is **Party** | Runs as a *command*, not sent as party chat | **PASS 2026-08-04** — ran as a command, not echoed to party chat; both seats saw the departure line |
| N7 | Seat B whispers A, then A presses **Reply** | Channel switches to Whisper with B pre-filled. Receiving a whisper must **not** switch the channel on its own | **PASS 2026-08-04** — Reply pre-fills, and receiving does not switch the channel. **UX finding raised by the tester: replying is obtuse.** Switching to the Whisper channel by hand leaves the target field blank, so the only discoverable path to answering is the Reply button. See §Whisper UX below |
| N8 | A invites B to a party | B's popup says **"test invites you to join …"** — the name comes from fork packet `0x0EFF`; a bare party name means the Hercules delta is missing | **PASS 2026-08-04** — read "test invites you to join testing", so `0x0EFF` and the Hercules delta are both live. **Wording fixed on the tester's point:** an unqualified party name reads as a verb phrase ("join testing"), now "join **the party** *testing*" |
| N9 | Invite popup **Whisper** button | Aims chat at the inviter | **PASS 2026-08-04** — puts the inviter's name into the whisper target |
| N10 | Party window as **leader**: Kick / Promote | Enabled; kicking removes the row on both seats, promoting moves the ★ | **PASS 2026-08-04** — kick removes the row; promote moves the leader marker. Note the marker is now a **gold `*`**: `★` is not in the bundled font and drew as tofu for leader *and* non-leader alike, so this row could not have been read before that fix |
| N11 | Party window as **non-leader** | Kick / Promote greyed with "Only the party leader can do this" | **PASS 2026-08-04** |
| N12 | Share EXP / pickup / loot toggles | Flip and **stay** flipped — they reflect the server's answer, not the click | **PASS 2026-08-04** — leader set a share and it *stayed* set, which is the real assertion (the toggle shows the server's answer, not the click); non-leader cannot click them at all |
| N13 | Roster rows | Show **class name** and, when a member dies (`@die` on B), read `DEAD` | **PASS 2026-08-04** — rows read name, level and class ("Champion", "Gypsy"), and `DEAD` after `@die`. The class half was broken by the N1 empty-table bug and could never have passed before it |
| N14 | A requests a trade with B | B's popup names **A and their level**, and offers Whisper | **PASS 2026-08-04** — "test (Lv99) wants to trade with you", and the popup now exits via Reject rather than an X |
| N15 | In the trade, **right-click an inventory item → Add to trade** | The item appears on **B's** side of the window. This is the path the 0x0B42 fix restored | **PASS 2026-08-05** (after a fix, live-verified: the Axe left `HeadlessTwo`'s inventory on screen and the DB agreed). The item crosses and is **named** ("Axe x1"), closing row 6's trade half. Originally FAILed on a bug this row was not looking for: **the giver's inventory never updated** — see §The server never tells you an item left. The button sub-check was a **premise error, not a client bug**: the window defines **four** buttons (`trade.rs:96-112`) and all four render; the "seven" came from counting the seven `elements:` entries, three of which are two texts and a text box, so the `cache.rs:133` clipping theory is dead |
| N16 | Trade zeny field → **Add zeny** | Amount appears in the offer; non-numeric input is ignored, not an error | **PASS 2026-08-04** (after a fix, re-verified live). Originally FAILed: zeny transferred but never displayed — `set_our_zeny` was called only from the `/trade zeny` *chat command*, never from the window's button. Not fixable on the round trip — `ZC_ACK_ADD_EXCHANGE_ITEM` carries an index and a result but **no amount**, and its zeny index is the wire's 0 (`InventoryIndex(65534)` after the −2 decode), matching no inventory slot |
| N17 | A sends B a friend request | B's popup offers **Whisper** alongside Accept / Reject | **PASS 2026-08-04** — popup appears as specified. Prompted the sender-feedback work: the *sender* saw nothing at all, see §Findings |
| N18 | Friend list rows | Whisper / Invite / Trade / Remove; **online friends sort first** | **PASS 2026-08-04** — rows correct. Ordering **cannot be observed with two seats**, so it was verified in code at the tester's request, which found a real gap: `FriendAdded` pushed without re-sorting, so a newly accepted friend sat at the bottom below offline entries until a relog. Fixed; `SetFriendList` and `FriendOnlineStatus` already sorted |
| N19 | B logs out, then back in | Friend glyph flips `●`→`○`→`●` **without reopening the window**, and the list survives the relog | **PASS 2026-08-05**, no fix needed. The live re-render assertion holds: green → red → green with the window left open, plus an extra char-select round trip the tester happened to exercise. Presence had already been changed from tofu glyphs to a coloured dot plus a word on 2026-08-04 (see §Findings); this closes the half that was still unconfirmed. **Predicted to fail and did not** — see below |

#### Findings from the 2026-08-04 walk

**N1 — every class in the client read "Adventurer", and the prediction table
below sent me to the right file for the wrong reason.** It says "`JobName`
missed the job". `JobName` missed *every* job: our `jobidentity.lub` names its
global **`JTtbl`**, `job_name.rs` asked for **`jobtbl`**, and the
`if let Ok(...)` around it swallowed the failure — so the map loaded with **zero
entries** and every id fell through to the `Adventurer` default.
`job_identity.rs:33-45` reads *both* spellings; `JobName` was written later and
got only the one this GRF does not define. Verified from the data, not inferred:
the Lua 5.1 bytecode contains no `jobtbl` string at all, and its `JTtbl` does
hold 4021.

A **second** defect sat behind the first and would have failed the row again
after a naive fix: the `JT_` constants are *sprite* identities, and Gravity kept
the pre-rebirth name at rebirth. Reading the right table gives "Dancer H" for a
Gypsy (4021) and "Monk H" for a Champion (4016) — this row's own example,
"High Wizard", is a name no `JT_` constant produces. Player classes are now
overlaid from the connected server's `db/constants.conf` via
`tools/export_job_names.py` → `hercules_job_names.tsv` (151 jobs), matching the
`hercules_item_names.tsv` precedent; the lub still supplies monsters and NPCs,
which Hercules never names. The empty-table case now warns instead of failing
silently.

**Every class label in the client came through this table**, so N13 and the
party window's class column were failing for the same cause and could never have
passed. Re-check them together.

**N4 — the sender's own client never showed the whisper it just sent.** The
recipient half was fine, which is single-seat blindness once more: the server
does not echo a whisper back to its sender, only `ZC_ACK_WHISPER` with a status
code and no text. Nothing on the `/w`, `/r`, or Whisper-channel path pushed a
local line, so the player had no record of what they sent or to whom. Now
echoed as `[Whisper] To <name>: <text>` in `WHISPER_COLOR` from
`echo_own_whisper`, which all three paths funnel through. Echoed optimistically,
so an offline or ignoring target still prints its `WhisperResult` failure
underneath.

**N5's neighbour — an unrecognised slash command was broadcast to public chat,
which is the very leak N5 exists to prevent.** Found by typing a bare `/w`
rather than walking the row as written. Every handler in the chain matches
`/name ` *with* its trailing space and arguments, so a bare `/w`, or a typo like
`/wisper`, or `/W` with a capital, matched nothing and fell through to
`send_chat_message` — meaning **`/W <name> <secret>` reached everyone on the
map**. The server's own commands are `@` and `#` (`conf/atcommand.conf`), so a
leading `/` is always client-side and is never something to send. Text starting
with `/` is now answered locally: the command's own usage line if we implement
it (so a bare `/w` gives the whisper usage, which is what the player wanted),
otherwise "Unknown command". Guarded by unit tests over `slash_command_usage`.

**Walking a row exactly as written would not have found this.** The row says to
empty the target *field*; the leak needed a malformed *command*. Worth keeping
in mind for the rest of the pass — the written steps are a floor, not a ceiling.

**N14/N15 — closing a trade window silently bricked trading for the rest of the
session.** Reported as "I closed it, tried to trade again, and nothing happens,
in either direction, with no error line". The silence was the diagnostic: the
client prints a chat line for *every* `TradeStart` result including a catch-all,
so no line at all meant no packet came back.

`trade.c:185` sets `sd->state.trading = 1` on **both** players when a trade
starts, and only `CZ_CANCEL_EXCHANGE_ITEM` clears it. Both trade windows were
`closable: true`, and the interface framework's close button has **no close
hook** — it sends nothing. So dismissing the window by its X left the flag set
server-side, and from then on `clif_parse_TradeRequest` returns on
`sd->state.trading` **before generating any response at all**. Nothing to
receive, so nothing to report: a permanently dead Trade button until relog.

The trade *request* popup had the same defect one step earlier: closing it sent
no reply while `trade_request` had already set `trade_partner` on both sides.

**Re-verified live 2026-08-04:** cancelling a trade and immediately requesting
another now opens a second trade window. That is the direct regression test —
the previous behaviour was silence.

Both windows are now `closable: false` — Reject and Cancel are the exits, and
they tell the server. Neither can strand: `TradeCancelled` / `TradeCompleted`
close them, which is what arrives if the partner cancels or logs out. Hercules
result `2` also gained a real message instead of falling to the generic
"Trade failed (result N)".

**The general trap, worth checking on every other popup in this pass:** in this
framework a close button is *decoration* unless something wires it up. Any window
whose dismissal ought to tell the server needs `closable: false` and an explicit
button.

**`friend_request.rs` audited 2026-08-04 — no lock, but a sharper bug.**
Dismissing it is harmless: there is no `state.friending` equivalent, and
`clif_parse_FriendsListAdd` simply overwrites `friend_req`, so a re-request
works. `closable: true` is therefore fine here, unlike trade.

What the audit *did* find is a three-player defect. `open_window` **silently
drops** a window whose class is already open
(`korangar-interface/src/lib.rs:423`), and `FriendRequestWindow` captures its
requester **by value at construction**, so it can never update — while Hercules
keeps exactly **one** pending request per player and a newer one overwrites it.
So: A asks B, B does not answer, C asks B → **C's popup never appears and B's
popup still names A**. Worse, `clif_parse_FriendsListReply` requires `friend_req`
to match on *both* sides, so B pressing Accept sends **A a rejection** while C
hears nothing at all. The popup is now replaced rather than dropped, so what is
on screen is always what the server will accept.

Note the contrast: the *trade* request popup avoids this by design — it stores
the requester in `TradeState` and the already-open window re-renders from that,
so a newer request updates it. **Capturing by value is the anti-pattern**; check
any other popup built the same way.

Also fixed alongside: `clif_parse_FriendsListAdd` returns on `sd->state.trading`
before sending anything, so mid-trade the Add friend button was silently inert.
It now says why.

**Needs three characters to reproduce**, which is why two-seat testing could
never have found it — the same limit that hid the friend-list sort bug at N18.

**N19 — `HeadlessTwo []`: the bundled font cannot draw the status glyphs.** Not
the re-render failure this row predicted. The label was built correctly the whole
time; `archive/data/font/NotoSans.ttf` is a **3095-codepoint subset** with no
`●` (U+25CF) and no `○` (U+25CB), so both drew as tofu — and identically in both
states, which is why nothing ever appeared to change.

Auditing every non-ASCII character in user-facing string literals against the
font found **three more**, one of which this pass had not reached yet:

- **`★` (U+2605), the party leader marker** (`state/party.rs`) — N10's Promote
  check is "the ★ moves", and the ★ was a tofu box for leader *and* non-leader.
- **`→`** in the atcommand echo (`lib.rs`, every `@command` the player types) and
  in a DM command tooltip.

Replaced with glyphs verified through the **whole** pipeline — codepoint → cmap →
glyph id → atlas entry: `•` U+2022 online, `·` U+00B7 offline, `*` leader, `>`
arrow. (`…` U+2026, used by the "waiting for an answer…" lines, checks out.)

**Two things worth knowing before adding any glyph:**

1. **The atlas, not the TTF, is the authority.** `NotoSans.png` +
   `NotoSans.csv.gz` are pre-baked and the CSV is keyed by **glyph id**, not
   codepoint. A character can be in the TTF and still not be in the atlas.
2. **`lib.rs:2485` asks for `["NotoSans", "NotoSansKR"]` and no NotoSansKR file
   ships** — `archive/data/font/` has only NotoSans. The missing family fails
   silently, so there is no Korean fallback and no wider symbol coverage.

**The `online_glyph_updates` unit test passed throughout.** It compares `char`s,
which are equal whether or not anything can draw them. A green test asserting a
glyph proves nothing about visibility — only the GUI pass catches this.

#### The server never tells you an item left — N15, 2026-08-05

**A completed trade left a phantom copy in the giver's inventory forever.** The
item transferred correctly and the receiver's client was right; only the giver's
display was stale, and nothing corrected it short of a relog.

**The cause is the opposite of the 0x0B42 bug, and that is the lesson.** That one
was a packet we failed to model. This one is a packet the server **deliberately
never sends**: `trade.c:600` deletes the traded item with `type = 1`, and
`pc.c:4960` reads that flag as *suppress the client notification* —

```c
if(!(type&1))
    clif->delitem(sd,n,amount,reason);
```

Both `0x07FA` and `0x00AF` are modelled and registered client-side
(`version_20220406.rs:892`, `:909`) and both are correct; they simply never
arrive for your own side. The official client removes its offered items locally,
because it already knows what it put in the window, and so must this one. The
receiver goes through `pc->additem` → `clif->additem` and *is* notified, which is
precisely why **one seat could never have seen this**.

**Before assuming a packet is missing, check whether the server chose not to send
it.** Grepping for the client's handler proves nothing when the emitter is
guarded by a flag.

**The stale count manufactures false positives for other bugs.** With the phantom
on screen the tester reasonably reported "I think it traded even if I hit cancel"
— the DB showed no transfer and no inventory save at all, so cancel was correct
the whole time. A display that never self-corrects will be read as evidence about
whatever is tested next; fix it before trusting later rows.

**A second, latent bug sat in the same path and would have made the fix worse.**
`TradeAddItemResult` recorded the amount by reading the **whole stack** out of the
inventory (`lib.rs:4569`), because `ZC_ACK_ADD_EXCHANGE_ITEM` carries an index and
a result but **no amount** — the same wire gap already recorded at N16 for zeny.
Offering 1 of a stack of 20 recorded 20. Harmless while nothing consumed the
figure; the moment the completion handler used it to decrement, it would have
deleted the whole stack. The amount is now captured at *send* time
(`pending_adds`) where it is actually known.

`TradeOfferItem` also carries the inventory slot now, as
`Option<InventoryIndex>` — `None` for the partner's items, whose slots live on
their client. **Item id alone is ambiguous**: the bug was found with two identical
axes, where nothing distinguishes the stacks. And the removal has to read
`our_items` *before* `TradeCompleted`'s `.clear()`, which drops the only record of
what was offered.

Guarded by `our_offer_records_the_slot_and_the_amount_actually_offered`, because
the failure is **silent** — no log line, on either side, in any of it.

#### A party invite from outside a party promised an answer that never comes — 2026-08-05

Found off-checklist, while *setting up* for N24: the tester clicked a player and
pressed **Invite**, and the party window said *"Invited HeadlessTwo; waiting for
an answer…"* while the other seat got nothing. N8 passes and still does — it
invites **from within a party**, which is the only case the checklist covers.

**Hercules requires the inviter to already be in a party, and refuses silently.**
`party.c:382` returns a bare `0` when `party->search(sd->status.party_id)` finds
nothing. It is the **only** failure path in `party_invite` that sends the client
nothing at all: every branch below it answers with `clif->party_inviteack`
(party full, no such player, target already engaged, target offline) or a
`clif->message` (not the party leader). So there was no reply to react to, and
none was coming.

**The client read a successful socket write as acceptance.**
`invite_to_party(..).is_ok()` means the packet went out, nothing more, and both
invite paths — the target window's button and `/party invite` — printed the
"waiting for an answer" line off it. Worse, `rebuild_status_text` matches the
outgoing-invite arm **before** the party-less arm (`party.rs:491-497`), so the
accurate *"Not in a party."* status was overwritten by the false one.

**Same anti-pattern as N16's zeny: showing the click instead of the server's
answer.** N12's share toggles are the model — they render what the server
confirmed, which is exactly why that row passed. Both invite paths now check
`in_party()` first and say so. Fix live-verified 2026-08-05.

**Setup lesson: `@jobchange`-style state is not the only thing that drifts.**
`test` had `party_id = 0` because N6 deliberately tested `/party leave` and
nothing rejoined it. Check `char.party_id` before blaming an invite, the same way
the doc already says to check the `inventory.equip` column before blaming a
stance.

**Then the fix appeared not to work, which found the real bug underneath it.**
Re-testing produced the false "waiting for an answer" again — from a client that
had just been told it was not in a party. Ground truth said otherwise twice over:
both characters at `party_id = 0` **and an empty `party` table**.

**Leaving a party arrives as a removal of yourself, and only your own row was
dropped.** `clif_party_withdraw` sends to `PARTY` at `party.c:632` — deliberately
*before* the member is zeroed on the two lines after it — so the leaver receives
their own `ZC_DELETE_MEMBER_FROM_GROUP`. `remove_member` retained every other
row, so the roster stayed populated for a character in no party, `in_party()`
stayed true, and the new guard waved the invite straight through to the silent
refusal. Our own removal now clears the roster outright, keyed off the existing
`is_local()`. Stale `share_experience` and `outgoing_invite` go with it — the
same staleness one row down, where a share rule from the party you just left
renders as your own state.

**Two lessons, and the second is the expensive one.** A guard is only as good as
the state it reads: `in_party()` was the right question asked of a lie. And **a
fix that appears not to work is evidence, not a mistake to undo** — the second
bug was only reachable *because* the first was fixed, and reverting would have
buried it.

**All nine party scenarios pass, and passed before the fix.** They cannot see
this: the suite does not link the crate `PartyState` lives in. Third bug in one
day that was structurally invisible to it.

Fix live-verified 2026-08-05, along with a cycle the checklist does not ask for
and should: **join → leave → re-invite → reject → re-invite → accept**, all of
which now behave. N8 re-verified in passing ("HeadlessTwo invites you to join the
party Testing"), so fork packet `0x0EFF` and its Hercules delta are both alive.

#### Whisper UX — raised by the tester at N7, fixed 2026-08-04

N7 passes on its own terms, but replying was *obtuse*: switching to the Whisper
channel by hand landed on a blank target box, so the Reply button was the only
discoverable way to answer anyone.

The design was deliberate and half right. `last_whisper_sender` is kept separate
from `whisper_target` (`chat.rs:177-182`) so that an incoming whisper cannot
redirect a message you are part-way through typing — a good invariant, kept.
What was wrong is that leaving `whisper_target` **empty** protects nothing.
`note_whisper_from` now aims the channel at the sender when *nothing is at
stake*: no target already set **and** nothing being composed. The channel itself
is still never switched. Three unit tests pin the two guards, which are the parts
a well-meaning edit would break.

**A near-miss worth recording, because it is the shape of a false pass.** The
first "verification" of this was reported from clients started **six minutes
before the fix was built**. The observed behaviour was real but came from
leftover state: `start_whisper` sets the target and is wired to the Whisper
buttons on the friend row, party row, target frame and invite popup, plus Reply —
so the field held a name from earlier in the session. **The observation matched
the prediction exactly, which is what made it dangerous.** Re-verified against a
client started after the build. *Check process start time against the binary
mtime before recording any pass.*

#### If a Block A row fails, suspect this first

Written before walking it, from the code rather than from experience. A red row
should send you somewhere specific instead of into a debugger.

| Row | Most likely cause | Why that one |
|---|---|---|
| **N1** class shows an id or "Adventurer" | `JobName` missed the job | It reads the `JT_` constants from `jobidentity.lub`; `Adventurer` is the explicit fallback for an unknown id |
| **N1** name stale after the target changes job | Expected, not a bug | The frame captures name and class **at open** — a snapshot, not a live view |
| **N3/N4** colour identical to system text | The theme, not the routing | If routing were broken the *text* would go to the wrong place, not merely look wrong |
| **N7** channel switches by itself when a whisper arrives | A regression, and deliberately avoided | `note_whisper_from` only records the sender; switching would redirect a half-typed message |
| **N8** popup names the party but not the inviter | **Server**, not client | `0x0EFF` did not arrive — check the Hercules delta is built and the binary restarted. The client holds the name as `Option` precisely so it degrades this way |
| **N10** Kick/Promote do nothing, no error | Leader gating, or the packet | Both are leader-only and the server ignores them **silently** from anyone else — confirm the ★ is on your row first |
| **N11** buttons enabled when you are *not* leader | `local_account_id` never arrived | Gating needs `NetworkEvent::AccountId`, which was discarded until 2026-08-02 |
| **N12** a toggle flips back | Correct behaviour | The button reflects the server's answer, not the click. Flipping back means the server refused — most likely you are not the leader |
| **N13** your **own** row never says `DEAD` | Expected | `ZC_GROUP_ISALIVE` is sent `PARTY_WOS`, so it never describes you |
| **N13** class missing on a member who *changed* job | The `update_job_and_level` path | Names are resolved by the caller in three separate places; that one is the least travelled |
| **N15** item appears on your side but not theirs | The 0x0B42 fix, or rendering | The wire half is covered by `trade-add-item`; if that scenario passes and this row fails, it is rendering |
| **N16** zeny silently does nothing | Non-numeric input is ignored **by design** | Deliberate — a stray character must not cost a trade. Check the field parses as a number |
| **N18/N19** ordering looks wrong | Was a real bug, fixed 2026-08-02 | `FriendOnlineStatus` updated the glyph without re-sorting, leaving rows in place until relog. If it recurs, that is the site |
| **N19** glyph does not flip at all | Re-render, not state | `set_online` rewrites `display_label` and has a unit test, so a static glyph means correct state never reached the renderer — **the cast-bar shape, and the single most likely defect in this block** |

**The pattern to hold in mind:** four separate times on 2026-08-02 the data was
already arriving correctly and nothing displayed it — the observer's cast bar,
the trade requester's name, `PartyMemberState.job_id`, and
`NetworkEvent::AccountId`. When a row fails, ask *"is the field absent from the
wire, or merely unused?"* before assuming a protocol problem.
`KORANGAR_PACKET_LOG=1` on the observing seat answers it, and its
`[skill-cast]`-style lines sit **before** the entity lookup, which is what makes
them a clean bisector.

#### Block B — Sage / Wizard seat

`test` is already a Sage with `@allskill`. Setup for the Ice Wall row: `@jobchange 9` + `@allskill`.

| # | Check | Watch for | Result |
|---|---|---|---|
| N20 | Cast **Auto Spell** (`SA_AUTOSPELL`) | A window lists the offered spells **by name**; picking one closes it and the server accepts | ☐ |
| N21 | As a Wizard, cast **Ice Wall**, then try to walk through it | The client **refuses to path through** it, and the cells free up when it expires. **No headless test can cover this** — the suite does not link the `korangar` crate | **PASS 2026-08-04** — the character paths *around* the wall while it stands, and walks straight through once it expires. Both halves, and the only coverage this will ever have |
| N22 | Row 4 — Land Protector armed | Its real cap is **Lv5 = 121 cells**, not the "225" this row used to claim. Then Fire Wall facing different directions | **PASS 2026-08-04** — draws 11x11 at max level, which is correct (see row 4). Fire Wall reorients with the caster-to-cursor bearing and resolves diagonals to the diagonal shape. **Cell counts confirmed live: 3 for N/S and E/W, 5 on the diagonals**, matching `fire_wall()`'s ported shapes exactly |
| N23 | Row 4b — cast circle | **Expected FAIL**, root-caused: nothing triggers one. Confirm rather than investigate | **CONFIRMED FAIL 2026-08-04** — no circle under the caster; **the cast bar draws**, which is the half that matters: `SkillCast` arrives and is handled, so this is a visual that was never built, not a packet that is not landing |

#### Block C — instance and NPC refine

| # | Check | Watch for | Result |
|---|---|---|---|
| N24 | In a party, `@dminstance start prontera 156 191` | Instance window opens naming it; `@dminstance end` closes it | **FAIL 2026-08-05**, root-caused, and **not primarily a client bug** — the instance cannot be entered at all for a map called `prontera`. Both seats went to a black screen with the interface still drawn and **no input**, so the only way out was killing the process. See §Instance map names below |
| N25 | Refine equipment at a **blacksmith NPC** | Result line names the item and the new level. This is `0x0188`, a *different* path from the Weapon Refine skill | **PASS 2026-08-05** — Cotton Shirt refined to **+1** at Hollgrehenn, named with its level rather than an id. Confirmed server-side in `picklog`. **This closes row 6.** Two setup facts the row omitted, both of which cost time — see below |

#### N25 — the row omitted its two preconditions, 2026-08-05

**The classic refiner only offers what you are *wearing*.** `refinemain` builds
its menu from `getequipisequiped()` over the ten equip slots
(`npc/merchants/refine.txt:602-611`); with nothing equipped it prints *"I don't
think I can refine any items you have..."*, which reads like a client or item
problem and is neither. Equip the piece first. Same family as the 2026-08-02
note that a job change does not give you gear — **check the `equip` column
before believing a gear-shaped symptom.** Armor takes Elunium (985); weapons take
Phracon / Emveretarcon / Oridecon by weapon level.

**`picklog` is the timely source; `inventory` is not.** After a successful
refine the `inventory` table still read `refine = 0` with the Elunium untouched,
because it is only written on autosave, and an unrelated `inventory save
complete` line in the server log made the stale read look fresh. `picklog`
carries the truth immediately — the refine appears as three rows (`985 -1`,
`2301 -1 refine 0`, `2301 +1 refine 1`). **Twice in one session a stale
`inventory`/`char` read produced a wrong conclusion** (the other was reading
`char.last_map` for a logged-in character during the N24 hunt). For anything
about a live session, use `picklog`, the server log, or the client itself.

#### N19 passed after being predicted to fail twice — 2026-08-05

The row was called the likely failure of the block, on the reasoning that
presence is a cached display string and therefore the same shape as the cast bar
and as all three bugs fixed earlier that day. **It was not.** `FriendOnlineStatus`
updates the entry and re-sorts in place (`lib.rs:4937`), and it holds through
every transition with the window open.

**A "wrong" reading was the tester's client being at character select.** The
sequence reported was offline → online → offline, which looked like a value
settling wrong. All three were *correct*: presence is map-server state, so
returning to character select leaves the map server and fires the offline
notification at `unit.c:2974` exactly as logging out does. There are only two
emitters — `clif.c:11534` sends online on map entry, `unit.c:2974` sends offline
on `unit_free` — and one fires per real transition.

**Two things to check before chasing a presence bug:** whether the other seat is
actually *in-game* rather than at character select, and whether what is being
read is the friend list or the **chat log**, which prints a line per event
(*"Friend X is now online."*) and is a history, not a state. Confusing those two
is what made a passing row look broken.

**Hercules truncates the instanced map's name, and it is an upstream bug that
breaks most maps.** `instance.c:240`:

```c
snprintf(map->list[im].name, MAP_NAME_LENGTH, (usebasename ? "%.3d#%s" : "%.3d%s"), instance_id, name);
```

`MAP_NAME_LENGTH` is `11 + 1` (`mmo.h:343`), so the buffer holds **11**
characters. The `NNN#` prefix takes four, leaving **seven for the map name**.
`000#prontera` needs twelve and is clipped to **`000#pronter`**;
`mapindex_getmapname_ext`'s `sscanf(string, "%*[^#]%*[#]%15s", buf)` then strips
the prefix and sends the client **`pronter.gat`** — a map that does not exist in
any GRF.

**The rule worth remembering: a map name longer than seven characters cannot be
instanced this way.** `prontera` (8) fails, and so would `prt_fild08` (10) →
`prt_fil`. An official client fails identically; nothing here is korangar's
fault. Try a short name (`izlude`, 6) to exercise the instance *window*, which is
what this row was written to test.

**A stuck instance survives every client restart.** Instance 0 was never
destroyed (the 4h timeout was still running, and `@dminstance end` needs a
working client), so *every* subsequent login was pushed back onto the phantom
map — including logins whose `char.last_map` **and** `save_map` both read a clean
`prontera`. The DB is not evidence about a live session here. Only a server
restart cleared it.

**Clean the `$dm_inst_*` records after any instance that ends badly.**
`map_reg_num_db` held `$dm_inst_12`…`$dm_inst_19` from suite runs that never
ended, and `DM_InstanceStart` refuses while the record is non-zero, so those
party ids were permanently barred from hosting one. Cleared 2026-08-05.

#### The client hid all of it — a failed load was silent *and* unrecoverable

`loaders/async/mod.rs:477` reported a failed asynchronous load **only under
`#[cfg(feature = "debug")]`** — the `_error` binding says so outright — so in a
release build the error was dropped. A failed *map* load therefore produced a
black world with the interface still compositing over it and not one line
anywhere. Worse, with no map there is **no input handling**: chat cannot be used
to warp out, and the process has to be killed. Now reported unconditionally, one
line per failed load.

**Instrument the boundary before reasoning inward from it.** The investigation
audited the packet field, the string decoder, the trim chain and
`resolve_map_name` — each individually correct, so the audit kept coming up
empty. One `eprintln` of the wire name settled it instantly:
`[map-change] wire="pronter"`, arriving already truncated. **Same lesson the
2026-08-02 cast-bar hunt recorded** ("arrives and isn't drawn" vs "never
arrives") and it was not applied.

#### Block D — Priest

`@jobchange 8` + `@allskill`.

| # | Check | Watch for | Result |
|---|---|---|---|
| N26 | Row 3 — Heal an ally ~15 cells away | Walks into range then casts; **self-buffs still fire instantly** | **PASS 2026-08-06** on both halves — Heal from ~15 cells walked into range and then cast, and Blessing on self fired instantly. The row's own assertions needed no fix. It did surface a bug it was not looking for: **every support skill landed invisibly**, see below |

#### Finding from the Block D walk — eleven support skills had no trigger

**Fixed and live-verified 2026-08-06.** The tester's report was "Blessing fired
immediately, got the casting motion, but no visual effect" — and the bug was
neither the cast nor the packet.

Heal and Blessing on an ally both land through `clif->skill_nodamage`
(`clif.c:6077`) → `ZC_USE_SKILL`, which at this packetver is **`0x09CB`, not
`0x011a`** (`packets_struct.h:4788`, guarded on `PACKETVER_RE_NUM >= 20130724`).
That packet is modelled and *was* being processed — it drove the cast motion and
Heal's green number. But its handler spawned only `successful_caster_effect`, an
enum covering five caster-centered skills (Magnum Break, Frost Nova, Raid, Meteor
Assault, Ignition Break).

The eleven Acolyte/Priest recipes declared `hit_effects: &[HOLY_HIT]`, and
`hit_effects` fires from **exactly one place** — `apply_damage_impact`
(`lib.rs:3099`). A buff produces no damage packet, so that track was
**unreachable for all eleven**: Heal, Increase AGI, Decrease AGI, Angelus,
Blessing, Impositio, Suffragium, Aspersio, Kyrie, Magnificat, Gloria. The
recipes read as complete and had no trigger at all.

**The general lesson: a presentation table is only as live as its trigger.** The
recipe file is organised as a "five-phase contract", which makes a declared track
look wired up. Nothing in it says which packet fires which phase, so
`hit_effects` on a no-damage skill was a contradiction the type system happily
accepted. This is the **same shape as six of Block A's twelve bugs** — the data
arrives and nothing displays it — and it is now the third recorded instance.

Fixed by declaring the missing phase (`no_damage_target_effects`) and firing it
from the `0x09CB` arm at the target's position. The support recipes **keep**
their `hit_effects`, because Heal on an *undead* target is routed the other way
(`skill->attack`, `skill.c:5528`) and genuinely arrives as damage — both paths
now render, and neither double-fires, because Hercules sends one or the other for
a given cast, never both.

**Live-verified:** Blessing flashes on the target, Heal flashes *and* keeps its
green number. **These skills are silent by design for now** — the recipes declare
no sounds, and wav names were not guessed at (the effect-fidelity rule). Worth a
follow-up against roBrowserLegacy's tables.

#### Block E — last, replaces gear

Row 5 (Moonlit / Hermode, Clown + Gypsy) — see §5. **Started 2026-08-06 and
left OPEN**; two of its four questions are answered, one regressed on a fix
made during the walk, and Hermode was never reached.

| Question | State |
|---|---|
| 9×9 field appears | **PASS** — 9×9, centred on the caster |
| Tile alpha (the calibration sample) | **PASS at α 0.6** — see §5 |
| Tile reads flat, not patterned | **CLOSED 2026-08-08** by the redesign in `b6148b5c` — see §5 |
| Hermode is audible and invisible | **PASS 2026-08-16** — wav heard on both seats, 49 units all `body=none str=None`. **This closes Block E.** The empty status window and the ~31 s rooted caster are both correct (no `Icon:` for `SC_HERMODE`; `SC_DANCING` blocks `unit_can_move`) — the character freed itself on schedule |

**Setup for whoever resumes.** `test` is a **Clown (4020)** with a Violin,
`HeadlessTwo` a **Gypsy (4021)** with a Rope, both partied on `prt_fild08`.
(Party **280 is long gone** — the seats drift and the party is recreated each
session; `tools/testing/preflight-seats.sh` reports the live state, including
**which database** the servers are actually on.) **The partner needs the
instrument equipped too, not just the caster** — `skill.c:15402` requires
`tsd->weapontype1 == W_MUSICAL || W_WHIP` of the *partner*, and a bare-handed
Gypsy fails the row with the ensemble-partner message. That cost a cast on
2026-08-16 and is invisible in the DB mid-session, but shows in the client's
`[packet-log] local equipped weapon=` line.
Both know the skills — `CG_MOONLIT` is on **both** job trees (Clown via Musical
Lesson, Gypsy via Dancing Lesson), so either seat can cast.

**Two name traps cost real time here, and they are a *different* trap from the
one already recorded.** The skill window shows **"Moonlit"** and **"Hermode"** —
not "Sheltering Bliss" / "Wand of Hermode", which are Hercules `skill_db`
**Descriptions** and appear only in `docs/skills.json`. The window's names come
from the GRF's `skillinfolist.lub`, and ours is a Korean build: 395's `SkillName`
is `달빛 물레방앗간에 떨어지는 꽃잎`, so `needs_ascii_fallback` fires and the name
is **synthesised from the `SKID` identifier** (strip the ≤4-char job prefix,
title-case the rest). So the on-screen name matches neither the code name nor the
server description. The previously recorded trap ("Magnetic Earth", "Hindsight")
is the *other* case — an English lua name that differs from the code name.

**Ensemble state, which reads exactly like a client bug.** `CG_MOONLIT`'s
`SkillData1` is **20 000 ms**, and a successful cast also starts
**`SC_ENSEMBLEFATIGUE` for 10 s on both partners** (`skill.c:15469`). Neither
seat can use **any** skill during it. Retry discipline: **wait ~30 s**, or clear
it with `@die` / a relog. The partner must also be within **4 cells**
(`ensemble_range` is 4 on RENEWAL, 1 otherwise).

**CLOSED 2026-08-08 — the red square is resolved; do not re-investigate it.**
It was **two independent faults**, and the working theory recorded here at the
time (an opaque white carrier reading as saturated pink-red) was wrong on the
part that mattered. **The colour was never wrong**: the instrumentation resolved
`0xff8abb @ 0.6`, exactly what the table specifies. The bloom was **81 point
lights, one per cell** — the recipe's `light` second value is a **radius, not a
brightness**, so the "keep it dim" hedge could not work — and the tile,
faithful as it was, still read as a slab, which is simply what 81 full-coverage
tiles are. `b6148b5c` redesigned it by decision rather than fidelity: tint off,
colour moved into the light, per-cell bobbing, soft-edged overlapping carrier.
Full account in §5.

**What that leaves for this block is Hermode alone**, and it is cheap: hear
`헤르모드의 지팡이.wav`, see nothing, PASS. The reason it has never been reached
is setup, not difficulty — every earlier attempt was refused by the ensemble
check above. Budget the 30 s between casts and do this row **before** any
`@jobchange`.

The instrumentation built for the red square is still there and still worth
knowing: `KORANGAR_PACKET_LOG=1` prints one `[skill-unit] spawn` line per cell
with the resolved colour, the texture, and whether that texture reports
transparent. It is the first thing to reach for on **Gospel / Fog Wall / Evil
Land**, whose recipes carry estimated hover sizes and opacities — the same class
of unknown, now with a boundary already instrumented.

### 3. Support walk-into-range — **give this real attention**

| | |
|---|---|
| **How** | Heal or Blessing an ally ~15 cells away (Heal range is 9) |
| **Watch for** | Walks into range, then casts. **Self-buffs must still fire instantly** (self is distance 0) |
| **Why** | This path *changed behaviour* in the 26 July batch. Wants two seats: the client path is entity-agnostic, so a mob exercises the walk — but a mob closes the distance you are trying to measure |
| **Setup** | Neither test char has Heal; `@jobchange 8` + `@allskill` |
| **Result** | **PASS 2026-08-06** (N26). Heal from ~15 cells walks into range and then casts; Blessing on self fires instantly. The behaviour that changed in the 26 July batch is correct. See the Block D finding for the *visual* bug this row uncovered |

### 4. Ground-skill aiming footprint — shape or slab?

| | |
|---|---|
| **How** | Arm Storm Gust (81 cells), then Magnetic Earth / Land Protector at its real cap, Lv5 (**121 cells**) |
| **Watch for** | Does a large area read as a *shape*, or as a solid slab? Out-of-range should tint red |
| **Geometry** | Storm Gust `Layout: 4` → 9×9 = **81**. **Land Protector's "225" was wrong** — see below |
| **Status** | **PASS 2026-08-04** — Land Protector draws **11×11 = 121** at max level, Storm Gust **9×9 = 81**, Lord of Vermilion **11×11 = 121**. All three correct |
| **Result** | **PASS.** The premise was the bug, not the client |

**The 225-cell case does not exist, and this row asked for it for four days.**
`SA_LANDPROTECTOR` has **`MaxLevel: 5`**, and its `Layout` is
`[3,3,4,4,5,5,6,6,7,7]` — an array **indexed by level**, so its value at max
level is `levels[MaxLevel-1]` = **5** → 11×11 = 121. The original row read the
array's *last* entry (7 → 15×15 = 225), which is the level-10 slot of a skill
that stops at 5.

**121 is the largest square any skill in `docs/skills.json` reaches** (45 skills
carry a `Layout`). The only other one is `WZ_VERMILION` (Lord of Vermilion,
Wizard, Lv10) — worth arming during the Wizard block, because it arrives at 121
by a different route (a genuine max level rather than a truncated array) and
would expose an off-by-one in the level→layout lookup that Land Protector cannot. **Checked 2026-08-04: Vermilion draws 11×11, so the lookup is right at a real max level too.**

Same family as the trap already recorded for skill *ids*: **resolve it from
`docs/skills.json`, and mind that `Layout` is per-level.** Note also the in-game
name is **"Magnetic Earth"**, not Land Protector.

### 4b. Cast circles — never looked at, expect a rebuild

| | |
|---|---|
| **How** | Cast anything with a cast bar and watch the ground at the caster's feet. `Lockon` plus six `Beginspell` recipes exist in the recipe tables |
| **Watch for** | Whether they read as the original client's cast circles at all |
| **Expectation** | These are **procedural placeholders over generic ring textures**. CLAUDE.md's own note says to expect a rebuild, not a tick — so a "fail" here is the expected outcome and the useful output is a description of what is wrong |
| **Result** | **FAIL as expected, but for a sharper reason than "the placeholder looks wrong": nothing triggers a cast circle at all.** Re-confirmed 2026-08-04 on a Sage. Originally 2026-08-02, both seats, Sage Fire Bolt. The **cast bar draws correctly** on the caster — so `SkillCast` arrives and is handled — and the ground at the caster's feet stays empty on both clients |

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
| **Why it gates other work** | Moonlit's tile is at **α 0.6**; the recovered roBrowser table for the whole song/Gospel/Fog-Wall family uses **α 0.05**, calibrated to a different renderer. **Moonlit is the calibration sample for that entire family** — note how it reads before anyone ports the rest. **ANSWERED 2026-08-06: α 0.6 reads correctly on screen** ("a salmon, and about right"). The table's 0.05 does **not** port directly to this renderer — port the rest of the family at *this* magnitude, not the table's, and treat any 0.05 value as needing the same live check. This is a measurement, not an inference, and it unblocks Gospel and Fog Wall |
| **Do this last** | It replaces the bow gear rows 2-3 need |
| **Ensemble rules** | Both skills are `Ensemble: true` and GM 99 does **not** bypass the partner check. The partner must be opposite sex, job-mask to `MAPID_BARDDANCER`, know the same skill, wield an instrument or whip, be in the same party, not already dancing, not sitting — **and be within 4 cells** |
| **Result (2026-08-08)** | **Moonlit CLOSED; Hermode still not reached.** The red square was two independent faults. **The colour was never wrong** — the log resolved `0xff8abb @ 0.6` exactly as the table specifies. The bloom was **81 point lights, one per cell**: the recipe's `light` second value is a **radius, not a brightness**, so "keep it dim" could not work, and overlap decides the hue (a saturated colour saturates *as its hue* — Land Protector stacks 121 blues and stays blue — while a pale one drives every channel to 1.0). Even blue whited out at radius 9. Radius also has a **floor of ~4.7**, because the light sits 4.0 above the ground; 4.0 lit nothing at all. **The tile itself was faithful and still read as a slab**, which is simply what 81 full-coverage tiles are. Redesigned by decision, not by fidelity: tint **off**, colour moved into the light, per-cell bobbing note (`melody_a.bmp`, borrowed from the sibling songs in the original's own table), soft-edged carrier with overlapping tiles (edge-to-edge soft tiles show a *grid*), staggered bloom-in and fade-out. **`UnitBody::LayeredGroundQuad` now exists**, which unblocks Gospel / Fog Wall / Evil Land. |
| **Result** | **PARTIAL, 2026-08-06 — see Block E.** The 9×9 field draws and the **α 0.6 tile reads correctly**, which is the answer this row existed to get. Its sound played **81 times** (once per cell) — fixed and live-verified. The tile read as Land Protector's *pattern*, because it borrowed that skill's texture as a carrier; the fix (a genuine flat colour, no artwork) is shipped but **unverified**, and the tile then drew as **one solid red square**. Hermode not reached |

### 5b. Gospel / Fog Wall / Evil Land — the family Moonlit unblocked, never on screen

Added by `b6148b5c` on the back of `UnitBody::LayeredGroundQuad`. **Tile colours
are the original table's verbatim values; the hover sizes and opacities are
estimates and the commit says so.** Nobody has looked at any of the three.

**Read §5 first.** These are the family Moonlit was the calibration sample for,
and the calibration has a verdict: **the table's α 0.05 does not port to this
renderer.** Cast each of the three *after* Moonlit in the same session if you
can, so the comparison is on one screen and one set of eyes.

| Skill | Id | How to reach it | Cast | Recipe as coded |
|---|---|---|---|---|
| **PA_GOSPEL** (Battle Chant) | **369** | Paladin, `@allskill` | **Self-cast toggle**, SP 80–100, 60 s. Re-cast to cancel | tile **white α 0.05**, hover `effect\cross_old.bmp` at a **full** cell, opacity 1.0 |
| **PF_FOGWALL** (Blinding Mist) | **404** | Professor, `@allskill` | Ground target, range 9, SP 25, 20 s | tile **grey α 0.6**, hover `effect\lens_w.bmp` at a full cell |
| **NPC_EVILLAND** | **670** | **No job learns it** — `@useskill 670 10 <your name>` | Ground-placed, range 9, 30 s | tile **grey α 0.2**, hover `effect\curse.bmp` at a **half** cell — one of only two entries the table gives an explicit size for |

**Gospel is predicted to be invisible, and that prediction is the point of
walking it first.** It is coded at **α 0.05** — the exact value §5 measured as
not porting to this renderer, after Moonlit needed **0.6** to read as anything.
If the tile cannot be seen, that is not a new discovery to work out on the day;
it is this document's own rule arriving on screen, and the fix is the magnitude,
not the artwork. Note which of the tile and the hovering texture you can see:
`LayeredGroundQuad` is two layers, and "I see nothing" and "I see the cross but
no tint" are different failures.

**Two setup facts, so they do not cost a session the way N25's did:**

- **Gospel strips the caster's buffs when it starts** (`status->change_clear_buffs(src,3)`,
  `skill.c:12918`). Cast it **before** setting anything else up, or the setup
  disappears and reads as a client bug.
- **Fog Wall needs no gemstone on this server.** Its `Requirements` block is
  `SPCost: 25` and nothing else, so do not go shopping for a Blue Gemstone —
  official RO wants one and this build does not.

Instrument before theorising, exactly as §5 had to learn: `KORANGAR_PACKET_LOG=1`
prints one `[skill-unit] spawn` per cell with the resolved colour, the texture,
and whether that texture reports transparent. For a field you cannot see, that
log distinguishes "never spawned" from "spawned and drew nothing".

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
- ~~**Do not run `rustfmt` in this repo.**~~ **No longer true, and following it
  now fights CI.** The tree was formatted end to end on 2026-08-11 (`d5eb977a`,
  ~60 files) and `.github/workflows/formatting.yml` runs
  `cargo fmt --all --check` on every push and PR, so the committed tree *is*
  rustfmt-clean and has to stay that way. Use `cargo fmt --all`; the old
  warning was about `rustfmt <file>` reformatting a whole crate's worth of
  unrelated drift, and that drift is gone.
- The **hotbar is server-side** (Hercules hotkey table), so it cannot be edited
  from the client — change it offline.
