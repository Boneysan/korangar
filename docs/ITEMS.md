# Items Database - Full Loot Table Source

**Source:** Hercules_RO `db/re/item_db.conf` + `db/re/mob_db.conf`

**Total items:** 13182
**Items with drops:** 2205

This is the master item list for building DM loot/reward tables. Combine with bestiary.json (mob levels) to generate level-appropriate rewards.

**Recommended for loot:**
- Filter by Type + Buy price + mob level from bestiary.
- Use DropsFrom for specific mob loot tables.
- Common consumables (potions, etc.) have high drop rates from low mobs.

Full machine-readable data (all fields + complete drop lists) in `items.json`.

### Sample Items with Drops
- **Orange_Potion** (ID 502, Type IT_HEALING, Buy 200): A_LUNATIC:2000, A_ANCIENT_MUMMY:2000, A_MUNAK:2000
- **White_Potion** (ID 504, Type IT_HEALING, Buy 1200): DRAKE:5000, MOONLIGHT:1500, TAO_GUNKA:3000
- **Blue_Potion** (ID 505, Type IT_HEALING, Buy 5000): DOPPELGANGER:6000, NIGHTMARE:100, DARK_PRIEST:100
- **Red_Herb** (ID 507, Type IT_HEALING, Buy 18): FARMILIAR:700, SPORE:800, ARCHER_SKELETON:1800
- **Yellow_Herb** (ID 508, Type IT_HEALING, Buy 40): SCORPION:200, PECOPECO:200, THIEF_BUG_MALE:90
- **White_Herb** (ID 509, Type IT_HEALING, Buy 120): VERIT:600, THARA_FROG:30, GHOUL:700
- **Blue_Herb** (ID 510, Type IT_HEALING, Buy 60): SPORE:50, NIGHTMARE:500, MEGALODON:80
- **Green_Herb** (ID 511, Type IT_HEALING, Buy 10): HORNET:350, FABRE:700, RODA_FROG:300
- **Apple** (ID 512, Type IT_HEALING, Buy 15): PORING:1000, PORING:150, POPORING:5
- **Banana** (ID 513, Type IT_HEALING, Buy 15): YOYO:1500, CHOCO:5000, L_CHOCO:5000


See `items.json` for everything. For a ready-to-use loot table generator, we can script filters by mob level ranges next.