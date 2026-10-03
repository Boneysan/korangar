# Missing-files sweep — 2026-10-02

Produced by `python3 tools/audits/orphan_sweep.py` (re-run it; it reports, never fixes).
Evidence labels: **Observed** = ran it; **Source-confirmed** = read the code;
**Automated-verified** = a named passing test; **Hypothesis** = needs a check.

## Resolved

| Item | Resolution | Evidence |
|---|---|---|
| `campaign_quest_locations.tsv`, `hunt_guidance.tsv` | Generated from script-derived turn-in NPCs (the old per-arc hub table held placeholder cells); quest log shows the outline and an exact-cell route. | Automated-verified (12 loader tests). Not seen on screen. |
| `chests.tsv` | 146 hidden chests listed per map in the Adventure Guide, with route links. | Automated-verified (loader + Guide tests). Not seen on screen. |
| `hunt_story.tsv` | Now read by the client (`hunt_story.rs`), gated on server-reported flags. | Automated-verified (8 + 4 tests, `dm-flag-channel`). Not seen on screen. |
| `hunt_objectives.tsv` | Dropped from the generator: it repeated what the quest log derives from the bestiary. | Source-confirmed. |
| **`ZC_RECOVERY_STATE` (0x0EFD)** | The client now turns it into `NetworkEvent::RecoveryState` (raw bytes), keeps it in `RecoveryState`, and shows a HUD line (sitting, post-respawn, blocked by status/weight/combat). | Automated-verified: `recovery-state-packet` (sit → mode 2, stand → mode 1), 6 state tests, a packet layout test; `check-recovery-paths.sh` passes. HUD not seen on screen. |
| `warp_graph.tsv` | **Compared, then retired** (generator and check step removed). The client already routes from `navigation_graph.json`: 3,186 edges against the TSV's 764. All 737 ungated shared edges match. The TSV's 27 extras are 26 gated script-body warps (e.g. a Transport Device that calls `warp "dali"`, which the client graph deliberately omits) and 1 ungated edge (`new_1-3` → `iz_ac01`). | Observed. |
| `equipment_eligibility.tsv` | **Retired** (generator and check step removed). The Guide already prints job, sex, level, slot and weapon level from its own item data. The generator was lossy: ~570 items have `Upper` restrictions written as flag lists that it zeroed, and third classes were absent from its job map, so a "you can't equip this" verdict built on it would be wrong for some players. | Source-confirmed and Observed (audit of `item_db`). |
| `navigation_services.json` | Was a **false positive**: it is the navigation generator's input. The sweep now counts Python tools (not writing generators) as readers. | Observed. |
| Fork packet docs | `ZC_RECOVERY_STATE` is now documented in `korangar/CLAUDE.md` §3b. | Source-confirmed. |
| `check-recovery-paths.sh` | Switched from `rg -q` to `grep -Eq` (same plain patterns), so it runs without ripgrep; verified with a PATH holding only `/usr/bin:/bin`. | Observed. |
| `check-campaign.sh` | Now passes end to end for the first time, including a boot-log assertion that the server read the 9 AI profiles. Two stale items fixed: a removed warp step, and a grep for `DM_CampaignCheckpointEvents` (27 characters, over the 24-character NPC-name limit; the NPC is `DM_CampEvents`). | Observed. |

## Flag channel and `hunt_story.tsv` (built, not seen live)

Slices C1–C3 of [dm-flag-channel.md](../specs/dm-flag-channel.md) shipped: the client parses and hides
every `[DMJ]` line, the server reports an allowlist of flags (delta + snapshot), and the quest log shows
a Clues section for the Arc 1 steps the server has revealed. **Observed** by `dm-dmj-echo` and
`dm-flag-channel`; the raw-JSON-in-chat symptom and the Clues section are still not seen in the UI (C4).
Acts II–IV have no story rows yet.

## Still open

1. **29 stale doc paths** (the sweep's DOCS list). Triage by kind:
   - *Renamed, work shipped:* `bestiary_journal.rs` → `interface/windows/dm/bestiary.rs`;
     `loot_generator.rs` → `dm/loot.rs`. Fix the doc paths.
   - *Planned, never built:* `initiative.rs`, `encounter.rs`, `check_console.rs`,
     `quest_journal.rs`, `dice_card.rs`, `dm/parser.rs`, `dm/commands.rs`, `dm/state.rs`,
     `dm/quests.rs`. Mark the plans "not built" or drop them; `dm/parser.rs` is slice C1.
   - *Tools that no longer exist:* `campaign_quest_merge.py`, `merge_bestiary.py`,
     `package-client.sh` (named in `friends-distribution.md`), `examples/trail_probe.rs`,
     `planning/mvp-party-balance-checker.md`.
   - *Path written relative to a crate or a deleted fork:* `archive/native/mod.rs`,
     `gamefile/mod.rs`, `headless-tester/main.rs`, `Hercules_RO/...`, `docs/quests.json`,
     `docs/quest_flavor.json`; and `db/re/quest_db.conf` (GDD.md:623; the live file is
     `db/quest_db.conf`).
2. **Unroutable chest/turn-in maps.** `iz_dun05` and `kh_dun02` (chests) and `ba_in01`
   (quests 20212/20213) are not in the navigation graph, so their routes show as text.
   `iz_dun05` is reachable only through a gated script warp, which the graph omits.

## Not covered by this sweep

GDD rows marked Partial whose *named behaviour* is missing (as opposed to a named file)
need a row-by-row read. The tool finds missing files and ignored packets only.
