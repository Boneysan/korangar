# DM flag channel — server campaign state to the client (design)

**Status: C1–C3 built 2026-10-02; C4 (live pass) not run.** Needed first by
`hunt_story.tsv` (Arc 1 clue reveal), and by anything else that must show campaign
state the client cannot otherwise see (quest journal, clue notebook, ending cards).

## What exists (Source-confirmed unless marked)

- The server already emits **five typed `[DMJ]` messages** with `dispbottom`:
  `objective`, `reconcile` (`dm_dmj.txt`), `checkpoint`, `flag`, `reset`
  (`dm_checkpoint.txt`). Every arc calls `DM_DMJStoryObjective`, so they fire in
  Acts I–IV. Shape: `[DMJ]{"t":"<type>","v":1,"seq":N,...}`.
- **Automated-verified** (`dm-dmj-echo`, passes): `@dm reconcile preview` in a party
  produces `[DMJ]` lines that reach the client as `NetworkEvent::ChatMessage`, each
  valid JSON with a string `t` and `v: 1`.
- The **client reads none of them.** No Rust code mentions `DMJ`. `docs/plans/modern-mechanics.md`
  specified `dm::parser::parse_dm_json` to intercept them; that module was never built
  on `main` (the same `dm/parser.rs` the missing-files sweep lists). Consequence
  (Hypothesis — not yet seen in the UI): the lines show in the chat window as raw JSON.
- The `flag` message is **not a flag channel**: it is sent only for the single flag that
  triggered a checkpoint sync, to the members being synced. There is no snapshot, no
  allowlist, and ordinary `DM_SetFlag` / `DM_InstanceSetFlag` send nothing
  (**Observed**: `@dmflag set` produced no `[DMJ]` line).

## Why `[DMJ]` and not a new packet

The fork's packet ids are nearly gone. Hand-declared ids sit at `0x0EFB`–`0x0F00`, and
`0x0F00` is `MAX_PACKET_DB` (see `korangar/CLAUDE.md` §3b), so a new packet would first need
that ceiling moved. `[DMJ]` rides an existing, bounded, already-registered path, costs no
protocol change, and degrades safely: an old client just shows a line it cannot use.
The price is the 255-byte chat limit, handled by chunking below.

## Design

### Wire (server → client, JSON v1)

Keep the five existing types unchanged. Add one:

```
[DMJ]{"t":"flags","v":1,"seq":N,"part":i,"of":n,"flags":{"dm_arc01_clue_mask":5,...}}
```

- A **snapshot** is `of` messages of at most ~8 flags each (each line stays under 200
  bytes). The client applies it only when all `n` parts of one `seq` have arrived, then
  *replaces* its flag table. A partial snapshot never changes state.
- A **delta** is the existing `flag` type: `{"t":"flag","name":...,"value":...}`, applied
  last-write-wins by `seq`; a delta older than the snapshot is dropped.
- Sent to each online party member (as `DM_DMJObjective` already does, via `attachrid`)
  on: map load / quest-log restore / party join (the existing `OnPC*` replay hooks), and
  after any allowlisted flag changes. Characters get their own character-scoped flags.

### What may be sent: a server-side allowlist

A new registry beside `dm_flags.txt` lists the flags the client may see, each with a
visibility class: `clue` (reveals investigation steps), `status` (UI state), and never
`secret`. The server refuses to send a flag that is not listed. This is the spoiler
control: a flag that names the culprit or an unrevealed ending is simply not exported.
Start with the Arc 1 flags `hunt_story.tsv` already names (`dm_arc01_*`).

### Client

1. **Intercept before display.** In the chat-event handler, a text starting `[DMJ]` is
   parsed (serde_json, `v == 1`) and **never shown in chat**. Unknown `t`, wrong `v`,
   malformed JSON, or text over a hard cap (1 KiB) is dropped and counted, not displayed
   and not an error.
2. **`DmJournalState`** (in `dm/`, per the rebase rule): `checkpoint {arc, step}`,
   `flags: HashMap<String, i64>`, `objectives` keyed by `(quest_id, objective_id)`; `reset`
   clears; cleared on logout.
3. **Consumers.** `hunt_story.tsv` rows become visible when their reveal flag is set
   (semantics to fix: `name` = non-zero; `name&N` = bit set), shown as a *Clues* section in
   the quest log and Guide. Authoritative: the client never infers a clue from chat or
   local action.

## Built (2026-10-02)

| Slice | What shipped | Evidence |
|---|---|---|
| C1 | `state/dm_journal.rs`: parses all five existing types, every `[DMJ]` line is kept out of chat (understood or not), size/name/table bounds, a checkpoint never moves backwards. Interception is in the `ChatMessage` handler, server-coloured lines only, as the discovery channel does. | 13 unit tests; `dm-dmj-echo` asserts real server lines arrive **server-coloured and starting with the prefix**, the two things the interception depends on. |
| C2 | Server: `dm_client_flags.txt` (allowlist `DM_ClientFlagList`, `DM_SendFlagDelta`, `DM_SendFlagSnapshot` in parts of 4), delta from `DM_SetFlag` / `DM_PartyApplyFlag`, snapshot from the `OnPCPartyPush` / `OnPCQuestLog` / `OnPCPartyJoin` / `OnPCLoadMapEvent` hooks. Client: assembles parts and applies only a complete snapshot of one `seq`. | `dm-flag-channel`: delta reaches the setter; a non-allowlisted flag is never sent; the other member gets a snapshot of **exactly** the allowlist after a party publish and again after login, carrying the replayed value. A negative control (allowlisting the private flag) makes it fail. |
| C3 | `world/library/hunt_story.rs` + a Clues section in the quest log (header, step text, next action). Reveal grammar: `flag` = non-zero, `flag:N` = bit N set, `always`. `gen-hunts.py` now fails if a reveal flag is missing from `DM_ClientFlagList`. | 8 loader tests, 4 quest-log tests; generator negative control. |

Open questions, as decided: (1) **flags are character variables**, and party-wide sets replicate into each member's own variables, so the snapshot is per recipient and unambiguous; (2) a late joiner receives the party's flags through the existing replay, then a snapshot of their own, so they see only allowlisted flags; (3) bit masks are tested per step with `flag:N`. (4) Acts II–IV still have no story rows.

**C4 (live pass) is not done** and cannot be done headlessly: nobody has watched Arc 1 clues appear in the quest log, or watched a `[DMJ]` line stay out of the chat window.

## Slices (each independently shippable, in order)

| # | Slice | Acceptance |
|---|---|---|
| C1 | Parser + `DmJournalState` + chat interception for the five existing types | Unit tests: each type parses; malformed/oversize/unknown dropped; a `[DMJ]` line produces no chat entry. Observe once in the client that Arc 1 shows no JSON in chat. |
| C2 | Server `flags` snapshot + allowlist + hooks | Headless: a party member receives a complete snapshot after login; a partial snapshot is not applied (client unit test); a non-allowlisted flag is never sent (negative scenario). |
| C3 | `hunt_story.tsv` loader + Clues section | Unit: a hidden step is not shown with the flag clear and is shown with it set; stale `seq` cannot hide a revealed step. Rewrite the Arc 1 rows from the rebuilt `arc_01_prontera.txt` (they predate it). |
| C4 | Live pass | Walk Arc 1 and watch the clues appear. Not verifiable headlessly. |

## Open questions (decide before C2)

1. **Instance vs character flags.** `DM_InstanceSetFlag` is party-wide, `DM_SetFlag` is
   per character. The snapshot must say which, or a clue could show for the wrong person.
2. **Late joiners.** A character who joins mid-arc must get the party's clue state; the
   existing replay hooks make this cheap, but the allowlist must define what "party" flags
   a joiner may see (spoilers for what they did not witness).
3. **Reveal semantics for bit masks** (`dm_arc01_clue_mask`): is "revealed" any bit, or a
   specific bit per step? `hunt_story.tsv`'s current rows only name the flag.
4. **Act II–IV.** `hunt_story.tsv` covers Arc 1 only; later arcs need rows authored from
   the rebuilt scripts before C3 means anything for them.

## Not verified

The client has never been seen displaying or hiding a `[DMJ]` line; the chat exposure above
is inferred from the server and the headless client, not observed in the UI.
