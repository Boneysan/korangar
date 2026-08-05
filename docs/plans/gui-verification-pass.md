# GUI verification pass — everything that has never been on screen

| | |
|---|---|
| **Status** | **IN PROGRESS.** Written 2026-07-31. Observer rows closed 2026-08-02. **Blocks A and B COMPLETE 2026-08-04.** A: 19/19, and it found 12 bugs, every one invisible to the headless suite, which stayed green throughout. B: 4/4 — N20/N21/N22 PASS and N23 a confirmed, root-caused FAIL (the cast circle is unbuilt, now a scoped feature). Row 4's premise was corrected: the 225-cell case does not exist. **Open: rows 3/4b/5/6 and blocks C–E** |
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
| N15 | In the trade, **right-click an inventory item → Add to trade** | The item appears on **B's** side of the window. This is the path the 0x0B42 fix restored | **PARTIAL 2026-08-04** — the menu is correct during a live trade (both trade entries present and enabled). First reported as "no trade option", which was the stuck-`state.trading` session: the trade never really opened, so `is_active()` was false and both entries sat greyed. **The item-actually-crosses half is still untested.** Separately: the tester counted **six** buttons where the window defines **seven** — `Cancel` may be clipped by the fixed 280×220 in `cache.rs:133` |
| N16 | Trade zeny field → **Add zeny** | Amount appears in the offer; non-numeric input is ignored, not an error | **PASS 2026-08-04** (after a fix, re-verified live). Originally FAILed: zeny transferred but never displayed — `set_our_zeny` was called only from the `/trade zeny` *chat command*, never from the window's button. Not fixable on the round trip — `ZC_ACK_ADD_EXCHANGE_ITEM` carries an index and a result but **no amount**, and its zeny index is the wire's 0 (`InventoryIndex(65534)` after the −2 decode), matching no inventory slot |
| N17 | A sends B a friend request | B's popup offers **Whisper** alongside Accept / Reject | **PASS 2026-08-04** — popup appears as specified. Prompted the sender-feedback work: the *sender* saw nothing at all, see §Findings |
| N18 | Friend list rows | Whisper / Invite / Trade / Remove; **online friends sort first** | **PASS 2026-08-04** — rows correct. Ordering **cannot be observed with two seats**, so it was verified in code at the tester's request, which found a real gap: `FriendAdded` pushed without re-sorting, so a newly accepted friend sat at the bottom below offline entries until a relog. Fixed; `SetFriendList` and `FriendOnlineStatus` already sorted |
| N19 | B logs out, then back in | Friend glyph flips `●`→`○`→`●` **without reopening the window**, and the list survives the relog | **PARTIAL 2026-08-04.** Found the row unreadable, not failing: both glyphs were tofu (see §Findings). Presence is now a **coloured dot plus a word**, green online / red offline, confirmed rendering in both states in both the friend list and the party roster. **The live re-render assertion — does it flip with the window left open — is still unconfirmed** |

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
