# Qwen3 encyclopedia completion loop

**Mission:** complete the Adventure Guide for the active Hercules server revision against the [E0–E7 roadmap](encyclopedia-roadmap.md) and [field inventory](encyclopedia-coverage.md). Work through the whole in-scope backlog, not a fixed number of records. Players should be able to answer what something does, where to get or use it, and which conditions change that answer. A specific, visible unknown is acceptable when source or live state cannot establish a fact.

**Driver:** Qwen3 works in bounded source-grounded units, validates each one, commits it, and resumes from a durable ledger. It may publish a claim after the gates below succeed; routine per-record human approval is not required. Attribute model-assisted reviews honestly. Ask a person only for an inaccessible source or live session, or a real product decision.

**Active queue (2026-09-30):** `tools/encyclopedia_unit.py` runs this plan one unit at a time. It holds the queue (`docs/plans/encyclopedia-unit-queue.json`, built from loaded source in the order of [encyclopedia-completion-plan.md](encyclopedia-completion-plan.md)), writes one packet per unit, validates drafts, records an independent verifier's verdict, runs the exporters and the Rust parse test, and commits. A local model follows [encyclopedia-procedures.md](encyclopedia-procedures.md) and does not read this file. The repeat loop below is the rule the tool implements; read it when changing the tool. `tools/encyclopedia_loop.py` only proves braced one-item exchanges and its queue is empty. E7 stays blocked until the user ends the live-test deferral.

## Setup and current baseline

As of 2026-09-28, E0–E7 are partial. The roadmap and coverage inventory hold current counts and open fields; do not create a new permanent denominator. The reviewed data produced before this loop was committed in `korangar` as `63879756`.

Paths in this plan are relative to the `korangar/` repository root. `Hercules/` is its sibling (`../Hercules`), and every exporter finds it that way. The workspace may be mounted at different absolute paths on different machines (for example `/media/bigz/T7/...` on Linux or `/Volumes/T7/...` on macOS). Never write an absolute path into data, the ledger, or the journal.

`Hercules/` has no uncommitted work as of Hercules `cc83ca152`; its database settings (`inter_user` / `herc_re_db`) are committed. Do not edit Hercules config or commit in `Hercules/`. `db/quest_db.conf` is committed (Hercules `fa63cd8af`); if it is ever dirty again, `tools/export_quest_reference.py` reads its tracked `HEAD` snapshot and reports the fallback. A revision plus `source_worktree_dirty: true` alone is inadequate provenance for a changed file; record the exact digest of the cited content or cite the tracked snapshot.

**Model and context.** Run [`qwen3-coder:30b`](https://ollama.com/library/qwen3-coder) under [Ollama](https://docs.ollama.com) (on macOS, the Ollama app; on Linux, the [Linux setup](https://docs.ollama.com/linux)); fall back to [`qwen3:14b`](https://ollama.com/library/qwen3) under the same gates if throughput or memory makes it impractical. Check `ollama ps` for CPU/GPU offload. Use a context of **224K tokens**. Ollama's [default context is much smaller](https://docs.ollama.com/faq), and Qwen Code's own context setting does not enlarge the server's, so set it on the server: `OLLAMA_CONTEXT_LENGTH=229376 ollama serve`, or a Modelfile with `PARAMETER num_ctx 229376`. With this window the whole session-start reading (about 25K tokens) fits comfortably; do not trim it. Prefer [schema-constrained JSON](https://docs.ollama.com/capabilities/structured-outputs) at temperature 0 for extraction. Log model tag, quantization, Ollama version, context, offload, and prompt version in the ledger's `model` block. None of these settings proves a claim.

**Agent.** Install [Qwen Code](https://github.com/QwenLM/qwen-code/blob/main/docs/users/quickstart.md) (Node.js 22+, `npm install -g @qwen-code/qwen-code@latest`); plain `ollama run` is a chat interface and cannot execute this loop. Qwen Code [supports Ollama through its OpenAI-compatible endpoint](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/). It reads `QWEN.md` at the repository root, which points here. From `korangar/`, with Ollama serving:

```sh
qwen --auth-type openai --model qwen3-coder:30b \
  --openai-api-key ollama --openai-base-url http://localhost:11434/v1
```

Paste the instruction at the end of this file into the agent. Qwen Code may ask for file-edit approval; use its session acceptance mode if you want the authorized loop to proceed unattended. A new session resumes by reading the ledger and using the same instruction.

## Tools: where each kind of claim lives

Reviewed claims are hand-maintained JSON under `tools/`; exporters validate them against loaded Hercules source and write generated `docs/*.v1.json`. **Edit the review file, never the generated file.** Do not write a new exporter when an existing one covers the claim type; extending an exporter is an E0-style unit of its own, with a fixture.

| Package | Review file (edit this) | Exporter | Generated output |
|---|---|---|---|
| E1 acquisition | `tools/item_exchange_reviews.json` | `tools/export_item_exchange_reviews.py` | `docs/item-exchanges.v1.json` |
| E1 clues (read-only) | — | `export_item_grant_reference.py`, `export_crafting_reference.py` | `docs/item-script-grants.v1.json`, `docs/crafting.v1.json` |
| E2 NPC services | `tools/npc_service_reviews.json` | `export_npc_service_clues.py`, then `export_npc_service_reviews.py` | `docs/npc-service-clues.v1.json`, `docs/npc-service-reviews.v1.json` |
| E2/E1 NPCs, shops | — | `export_npc_reference.py` | `docs/npcs.v1.json` |
| E3 effects | — (translation code) | `export_item_reference.py` | `docs/items.v1.json`, `docs/cards.v1.json` |
| E4 formulas | `tools/skill_formula_reviews.json` | `export_skill_formula_reviews.py` | `docs/skill-formula-reviews.v1.json` |
| E4 statuses, rules | — | `export_status_reference.py`, `export_server_rules.py` | `docs/status-effects.v1.json`, `docs/server-rules.v1.json` |
| E5 quests | `tools/quest_flow_reviews.json`, `tools/quest_npc_routes.json` | `export_quest_reference.py` | `docs/quests.v1.json` |
| E6 bosses | `tools/boss_behavior_reviews.json` | `export_boss_behavior_reviews.py` | `docs/boss-behavior.v1.json` |
| E6 scripted spawns | `tools/scripted_spawn_reviews.json` | `export_scripted_spawn_reviews.py` | `docs/scripted-spawn-reviews.v1.json` |
| E6 map flags | `tools/map_runtime_flag_reviews.json` | `export_map_flags.py` | `docs/map-flags.v1.json` |
| Any: unknown, absent, configured, or not applicable | `tools/encyclopedia_dispositions.json` | `tools/export_encyclopedia_dispositions.py` | `docs/encyclopedia-dispositions.v1.json` |
| E0 coverage | — | `export_encyclopedia_coverage.py` | `docs/encyclopedia-coverage.v1.json` |

Before adding the first entry to a review file in a session, read two or three existing entries in it and copy their exact shape (source path, line anchors, quoted text, conditions, unknowns). The exporter's `validate_sources`-style checks are the citation validator; a claim that does not pass them is not accepted.

Checks, in this order:

1. The exporter you touched: `python3 tools/<exporter>.py`, then `python3 tools/<exporter>.py --check`.
2. The whole pipeline, since outputs feed each other: `python3 tools/export_supported_data.py`, then `python3 tools/export_supported_data.py --check`. It must exit 0.
3. If Rust changed, or a generated file's shape changed: `cargo test -p korangar`. The client embeds the generated JSON through `korangar/src/dm/reference_data.rs` and renders it in `korangar/src/interface/windows/adventure_guide.rs`.

## State, work units and commits

Use [`encyclopedia-qwen3-progress.json`](encyclopedia-qwen3-progress.json) as the resume ledger. Update it after each accepted unit by writing the whole file and confirming it still parses (`python3 -m json.tool`). Keep a concise per-unit journal at `docs/plans/encyclopedia-qwen3-journal.md` with evidence, decision, coverage delta, checks, unresolved questions, and next action. Keep large raw model responses in a staging area outside the repository.

A person or a strong model changing the tool or the queue reads this plan, the roadmap, coverage inventory, [data contract](../specs/encyclopedia-data.md), ledger, generated coverage, loaded-script manifest, and Git status of both repositories. A local model running units reads only its packet; that reading list would fill its context before the unit starts. **If the loaded-script manifest does not exist yet, the next unit is `E0-loaded-source-manifest`; it creates the manifest and dirty-file digests that every later staleness check depends on.** Judge staleness by the digests of cited source files, not by repository heads: a commit that only touches `docs/plans/`, `QWEN.md`, or review files does not make any unit stale. Reopen any unit whose cited source changed. Never claim coverage of unloaded scripts, live SQL, runtime state, or a client session based on a static scan.

Unit states are `todo`, `active`, `done`, `blocked`, and `stale`. A blocker names the missing input, its effect, and the condition that would unblock it. A blocked unit does not stop other units. Record `attempts` on a unit; after **two** attempts that end without an accepted result, mark it `blocked` with a diagnostic and move on.

Build a finite queue from generated coverage plus loaded-source inventories. One unit is a complete NPC branch/shared helper, stock declaration family, item-script pattern, skill/formula/status family, quest chain, spawn/flag/route family, or client journey. Give each a stable ID, package, source scope, expected output, and exit check. Split a call graph that cannot fit in context. **The ledger's initial `*-queue` and `*-inventory` entries are planning units: their job is to write that package's concrete units into the ledger, not to review records.** Select units in this order: E0 infrastructure blocking other work; broken/stale claims; common player journeys and custom content; common patterns unlocking many records; then remaining candidates in deterministic path/ID order. Interleave E7 checks as slices become usable.

Every in-scope candidate eventually needs a final disposition: verified/conditional/configured estimate with exact evidence; a specific unknown because source or live state cannot resolve it; or a documented absence after searching the declared scope. A clue or `not reviewed` remains open work. Never turn model confidence, a nearby call, or dialogue alone into a verified fact.

**Commits.** Commit each accepted unit in `korangar/` on the current branch, one commit per unit, message `Encyclopedia <unit-id>: <one-line result>`. Stage only the files that unit changed, by explicit path (`git add <paths>`); never `git add -A` or `git add .`. Never commit, stage, reset, or stash in `Hercules/`, and never edit its config. A blocked unit's ledger and journal update may be committed on its own. Do not push.

## Repeat loop

```text
repeat:
  1. Read ledger, roadmap, coverage, loaded-source manifest and both Git statuses.
     If the manifest is missing, run E0-loaded-source-manifest.
     Recompute candidate queue; mark units whose cited source digests changed stale.
  2. Pick the highest-priority runnable unit. If none exists, audit exit gates.
  3. Assemble a bounded packet: executable branch, called helpers, active config/
     DB records, exact path and lines, stable IDs, source revision/digest.
  4. Trace reachable branches affecting the claim; draft evidence and unknowns.
     Save the draft to staging before step 6.
  5. Validate schema, IDs, loaded scope, exact citations/quotes, revision/digest,
     links, and all stated conditions mechanically.
  6. Independent reread: a separate, fresh session (not a subagent) runs
     `encyclopedia_unit.py verify`, which shows only the source and the draft's
     claims. It judges each claim and lists omissions: costs, rewards, failure
     paths, prerequisites, random outcomes, timers and exceptions. The drafter
     corrects or downgrades every claim it does not support.
  7. Edit the review file (see the Tools table), run the exporter, then
     export_supported_data.py and its --check, and cargo test if Rust or a
     generated file's shape changed. Inspect the generated claims. Compare with
     the live server when claiming live behavior.
  8. Journal source/evidence, accepted files, before/after coverage, checks, open
     questions and next unit. Checkpoint the ledger, then commit the unit.
  9. If blocked, record why and move to another unit. After two attempts with no
     accepted result, mark the unit blocked with a diagnostic and resume point.
until all E0–E7 exit gates and the final live pass are recorded as met.
```

Do not call a unit done merely because JSON was generated or an exporter passed. Add a small source fixture or citation-validation case for each new reviewed family. Preserve source bugs and dialogue/script mismatches as documented behavior; server fixes require a separate task.

## Package queue

| Package | Work | Exit evidence |
|---|---|---|
| E0 Evidence | Build the loaded-script manifest and dirty-file digests; audit claim types, clue links, empty results, denominators, per-assertion labels, source snapshots and citation validation. | Manifest regenerates and `--check` catches source changes; item, quest, NPC, map flag, spawn and formula samples show correct states; no clue is shown as confirmed; indexed, reviewed, unknown and open counts differ. |
| E1 Acquisition | Classify loaded grant/removal branches, exchanges, drops, shops, crafting, containers and quest rewards. Trace costs, gates, outcomes and repeat limits. | Verified paths have exact quantities, source, conditions and reciprocal item/NPC/quest/map links; unresolved dynamic paths have reasons. |
| E2 NPC services | Review ordinary shop/trader/barter/cash and dynamic stock/roles; maintain reviewed literal service calls. Trace fees, availability, destinations and routes. | Literal, conditional, runtime and unknown offers are distinct; claimed routes are reachable. |
| E3 Effects | Cluster unsupported item/card/combo scripts; implement only proven patterns. Explain trigger, target, scaling, chance, duration, condition and interaction. | Pattern examples match code; translated and fully explained counts differ; unsupported fields remain visible. |
| E4 Skills and rules | Trace skill/status/combat code, effective config precedence, formulas, cures, immunity, stacking, interruption, jobs and account rules. | Formulas have units, order, rounding, modifiers and reconciled examples; an icon or call site alone is not called an effect. |
| E5 Quests | Trace in-scope families from eligibility through objectives, branches, turn-in, reward, reset and failure, including quests without nearby grant clues. | Each published journey is followable; each reward is on a cited completion path; unknown stages show. |
| E6 World | Resolve static/scripted/dynamic spawns, boss phases, map-zone overrides, flags, entrances and routes; separate configured from live data. | Map/species relations have correct states; rule order is cited or unknown; live and route claims have observations. |
| E7 Release | Search aliases/effects/sources; inspect cross-links, spoiler labels, empty states and non-DM access; run build/export checks and live journeys. | Dated fresh non-DM item → source → route, skill → status → cure, and quest → objective → turn-in passes against the running server; no dead-end verified link. |

The exact field backlog and denominators are in the coverage inventory and `docs/encyclopedia-coverage.v1.json`; refresh them during the loop. Scope includes all supported categories, including jobs and account rules. A dynamic/live-only fact may finish as unknown only with the precise reason and best verified static answer. E7's on-screen checks need a running server and client that a person watches. Record them as blocked with exactly what to look at, and never mark them passed from static work. Missing live E7 access is a blocker, never a pass.

## Completion and resume rule

Finish only when the [roadmap checklist](encyclopedia-roadmap.md#completion-checklist) passes for a named server revision and loaded-source scope, every in-scope candidate has a final disposition, all E0–E7 exit evidence is journaled, exports and relevant code checks pass, and dated fresh non-DM live journeys pass. An unresolved clue, unreviewed supported field, stale citation, failed check, or absent live pass keeps its package open. No arbitrary target count or fixed iteration limit substitutes for these gates.

At the end of each session, print the ledger location, completed units, commits, coverage delta, checks, blockers, and next runnable unit. If context, time, or resources run out, checkpoint and commit first. If no runnable work remains because a gate needs external input, ask only for that exact input and include the completed work. Resume from the ledger when it arrives.

## Paste into Qwen3 Code

Qwen3 sessions use the two prompts in [encyclopedia-procedures.md](encyclopedia-procedures.md): a drafter session and a separate verifier session. They do not read this plan, the roadmap, or the coverage inventory; the unit packet carries what a unit needs. A person runs the one-time setup in that file first.
