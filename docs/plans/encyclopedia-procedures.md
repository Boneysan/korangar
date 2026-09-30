# Encyclopedia unit procedure

This is how a local model (Qwen3 Coder, or any model that follows commands
reliably but should not choose its own work) completes encyclopedia units.
The tool `tools/encyclopedia_unit.py` finds the unit, builds the packet,
validates the draft, records the verifier's verdict, runs the exporters and the
Rust parse test, and commits. The model reads one packet and writes one JSON
draft. Nothing else.

The queue order and unit definitions come from
[encyclopedia-completion-plan.md](encyclopedia-completion-plan.md). The
templates the packets copy from are in
[encyclopedia-unit-templates.md](encyclopedia-unit-templates.md).

## Before the first session (a person, once)

Run from `korangar/`:

1. Commit or discard the uncommitted encyclopedia work already in the tree.
   `commit` refuses to include files that were dirty before a unit started, so
   the loop stops at the first commit until this is done.
2. `python3 tools/encyclopedia_unit.py queue` builds or refreshes
   `docs/plans/encyclopedia-unit-queue.json` from loaded Hercules source.
   Rebuilding keeps every unit's state and history.
3. `python3 tools/encyclopedia_unit.py model qwen3-coder:30b` (use the exact
   tag you run). Accepted records are attributed to that tag.
4. `python3 tools/encyclopedia_unit.py e0` checks the evidence-label rules.

## Session A: the drafter

Start a fresh session in `korangar/` and paste:

```text
You are completing one encyclopedia unit. Work in korangar/.
1. Run: python3 tools/encyclopedia_unit.py next
   If it prints STOP, report the output and end the session.
2. It names a packet.md. Read that file and no other file.
   Do not open the roadmap, the queue, docs/*.v1.json, or Hercules files.
3. Write draft.json at the path the packet gives, following its template.
   When you cannot tell what the source does, write an "unknown"
   disposition that quotes the line. Never guess.
4. Run: python3 tools/encyclopedia_unit.py validate
   If it lists errors, fix draft.json and run validate again. Fix only
   what the errors name. After five failed validates, run
   python3 tools/encyclopedia_unit.py block --reason "<first error>"
   and end the session.
5. When validate says to hand off, end the session.
If the tool drafted the unit itself, run validate, then accept, then commit,
and end the session.
Do not edit any other file. Do not run git yourself.
```

`next` always resumes an open unit before starting a new one. If a verifier
rejected the previous draft, it tells the drafter to read `packet.md` and the
newest `feedback-N.md`, then fix or downgrade each listed claim to `unknown`.
If the unit is waiting for a verifier, `next` prints STOP.

## Session B: the verifier

Start a **new** session. It must not be the drafter's session: the point is a
reader who has not seen the draft's reasoning. Paste:

```text
You are verifying one encyclopedia unit. Work in korangar/.
1. Run: python3 tools/encyclopedia_unit.py verify
2. Read the verify.md it names and no other file.
3. Judge every numbered claim against the source lines shown. A claim the
   source does not show is "unsupported", even if it sounds right.
   List anything the source shows that the claims leave out.
4. Write verdict.json next to verify.md in the shape verify.md gives.
5. Run: python3 tools/encyclopedia_unit.py verify --record
6. If it says accept, run: python3 tools/encyclopedia_unit.py accept
   then: python3 tools/encyclopedia_unit.py commit
Stop at any line starting with STOP. Do not edit any other file.
```

## What the tool guarantees

- A draft cannot cite a line outside its packet, quote text that is not on the
  cited line, or cite a line without quoting it.
- An exchange cannot list an item or amount that differs from the quoted
  `delitem` / `getitem`, or leave a quoted one out.
- Every item call (E1), quest call (E5), skill ID (E4), and declaration (E2)
  in scope must be covered by the new draft or by an already accepted record.
- Two rejected drafts, or two failed accepts, block the unit with a reason.
- `accept` restores the review files if the exporters, the `--check` pass, or
  the Rust parse test fail.
- Every tenth accepted unit is flagged for a stronger model's audit
  (`audit`); see `status`.

## What it does not guarantee

- That a supported claim is complete. The verifier is asked for omissions, and
  the audit samples accepted units, but neither proves completeness.
- Anything on screen. E7 stays blocked until the user ends the live-test
  deferral.
- Strong-tier units. They are too large or branchy for a local model's packet
  and wait for `next --tier strong` with a stronger model or a person.
