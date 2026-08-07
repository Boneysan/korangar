# How the server reports failures, and what the client does with it

| | |
|---|---|
| **Status** | Audit complete 2026-08-07, **none of it live-verified** — see §Not yet seen on screen |
| **Scope** | Every path by which Hercules tells a player something failed, across all three connections |
| **Why** | Row 5 of the GUI pass spent an evening on "skill level is not high enough" from a Clown who had the skill at max. That message meant *no ensemble partner*. The audit started there and kept finding channels that were silent or wrong |

## The channels

`clif->skill_fail` is **not** the only way a failure reaches the player. There are
four, and three of them were dark:

| Channel | Carries | State before | Now |
|---|---|---|---|
| `ZC_ACK_TOUSESKILL` (0x0110) | a numeric cause | 22 causes rendered | **all 46 emitted causes** |
| `ZC_NOTIFY_MAPINFO` (0x0189) | map-zone refusals | handled | unchanged |
| `ZC_MSG_SKILL` (0x07E6) | msgstringtable id + skill | **silent** | rendered |
| `ZC_MSG_VALUE` (0x07E2) | msgstringtable id + a number | **silent** | rendered, `%d` filled |
| `ZC_MSG` (0x0291) | msgstringtable id | handled | unchanged |
| `ZC_DEBUGMSG` (0x0ADB) | free text + colour | **silent** | rendered |
| `ZC_MOVE_ITEM_FAILED` (0x0AA7) | inventory slot | **silent** | names the item |
| `ZC_ACK_WHISPER_LIST` (0x00D2) | ignore-all result | **silent** | rendered |
| `ZC_ACK_REMEMBER_WARPPOINT` (0x011E) | warp memo result | **silent** | rendered |
| `SC_NOTIFY_BAN` (0x0081) **on the map connection** | disconnect reason | **silent** | rendered |

Three of those deserve individual notes:

- **`ZC_MSG_SKILL` is the only channel production skills use.** Rune Mastery,
  Change Material and the Genetic/Whitesmith crafts report `MSG_SKILL_SUCCESS`
  **and** `MSG_SKILL_FAIL` here, never through `ZC_ACK_TOUSESKILL`.
- **`ZC_DEBUGMSG` is not a debug channel** despite the name — it is Hercules'
  **`servicemessage`** script command (`script.c:17955`), so it is a message
  channel this fork's own campaign scripts can use.
- **`SC_NOTIFY_BAN` on the map connection was the worst find.** The same header
  the login and character servers use, but `LoginFailedPacket` derives only
  `LoginServer, CharacterServer`, so in-game it fell through the length fallback.
  `clif_authfail_fd` calls `sockt->eof(fd)` immediately after sending it, making
  it the **only** explanation a player ever gets for a `@kick`, a ban, a
  shutdown, or someone else logging into their account — and the client dropped
  it and silently bounced to character select. **The same header can mean
  different things on different connections; audit per connection.**

## The ensemble message, which started all this

`unit.c:1566` reports a **missing ensemble partner** as `USESKILL_FAIL_LEVEL`
(cause 0) — while Hercules' own enum already contains
**`USESKILL_FAIL_ENSEMBLE_PARTYNER` (94)** and simply does not use it. Fixed
client-side rather than with a server delta: the file already special-cases cause
0 by skill id, it needs no rebuild, and it cannot be lost in an upstream merge.
Cause 94 is mapped too, so a future delta needs no client change.

All 11 `Ensemble: true` skills now name the real requirement. Benedictio gets its
own text — its helpers are two flanking Acolytes, not a Bard/Dancer partner.

## Method — what actually worked

Four sweeps were needed, because each one's blind spot was invisible from inside
it. **Do all four, in this order, before calling a packet audit complete.**

1. **Grep the call sites, not the constants.** Matching `USESKILL_FAIL_*` finds
   202 sites; matching *any* third argument finds **323**. The extras hide behind
   hex literals (`0xa`), ternaries, and `cause` variables — that is where
   `USESKILL_FAIL_MANUAL_NOTIFY` (70) was.
2. **`DEFINE_PACKET_HEADER` is not where most message packets are declared.** The
   older channels are written as raw `WFIFOW(fd,0) = 0x…` — **180 headers** the
   struct sweep never sees. Attribute each to its **writing function**
   (`clif_wisall`, `clif_msgtable_num`, …) to put names to the numbers. Also
   sweep `WBUFW` (area sends) and `->PacketType` literals.
3. **Resolve the header set with the C preprocessor, not by reading `#if`
   branches.** `tools/generate_packet_lengths.sh` already uses this trick; the
   same one over `packets_struct.h` gives the **217 headers active at PACKETVER
   20220406**. Reading the branches by eye gets the wrong variant.
4. **Audit per connection.** `src/map/` is not the whole server. The login and
   char servers have their own client-facing refusals, and a header can be
   registered on one connection and missing on another.

**Reachability filter.** "The client never sends the matching request" is *not*
sufficient — broadcasts and script commands arrive unsolicited. What *is*
sufficient: cross-reference the client's 74 client→server packets, then check
separately for broadcast/script emitters. Of the 25 unhandled ack families, only
two were reachable this way; the rest answer requests this client never makes
(RODEX, roulette, macro detector, cash shop, grade enchant, item reform, ranking,
guild agit). Two are unreachable for reasons worth citing rather than assuming:
`ZC_BROADCAST_ITEMREFINING_RESULT` fires only from the refinery UI, and
`replace_refine_npcs` is **false** in `conf/map/battle/feature.conf`; and
`ZC_DYNAMICNPC_CREATE_RESULT` needs a CZ request this client does not send —
though **its script is loaded** (`npc/scripts.conf:231`), so it becomes reachable
the moment that request is implemented.

## The recurring bug shape: an off-by-one that still reads plausibly

Four instances in one session. **Silence is obvious; a confident wrong sentence
is not.** A message arriving proves nothing about the string.

**`msgstringtable.txt` ships with the *client* data and ours is a different build
from the server.** Measured against Hercules' documented glosses: of 4006 ids,
**1614 disagree and 433 are missing** (the table is 3577 lines; ids run past
4000). Most is harmless rewording, but the drift is not uniform, and where a
region is off by a line the text is *wrong*: `MSG_SKILL_SUCCESS` resolved to
"Item does not exist." and `MSG_SKILL_FAIL` to "Successful." — **inverted**.

Fixed by generating `hercules_messages.tsv` from `messages_main.h`'s own English
glosses (`tools/generate_message_table.py`), same principle as the packet-length
table: derive it from the server rather than hope the client data matches.
Resolution order is **curated wording → server gloss → shipped table**.

**That generator then had the same class of bug.** A block documents an id for
several *client version ranges*, and taking the last English line picks the
**oldest, obsolete** one — `MSG_CANNOT_EQUIP_ITEM_LEVEL` rendered "Nothing found
in the selected map." Each id now takes the range covering PACKETVER 20220406,
preferring the most specific match, since an open "to latest" range overlaps the
narrow historical ones.

**Narrowing the question is what made it checkable.** Of 4006 documented ids the
server only ever sends **28**. The useful question is not "are all 4006 right"
but "are those 28 right".

### The other id-keyed tables, audited the same way

| Table | Result |
|---|---|
| `EMOTION_NAMES` (81 positional) | **clean** — anchored by `e_dice1..6` → 58..63 and `e_antenna1..3` → 71..73, runs any drift would break |
| `status_effects.json` | **clean** — 700 ids vs 700 `SI_`, nothing missing, 9 deliberate renames |
| `EffectId` (1128 variants) | **clean** — 0 disagreements across the 1098 ids Hercules defines |
| `UnitId` | **DRIFTED** — `Nyanggrass` sat at `0x107` where this server has `UNT_SV_ROOTTWIST` and defines no NYANGGRASS at all |

**Two traps in the audit tooling itself, both of which nearly produced a false
"clean":** `#[numeric_value(0x7E)]` is **hex**, and a `\d+` regex silently makes
an enum look like it starts at 0; and **naive positional indices are only valid
before an enum's first explicit value**.

## Not yet seen on screen

**Every fix above is code and tests only.** Suggested live checks, most valuable
first — one action each:

1. **`@kick` from a second seat** — should name the reason instead of silently
   bouncing to character select. The biggest behavioural change here.
2. **Cast Moonlit with no partner nearby** — should name the partner
   requirement, not the skill level.
3. **Raise a skill without enough first-tier points** — exercises `ZC_MSG_VALUE`
   and the regenerated table's `%d` substitution.
4. **Fill storage, then deposit** — exercises `ZC_MOVE_ITEM_FAILED`.

**What no audit can find:** failures where Hercules simply `return 0`s and sends
nothing stay silent by construction. That is the class the walk-into-range work
fixed, and it only surfaces by playing and noticing nothing happened.

## Re-run after

Any PACKETVER change or upstream Hercules merge. In particular
`tools/generate_message_table.py` must be re-run (it is pinned to 20220406), and
the stale-variant check — *does the client register the header active at this
packetver?* — should be repeated, since a stale registration is invisible: the
real packet falls through the length fallback and goes quiet.
