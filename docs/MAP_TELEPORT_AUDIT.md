# Map and Teleport Safety Audit

This is the canonical engineering entry point for diagnosing black maps,
missing terrain, immobile characters after a warp, and unsafe scripted
teleport destinations. Agents and engineers working on maps, campaign beats,
warpers, instances, or client asset updates should run this audit before live
testing.

## What the audit checks

The `map-asset-audit` binary:

1. Reads every map enabled in Hercules `conf/map/maps.conf`.
2. Reads and decrypts the configured GRF file tables and map assets.
3. Parses each available RSW and its referenced GND/GAT.
4. Recursively scans supplied Hercules NPC files/directories for static
   `warp`, warp-portal, and `DM_WarpParty` destinations.
5. Checks destination bounds and the GAT walkable flag.
6. Reports a nearest walkable coordinate for unsafe destinations.

It does not prove dynamic coordinates supplied through variables or GM input.
Those require runtime validation. Missing GAT results mean the current client
archives cannot validate that destination; they are not permission to alter
the server script blindly.

## Run it

From `korangar/korangar`:

```bash
cargo run --release --bin map-asset-audit -- \
  ../../Hercules/conf/map/maps.conf \
  data.grf rdata.grf \
  ../../Hercules/npc
```

Focused DM campaign gate:

```bash
cargo run --release --bin map-asset-audit -- \
  ../../Hercules/conf/map/maps.conf \
  data.grf rdata.grf \
  ../../Hercules/npc/custom/dm_campaign/shared/dm_beats.txt
```

The command exits nonzero when map/reference failures or unsafe destinations
remain. Do not suppress that exit code in CI.

## Current baseline and backlog

- The focused DM campaign scan has zero unsafe statically analyzable party
  warps after the 2026-07-11 remediation.
- The full Hercules scan found 5,677 static destinations.
- The exact unresolved inventory is preserved in
  [reports/teleport-audit-2026-07-11.md](reports/teleport-audit-2026-07-11.md).
- M1 live verification history and the Arc 19 root cause are in
  [plans/M1-p0-verification.md](plans/M1-p0-verification.md).

## Triage rules

- `missing rsw` or `missing referenced ...`: acquire/mount the matching client
  asset set or remove the server map from supported scope.
- `rsw/gnd/gat parse`: treat as a client format compatibility defect.
- `missing gat` on a teleport: preserve it in the backlog until matching client
  data is available.
- `nearest walkable` on active custom content: review the intended landmark,
  update the warp and any coupled NPC/boss/hazard coordinates together, run
  `@reloadscript`, and verify movement plus the destination marker live.
- Stock, inactive pre-renewal, instance, and battleground coordinates need
  contextual review; never bulk-rewrite them solely from the nearest-cell
  suggestion.

## Arc 19 example

`moc_fild22` originally appeared black and the player could not move because
Arc 19 used non-walkable void cells `(150,150)` and `(155,150)`. The map assets,
lighting, terrain, and pathing parsed correctly. Moving the encounter to
walkable `(170,140)` and `(175,140)` restored visible ground, movement, the
destination marker, and Central Choice interaction. This is the model for
distinguishing an asset/render failure from an unsafe spawn coordinate.
