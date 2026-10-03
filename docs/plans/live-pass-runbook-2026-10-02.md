# Live pass runbook — 2026-10-02

Everything below is **built and automated-tested but has never been seen by a person**.
This is the checklist for the person at the client. Tick **Seen** only for what you
watched with your own eyes; write what you saw next to anything that is not as expected.
The automated results behind each item are in
[test-intent-audit-2026-10-02.md](test-intent-audit-2026-10-02.md).

## Setup

- Servers: `cd Hercules && ./dev.sh restart && ./dev.sh wait`
- Client: `cd korangar/korangar && cargo run --release --bin korangar` (macOS, see
  `docs/MACOS_WORKFLOW.md`). Windows/Vulkan notes: `docs/PLATFORM_BRINGUP.md`.
- **Two clients** (two accounts, one party) for sections 5 and 6. A GM account for `@` commands.
- Start each section from a fresh state: `@dm reset confirm`, then `@dm mode on` in a party
  when a section says "campaign".

## 1. Recovery HUD line (new)

| Do | Expect | Seen |
|---|---|---|
| Sit (Home / `/sit`) out of combat | HUD line **Resting: fast recovery** | [ ] |
| Stand | Line disappears | [ ] |
| Get hit by a monster, then stand still | **Recovery paused: in combat** | [ ] |
| Add weight past the limit (`@item 501 1000`) | **Recovery blocked: overweight** | [ ] |
| Die and respawn | **Recovering after respawn** while filling | [ ] |

## 2. Quest log: outlines and routes (new)

In a party with `@dm mode on`: `@dmquest start 20002`.

| Do | Expect | Seen |
|---|---|---|
| Open the Quest Log | "Contract: Cellar Vermin", **Found around:** Prontera Culverts, the steps, the item counts | [ ] |
| Click **Route: Turn in to Quartermaster Wynne — prontera 156,191** | A breadcrumb to that cell | [ ] |
| `@dmquest start 20001` | A **Quest contact** line for Wynne | [ ] |
| `@dmquest start 20212` (Varmundt) | Plain text line, **no** route button (map not in the graph) | [ ] |

## 3. Arc 1 clues and the hidden `[DMJ]` lines (new)

| Do | Expect | Seen |
|---|---|---|
| `@dm reconcile preview` | **Nothing like `[DMJ]{…}` appears in the chat window** | [ ] |
| `@dmflag set dm_arc01_started 1` | A **Clues** section: Quartermaster Wynne | [ ] |
| `@dmflag set dm_arc01_clue_mask 1` | Painted Sluice appears, Tibbets does **not** | [ ] |
| `@dmflag set dm_arc01_clue_mask 5` | Painted Sluice **and** Tibbets | [ ] |
| `@dmflag set dm_arc01_clue_mask 2` | Neither of them (bit 2 is the child) | [ ] |
| `@dmflag clear dm_arc01_started` | Wynne's clue goes away | [ ] |
| Log out and in | Clues come back from the server (snapshot) | [ ] |

## 4. Adventure Guide (new)

| Do | Expect | Seen |
|---|---|---|
| Search map `prt_fild01` | **Hidden treasure chests (1)** with a route to (146,126); says it cannot show which you opened | [ ] |
| Open any server rule page | A line **Exported from Hercules 593d20ef94 (renewal mode); …** | [ ] |
| Search "DM mode", "import precedence", "discovery", "quest guidance" | Four provenance pages; DM mode says MVP/boss suppression is **server-wide** | [ ] |

## 5. Trade, two clients (new guard, F28)

| Do | Expect | Seen |
|---|---|---|
| B offers 5,000 zeny | A's window reads **Their zeny: 5000** (not an "item #0" row) | [ ] |
| A locks. B then lowers zeny to 1 | A's window shows **!! … CHANGED their offer after you locked** | [ ] |
| B locks. A presses **Confirm trade** | Refused, with the reason (chat + toast) | [ ] |
| A presses Confirm again (after >1.5 s) | The trade completes | [ ] |
| Repeat; this time A types **`/trade commit`** the first time | Refused the same way (the command cannot bypass it) | [ ] |
| Both locked, no changes | Window shows **FINAL … You give / You receive** matching what moves | [ ] |

## 6. Party (F20, F22)

| Do | Expect | Seen |
|---|---|---|
| Select a monster, then `/mark focus` | World label + outline on it, expires in ~30 s; the other client sees it | [ ] |
| `/goal add clear the cellar` | Appears under **Tonight:** in the HUD on both clients | [ ] |
| Click a party member's frame | Targets them; **Navigate** draws a route; leaving the party clears both | [ ] |

## 7. UI settings and chat (F23–F26, F32, F35)

| Do | Expect | Seen |
|---|---|---|
| Chat tabs All/Party/Whisper/System/Loot; timestamps toggle | Each tab filters; unread markers | [ ] |
| Paste a forged item link (`<ITEM>[Godly]<INFO>999999,0,0,0,0,0,0</INFO></ITEM>`) | Shown as plain `[Godly]`, not a link | [ ] |
| HUD profiles; stand out of combat 5 s | Hotbar/target frames fade to ~35%, restore on hover or damage | [ ] |
| **`/saveset rings` with two identical rings worn, unequip both, `/equip rings`** | **Both rings go on** (this was a bug today) | [ ] |
| `/loot cards`; `/wishlist 914`; kill something dropping a wishlisted item | Cards and wishlisted items stay visible; the cue, toast and Loot tab all fire | [ ] |
| Fill weight to 50% then 90% | Yellow then red; at 90% attacking/casting is refused | [ ] |
| Effect density Full → Reduced → Minimal with several players casting | Others' cosmetic effects thin out; yours, hostile ones and **cast telegraphs never** do | [ ] |
| **Listen:** dangerous cast, interrupt, quest complete, party ping, card drop | Five distinct, restrained sounds; none spam; fine with audio off | [ ] |

## 8. GM metrics (F36)

| Do | Expect | Seen |
|---|---|---|
| `@metrics status` | **OFF** by default | [ ] |
| `@metrics on`, `@metrics baseline save a`, play, `@metrics baseline diff a` | Counts rise (deaths, kills, level-ups, logins) and the zeny gauge moves | [ ] |
| `@metrics party`, `@metrics spawns`, `@metrics economy` | Sensible numbers | [ ] |
| **A non-GM account types `@metrics`** | **It must not work.** (Never tested: every test account is a GM.) | [ ] |

## 9. Fights and the finale (cannot be tested headlessly)

| Do | Expect | Seen |
|---|---|---|
| Fight an Orc Skeleton on `orcsdun01` | After three hits it steps away once, with a cooldown, not repeatedly | [ ] |
| Look for **[Elite] Orc Skeleton** on `orcsdun01` | Two exist; tougher; no client badge | [ ] |
| Eddga on `pay_fild10` | Magnum, Meteor, a power-up under 30% HP; no teleport-on-hit | [ ] |
| Finale | Run tests A–J in `Hercules/planning/design/finale-test-script.md` | [ ] |

## Reporting

For each miss: what you did, what you saw, and a screenshot if it is visual. The
`[Metrics]`, `[DM]` and `[Warning]` lines in `Hercules/log/server-latest.log` are worth
attaching. Anything wrong becomes a numbered row in the plan, not a silent edit.
