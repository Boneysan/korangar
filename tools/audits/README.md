# Audits — runbook

Two audits live here. The packetver-variant check is below; the observer-parity
suite is the rest of this document.

## `packetver-variants.py` — is the client registering the *live* header?

```sh
cd korangar
python3 tools/audits/packetver-variants.py          # exits non-zero on a mismatch
```

**Re-run after any PACKETVER change or Hercules merge.** Many `ZC_` families
change header across packetvers — `idle_unitType` alone has nine values — and
registering the wrong one is **completely silent**: the packet is framed
correctly by `register_length_fallbacks`, consumed cleanly, and simply never
handled. No error, no ledger entry, no failing test. The feature just goes
quiet.

The `#if` chains are decided by the **C preprocessor**, not by reading them —
reading them by eye is how a stale variant gets certified as correct. Clean as
of 2026-08-09: 29 multi-variant families, 290 client headers, no stale variants.

Confidence that it works comes from pointing it at a packetver we are *not*
building: `--packetver 20140101` correctly reports `spawn_unitType` registered
as `0x09FE` where `0x09DC` would be live. Registering nothing is not a finding —
that is an unmodelled packet, which the length fallback handles by design.

# Observer-parity audits

Static checks that ask one question in ten places: **does state that reaches a
client out-of-band from the spawn packet have a recovery mechanism?** They exist
because six bugs in one session (2026-07-29) shared that shape, and four of the
six survived code review — they were only killed by measurement.

A seventh (probable) bug was found by A9 within an hour of the suite existing.

No server, no build, no GPU. Safe in CI. ~2 seconds.

## Run it

```sh
cd korangar
./tools/audits/observer-parity.sh
```

Needs the sibling `Hercules/` tree for the three server-side audits; without it
they are skipped and reported as `SKIP|hercules-tree-not-found=…`. Override the
location with `HERCULES_DIR=/path/to/Hercules`.

```sh
./tools/audits/observer-parity.sh --list     # findings only, no comparison
./tools/audits/observer-parity.sh --update   # reconcile the baseline in place
```

`--update` edits the baseline in place rather than regenerating it: rationale
comments stay next to the findings they explain (that adjacency *is* the audit),
vanished hits are dropped, and genuinely new hits are appended under a
`NEEDS CLASSIFICATION` banner so they cannot be mistaken for triaged ones. It is
idempotent — running it on a clean tree changes nothing.

## The rule that makes this different from a linter

> **An audit does not pass by returning nothing. It passes when every hit is
> classified.**

Most hits are *fine* — they have a recovery mechanism, or the fork does not use
that feature. Deleting them from the output would throw away the reason they are
fine, and the next person would re-derive it. So every hit lives in
`observer-parity.baseline` with a `# why:` comment above it, and the script only
complains about **changes**.

## Reading the exit code

| Exit | Means | Do this |
|---|---|---|
| **0** | Nothing new. | Read the `Classified but still open` list it prints — those are real, known bugs. Green means "nothing new", never "nothing wrong". |
| **1** | **Unclassified finding.** Something appeared that nobody has triaged. | Triage it (below). Then either fix it, or add it to the baseline **with a rationale**. Adding it without one defeats the tool. |
| **2** | **Stale baseline entry.** A hit no longer occurs. | Something was fixed or moved. Confirm which, then `--update` and commit the smaller baseline. |

## Triaging a new finding

Every audit is asking the same question, so triage is the same everywhere: **name
the recovery mechanism.** There are exactly three in this client.

1. **Carried by the spawn packet.** `EntityData` re-supplies it on every spawn
   and rebuild. Strongest — it covers spawn, respawn, entity rebuild, enter-view
   and login all at once, because `clif_getareachar_unit` calls `set_unit_idle`
   unconditionally.
2. **Lazily re-requested.** The client notices it is missing and asks again
   (entity names: `are_details_unavailable` → `set_details_requested`).
3. **Nothing.** The value arrives once. Miss it and it is gone until something
   unrelated resends it.

If the answer is 1 or 2, write it in the baseline and move on. **If the answer is
3, you have found a bug waiting for the right timing** — ammunition was
category 3, which is why it broke in four different ways.

## What each audit checks

| ID | Boundary | Question |
|---|---|---|
| **A1** | event → state | How many entity-keyed `find(...)` handlers exist? Tripwire on the count — a new one is a new handler to classify. |
| **A2** | event → state | Which `Common` fields are absent from `EntityData`? `AddEntity` rebuilds from `EntityData` alone, so anything not re-derivable is wiped on rebuild. |
| **A3** | state → sprite | Is any appearance state gated on `if let Self::Player`? Remote players are `Entity::Npc` and use `Common`, so `Player`-only state is invisible to every observer. **Run this before shipping headgear, robe or dye.** |
| **A4** | server → wire | Are any looks broadcast before `map->addblock`? An `AREA` send walks the map's block list, so those reach nobody. |
| **A5** | server → wire | Which enter-view re-sends are guarded so they cannot transmit "none"? Harmless for anything the spawn packet carries — see the baseline. |
| **A8** | server state | What writes `sd->vd` wholesale? A `memcpy` over the struct zeroes every fork-added field. **Any new field on `view_data` must be re-checked here.** |
| **A9** | event → state | Which mutations does the local-player path perform that the `AddEntity` path does not, or vice versa? The two are separate code that nothing keeps in step. Valid classifications: "derived from `EntityData` at construction" or "transient". |
| **B2a** | wire → event | Which `SpriteChangeType` variants never become a `NetworkEvent`? |
| **B2b** | wire → event | What is `register_noop`'d — i.e. things the server says and the client ignores? |
| **B2c** | wire → event | Which spawn-packet fields does `EntityData` discard? |

Not scriptable, and deliberately absent:

- **A6 — fallback values that collide with real ones.** A diagnosability rule,
  not a grep. When testing a path that has a fallback, choose a probe value the
  fallback cannot produce (Silver Bullet `13201`, not `13200`, which *is* the
  default). When writing one, log *which branch ran*, not just the result.
- **A7 — the two-session harness.** That is runtime, not static. See
  [observer-parity-harness.md](../../docs/plans/observer-parity-harness.md) §5.

## Adding an audit

1. Add a block to `findings()` in the script. Emit one line per hit via
   `emit <ID> "<stable-key>=<value>"`.
2. Make the key **churn-resistant** — a symbol name or a normalised source
   fragment, never a line number. The baseline is diffed literally, so a key that
   moves with unrelated edits produces false failures and the tool gets ignored.
3. `--update`, then write the `# why:` comment for every hit it added.
4. Note it in the table above and in
   [observer-parity-audits.md](../../docs/plans/observer-parity-audits.md).

## When to run

- **Whenever a `NetworkEvent` is added** — A1 and A2 are for exactly that moment.
- **Before shipping any appearance feature** (headgear, robe, dye, guild emblem)
  — A3 and B2c are what stop it landing in the known trap.
- **After any upstream Hercules merge** — A4, A5 and A8 read server source that
  the merge can silently change out from under the fork.
- **In CI**, on every commit. It is two seconds and it has already found three
  live gaps nobody was looking for.

## Related

- [docs/plans/observer-parity-audits.md](../../docs/plans/observer-parity-audits.md)
  — what each check is looking for, and the bugs that motivated it
- [docs/plans/observer-parity-harness.md](../../docs/plans/observer-parity-harness.md)
  — the four boundaries, the runtime harness, and the fix order
- [docs/plans/observer-view-verification.md](../../docs/plans/observer-view-verification.md)
  — the manual two-client checklist these were derived from
