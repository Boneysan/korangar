# Qwen3 encyclopedia completion loop

**Mission:** complete the Adventure Guide for the active Hercules server revision against the [E0–E7 roadmap](encyclopedia-roadmap.md) and [field inventory](encyclopedia-coverage.md). Work through the whole in-scope backlog, not a fixed number of records. Players should be able to answer what something does, where to get or use it, and which conditions change that answer. A specific, visible unknown is acceptable when source or live state cannot establish a fact.

**Driver:** Qwen3 works in bounded source-grounded units, validates each one, and resumes from a durable ledger. It may publish a claim after the gates below succeed; routine per-record human approval is not required. Attribute model-assisted reviews honestly. Ask a person only for an inaccessible source or live session, or a real product decision.

## Setup and current baseline

As of 2026-09-28, E0–E7 are partial. The roadmap and coverage inventory hold current counts and open fields; do not create a new permanent denominator. Both `korangar/` and `Hercules/` already have uncommitted work. Inspect both Git statuses and diffs before editing; never reset, overwrite, or silently fold in existing edits. In particular, local `Hercules/db/quest_db.conf` is dirty: `tools/export_quest_reference.py` intentionally reads its tracked `HEAD` snapshot and reports the fallback. A revision plus `source_worktree_dirty: true` alone is inadequate provenance for a changed file; record the exact digest of the cited content or cite the tracked snapshot.

The host has a 16 GiB RTX 5060 Ti and about 62 GiB RAM. Ollama was not installed when this plan was written. Follow [Ollama's Linux setup](https://docs.ollama.com/linux), pull [`qwen3-coder:30b`](https://ollama.com/library/qwen3-coder), and inspect `ollama ps` for CPU/GPU offload. Its listed package is 19 GB, so expect partial CPU offload. If throughput or memory makes it impractical, use [`qwen3:14b`](https://ollama.com/library/qwen3) under the same gates. Start with an explicit 8K–16K `num_ctx`; Ollama's [default context is smaller](https://docs.ollama.com/faq). Prefer [schema-constrained JSON](https://docs.ollama.com/capabilities/structured-outputs) at temperature 0 for extraction. Log model tag, quantization, Ollama version, context, offload, and prompt version. None of these settings proves a claim.

Install [Qwen Code](https://github.com/QwenLM/qwen-code/blob/main/docs/users/quickstart.md) as the file-editing agent (for example, with Node.js 22+ and `npm install -g @qwen-code/qwen-code@latest`); plain `ollama run` is a chat interface and cannot itself execute this repository loop. Qwen Code [supports Ollama through its OpenAI-compatible endpoint](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/). From `korangar/`, after the model is pulled and Ollama is serving, launch:

```sh
qwen --auth-type openai --model qwen3-coder:30b \
  --openai-api-key ollama --openai-base-url http://localhost:11434/v1
```

Set Ollama's `num_ctx` to match the chosen packet size; Qwen Code's context setting alone does not enlarge the server context. Paste the instruction at the end of this file into the agent. Qwen Code may ask for file-edit approval; use its session acceptance mode if you want the authorized loop to proceed unattended. A new session resumes by reading the ledger and using the same instruction.

## State and work units

Use [`encyclopedia-qwen3-progress.json`](encyclopedia-qwen3-progress.json) as the resume ledger. Update it atomically after each accepted unit. Keep a concise per-unit journal at `docs/plans/encyclopedia-qwen3-journal.md` with evidence, decision, coverage delta, checks, unresolved questions, and next action. Keep large raw model responses in a local staging area outside the repository.

At every session start, read this plan, roadmap, coverage inventory, [data contract](../specs/encyclopedia-data.md), ledger, generated coverage, loaded-script manifest, and Git status of both repositories. Compare revisions and dirty-file digests with the checkpoint. Reopen any unit affected by source changes. Never claim coverage of unloaded scripts, live SQL, runtime state, or a client session based on a static scan.

Unit states are `todo`, `active`, `done`, `blocked`, and `stale`. A blocker names the missing input, its effect, and the condition that would unblock it. A blocked unit does not stop other units.

Build a finite queue from generated coverage plus loaded-source inventories. One unit is a complete NPC branch/shared helper, stock declaration family, item-script pattern, skill/formula/status family, quest chain, spawn/flag/route family, or client journey. Give each a stable ID, package, source scope, expected output, and exit check. Split a call graph that cannot fit in context. Select units in this order: E0 infrastructure blocking other work; broken/stale claims; common player journeys and custom content; common patterns unlocking many records; then remaining candidates in deterministic path/ID order. Interleave E7 checks as slices become usable.

Every in-scope candidate eventually needs a final disposition: verified/conditional/configured estimate with exact evidence; a specific unknown because source or live state cannot resolve it; or a documented absence after searching the declared scope. A clue or `not reviewed` remains open work. Never turn model confidence, a nearby call, or dialogue alone into a verified fact.

## Repeat loop

```text
repeat:
  1. Read ledger, roadmap, coverage, loaded-source manifest and both Git statuses.
     Recompute candidate queue; mark changed-source units stale.
  2. Pick the highest-priority runnable unit. If none exists, audit exit gates.
  3. Assemble a bounded packet: executable branch, called helpers, active config/
     DB records, exact path and lines, stable IDs, source revision/digest.
  4. Trace reachable branches affecting the claim; draft evidence and unknowns.
  5. Validate schema, IDs, loaded scope, exact citations/quotes, revision/digest,
     links, and all stated conditions mechanically.
  6. Independently reread source for costs, rewards, failure paths, prerequisites,
     random outcomes, timers and exceptions; correct or downgrade unsupported claims.
  7. Update review manifest/exporter/client, regenerate relevant data and coverage,
     run exporter --check and focused checks for code changed, inspect rendered
     claims, and compare with live server when claiming live behavior.
  8. Journal source/evidence, accepted files, before/after coverage, checks, open
     questions and next unit. Atomically checkpoint the ledger.
  9. If blocked, record why and move to another unit. If repeated attempts make
     no progress, stop with a concrete diagnostic and resume point.
until all E0–E7 exit gates and the final live pass are recorded as met.
```

Do not call a unit done merely because JSON was generated or an exporter passed. Add a small source fixture or citation-validation case for each new reviewed family. Preserve source bugs and dialogue/script mismatches as documented behavior; server fixes require a separate task.

## Package queue

| Package | Work | Exit evidence |
|---|---|---|
| E0 Evidence | Audit claim types, clue links, empty results, denominators, per-assertion labels, source snapshots and citation validation. | Item, quest, NPC, map flag, spawn and formula samples show correct states; no clue is shown as confirmed; indexed, reviewed, unknown and open counts differ. |
| E1 Acquisition | Classify loaded grant/removal branches, exchanges, drops, shops, crafting, containers and quest rewards. Trace costs, gates, outcomes and repeat limits. | Verified paths have exact quantities, source, conditions and reciprocal item/NPC/quest/map links; unresolved dynamic paths have reasons. |
| E2 NPC services | Review ordinary shop/trader/barter/cash and dynamic stock/roles; maintain reviewed literal service calls. Trace fees, availability, destinations and routes. | Literal, conditional, runtime and unknown offers are distinct; claimed routes are reachable. |
| E3 Effects | Cluster unsupported item/card/combo scripts; implement only proven patterns. Explain trigger, target, scaling, chance, duration, condition and interaction. | Pattern examples match code; translated and fully explained counts differ; unsupported fields remain visible. |
| E4 Skills and rules | Trace skill/status/combat code, effective config precedence, formulas, cures, immunity, stacking, interruption, jobs and account rules. | Formulas have units, order, rounding, modifiers and reconciled examples; an icon or call site alone is not called an effect. |
| E5 Quests | Trace in-scope families from eligibility through objectives, branches, turn-in, reward, reset and failure, including quests without nearby grant clues. | Each published journey is followable; each reward is on a cited completion path; unknown stages show. |
| E6 World | Resolve static/scripted/dynamic spawns, boss phases, map-zone overrides, flags, entrances and routes; separate configured from live data. | Map/species relations have correct states; rule order is cited or unknown; live and route claims have observations. |
| E7 Release | Search aliases/effects/sources; inspect cross-links, spoiler labels, empty states and non-DM access; run build/export checks and live journeys. | Dated fresh non-DM item → source → route, skill → status → cure, and quest → objective → turn-in passes against the running server; no dead-end verified link. |

The exact field backlog and denominators are in the coverage inventory and `docs/encyclopedia-coverage.v1.json`; refresh them during the loop. Scope includes all supported categories, including jobs and account rules. A dynamic/live-only fact may finish as unknown only with the precise reason and best verified static answer. Missing live E7 access is a blocker, never a pass.

## Completion and resume rule

Finish only when the [roadmap checklist](encyclopedia-roadmap.md#completion-checklist) passes for a named server revision and loaded-source scope, every in-scope candidate has a final disposition, all E0–E7 exit evidence is journaled, exports and relevant code checks pass, and dated fresh non-DM live journeys pass. An unresolved clue, unreviewed supported field, stale citation, failed check, or absent live pass keeps its package open. No arbitrary target count or fixed iteration limit substitutes for these gates.

At the end of each session, print the ledger location, completed units, coverage delta, checks, blockers, and next runnable unit. If context, time, or resources run out, checkpoint first. If no runnable work remains because a gate needs external input, ask only for that exact input and include the completed work. Resume from the ledger when it arrives.

## Paste into Qwen3 Code

```text
Work in /media/bigz/T7/GitHub/Ragnarok_Online/korangar on the Adventure Guide
encyclopedia. Read docs/plans/encyclopedia-local-model-plan.md in full, then
docs/plans/encyclopedia-roadmap.md, docs/plans/encyclopedia-coverage.md,
docs/specs/encyclopedia-data.md and docs/plans/encyclopedia-qwen3-progress.json.
Follow the repeat loop across all E0–E7 packages. Preserve both repositories'
dirty work; never reset or overwrite unrelated edits. Cite exact loaded Hercules
source, stable IDs, conditions and source revision/digest for every claim. Leave
unsupported facts visibly unknown. Finish one bounded unit, validate it, update
exports and coverage, then checkpoint the ledger and journal before moving on.
Continue from the ledger across sessions. Stop only when the full exit rule is
met or no runnable task remains because of a recorded blocker. Report the
precise next action at every checkpoint.
```
