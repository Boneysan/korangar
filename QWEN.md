# Korangar — Qwen agent notes

Your job in this repository is the Adventure Guide encyclopedia loop. Everything
you need is in [docs/plans/encyclopedia-local-model-plan.md](docs/plans/encyclopedia-local-model-plan.md):
read it in full first, then resume from the ledger it names
(`docs/plans/encyclopedia-qwen3-progress.json`).

Hard rules, repeated here because they are the costly ones:

- Paths are relative to this repository root; Hercules is `../Hercules`.
- Edit review files under `tools/`, never the generated `docs/*.v1.json`.
- Never commit, stage, reset, or edit Hercules' local-only files, and never
  commit in `Hercules/` at all.
- Commit one accepted unit at a time in this repository, staging explicit
  paths only. Do not push.
- An unknown stated plainly beats a guess. Model confidence is not evidence.

`CLAUDE.md` holds notes for other work (client rendering, networking, the
headless test suite). You do not need it for the encyclopedia loop.
