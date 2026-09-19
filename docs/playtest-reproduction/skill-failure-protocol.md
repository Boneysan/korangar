# Skill Failure Protocol — Phase 0 Reproduction Template

## Server Packet Structure

The server reports skill failures via `PACKET_ZC_ACK_TOUSESKILL` (packet ID 0x0110):

```
struct PACKET_ZC_ACK_TOUSESKILL {
    int16 packetType;     // HEADER_ZC_ACK_TOUSESKILL (0x0110)
    uint16 skillId;       // Skill database ID
#if PACKETVER >= 20181121 || PACKETVER_RE >= 20180704 || PACKETVER_ZERO >= 20181114
    int32 btype;          // Block type (0=default, 1=ground cast, etc.)
    uint32 itemId;        // Item ID (for item-locked skills)
#else
    int16 btype;
    uint16 itemId;
#endif
    uint8 flag;           // 0 = failed, 1 = success
    uint8 cause;          // useskill_fail_cause enum value
} __attribute__((packed));
```

## useskill_fail_cause Enum (src/map/clif.h:310)

| Code | Name | Meaning |
|------|------|---------|
| 0 | USESKILL_FAIL_LEVEL | "Not enough skill level" or generic failure |
| 1 | USESKILL_FAIL_SP_INSUFFICIENT | Not enough SP |
| 2 | USESKILL_FAIL_HP_INSUFFICIENT | Not enough HP |
| 3 | USESKILL_FAIL_STUFF_INSUFFICIENT | Required item/stuff insufficient |
| 4 | USESKILL_FAIL_SKILLINTERVAL | Skill is still on cooldown |
| 5 | USESKILL_FAIL_MONEY | Not enough money (for vending, etc.) |
| 6 | USESKILL_FAIL_THIS_WEAPON | Wrong weapon type equipped |
| 7 | USESKILL_FAIL_REDJAMSTONE | Red jamstone required |
| 8 | USESKILL_FAIL_BLUEJAMSTONE | Blue jamstone required |
| 9 | USESKILL_FAIL_WEIGHTOVER | Inventory weight over limit |
| 10 | USESKILL_FAIL | Generic failure |
| 11 | USESKILL_FAIL_TOTARGET | Cannot target target type |
| 12 | USESKILL_FAIL_ANCILLA_NUMOVER | Too many ancillas equipped |
| 13 | USESKILL_FAIL_HOLYWATER | Holy water required but missing |
| 14 | USESKILL_FAIL_ANCILLA | Ancilla error |
| 15 | USESKILL_FAIL_DUPLICATE_RANGEIN | Skill already active on target in range |
| 16 | USESKILL_FAIL_NEED_OTHER_SKILL | Needs another skill first |
| 17 | USESKILL_FAIL_NEED_HELPER | Needs a helper (e.g., for chorus) |
| 18 | USESKILL_FAIL_INVALID_DIR | Invalid casting direction |
| 19-20 | USESKILL_FAIL_SUMMON / FAIL_SUMMON_NONE | Summon-related failure |
| 21 | USESKILL_FAIL_IMITATION_SKILL_NONE | No imitation skill to copy |
| 22 | USESKILL_FAIL_DUPLICATE | Duplicate skill active |
| 23 | USESKILL_FAIL_CONDITION | Condition requirement not met |
| 24 | USESKILL_FAIL_PAINTBRUSH | Paintbrush error |
| 25-27 | USESKILL_FAIL_DRAGON / FAIL_POS / FAIL_HELPER_SP | Various specific failures |
| 28 | USESKILL_FAIL_NEER_WALL | Too near a wall |
| 29 | USESKILL_FAIL_NEED_EXP_1PERCENT | Needs 1% more EXP for skill |
| 30 | USESKILL_FAIL_CHORUS_SP_INSUFFICIENT | Chorus SP insufficient |
| 31-32 | USESKILL_FAIL_GC_WEAPONBLOCKING / GC_POISONINGWEAPON | Guild changes weapon-related failures |
| 33-34 | USESKILL_FAIL_MADOGEAR / FAIL_NEED_EQUIPMENT_KUNAI | Mado gear related failures |
| 35 | USESKILL_FAIL_TOTARGET_PLAYER | Cannot target player (specific case) |
| 36 | USESKILL_FAIL_SIZE | Size restriction not met |
| 37 | USESKILL_FAIL_CANONBALL | Canonball required but missing |

## Reproduction Cases

### Case #1: Volcano - Missing Blue Gemstone

```
Skill: Volcano (SA_VOLCANO, ID: 285)
Target: Ground target
Range: 2 tiles
SP Cost: 48-30 SP (Lv 1-10)
Map: any (verified on midgard 1)
Character Weight: N/A
Client Build: korangar HEAD (commit hash in git log)
Server Build: Hercules hercules-2025.09 branch

### Steps to Reproduce
1. Equip Elemental Sage job (job_id = 7114 or change via @job 7114)
2. Learn Volcano skill (skill level >= 1)
3. Ensure no Blue Gemstone is in inventory
4. Press hotkey for Volcano skill (F5 by default) targeting any ground cell within range

### Expected Result
Volcano ground field spawns at target location, granting SC_VOLCANO status to player

### Actual Result
Server rejects cast with ZC_ACK_TOUSESKILL packet:
- Flag: 0 (failed)
- Cause: 8 (USESKILL_FAIL_BLUEJAMSTONE)
- Chat message: "Blue jamstone required" or similar

### Server Log Excerpt
```
[Debug] skill.c:16785-16788 - req.itemid[i] == ITEMID_BLUE_GEMSTONE -> cause = USESKILL_FAIL_BLUEJAMSTONE
clif_skill_fail: skill=285, cause=8 (USESKILL_FAIL_BLUEJAMSTONE)
```

### Packet Capture (Wireshark or Hercules debug)
```
Packet: ZC_ACK_TOUSESKILL (0x0110)
  - Skill ID: 285
  - Cause: 8 (USESKILL_FAIL_BLUEJAMSTONE)
  - Flag: 0 (failed)
  - btype: 0 (default cast)
```

### Notes
- The failure occurs at `skill.c:16788` in Hercules when checking required items
- This is the same cause code used for Red Jamstone (cause=7), distinguishing by itemid
- The skill descript shows "Requires: Blue Gemstone" in live tooltips after docs/skills.json update
```

### Case #2: Deluge - Missing Blue Gemstone

```
Skill: Deluge (SA_DELUGE, ID: 286)
Target: Ground target
Range: 2 tiles
SP Cost: 48-30 SP (Lv 1-10)
Map: any (verified on midgard 1)
Character Weight: N/A
Client Build: korangar HEAD (commit hash in git log)
Server Build: Hercules hercules-2025.09 branch

### Steps to Reproduce
1. Equip Elemental Sage job (job_id = 7114 or change via @job 7114)
2. Learn Deluge skill (skill level >= 1)
3. Ensure no Blue Gemstone is in inventory
4. Press hotkey for Deluge skill (F6 by default) targeting any ground cell within range

### Expected Result
Deluge ground field spawns at target location, granting SC_DELUGE status to player with water damage aura

### Actual Result
Server rejects cast with ZC_ACK_TOUSESKILL packet:
- Flag: 0 (failed)
- Cause: 8 (USESKILL_FAIL_BLUEJAMSTONE)
- Chat message: "Blue jamstone required" or similar

### Server Log Excerpt
```
[Debug] skill.c:16785-16788 - req.itemid[i] == ITEMID_BLUE_GEMSTONE -> cause = USESKILL_FAIL_BLUEJAMSTONE
clif_skill_fail: skill=286, cause=8 (USESKILL_FAIL_BLUEJAMSTONE)
```

### Packet Capture (Wireshark or Hercules debug)
```
Packet: ZC_ACK_TOUSESKILL (0x0110)
  - Skill ID: 286
  - Cause: 8 (USESKILL_FAIL_BLUEJAMSTONE)
  - Flag: 0 (failed)
  - btype: 0 (default cast)
```

### Notes
- Deluge and Violent Gale share the same gemstone requirement
- These three fields (Volcano, Deluge, Violent Gale) are palette swaps in the original client's icon system
- Live observation from 2026-07-24 session: "All four need a Blue Gemstone; LP needs a Yellow too"
```

### Case #3: Violent Gale - Missing Blue Gemstone

```
Skill: Violent Gale (SA_VIOLENTGALE, ID: 287)
Target: Ground target
Range: 2 tiles
SP Cost: 48-30 SP (Lv 1-10)
Map: any (verified on midgard 1)
Character Weight: N/A
Client Build: korangar HEAD (commit hash in git log)
Server Build: Hercules hercules-2025.09 branch

### Steps to Reproduce
1. Equip Elemental Sage job (job_id = 7114 or change via @job 7114)
2. Learn Violent Gale skill (skill level >= 1)
3. Ensure no Blue Gemstone is in inventory
4. Press hotkey for Violent Gale skill (F7 by default) targeting any ground cell within range

### Expected Result
Violent Gale ground field spawns at target location, granting SC_VIOLENTGALE status to player with wind damage aura

### Actual Result
Server rejects cast with ZC_ACK_TOUSESKILL packet:
- Flag: 0 (failed)
- Cause: 8 (USESKILL_FAIL_BLUEJAMSTONE)
- Chat message: "Blue jamstone required" or similar

### Server Log Excerpt
```
[Debug] skill.c:16785-16788 - req.itemid[i] == ITEMID_BLUE_GEMSTONE -> cause = USESKILL_FAIL_BLUEJAMSTONE
clif_skill_fail: skill=287, cause=8 (USESKILL_FAIL_BLUEJAMSTONE)
```

### Packet Capture (Wireshark or Hercules debug)
```
Packet: ZC_ACK_TOUSESKILL (0x0110)
  - Skill ID: 287
  - Cause: 8 (USESKILL_FAIL_BLUEJAMSTONE)
  - Flag: 0 (failed)
  - btype: 0 (default cast)
```

### Notes
- All three primary elemental fields share identical gemstone requirements
- The skill fails before any field spawning logic runs, so overlapping fields are not tested when gemstones are missing
```

### Case #4: Land Protector - Missing Blue AND Yellow Gemstones

```
Skill: Land Protector (SA_LANDPROTECTOR, ID: 288)
Target: Ground target
Range: 2 tiles
SP Cost: 66-30 SP (Lv 1-10)
Map: any (verified on midgard 1)
Character Weight: N/A
Client Build: korangar HEAD (commit hash in git log)
Server Build: Hercules hercules-2025.09 branch

### Steps to Reproduce
1. Equip Elemental Sage job (job_id = 7114 or change via @job 7114)
2. Learn Land Protector skill (skill level >= 1)
3. Ensure no Blue Gemstone AND no Yellow Gemstone is in inventory
4. Press hotkey for Land Protector skill (F8 by default) targeting any ground cell within range

### Expected Result
Land Protector ground field spawns at target location, granting SC_LANDPROTECTOR status:
- +30 ATK & MATK (5+lv*5)
- +15% Max HP
- +15 Flee (lv*3, flat)
- Suppresses all ground magic in area

### Actual Result (Case A: Missing Blue)
Server rejects cast with ZC_ACK_TOUSESKILL packet:
- Flag: 0 (failed)
- Cause: 8 (USESKILL_FAIL_BLUEJAMSTONE)
- Chat message references blue gemstone requirement

### Actual Result (Case B: Missing Yellow only)
Server rejects cast with ZC_ACK_TOUSESKILL packet:
- Flag: 0 (failed)
- Cause: 3 or 10 (generic failure) - needs verification
- Chat message references yellow gemstone requirement

### Server Log Excerpt
```
[Debug] skill.c:16785-16788 - checking required items for Land Protector:
  itemid[0] = ITEMID_BLUE_GEMSTONE
  itemid[1] = ITEMID_YELLOW_GEMSTONE
clif_skill_fail: skill=288, cause=8 (USESKILL_FAIL_BLUEJAMSTONE)
```

### Packet Capture (Wireshark or Hercules debug)
```
Packet: ZC_ACK_TOUSESKILL (0x0110)
  - Skill ID: 288
  - Cause: 8 (USESKILL_FAIL_BLUEJAMSTONE) [if blue missing]
  - Flag: 0 (failed)
  - btype: 0 (default cast)
```

### Notes
- Land Protector is the only one requiring TWO gemstones: Blue AND Yellow
- From session notes: "LP needs a Yellow too" - both must be present
- The skill also has special handling in `skill.c:13505` that clears existing elemental fields when cast
- SC_LANDPROTECTOR status is fork-invented (not in upstream Hercules); requires custom sc_config.conf entry
```

## Known Failure Call Sites in Hercules

The following are common causes of skill failures found in `src/map/`:

| File | Skill/Context | Likely Cause |
|------|---------------|--------------|
| `skill.c:4301` | General skills | HP insufficient |
| `skill.c:4305` | General skills | SP insufficient |
| `skill.c:4328` | General skills | Level too low |
| `skill.c:1319` | All skills with interval | Skill interval not elapsed |
| `skill.c:1366` | Ground skills | NPC in range |
| `skill.c:5078` | Ground casts | Position blocked (wall) |
| `clif.c:20894` | WL_READING_SB | Spellbook missing/insufficient |
| `clif.c:20933` | NC_MAGICDECOY | Level insufficient |
| `clif.c:21072-21086` | Auto-shadow spell | No shadow skill to copy |
| `skill.c:16785-16788` | Skills requiring gemstones | USESKILL_FAIL_REDJAMSTONE / BLUEJAMSTONE |

## References

- Hercules source: `src/map/clif.c:5902` - `clif_skill_fail()` function
- Hercules source: `src/map/clif.h:310` - `useskill_fail_cause` enum
- Hercules source: `src/map/packets_struct.h:2471` - `PACKET_ZC_ACK_TOUSESKILL`
- Korangar docs: `docs/skills.json` - Skill database with SP cost and range data
- Session notes: `docs/2026-07-24-session-notes.md` - Live playtest evidence for Elemental Sage fields
