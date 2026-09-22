# Solo and Party Monster EXP Measurement Matrix — Authoritative Reconstruction

## 1. Overview & Verification Summary

This document provides the authoritative protocol, formula, and live test verification for monster Base and Job Experience distribution under Hercules Renewal mechanics in both solo play and various party configurations.

The test scenario `solo-and-party-exp-measurement` in [`combat.rs`](file:///Volumes/T7/GitHub/Ragnarok_Online/korangar/korangar-networking/examples/headless-tester/scenarios/combat.rs) executes all required cases against a running Hercules server stack, verifying exact integer outcomes against the authoritative C source code.

| Case | Scenario Condition | Killer Level / Target | Party Configuration | Expected Base / Job EXP | Observed Base / Job EXP | Result |
|---|---|---|---|---|---|---|
| **Case 1** | Solo kill | Lv 1 vs Poring (Lv 1) | No party | Killer: **36 / 20** | P1: **36 / 20** | **PASS** |
| **Case 2** | 2-player even share | Lv 1 vs Poring (Lv 1) | 2 players, same map, spread 0 | Both: **18 / 10** | P1: **18 / 10**<br>P2: **18 / 10** | **PASS** |
| **Case 3** | 3-player even share | Lv 1 vs Poring (Lv 1) | 3 players, same map, spread 0 | All: **12 / 6** (truncation) | P1: **12 / 6**<br>P2: **12 / 6**<br>P3: **12 / 6** | **PASS** |
| **Case 4A** | Off-map party member | Lv 1 vs Poring (Lv 1) | 2 players, P1 on map, P2 off-map | P1: **36 / 20**<br>P2: **0 / 0** | P1: **36 / 20**<br>P2: **0 / 0** | **PASS** |
| **Case 4B.1** | Level spread > 15 (individual) | Lv 1 vs Poring (Lv 1) | 2 players, P1 Lv 1, P2 Lv 17 (spread 16) | P1: **36 / 20**<br>P2: **0 / 0** | P1: **36 / 20**<br>P2: **0 / 0** | **PASS** |
| **Case 4B.2** | Level spread > 15 + Renewal penalty | Lv 17 vs Poring (Lv 1) | 2 players, P1 Lv 1, P2 Lv 17 (spread 16) | P1: **0 / 0**<br>P2: **30 / 17** (85% penalty) | P1: **0 / 0**<br>P2: **30 / 17** | **PASS** |

---

## 2. Test Environment & Reference Data

### 2.1 Target Monster: Poring (`Hercules/db/re/mob_db.conf:165-172`)
```hocon
{
    Id: 1002
    SpriteName: "PORING"
    Name: "Poring"
    Lv: 1
    Hp: 60
    Sp: 0
    Exp: 36
    JExp: 20
    AttackRange: 1
    Attack: [8, 11]
    Def: 2
    Mdef: 5
    Stats: {
        Str: 6
        Agi: 1
        Vit: 1
        Int: 1
        Dex: 6
        Luk: 30
    }
    ViewRange: 10
    ChaseRange: 12
    Size: "Size_Medium"  // In re/mob_db.conf: Medium
    Race: "RC_Plant"     // Non-Boss
    Element: ("Ele_Water", 1)
    Mode: {
        CanMove: true
        Looter: true
    }
}
```

### 2.2 Active Server Configuration Flags
- **`Hercules/conf/map/battle/exp.conf`:**
  - `base_exp_rate: 100` (100% rate multiplier)
  - `job_exp_rate: 100` (100% rate multiplier)
  - `exp_calc_type: 0` (standard attacker damage ratio share)
- **`Hercules/conf/map/battle/party.conf`:**
  - `party_default_share: 7` (bits: `0x1` item pickup share, `0x2` item division share, `0x4` even EXP share)
  - `party_even_share_bonus: 0` (no extra artificial bonus per party member)
  - `idle_no_share: 0` (idle members not excluded by inactivity timer)
- **`Hercules/conf/common/inter-server.conf`:**
  - `party_share_level: 15` (maximum allowed base level difference between members for even EXP sharing)
- **`Hercules/db/re/level_penalty.conf`:**
  - Index `0` (`diff = 0`): `rate: 100` (100% award when monster level equals player level)
  - Index `-16` (`diff = 1 - 17 = -16`): `rate: 85` (85% award when player exceeds monster level by 16)

---

## 3. Authoritative Hercules C Mathematics & Formulas

### 3.1 EXP Calculation: `pc_calcexp` (`src/map/pc.c:11517-11560`)
When a monster dies, Hercules evaluates experience through `pc->calcexp`:
```c
base_exp = (uint64)exp * battle_config.base_exp_rate / 100;
job_exp  = (uint64)exp * battle_config.job_exp_rate / 100;

#ifdef RENEWAL
if (battle_config.mob_level_penalty && mob_level) {
    int penalty = pc->level_penalty_mod(mob_level - sd->status.base_level,
                                        race, mode, 1);
    if (penalty != 100) {
        base_exp = (uint64)apply_percent(base_exp, penalty);
        job_exp  = (uint64)apply_percent(job_exp, penalty);
    }
}
#endif
```
- For a Level 1 player killing Level 1 Poring:
  - `diff = 1 - 1 = 0` $\rightarrow$ `penalty = 100%`.
  - $\text{base\_exp} = 36 \times 100 / 100 = 36$.
  - $\text{job\_exp} = 20 \times 100 / 100 = 20$.
- For a Level 17 player killing Level 1 Poring:
  - `diff = 1 - 17 = -16` $\rightarrow$ Looked up in `level_penalty.conf` index `-16`: `rate = 85%`.
  - $\text{base\_exp} = \lfloor 36 \times 85 / 100 \rfloor = \lfloor 3060 / 100 \rfloor = 30$.
  - $\text{job\_exp} = \lfloor 20 \times 85 / 100 \rfloor = \lfloor 1700 / 100 \rfloor = 17$.

### 3.2 Party Even Share Division: `party_exp_share` (`src/map/party.c:1068-1105`)
When `p->party.exp == 1` (even share active):
1. **Online & Same-Map Filter:**
   ```c
   c = 0;
   for (i = 0; i < MAX_PARTY; i++) {
       if (p->data[i].sd && p->data[i].sd->bl.m == src->m && !pc_isdead(p->data[i].sd)) {
           sd[c++] = p->data[i].sd;
       }
   }
   ```
   Only members on the exact same map (`sd->bl.m == src->m`) are counted in $c$. Off-map members are ignored, leaving $c$ smaller.
2. **Bonus & Integer Division:**
   ```c
   if (battle_config.party_even_share_bonus && c > 1) {
       base_exp += (uint64)apply_percent(base_exp, (c - 1) * battle_config.party_even_share_bonus);
       job_exp  += (uint64)apply_percent(job_exp,  (c - 1) * battle_config.party_even_share_bonus);
   }
   base_exp /= c;
   job_exp  /= c;
   ```
   With `party_even_share_bonus: 0`:
   - For 2 players on map ($c = 2$):
     $$\text{base\_exp} = \lfloor 36 / 2 \rfloor = 18$$
     $$\text{job\_exp} = \lfloor 20 / 2 \rfloor = 10$$
   - For 3 players on map ($c = 3$):
     $$\text{base\_exp} = \lfloor 36 / 3 \rfloor = 12$$
     $$\text{job\_exp} = \lfloor 20 / 3 \rfloor = 6$$
     *Notice the integer truncation*: $20 / 3 = 6$ (remainder 2 discarded). Total Job EXP awarded across the party is $6 \times 3 = 18$ instead of 20.
3. **Award Distribution:**
   ```c
   for (i = 0; i < c; i++) {
       pc->gainexp(sd[i], &src->bl, base_exp, job_exp, 0);
   }
   ```
   Members not in array `sd[0..c-1]` (e.g. off-map characters) receive nothing (`gainexp` is never called for them).

### 3.3 Level Spread Boundary Guard (`src/char/int_party.c:67, 418, 538`)
Char-server enforces the 15-level spread guard:
```c
static int inter_party_check_exp_share(struct party_data *const p)
{
    return (p->party.count < 2 || p->family != 0 || p->max_lv - p->min_lv <= party_share_level);
}
```
If a member's level change causes `p->max_lv - p->min_lv > 15`:
```c
if (p->party.exp == 1 && inter_party->check_exp_share(p) == 0) {
    p->party.exp = 0;
    mapif->party_optionchanged(&p->party, 0, 0);
    inter_party->tosql(&p->party, PS_BASIC, 0);
}
```
And any client request to enable even share (`0x07D7 PartyOptionsPacket`) is denied:
```c
if (exp && !inter_party->check_exp_share(p)) {
    flag |= 0x01; // Denied
    p->party.exp = 0;
}
```
When `p->party.exp == 0`, `party->exp_share` is bypassed. The killer receives individual EXP calculated via `pc_calcexp` directly, while other party members receive 0.

---

## 4. Protocol & Wire Format

Experience updates are communicated via the standard status update packets:

### 4.1 Packet `0x00BE` (`ZC_STATUS_CHANGE` / `StatChangePacket`)
```
Offset | Type   | Description
-------|--------|------------------------------------
0x00   | uint16 | Packet ID (0x00BE)
0x02   | uint16 | Stat type:
       |        |   0x0013 (19) = StatType::BaseExperience
       |        |   0x0016 (22) = StatType::JobExperience
0x04   | uint32 | New absolute value
```

### 4.2 Packet `0x07D8` (`ZC_GROUPINFO_CHANGE_V2` / `PartyOptionsChangedPacket`)
```
Offset | Type   | Description
-------|--------|------------------------------------
0x00   | uint16 | Packet ID (0x07D8)
0x02   | uint32 | exp_share (0 = each keeps own, 1 = shared evenly)
0x06   | uint8  | item_pickup_share (0 = individual, 1 = shared)
0x07   | uint8  | item_division_share (0 = individual, 1 = shared)
```

### 4.3 Inter-Server Packet `0x302a`
Added in commit `f5685861f` to synchronize party member job class and base level between map-server and char-server upon `@job` or job change. Ensured char-server and map-server binary compatibility across `make`.

---

## 5. Test Execution Log (`run-suite.sh`)

```
[Running] solo-and-party-exp-measurement (phase 4)
    [solo-and-party-exp-measurement] connecting primary...
    [solo-and-party-exp-measurement] Case 1: Solo kill...
    Case 1 Solo: gained Base EXP: 36, Job EXP: 20
    [solo-and-party-exp-measurement] Case 2: Two players in party (even share)...
    Case 2 (2 players): P1 gains 18 / 10, P2 gains 18 / 10
    [solo-and-party-exp-measurement] Case 3: Three players in party (integer truncation)...
    Case 3 (3 players): P1=12/6, P2=12/6, P3=12/6
    [solo-and-party-exp-measurement] Case 4A: Party member on different map...
    Case 4A (off-map): P1=36/20, P2=0/0
    [solo-and-party-exp-measurement] Case 4B: Level spread > 15 & level penalty...
    Case 4B.1 (P1 kill, spread > 15): P1=36/20, P2=0/0
    Case 4B.2 (P2 kill at Lv 17, 85% penalty): P1=0/0, P2=30/17
    [solo-and-party-exp-measurement] all cases passed cleanly.
[PASS] solo-and-party-exp-measurement (40.1s)

=== Summary: 1 passed, 0 flaky-pass, 0 failed, 0 expected-skip, 0 unexpected-skip, 0 known-fail ===
  PASS solo-and-party-exp-measurement
```

---

## 6. Discrepancy Analysis

All observed values match the theoretical expectations derived from the C code 100% without discrepancies:
1. **Solo award:** Exactly matches database `Exp: 36`, `JExp: 20` because `base_exp_rate = 100` and level difference is 0.
2. **Two-player even share:** Exactly divides rewards by 2 ($36/2 = 18$, $20/2 = 10$).
3. **Three-player truncation:** Proves integer truncation in Hercules C: $36/3 = 12$, $20/3 = 6$ (remainder 2 lost).
4. **Map boundary:** Confirms that `party_exp_share` filters members by map index (`sd->bl.m == src->m`), giving the full reward to the on-map killer and 0 to the off-map member.
5. **Level spread > 15:** Confirms that char-server disables even share when spread exceeds 15 (`inter_party_check_exp_share`).
6. **Renewal level penalty:** Confirms that when the higher-level character (Lv 17) kills the Lv 1 monster, `pc_level_penalty_mod` applies the 85% modifier from `db/re/level_penalty.conf`, yielding $\lfloor 36 \times 0.85 \rfloor = 30$ Base EXP and $\lfloor 20 \times 0.85 \rfloor = 17$ Job EXP.
