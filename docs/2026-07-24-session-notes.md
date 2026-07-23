# 2026-07-24 — E2 batch 2 live pass, ground-field status text, disconnect crash

Branch: `agent/platform-connectivity-controls` (korangar) +
`agent/map-teleport-safety` (Hercules).
Previous: `3a231aaf` (batch-2 blueprint + runbook).

## 1. Phase E2 batch 2 — 6 units wired, 4 live-verified

Recipes in `world/unit_recipe.rs`; details and the live table live in
[plans/classic-effect-fidelity.md](plans/classic-effect-fidelity.md).

| Unit | Live |
|---|---|
| Volcano / Deluge / Violent Gale | PASS (Deluge and Gale are palette swaps — that *is* the original's design) |
| Land Protector | PASS as a low square glow |
| Venom Dust / Demonstration | **not yet cast** — the looping-sprite path still has zero live evidence |

Hunter traps stay deferred: their originals are RSM props and models are baked
into map vertex buffers at load, so runtime prop spawning is its own pipeline
task.

### The scaling law, learned the expensive way

**Effect-table sizes are the effect's own world units, not cells.** Reading
`PropertyGround`'s `top 3.0 / bottom 1.0` as cells and multiplying by
`GAT_TILE_SIZE` made each cone 6 cells wide — and since Hercules `Layout: 3`
is a **7×7 square the server sends 49 separate `AddSkillUnit` packets**, the
field rendered as one solid block of fire. Batch 1's Sanctuary/Magnus were
already at world-unit scale; that was the tell we missed.

Corollary: **anything per-cell is paid N times.** Land Protector is 121 cells
at Lv5 and 225 at Lv10, so per-cell alpha, light intensity and quad count all
matter. Alphas ended up *higher* than theory suggested anyway (the recurring
"ground effects want more brightness" lesson).

### Draw-order limit — effects have no depth

`EffectInstruction` carries only screen-space corners and renders in
`passes/postprocessing/effect.rs`, so **every** effect composites over the
whole scene, entities included. Vertical bodies get away with it; Land
Protector's flat ground quad landed on top of the player sprite. Fixing it
properly needs a depth-tested ground-decal pass (the forward pass's
`indicator.rs` is the closest thing but is a singleton in the global uniform).
Deferred; `UnitGroundQuad` stays wired and asset-audited for that day, and LP
uses the low square glow Sanctuary/Magnus already passed with.

## 2. Ground-field status text

`[G+] Groundmagic 245s` told the player nothing. Hercules gives
`SC_VOLCANO`, `SC_DELUGE` and `SC_VIOLENTGALE` the **same** icon
(`SI_GROUNDMAGIC`, 112 — all three declare it in `db/re/sc_config.conf`), so
no table change could tell them apart.

- **`world/skill_unit_registry.rs`** (new) — every live skill unit with its
  `UnitId` and position, maintained at the same lifetime points as
  `EffectHolder` (spawn / `RemoveSkillUnit` / entity removal / map change).
  Insert happens *before* the `unit_presentation` early return, so units with
  no recipe are still tracked. Resolves index 112 to the field the player is
  actually standing in.
- **`StatusChangePacket` already carried `value: [u32; 3]`** and all three
  registrations dropped it. Now forwarded, so descriptions quote the server's
  own numbers instead of re-deriving its formulas.
- **Server-side delta required.** `status_get_val_flag()` gates which statuses
  transmit their values, and these three were absent — the server sent
  `val1 = 1, val2 = 0`, so the UI could only render "+0". Added them in
  `Hercules/src/map/status.c`. **This is a C-source patch in the sibling tree;
  see CLAUDE.md §3b — it is lost on an upstream merge.**
- **Land Protector.** It grants no status at all because it acts on the ground
  rather than on people, so nothing told the player their magic was being
  suppressed. First tried a client-synthesised entry, but that could only show
  *that* you were inside, not *how long* — and faking the countdown would have
  meant copying `skill_db` durations into the client, where they would silently
  drift. Replaced with a real server status (below), and the synthetic
  machinery was removed so there is one source of truth.
  Live: `[ME·] Magnetic Earth 318s / Ground magic suppressed here`, the 318
  confirming *remaining* rather than full duration.

### Adding SC_LANDPROTECTOR — five places, all silent on failure

Hercules has no such status, so this is a fork-invented one. It spans
`status.h` (enum slot), `constants.conf` (**`SC_LANDPROTECTOR: 728` matching
the enum slot, and `SI_LANDPROTECTOR: 1150`**), `sc_config.conf`,
`skill_db.conf` (`StatusChange:` on the skill), and `skill.c`
(`skill_unit_onplace` / `skill_unit_onout`).

**Cost us a debugging round:** `sc_config.conf` and `skill_db.conf` both
resolve status names through `script->get_constant()` (`status.c:14522`,
`skill.c:25048`). With `SI_` added but `SC_` missing, both bindings were
rejected, `get_sc_type()` fell back to `SC_NONE`, and `sc_start` did nothing —
announced only by a `ShowWarning`. Worse, **server stdout goes to
`log/athena-start.out`, not `log/map.log`** (which stays empty), so an early
"no errors in the log" check was a false negative.

Our `onplace` deliberately diverges from upstream's `sg->limit` pattern
(`UNT_SUITON` et al) and sends `sg->limit - DIFF_TICK32(tick, sg->tick)` — the
actual remaining time, so walking into a half-expired field does not show a
full-length timer.

Live: `+30 ATK & MATK` (`5+lv*5`), `+15% Max HP` (`deluge_eff[]`),
`+15 Flee` (`lv*3`, flat — not a percentage).

## 3. Client crash on server disconnect — fixed

Stopping Hercules while in game panicked the client:
`Option::unwrap()` on `None` at `lib.rs:3015`. Deterministic: when both
servers drop together the **character**-server disconnect is delivered first
and runs `return_to_login_with_error`, clearing `saved_login_data`; the
map-server disconnect arm then unwrapped it. Now guarded exactly as the
character-server arm already was. Any server restart or network loss hit this.

`CharacterSelected` (`lib.rs:3152`) has the identical `.unwrap()` shape but
only fires post-selection — left alone rather than restructure a path with no
reproducible failure. Same family as `M1-017`.

## 4. Server behaviours that look like client bugs

All verified in Hercules source; none are ours.

| Symptom | Reality |
|---|---|
| Volcano does no damage | `NoDamage: true`. Grants `SC_VOLCANO` — a buff field, never a damage skill |
| It never goes away | `SkillData1` Lv5 = 300000 ms = 5 minutes |
| The other fields won't cast | `skill.c:18732`: *"the official implementation makes them fail to appear when casted on top of ANYTHING"* — a 7×7 Volcano blankets the area, so an overlapping Deluge spawns zero units |
| Land Protector cancelled my field | `skill.c:13505` — casting it calls `skill->clear_group()` on your existing elemental field |
| Nothing happens when I press the key | All four need a **Blue Gemstone**; LP needs a **Yellow** too. Rejections arrive as `ZC_ACK_TOUSESKILL` and print to chat — easy to lose in combat spam |
| I had to click myself to cast | `Range: 2` — the target cell must be within 2 cells |

**Diagnostics now distinguish these.** `spawn_skill_unit` logs *every* mapped
spawn with its body kind under `KORANGAR_PACKET_LOG`, not just unmapped ones.
`[skill-cast]` present + `[skill-unit] spawn` absent = the server rejected the
cast. That blind spot is what sent us chasing a phantom Land Protector
renderer bug.

## Next

- Cast Venom Dust (Assassin, `@item 716`) and Demonstration (Alchemist,
  `@item 7135`) — the only batch-2 units with no live evidence at all.
- Warp-window close path still leaves server `menuskill_id` set (from batch 1).
- Depth-tested ground-decal pass, if we want authentic flat ground effects.
