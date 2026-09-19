# Sitting HP/SP Regeneration at Weight Thresholds — Phase 0 Verification

## Problem Statement

**From the plan (lines 61-62):**
> Verify sitting HP/SP regeneration at normal weight and at every overweight threshold.

Specifically measured, source-analyzed, and automated-verified:
1. Exact base standing and sitting regeneration interval timers in Hercules.
2. Exact base HP and SP restore amount formulas in Hercules source (`status.c`).
3. Behavior across overweight thresholds: normal (<50%), overweight (>=50%, <90%), and severe overweight (>=90%).
4. Behavior under status ailment suppression (`SC_POISON`).
5. Resource capping controls when character is at 100% capacity.

---

## Test Environment

| Parameter | Value |
|-----------|-------|
| Client / Runner | `korangar-networking` headless tester (`sitting-regeneration-thresholds`) |
| Server Build | Hercules `0e86020f` (branch `agent/map-teleport-safety`, Packetver `20220406`, Renewal) |
| Test Venue | `prontera` (155, 180) |
| Character Profile | Level 99 Novice (Max HP: 2535, Max SP: 510, Max Weight: 24300, VIT: 1, INT: 1) |
| Live Execution Suite | `./tools/testing/run-suite.sh --scenario sitting-regeneration-thresholds` |

---

## Hercules Source & Mechanics Analysis

Source files inspected in `Hercules/src/map/`:
- `status.c:2691-2696`: `status_calc_regen_pc` (Amount formulas)
- `status.c:13973-14030`: `status_natural_heal_sub` (Tick intervals & sitting bonuses)
- `status.c:13954-13960`: `status_natural_heal_sub` (Overweight blocking check)
- `status.c:2658-2668`: `status_calc_regen_pc` (Status ailment suppression)
- `status.c:12487-12490`: `status_change_timer` (Poison 25% HP termination boundary)
- `conf/battle/battle.conf`:
  - `natural_healhp_interval: 6000` (Standing HP interval = 6.0s)
  - `natural_healsp_interval: 8000` (Standing SP interval = 8.0s)

### 1. Amount Formulas (`status.c:2692-2693`)
```c
regen->hp = 1 + (st->vit / 5) + (st->max_hp / 200);
regen->sp = 1 + (st->int_ / 6) + (st->max_sp / 100);
```
For our test character (Max HP = 2535, VIT = 1; Max SP = 510, INT = 1):
- **HP per tick:** `1 + (1 / 5) + (2535 / 200) = 1 + 0 + 12 = 13 HP`
- **SP per tick:** `1 + (1 / 6) + (510 / 100) = 1 + 0 + 5 = 6 SP`

### 2. Sitting Timer Multiplier (`status.c:13978-13984`)
When sitting (`vd->dead_sit == 2`):
```c
hp_bonus *= 2;
sp_bonus *= 2;
```
Interval formula (`tick = max(cap, interval * 100 / bonus)`):
- **Standing HP Tick Interval:** `6000ms * 100 / 100 = 6000ms (6.0s)`
- **Standing SP Tick Interval:** `8000ms * 100 / 100 = 8000ms (8.0s)`
- **Sitting HP Tick Interval:** `6000ms * 100 / 200 = 3000ms (3.0s)` (2x frequency)
- **Sitting SP Tick Interval:** `8000ms * 100 / 200 = 4000ms (4.0s)` (2x frequency)

### 3. Overweight Natural Regen Suppression (`status.c:13954`)
```c
if (flag != 0 && regen->state.overweight != 0) {
    flag = 0;
    if (sc != NULL && sc->data[SC_TENSIONRELAX] != NULL)
        flag |= RGN_HP | RGN_SHP;
}
```
Both `SC_WEIGHTOVER50` (weight >= 50%) and `SC_WEIGHTOVER90` (weight >= 90%) assert `regen->state.overweight = 1`. This zeroes the natural healing flag (`flag = 0`), which completely halts natural and sitting HP and SP regeneration (unless overridden by Tension Relax).

### 4. Poison Suppression (`status.c:2658-2668`)
```c
if ((sc->data[SC_POISON] != NULL && sc->data[SC_SLOWPOISON] == NULL)
    || (sc->data[SC_DPOISON] != NULL && sc->data[SC_SLOWPOISON] == NULL)
    ...
) {
    regen->flag = 0;
    return;
}
```
Under `SC_POISON`, `regen->flag` is zeroed, blocking all natural recovery. In addition:
- `status_zap` inflicts poison tick damage: `val4 = 3 + st->max_hp * 3 / 200` every 1000ms (~41 HP/s).
- `status_change_timer` at line 12488 stops and ends `SC_POISON` if `st->hp <= max(st->max_hp / 4, sce->val4)` (HP <= 25%).
- Green Potion (item 506) runs `sc_end SC_POISON` to cure poison cleanly.

---

## Live Measured Test Results

Executed via `./korangar/tools/testing/run-suite.sh --scenario sitting-regeneration-thresholds`:

### Section 1: Normal Weight (<50%), Sitting
- **Starting State:** Weight 0/24300 (0.0%), HP damaged to 2135, SP damaged to 390
- **Observation Window:** 13.0 seconds
- **Observed HP Ticks:** 4 ticks of exactly **+13 HP**
  - T+2.5s: 2148
  - T+5.6s: 2161 (+3.1s interval)
  - T+8.5s: 2174 (+2.9s interval)
  - T+11.5s: 2187 (+3.0s interval)
- **Observed SP Ticks:** 3 ticks of exactly **+6 SP**
  - T+3.5s: 396
  - T+7.5s: 402 (+4.0s interval)
  - T+11.5s: 408 (+4.0s interval)
- **Verdict:** PASS (matches exact 3.0s HP and 4.0s SP doubled sitting intervals; matches exact formula delta +13 HP / +6 SP).

### Section 2: Normal Weight (<50%), Standing
- **Starting State:** Weight 0/24300 (0.0%), HP damaged, SP damaged
- **Observation Window:** 25.0 seconds
- **Observed HP Ticks:** 4 ticks of exactly **+13 HP**
  - T+4.4s: 1800
  - T+10.4s: 1813 (+6.0s interval)
  - T+16.4s: 1826 (+6.0s interval)
  - T+22.3s: 1839 (+5.9s interval)
- **Observed SP Ticks:** 3 ticks of exactly **+6 SP**
  - T+6.4s: 294
  - T+14.4s: 300 (+8.0s interval)
  - T+22.3s: 306 (+7.9s interval)
- **Verdict:** PASS (matches exact 6.0s HP and 8.0s SP standard standing intervals).

### Section 3: Overweight Threshold (50%–89%)
- **Starting State:** Loaded with Steel (item 999) to Weight 14600/24300 (**60.1%**), damaged HP/SP
- **Standing Observation (7.0s):** 0 HP ticks, 0 SP ticks
- **Sitting Observation (10.0s):** 0 HP ticks, 0 SP ticks
- **Verdict:** PASS (natural and sitting regeneration 100% blocked).

### Section 4: Severe Overweight Threshold (>=90%)
- **Starting State:** Loaded with Steel to Weight 23100/24300 (**95.1%**), damaged HP/SP
- **Sitting Observation (10.0s):** 0 HP ticks, 0 SP ticks
- **Verdict:** PASS (natural and sitting regeneration 100% blocked).

### Section 5: Status Ailment Suppression (`SC_POISON`)
- **Starting State:** Item 12238 (New Year Rice Cake) consumed, `SC_POISON` active (`health_state & 0x0001 != 0`), initial HP 2135, initial SP 390
- **Sitting Observation (9.0s):**
  - SP Regeneration: **0 ticks** across 9s sitting
  - HP Progression: Strictly decreasing from periodic poison zap damage (~41 HP every 1000ms), **0 upward regeneration ticks**
- **Cure Verification:** Item 506 (Green Potion) consumed, verified `health_state & 0x0001 == 0` (cured)
- **Verdict:** PASS (natural regeneration blocked under poison; poison cured cleanly by Green Potion).

### Section 6: Full Capacity Control (100% HP/SP)
- **Starting State:** Healed to 100% (HP: 2535, SP: 510)
- **Sitting Observation (7.0s):** **0 heal events** received
- **Verdict:** PASS (server produces 0 redundant heal packets when resource bars are full).

---

## Conclusion & Verification Status

QW-023 is **DONE**:
1. Every weight threshold, timer interval, sitting bonus multiplier, and status ailment suppression rule was verified in Hercules C source.
2. Live two-channel HP/SP telemetry across all 6 threshold sections passed in automated test suite `./tools/testing/run-suite.sh --scenario sitting-regeneration-thresholds` in 87.7s.
