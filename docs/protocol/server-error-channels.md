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

**It is not one skill family, it is a pattern — three more found 2026-08-07,
each with an unused dedicated cause sitting right beside it:**

| Skills | Cause 0 really means | Unused cause |
|---|---|---|
| The **12** with `State: "Shield"` in `db/re/skill_db.conf` | no shield equipped (`skill.c:16496`, one shared state check; Shield Spell also `skill.c:10759`) | `USESKILL_FAIL_NEED_SHIELD_WEAPON` (110) |
| `ALL_PARTYFLEE` (693) | not in a party (`skill.c:10003`) | `USESKILL_FAIL_NOT_PARTY_MEMBER` (92) |
| `PR_REDEMPTIO` (1014) | **three** paths: no party (`skill.c:7004`), the splash reached nobody (`skill.c:7013`), under 1% base or job experience (`skill.c:16056`) | (92 covers only the first) |

**The step that makes this safe is attributing every cause-0 emission to its
enclosing `case` labels before writing a word.** Keying text on a skill id is
only sound if cause 0 means one thing for that skill, and a grep cannot tell you
that — the shield check is nowhere near the skills it guards. Doing it turned up
that Redemptio has three conditions (so its text names all three rather than
guessing), that Shield Spell's second, stricter check means the same as the
first, and one honest exception: **Shield Reflect has a 5%-per-level `SC_KYOMU`
roll (`skill.c:16379`) that will read as a missing shield.** Left that way
deliberately — Kyomu is a Kagerou/Oboro debuff, and hedging all 12 messages for
it would cost every player clarity to spare one.

> **Superseded the same day by the section below.** The exception is gone, the
> 12-skill list is now 42 and generated, and the runtime half no longer guesses.
> Kept because the reasoning is what led to the measurement that replaced it.

### The real shape of it, and the fix (2026-08-07)

The one-at-a-time patching above was treating a pattern as a series of
accidents. Measured properly, **cause 0 is not laziness in Hercules — it is the
protocol's fallback for conditions Gravity never numbered**:

| | |
|---|---|
| States in `skill_check_condition_castbegin`'s switch | 33 |
| …reporting cause 0 | **21** |
| …of those, with a dedicated cause in the enum | **1** (`ST_SHIELD` → 110) |
| …with neither a cause nor a message-table id | **19** |
| Cause-0 emissions in `skill.c` alone | **174** |

So "make the server send the right cause" is not a fix that exists. The official
client shows "Skill level is not high enough" for all of these too.

**The split that makes it tractable is static vs. runtime.**

**Static preconditions** — needs a shield, a falcon, a cart, a stance — are
declared as `State:` in `skill_db.conf`. That is on disk at *both* ends, so
nothing needs to be sent: `tools/generate_skill_states.py` carries it into
`korangar-networking/src/packet_versions/skill_states.rs`. **42 skills across 13
states**, generated rather than hand-listed for the usual reason — a hand-kept
copy rots silently when `skill_db.conf` changes. It also caught four the
hand-written shield list had missed outright: **Brandish Spear** (Riding),
**Blitz Beat** (Falcon), **Raid** (Hiding) and **Cart Termination** (CartBoost).

**Runtime outcomes** — did the petrify roll miss, was anybody in range, is there
enough experience, is there a valid partner — only the server knows, at the
moment it decides. No table can reach them, and the skill-id texts above were
hedges: they enumerate every condition a skill has because they cannot tell which
one failed. These now ride **`ZC_SKILL_FAIL_REASON` (fork packet 0x0EFE)**,
sent immediately before the failure and paired by skill id, with 11 call sites
swapped in `skill.c` / `unit.c`. See `CLAUDE.md` §3b for the five touch points.

**Two things this got wrong on the first pass, both found by re-reading it
rather than by a failure.** The reason was modelled as a `ByteConvertable` enum,
so a value from a newer server **failed the packet and discarded the whole read
buffer** — the exact consequence written up two sections above, walked into in a
packet whose enum is documented *append only*, which made adding a reason a
wire-breaking change against every older client. It is a raw `u16` resolved by
`SkillFailReason::from_wire` now. And `clif_skill_fail_reason` mirrored only one
of `clif_skill_fail`'s three suppression guards, so a reason could in principle
be sent without the failure it explains; unreachable at the current eleven sites,
but the helper is meant to be a drop-in and must not depend on that.

**What that buys, concretely:** Redemptio's three conditions collapse to the one
that actually failed; Stone Curse now distinguishes a lost roll from a target
that cannot be affected at all; and **the Shield Reflect wart is gone** — its
`SC_KYOMU` roll was indistinguishable from its shield precondition by any static
means, and is now simply reported.

**Live-verified 2026-08-07** by `skill-fail-reason-packet`, and the guard was
checked in both directions: it passes against the real server, and neutering the
client's 0x0EFE handler makes it fail with the inferred text named in the error.
A guard for a silent regression is worth nothing until you have seen it fail.

**Writing that scenario found a separate bug, which is the argument for writing
it.** `@questskill` reported success and the skill never appeared, because
**`ZC_ADD_SKILL` (0x0111) was not modelled at all** — every skill *granted*
mid-session was consumed by the length fallback and dropped in silence, including
Plagiarism's copied skill and anything a campaign script grants. It left no trace
in the unmodelled-packet ledger because the fallback consumed it *cleanly*; the
only symptom was a command that appeared to do nothing. Fifth instance of "the
data arrives and nothing displays it".

**Cause 94 stays unused, and that is still right.** Emitting it would *lose*
information: it carries only "That needs an ensemble partner", where 0x0EFE's
`ENSEMBLE_PARTNER` reason renders the full requirement. A correct wire value is
not automatically a better message — the useful question is which channel carries
the most truth, not which one is most official.

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

**That narrowing was never actually carried out, and two bugs survived in it
(found 2026-08-07).** The check that finds them is cheap and mechanical: list
every `MSG_` constant referenced from `src/map/*.c` (45 of them), print each one
beside its generated gloss, and **read the enum's own name as an independent
statement of what the text should say**. Silence is obvious; a confident wrong
sentence is not, and the name is the only second opinion available.

1. **`MSG_ITEM_REUSE_LIMIT_SECOND` (0x746) rendered "Content has been saved in
   [SaveData_ExMacro%d]".** `messages_main.h` paired 0x745's and 0x746's English
   lines with each other's Korean, so the generator was faithfully reporting a
   swap. This is one of only **two** ids `ZC_MSG_VALUE` can carry, and every item
   with a `Delay` in `item_db.conf` sends it — an Yggdrasil Berry used twice in
   five seconds is the repro. **Fixed in Hercules**, not worked around here; see
   the delta below.
2. **A multi-line English gloss was truncated to its closing line.** The gloss
   sits under the range header as one or more lines; taking the last one left
   `MSG_MDUNGEON_SUBSCRIPTION_ERROR_EXIST` saying only "Please enter in 5
   minutes." The rule is now **split the range down the middle**, Korean half
   then English half, which is also the only rule that handles the 109 ranges
   with *no* Korean characters at all — where the Korean slot holds a
   romanisation ("Rouge" / "Rogue") and scanning for the last Korean-script line
   would render both halves. 24 glosses changed, none lost, no Korean leaked.
   The block regex needed `(?:(?!\*/).)*` too: a lazy body swallows the file's
   own licence header and hands `MSG_DO_YOU_AGREE` twenty lines of GPL.

**So a generated table needs the same treatment as the shipped one it replaced.**
Deriving it from the server removes the *build-drift* class of error and nothing
else; it inherits every defect in the source, and the generator adds its own.

### Fixing the source, and the check that scales

The header was worth auditing on its own once it became authoritative, and
**0x746 was not alone.** The check needs no translation: the Korean line carries
ASCII anchors — an identifier like `SaveData_ExMacro`, the digits in
"3시간"/"5시간" — that must reappear in *its own* gloss. When they land on a
**neighbour's** gloss instead, the English is shifted. Run over all 3023 ids with
both halves it flags 16, of which two are real:

| Region | Defect |
|---|---|
| **0x576–0x582** (13 ids) | English **lags its Korean by one slot** for the whole run. `MSG_USESKILL_FAIL_HOLYWATER` read "Unable to use the skill to exceed the number of Ancilla"; `MSG_NO_CHATTING` read "This skill requires other skills to be used"; `MSG_VET_3HOUR` read "Chat is not allowed in this map". The tail, `MSG_FAILED_MOBILE_LOCKSERVER`, had **no gloss at all** — its own had fallen off the end |
| **0x745/0x746** | A straight two-id swap |

Every enum name in the run corroborates its corrected line independently, which
is what made the fix mechanical rather than a translation exercise: the shift is
undone by moving each English line up one slot, and only the run's tail needed a
sentence written.

**Fixed in the Hercules tree, in all three variants** (`messages_main.h`,
`messages_re.h`, `messages_zero.h` — the defect is identical in each). Those
files are **banner-marked autogenerated**, the same trap as
`packets<year>_len_*.h`, so the delta is guarded at both ends: the generator
holds a `CORRECTED_UPSTREAM` sentinel table and **exits non-zero without
writing** if the header reverts, and `msgstringtable.rs` pins the same ids. A
silent regeneration cannot put the wrong text back.

**The rest of the 16 are not shifts and were left alone**, deliberately:
`MSG_AURA_OFF` ends "[ON]" where the Korean says OFF, `MSG_MISS_EFFECT_OFF`
duplicates the ON gloss, and `MSG_CANNOT_SELL_IN_PRIVATE_TAB` is a lone
mistranslation. All three would mean *inventing* English rather than moving it,
none is reachable from `src/map/*.c`, and a fabricated gloss is the exact failure
mode this whole section is about.

### The other id-keyed tables, audited the same way

| Table | Result |
|---|---|
| `EMOTION_NAMES` (81 positional) | **clean** — anchored by `e_dice1..6` → 58..63 and `e_antenna1..3` → 71..73, runs any drift would break |
| `status_effects.json` | **clean** — 700 ids vs 700 `SI_`, nothing missing, 9 deliberate renames |
| `EffectId` (1128 variants) | **clean** — 0 disagreements across the 1098 ids Hercules defines |
| `UnitId` | **DRIFTED** — `Nyanggrass` sat at `0x107` where this server has `UNT_SV_ROOTTWIST` and defines no NYANGGRASS at all; and see the range check below |

**Two traps in the audit tooling itself, both of which nearly produced a false
"clean":** `#[numeric_value(0x7E)]` is **hex**, and a `\d+` regex silently makes
an enum look like it starts at 0; and **naive positional indices are only valid
before an enum's first explicit value**.

**A third check these tables want, which a name-by-name diff cannot give: does
every value fall inside the enum's own declared bound?** `UnitId::Deepblindtrap`
carries `#[numeric_value(20852)]` — `0x5174`, two orders of magnitude past
`UnitId::Max` (`0x190`), and the only value in the enum that is. It and the three
variants after it are therefore unreachable. Inherited from upstream Korangar
(`4c9782f3`) and **left alone deliberately**: Hercules defines no such units, so
it cannot fire against this server, and there is no local authority to take the
right ids from. Flagged in place as `FIXME(unit-id)`.

**What that would have cost if it were reachable is the part worth remembering.**
An unmodelled enum value inside a *registered* packet is not a missing visual: it
returns `HandlerResult::InternalError`, and `korangar-networking/src/lib.rs:382`
answers that with `cut_off_buffer_base = 0; break` — **every packet batched
behind it in that read is discarded.** Coverage of the wide server-facing enums
is a framing concern, not a cosmetic one. Checked at the same time:
`DamageType` covers all of `enum battle_dmg_type` (which stops at 11), and
`StatType` decodes every `SP_` constant `clif_updatestatus` can send —
`SP_ATTACKRANGE` is the one apparent gap and is not one, since it leaves on
`ZC_ATTACK_RANGE` rather than as a stat id.

### Client→server layouts, checked mechanically against the same table

Every audit above runs server→client. The other direction was checked too, by
computing each packet struct's serialized size and diffing it against Hercules'
generated length table: **73 of 74 `ClientPacket`s match exactly**, and the
odd one out is `CancelCastPacket` (`0x0F00`), which is absent from the table
because it is a fork addition whose length lives in the hand-maintained
`src/common/packets_len.h` — a file `tools/generate_packet_lengths.sh` does not
read. `ZC_PARTY_INVITE_SENDER` (`0x0EFF`) is missing for the same reason. Both
are modelled, so nothing depends on the fallback for them; the gap is worth
knowing before trusting that table as a complete inventory of this server's
packets. Validation for the size calculation: run over the **210** server
packets it agrees with Hercules on **206**, the other four being packets modelled
with an explicit length field instead of `#[variable_length]`.

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


## The complement audit — what the server sends that the client never registers

Every earlier packet audit asked *for every family the client registers, is it
the variant active at this packetver?* Answer: zero mismatches across 218
headers. True, and it never once looked at the families the client does **not**
register. `ZC_ADD_SKILL` sat in that blind spot and was found by accident, so the
complement was worth running deliberately (2026-08-08).

**Method — three sources, unioned, then diffed against the client:**

| Source | Count |
|---|---|
| Active `DEFINE_PACKET_HEADER` ids, via the **C preprocessor** over `packets_struct.h` at `PACKETVER=20220406` (147 of them ZC) | 218 |
| Raw `WFIFOW(fd,0)=0x…` writers in `clif.c`, each attributed to its writing function | 232 |
| **Headers the map server can send** | **378** |
| Registered by `register_map_server_packets` | 188 |
| **Sendable but unregistered** | **243** |

Never read `#if` branches by eye — `ZC_ADD_SKILL` is 0x0111 here and 0x0B31 under
`PACKETVER_RE`/`_ZERO`, and only the preprocessor gets that right.

**Framing risk: none, but it was worth proving.** 13 of the 243 have **no length
entry**, which is the dangerous class — no handler *and* no fallback means
`UnhandledPacket` and the whole read buffer, not one message. Two of them
(`clif_PartyRecruitDeleteNotify`, `clif_PartyBookingVolunteerInfo`) send
`ALL_CLIENT`, so one player using party booking would have hit everyone. All 13
are unreachable: twelve sit behind `#ifdef PARTY_RECRUIT`, **off in this build**
(confirmed by preprocessing `config/core.h`, not by grep), and `clif_PVPInfo`
(0x0210) answers only `CZ_REQ_PVPPOINT`, which this client never sends — and the
server's own `packet_len(0x210)` is unset, so it would emit zero bytes anyway.

**The finding: four packets a campaign script can trigger, all silently dropped.**

| Script command | Packet | Was |
|---|---|---|
| `soundeffect` / `soundeffectall` | `ZC_SOUND` 0x01D3 | silence |
| `showscript` | `ZC_SHOWSCRIPT` 0x08B3 | no text |
| `progressbar` | `ZC_PROGRESS` 0x02F0 / `_CANCEL` 0x02F2 | no bar, and the player is locked out server-side meanwhile |
| `specialeffectnum` | `ZC_NOTIFY_EFFECT3` 0x0B69 | nothing |

Sixth instance of "the data arrives and nothing displays it", and squarely in
this fork's own purpose. **How the class hides, stated once more:** plain
`specialeffect` rides 0x01F3 and was already registered, so the obvious command
worked and nobody suspected the channel. The fallback consumed all four
*cleanly*, so the packet ledger showed nothing — and the ledger only ever lists
what a test provoked.

**Two are only half-fixed, deliberately.** `showscript` is meant to float over
the entity and `progressbar` wants a widget; the client has neither surface —
`OverheadMessagePacket` is itself routed to chat with a `FIX` comment for the
same reason. Both now produce a chat line, which beats silence and is honest
about being partial. `ZC_SOUND` and `ZC_NOTIFY_EFFECT3` are fully wired: real
spatial audio (flat when Hercules clears the id for a repeating sound) and the
same effect path as `specialeffect`.

**The rest of the 243 are genuinely out of scope** — guild, mail/RODEX, auction,
vending, buying store, battlegrounds, cash shop, roulette, macro detector, pet,
homunculus, mercenary. Not gaps; features this fork does not have.
