# Implementation Plan — Asset & Client Data Pipeline

| | |
|---|---|
| **Status** | Draft implementation plan |
| **Milestone** | E2 → M1 |
| **Parent** | [PROJECT_PLAN.md](../PROJECT_PLAN.md) E2, [SOFTWARE_DESIGN.md](../SOFTWARE_DESIGN.md) §6 |
| **Depends on** | M0 connectivity |

## 1. Scope

Define how Korangar loads RO assets and client-side data for this private server:

- GRF placement and archive order.
- `System/` data audit: item names, skill data, quest data, message strings.
- BGM path handling.
- Custom content sync between Hercules DB/scripts and client-facing files.

This plan is about repeatable asset/data setup. Feature-specific UI work stays in
[FEATURE_ROADMAP.md](../FEATURE_ROADMAP.md) and `docs/specs/`.

## 2. Inputs

Official client install:
- `/mnt/h/RO/client/data.grf`
- `/mnt/h/RO/client/rdata.grf`
- `/mnt/h/RO/client/renewal2021.grf`
- `/mnt/h/RO/client/resources2021.grf`
- `/mnt/h/RO/client/System/`
- `/mnt/h/RO/client/BGM/`
- `/mnt/h/RO/client/NavigationData/`

Korangar:
- Default archive list: `data.grf`, `rdata.grf`, `archive/`.
- Runtime archive list settings file: `korangar/client/game_archives.ron`
  (`client/game_archives.ron` after the binary changes cwd into `korangar/`).
- Built-in overrides: `korangar/archive/data/`.

Hercules_RO:
- `db/` item, mob, skill, quest data.
- `npc/custom/dm_campaign/` and its planning/tooling data.

## 3. Decisions

### D-A1 — GRF Storage

Options:
- Symlink to `/mnt/h/RO/client/*.grf`.
- Copy GRFs into `korangar/` on WSL ext4.

Recommendation:
- Use symlinks only for M0 if already present.
- Copy all four GRFs into WSL for day-to-day development if load-time or random
  access is visibly slow.

Decision record:
- [ ] Benchmark symlinked startup/map load.
- [ ] Benchmark copied startup/map load.
- [ ] Record final choice in [SOFTWARE_DESIGN.md](../SOFTWARE_DESIGN.md) §6.2.

### D-A2 — Archive Order

Runtime lookup target, highest priority first:

1. Custom loose `data/` or generated override archive.
2. `archive/` built-in Korangar overrides.
3. `resources2021.grf`
4. `renewal2021.grf`
5. `rdata.grf`
6. `data.grf`

Implementation detail:
- `GameFileLoader::add_archive` inserts each loaded archive at index `0`, so the
  lookup priority is the reverse of the `archives` array in
  `korangar/client/game_archives.ron`.
- To get the runtime priority above, write the settings list from lowest to
  highest priority:
  ```ron
  (
      archives: [
          "data.grf",
          "rdata.grf",
          "renewal2021.grf",
          "resources2021.grf",
          "archive/",
          "data/",
      ],
  )
  ```
- Omit `"data/"` until a loose custom overlay exists.

### D-A3 — Custom Content Packaging

Options:
- Loose `data/` overlay for rapid iteration.
- Extra GRF/7z archive for distribution.

Recommendation:
- Loose overlay in dev.
- Generated distribution archive later, after custom data sync is scripted.

## 4. Steps

1. Capture current archive state.
   - List `korangar/*.grf` and symlink/copy status.
   - Dump or create `korangar/client/game_archives.ron`.
   - Confirm which path wins when the same file exists in two archives.

2. Register all four GRFs.
   - Add `renewal2021.grf` and `resources2021.grf` to the archive list.
   - Verify maps, sprites, item icons, effects, and loading screens still resolve.

3. Audit Korangar client-data usage.
   - Search `korangar-loaders`, `ragnarok-formats`, and `korangar/src` for
     `itemInfo`, skill info, quest info, message tables, and Lua/LUB parsing.
   - Record each file as `used`, `ignored`, or `unknown`.

4. Map required `System/` data.
   - Item names/descriptions/icons.
   - Skill names/descriptions.
   - Quest journal entries, especially campaign IDs `20000–20234`.
   - Message strings used by packet/UI feedback.

5. Define sync source of truth.
   - Server DB/scripts remain authoritative.
   - Generated client data lives under a predictable path and is reproducible.
   - Existing Hercules tool `tools/campaign_quest_merge.py` is either reused or
     replaced by native Korangar codegen.

6. Wire BGM.
   - Decide copy vs reference for `/mnt/h/RO/client/BGM/`.
   - Verify BGM loads on at least the login map and Prontera.

7. Write the first sync script only after the audit.
   - Do not invent a broad converter before knowing what Korangar consumes.
   - Start with the smallest failing data class from M1 verification.

## 4a. Client-Data Usage Audit (2026-07-08)

What Korangar actually consumes from the archives, from `korangar/src/world/library/`
(all loaded through the regenerated-on-demand `lua_files.7z` cache — **delete
`korangar/lua_files.7z` whenever the archive list changes**, it only rebuilds
when missing):

| File (GRF path under `data\luafiles514\lua files\`) | Status | Consumer |
|---|---|---|
| `datainfo\iteminfo.lub` | **used** | `ItemInfo` (names, resources, descriptions) |
| `datainfo\jobidentity.lub` | **used** | `JobIdentity`, `IsBabyJob` |
| `datainfo\npcidentity.lub` | **used** | `JobIdentity` |
| `skillinfoz\jobinheritlist.lub` | **used** | skill info/requirements/tree |
| `skillinfoz\skillid.lub` | **used** | skill info/requirements/tree |
| `skillinfoz\skillinfolist.lub` | **used** | `SkillListInformation`, requirements |
| `skillinfoz\skilltreeview.lub` | **used** | `SkillTreeLayout` |
| `mapskydata\mapskydata.lub` | **used** (optional) | `MapSkyData` |
| `OngoingQuestInfoList*.lub` | **ignored** | nothing — no quest journal loader exists |
| `System/` (itemInfo.lua etc.) | **ignored** | Korangar reads GRF lub paths, not `System/` |

Quest markers over NPCs come from packets (`AddQuestEffect`), not client data.
A native quest journal (campaign IDs 20000–20234) is new E7 work; the
`OngoingQuestInfoList_True_EN.lub` copy in `korangar/` is reference material
for that, not something the client reads today.

Archive registration state (2026-07-08): all four GRFs copied to WSL ext4 and
listed in `client/game_archives.ron` (`data`, `rdata`, `renewal2021`,
`resources2021`, `archive/`); verified loading at startup. The regenerated
`lua_files.7z` shrank from 3.0 MB to 1.1 MB because the 2021 archives shadow
older duplicates.

## 5. Verification

Asset pipeline passes for M1 when:
- Client starts without missing core GRFs.
- Prontera map, player sprite, NPC sprites, item icons, effects, and BGM load.
- A known custom item has the correct name/icon/description in Korangar or is
  recorded as an explicit missing data task.
- Campaign quest data path is understood, even if the native journal is still E7.
- Re-running the setup from a fresh clone is documented.

## 6. Risks

- `/mnt/h` 9P I/O may be too slow for GRF random access.
- Archive precedence can be inverted if the list order is misunderstood.
- 2021-era client data may not line up cleanly with `PACKETVER=20220406`.
- Lua/LUB data may need a parser or codegen path rather than direct runtime use.
