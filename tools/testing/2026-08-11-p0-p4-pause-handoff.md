# Pause handoff — P0–P4 testing work (2026-08-11 evening)

**P0 closed later the same evening** — full + shuffle both green. Prefer
[headless-next-steps.md](headless-next-steps.md) for current status. This file
remains the historical pause note and resume checklist used to finish P0.

| | |
|---|---|
| **Repo** | Korangar (Hercules sibling used as live server only) |
| **Paused** | Mid full-suite validation after large depth batch |
| **P0 outcome** | **GREEN** — see §2a below |
| **Scenario count** | **147** registered (`--list`) |
| **Unit tests** | Green: 52 packet + 11 harness |

---

## 1. What is done (code landed, partially live-proven)

### Suite depth
- Expectation **enforcement** with reviewed exemptions (stem match for partner labels).
- Negative scenarios: `trade-reject`, `trade-invalid-offers`, `identify-cancel`,
  `equip-wrong-job`, `shop-close`, `use-drop-failures`, `storage-persistence`.
- Session: `connection-state`, `character-select-invalid`.
- `channeling-start-stop` (wire API only).
- `dm-golden-beats` (arcs 1–3 + `@dmstatus` `[DM]`).
- Comma-separated `--scenario a,b,c`.
- `ACTION_COVERAGE` unit test in `scenarios/mod.rs`.
- Graffiti packet **`0x01C9`** `NotifySkillUnitGraffitiPacket` → `AddSkillUnit`.

### Skill expectations / sweep
- `prepare_skill_cast`: unidentified item, undead target, trap entity/cell,
  graffiti setup, Crucis undead splash.
- AL_CRUCIS / MC_IDENTIFY no longer need exemptions (no-damage-effect /
  identify-list paths).
- Soul Link partner job mapping (`soul_link_partner_job`).
- HT_REMOVETRAP: entity-targeted cast when a trap was placed; **still on silence
  allowlist** for Rogue/Stalker (no placer skill → silent).

### Integration runner fixes (important)
In `tools/testing/run-integration-tests.sh`:
- Login fixture emails are **`a@a.com`** (matches `DeleteCharacterPacket`).
- Char import: `enable_char_creation: true`, `deletion.delay: 0`.
- Free-slot auto-create in `TestContext::connect_as` (not hard-coded slot 0).
- `provision-effect-roster` creates chars via char-select, not after map login.

**Local MariaDB TCP admin used successfully:**
`INTEGRATION_DB_ADMIN=korangar_int` / `INTEGRATION_DB_ADMIN_PASSWORD=korangar_int`

---

## 2. Live validation status at pause

### Green (scoped / multi-scenario)
- smoke, connection-state, character-select-invalid, channeling-start-stop
- lifecycle create/delete/slot-switch-rejected (after email fix)
- provision-effect-roster (after free-slot + char-select create fix)
- dm-golden-beats
- skills-acolyte, merchant, hunter, soul-linker (scoped)
- HT_REMOVETRAP on Hunter → `no-damage-effect` after setup

### Full suite at pause — **not finished green**
Several full runs were **killed mid-flight** (process cleanup / pause). Last
meaningful mid-run evidence (`/tmp/full-suite2.log`):

| Failure | Cause | Fix status |
|---|---|---|
| character-create-delete (early run) | email ≠ `a@a.com` | **Fixed** in integration SQL |
| provision-effect-roster (early run) | create after map login | **Fixed** |
| **skills-rogue** | `HT_REMOVETRAP` SILENT after allowlist removal | **Fixed** — re-added to silence allowlist with Rogue/Stalker reason |

**Suite3** (`/tmp/full-suite3.log`) was restarted with HT_REMOVETRAP allowlist
restored and was **in progress** (around skills-knight) when paused. Servers
were shut down for a clean stop.

### 2a. P0 closed after resume (2026-08-11 late)

Both acceptance runs completed cleanly after the pause:

| Run | Archive | Summary |
|---|---|---|
| Full | `runs/20260811-161233.log` | 146 pass, 0 flaky, 0 fail, 1 expected-skip (`skills-novice`), 0 unknown packets |
| Shuffle `20260810` | `runs/20260811-171208.log` | same scenario counts; 173/66 packets; 0 unknown |

`skills-rogue` / `skills-stalker` held with `HT_REMOVETRAP` on the silence
allowlist. Fixture cleanup clean both times.

---

## 3. Resume checklist (next session)

1. **Ports free / MariaDB up**
   ```sh
   brew services start mariadb   # if needed
   # ports 6900/6121/5121 must be free
   ```

2. **Full suite (P0)**
   ```sh
   cd korangar
   HERCULES_DIR=../Hercules \
   INTEGRATION_DB_ADMIN=korangar_int \
   INTEGRATION_DB_ADMIN_PASSWORD=korangar_int \
   INTEGRATION_SKIP_BUILD=1 \
     tools/testing/run-integration-tests.sh
   ```
   Expect ~55–70 minutes. Watch for `skills-rogue` / `skills-stalker` silence
   and any unexpected unmet expectations.

3. **Shuffle (P0)**
   ```sh
   HERCULES_DIR=../Hercules \
   INTEGRATION_DB_ADMIN=korangar_int \
   INTEGRATION_DB_ADMIN_PASSWORD=korangar_int \
   INTEGRATION_SKIP_BUILD=1 \
     tools/testing/run-integration-tests.sh --shuffle 20260810
   ```

4. **If green:** update [headless-next-steps.md](headless-next-steps.md) §1 with
   final counts (pass / expected-skip / unknown / unmet).

5. **Optional remaining P1**
   - RG_CLEANER / SA_SPELLBREAKER / SG_FEEL still in `EXPECTATION_EXEMPTIONS`
   - HT_SPRINGTRAP still silence-allowlisted

6. **Optional P3**
   - `python3 tools/audits/flaky.py tools/testing/runs/*.log` after new full logs land
   - Intermittents: `SL_SMA`, `HT_SPRINGTRAP`, historically `MG_NAPALMBEAT` / `HP_BASILICA`

---

## 4. Do not casually reverse

- Empty `REVIEWED_UNKNOWN_PACKET_HEADERS` / zero-unknown gate
- Fixture email `a@a.com` (deletion breaks without it)
- `deletion.delay: 0` in integration char import
- HT_REMOVETRAP silence allowlist entry (Rogue/Stalker load-bearing)
- Archive `.partial` / `.scoped` / `.log` semantics

---

## 5. File map (this batch)

| Area | Files |
|---|---|
| Expectations / sweep | `examples/headless-tester/scenarios/skills.rs` |
| Session | `…/scenarios/session.rs` |
| Items / social / dm | `…/scenarios/items.rs`, `social.rs`, `dm.rs`, `gm.rs` |
| Coverage | `…/scenarios/mod.rs` |
| CLI multi-scenario | `…/main.rs` |
| Integration | `tools/testing/run-integration-tests.sh` |
| Context free-slot create | `…/context.rs` |
| Graffiti packet | `ragnarok-packets/src/lib.rs`, `version_20220406.rs` |
| Forward list | `tools/testing/headless-next-steps.md` |

---

## 6. Honest claim at pause

**Code and fixtures are in a good place for a green full run.** Many high-value
scenarios were live-validated. A **complete** `--scenario all` (and shuffle)
acceptance number has **not** been recorded yet because full runs were
interrupted. That is the first job on resume.
