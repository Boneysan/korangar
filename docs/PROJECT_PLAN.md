# Project Plan — HerculesRO Custom Client (Korangar-based)

| | |
|---|---|
| **Status** | Living document |
| **Owner / PM** | boneysan |
| **Last updated** | 2026-07-05 |
| **Architecture** | [SOFTWARE_DESIGN.md](SOFTWARE_DESIGN.md) — architecture & technical decisions |
| **Feature roadmap** | [FEATURE_ROADMAP.md](FEATURE_ROADMAP.md) — UI/product roadmap and packet-handler backlog |
| **DM interface design** | [DM_INTERFACE.md](DM_INTERFACE.md) — native tabletop tooling |
| **Implementation plans** | [plans/README.md](plans/README.md) — near-term executable plans |

**Mission:** Replace the official RO client (`H:\RO\client`) with a Korangar-based
client for the HerculesRO private server, reaching feature parity for everything
the server actually supports, then exceeding the official client with custom features.

---

## 1. Inventory — what we have today

### 1.1 `H:\RO` (Windows, `/mnt/h/RO` from WSL) — full server + client stack

| Path | Contents | Relevance to project |
|---|---|---|
| `client/2019-06-05fRagexe_patched.exe` | **Patched official client, packet version 2019-06-05** — matches Hercules's current `PACKETVER 20190605` exactly | Reference client; A/B comparison; ties us to 20190605 until retired |
| `client/data.grf`, `rdata.grf` | Base kRO asset archives | **Required by Korangar** (sprites, maps, models, effects) |
| `client/renewal2021.grf`, `resources2021.grf` | Updated/renewal asset archives | Must be added to Korangar's archive list (not just the default two) |
| `client/data/clientinfo.xml` | Points official client at `192.168.20.60:6900`, langtype 1, version 55 | Template for Korangar's `sclientinfo.xml` |
| `client/System/` | Client-side Lua databases: `LuaFiles514/` (iteminfo, skillinfo, quests, navigation…), `MsgString.lub`, `OngoingQuestInfoList*.lub`, `PetEvolutionCln*.lub`, `CheckAttendance.lub`, `OptionInfo.lub` | **Authoritative client-side data** — Korangar must get equivalent data (it parses GRF contents; how much of System/ it needs is an open question, §5-I3) |
| `client/AI/`, `client/AI_sakray/` | Homunculus / mercenary user AI (Lua) | Only needed when we implement homunculus (Tier 3) |
| `client/BGM/` | Background music (mp3) | Korangar audio; copy or reference |
| `client/NavigationData/` | Official navigation system path data | Only for navigation feature (Tier 3) |
| `client/Replay/`, `RagnarokReplay.exe` | Replay recordings + player | Out of scope (official-format replays) |
| `client/Skin/` | Official UI skins | Not applicable — Korangar has its own UI/theming |
| `client/PatchClient/`, `Patchup_RE.exe`, `Ragnarok.exe` | Official patcher/launcher chain | Replaced by our own distribution story (§4 E8) |
| `client/GameGuard.des` | GameGuard stub | Ignore — not used with Hercules |
| `server/` | **Windows copy of Hercules** (VS solutions) | Redundant with WSL Hercules_RO — decide single source of truth (§5-D4) |
| `bin/`, `etc/`, `usr/`, `www/`, `data/mariadb-10.6` | **Laragon web stack**: Apache, PHP, MariaDB 10.6, phpMyAdmin, Adminer, HeidiSQL, ngrok, memcached | DB admin + future control panel / patcher hosting |
| `setup-wsl-portforward.ps1` | Windows→WSL port forwarding for 6900/6121/5121 | Needed whenever the client runs on Windows/LAN |
| `HerculesRO-client.zip` | Packaged client distribution | Existing distribution artifact; superseded later |
| `client/System/itemInfo*.lua/.lub` | Client-side item names/descriptions/icons (multiple variants) | **P1** — Korangar must show correct names for our item DB incl. customs (E2.1) |
| `client/System/OngoingQuestInfoList_True_EN.lub` (+ dated backups) | Client quest journal — **actively maintained** by `Hercules_RO/tools/campaign_quest_merge.py` for the DM campaign (IDs 20000–20234) | Native Korangar journal must consume this data or its lua source (E7) |
| `client/System/Towninfo.lub`, `achievement_list.lub`, `ShadowTable.lub`, `monster_size_effect*.lub`, `tipbox.lub`, `Font/` | Minimap POIs, achievements, render tables, tips, fonts | Per-feature; mostly superseded by native Korangar equivalents |
| `client/savedata/` (`OptionInfo.lua`, `ChatWndInfo_U.lua`, `MiniPartyInfo.lua`) | Official client's persisted settings/window layout | Not migrated — Korangar has its own settings persistence |
| `client/tipOfTheDay.txt`, `GuildTip.txt` | Login tips | P3 cosmetic |
| `client/rsu-kro-rag-lite.exe`, `rsu-kro-renewal-lite.exe` | RSU updaters that pull official kRO data updates | Ops: how base GRFs get refreshed (feeds D3 re-copy) |
| `client/v3hunt.dll`, `patch_allow*.txt`, `patch2.txt`, `patchRE*` | Anti-cheat remnant + official patcher config | Ignore / replaced by E8 |

### 1.2 `~/GitHub/Hercules_RO` (WSL) — the live server

- Hercules with `login-server :6900`, `char-server :6121`, `map-server :5121`, plus `api-server`.
- `PACKETVER` = default `20190605` (matches the patched exe above). `ENABLE_PACKETVER_RE` commented out.
- `char_ip`/`map_ip` bound to `192.168.20.60` for LAN reachability.
- Local overrides in `conf/import/`.

**Custom content audit (answers I1):**
- **`npc/custom/dm_campaign/` — "Seal Cascade" DM campaign engine** ⭐: 19 arcs / 4 acts,
  ~30 `@dm*` chat commands (beats, decisions, flags, d20 checks w/ adv/dis + inspiration,
  initiative, encounters/scaling, hazards, traps, puzzles, scenes, downed rules, rewards,
  session log). Documented in `CAMPAIGN.md`; tooling in `planning/` + `tools/`
  (`campaign_quest_merge.py` writes the client quest journal). **Drives [DM_INTERFACE.md](DM_INTERFACE.md).**
- `npc/custom/` utilities: warper, healer, jobmaster, stylist, platinum_skills,
  card_remover, item_signer, itembind, breeder, resetnpc, itemmall, bartershop,
  expandedbartershop, woe_controller, battleground + bgqueue, zeroui, specialpopup,
  dialogalign/dialogpossize.
- `npc/custom/events/`: seasonal events (cluckers, disguise, halloween, mushroom,
  xmas rings, valentines, uneasy_cemetery).
- `npc/custom/quests/`: classic custom quests + hunting_missions.
- Client-facing implication: all of the above run through **standard NPC dialog,
  shops, and chat** — covered by P0/P1 rows in §2, no extra packets needed
  (except the DM campaign, which gets dedicated UI — E7).

### 1.3 `~/GitHub/korangar` — the client codebase

- Rust nightly workspace; wgpu rendering; builds on Linux/Windows/macOS.
- Speaks exactly **one packet version: 20220406** (`korangar-networking`).
- Loads `data.grf` + `rdata.grf` from `korangar/korangar/`; additional archives configurable; overrides in `korangar/archive/data/`; server list in `archive/data/sclientinfo.xml`.
- Built-in dev tools behind `debug` feature: packet inspector, profiler, theme/frame inspectors, command window.

---

## 2. Official RO client — feature catalog & gap analysis

Legend — **Korangar status**: ✅ implemented · 🔶 partial/present in code, unverified against our server · ❌ missing.
**Pri**: P0 = blocks the "playable" milestone · P1 = expected by any player week one · P2 = full-server parity · P3 = nice-to-have / niche.

### 2.1 Account & session
| Feature | Korangar | Pri | Notes |
|---|---|---|---|
| Login (user/password), `_m`/`_f` registration | ✅ | P0 | Verified flow exists incl. saved credentials |
| Server selection | ✅ | P0 | Via `sclientinfo.xml` |
| Character select / create / delete / slot switch | ✅ | P0 | Events for all incl. failure paths |
| PIN code (kRO second password) | ❌ | P3 | Disable on Hercules instead |
| Password encryption (`passwordencrypt`) | ❌ | P2 | Official clientinfo enables it — confirm Hercules setting vs plaintext (§5-I5) |

### 2.2 Core gameplay
| Feature | Korangar | Pri | Notes |
|---|---|---|---|
| Click-to-move, pathing | ✅ | P0 | |
| Basic attack / attack move | 🔶 | P0 | `AttackFailed` handling exists; verify melee+ranged vs Hercules |
| Sit / stand | ✅ | P0 | |
| Item pickup (ground items) | ✅ | P0 | |
| Stats window, stat point allocation | ✅ | P0 | |
| Skill tree, skill points, skill use (self/target/ground) | ✅ | P0 | Skill units (ground AoE) handled |
| Hotbar / hotkeys (incl. server-stored hotkeys) | ✅ | P0 | `SetHotkeyData` |
| Death → respawn / resurrect | ✅ | P0 | |
| Job change visuals, hair/appearance change | 🔶 | P1 | Events exist; verify sprite swaps for all jobs in our DB |
| Experience/level-up display, job exp | 🔶 | P0 | Verify |
| Weight limit / overweight indicators | ❌ | P1 | |
| Status effect icons (buffs/debuffs) | ❌ | P1 | Large icon set; server sends status packets |
| Elemental/size/race damage display conventions | 🔶 | P2 | Damage numbers exist |
| Mounts (Peco, Dragon, Mado, cash mounts) | ❌ | P2 | Sprite + state handling |
| Falcon / Warg / Cart display | ❌ | P2 | Needed for Hunter/Merchant lines |

### 2.3 Items & economy
| Feature | Korangar | Pri | Notes |
|---|---|---|---|
| Inventory (use/equip/drop), equipment window | ✅ | P0 | |
| NPC buy / sell | ✅ | P0 | Full cart flow implemented |
| Item identification (magnifier) | ❌ | P1 | |
| Player↔player trade | ❌ | **P1** | Core multiplayer economy |
| Kafra storage | ❌ | **P1** | |
| Guild storage | ❌ | P2 | After guilds |
| Cart inventory | ❌ | P2 | Merchant line |
| Vending (open shop, buy from shops) | ❌ | P2 | Merchant line; big packet surface |
| Buying stores | ❌ | P3 | |
| Refinement UI (+upgrade at NPC) | ❌ | P2 | NPC dialog may partially cover |
| Card compounding | ❌ | P2 | |
| Enchant / socket enchant UIs | ❌ | P3 | Script-driven; NPC dialogs may suffice |
| Item link in chat (`<ITEM>` tags) | ❌ | P3 | |
| Bank (zeny storage) | ❌ | P3 | 2013+ feature; server has it |
| Mail / RODEX | ❌ | P2 | Attachment economy; api-server may be involved |
| Auction house | ❌ | P3 | Rarely used on private servers |
| Cash shop UI | ❌ | P3 | Only if the server monetizes/uses cash points |
| Roulette / lapine / daily rewards UIs | ❌ | P3 | |
| Attendance check | ❌ | P3 | `CheckAttendance.lub` exists in H:\RO |
| Favorites tab / inventory sorting | ❌ | P3 | Korangar UI can do better natively |

### 2.4 Social
| Feature | Korangar | Pri | Notes |
|---|---|---|---|
| Public chat | ✅ | P0 | |
| Whisper / private messages | ❌ | **P1** | |
| Party chat / Guild chat channels | ❌ | P1 | Depends on party/guild |
| Chat rooms (in-map chat bubbles/rooms) | ❌ | P3 | Legacy feature |
| Emotes (`/emotion`, Alt+numbers) | ❌ | P1 | Sprite-based, well-documented packets |
| Friends list (add/remove/requests) | ✅ | P1 | Implemented |
| **Party**: create, invite, leave, kick, exp/item share, party HP bars, party options | ❌ | **P1** | Highest-value missing social feature |
| Party booking / party finder | ❌ | P3 | |
| **Guild**: create, invite, positions, notice, skills, emblem, expel, alliance/antagonist | ❌ | P2 | Emblems may touch api-server (`conf/import/emblems.conf` exists) |
| Guild war / WoE participation (castle UI, emperium) | ❌ | P2 | After guild core |
| Marriage / adoption | ❌ | P3 | |
| Ignore/block list | ❌ | P3 | |
| `/who`, `/where`, GM `@commands` via chat | 🔶 | P1 | Chat passthrough works for @commands; verify |

### 2.5 Combat systems
| Feature | Korangar | Pri | Notes |
|---|---|---|---|
| Skill visual effects (STR effect files) | 🔶 | P1 | Effect system exists (`world/effect`); coverage per-skill unknown — audit needed |
| PvP maps | 🔶 | P2 | Mostly server-side; verify name-color/kill feed |
| WoE / siege mechanics display | ❌ | P2 | |
| Battlegrounds (queues, team chat, score) | ❌ | P3 | |
| Duel system | ❌ | P3 | |
| Monster info display (`/mi`-like) | ❌ | P3 | Korangar could do this better natively |

### 2.6 Companions
| Feature | Korangar | Pri | Notes |
|---|---|---|---|
| Pets: taming, hatch, feed, performance, equip | ❌ | P2 | |
| Pet evolution | ❌ | P3 | `PetEvolutionCln.lub` data available |
| Homunculus (Alchemist line): spawn, feed, skills, AI | ❌ | P2 | User AI (Lua) is a design question — reimplement or skip custom AI (§5-D6) |
| Mercenary | ❌ | P3 | |

### 2.7 NPC & world interaction
| Feature | Korangar | Pri | Notes |
|---|---|---|---|
| NPC dialog: next/close/menu choices | ✅ | P0 | |
| NPC input: number entry, text entry | 🔶 | P0 | Verify both input packet types — many quests/services need them |
| Warps, map changes, airship | ✅ | P0 | `ChangeMap` |
| Kafra services (storage/save/teleport/cart) | 🔶 | P1 | Dialog-driven; storage window itself is ❌ (2.3) |
| Instances / memorial dungeons (UI + countdown) | ❌ | P3 | Server-driven; base entry may work via dialog |
| Quest log / journal (accept/track quests) | ❌ | P2 | Quest effects (map markers) ✅; journal window missing |
| Achievements & titles | ❌ | P3 | |
| Navigation system (cross-map route guidance) | ❌ | P3 | `NavigationData/` available; Korangar could ship a better native version |
| World map / minimap | 🔶 | P1 | `maps` window exists — verify minimap markers (party/guild/quest) |

### 2.8 UI / UX / client system
| Feature | Korangar | Pri | Notes |
|---|---|---|---|
| Settings: graphics / audio / interface / game | ✅ | P0 | Korangar exceeds official here (free aspect ratio, real lighting) |
| Camera control (rotate/zoom) | ✅ | P0 | |
| Screenshots | ❌ | P2 | Trivial with wgpu; official binds ScrollLock |
| Battle-mode keys / macro keys | ❌ | P2 | Korangar hotbar partially covers |
| Chat history, `/savechat` | ❌ | P3 | |
| Replay recording/playback | ❌ | P3 | Korangar has `packet_inspector`; a native replay would be a custom feature |
| BGM + sound effects | ✅ | P0 | `korangar-audio`; point at `H:\RO\client\BGM` |
| Video cutscenes | ✅ | P3 | `korangar-video` |
| Localization (msgstringtable) | 🔶 | P2 | Korangar has own strings; decide translation source (§5-I4) |
| Item/skill names, descriptions, icons (itemInfo.lua etc.) | 🔶 | **P1** | Must reflect our DB incl. campaign customs; verify Korangar's data source (E2.1) |
| Settings/window-layout persistence (savedata/) | ✅ | P0 | Korangar has its own settings system |
| Tip of the day / guild tips | ❌ | P3 | `tipOfTheDay.txt` available |
| Official patcher/launcher | ❌ | P2 | Replace with own updater (E8) |

### 2.9 HerculesRO custom systems (beyond the official client)
| Feature | Korangar | Pri | Notes |
|---|---|---|---|
| **DM campaign interface** (Seal Cascade) | ❌ | **P1 custom** | Flagship custom feature — full design in [DM_INTERFACE.md](DM_INTERFACE.md); tasks in E7 |
| Player dice rolls (`@roll`, GM level 0) | 🔶 | P1 custom | Works today as chat text; native dice cards in E7.2 |
| Campaign quest journal (IDs 20000–20234) | ❌ | P1 custom | Supersedes official `OngoingQuestInfoList` pipeline; E7.3 |
| Custom utility NPCs (warper, healer, jobmaster, stylist…) | ✅ | P0 | Standard dialog/shops — covered by existing NPC support |
| Battlegrounds queue (`bgqueue`) | ❌ | P3 | Same as §2.5 BG row |
| Item mall / barter shops | 🔶 | P2 | Barter uses extended shop packets — verify vs 20220406 |

**Summary:** Korangar covers the P0 single-player core loop nearly completely.
The dominant gaps are **multiplayer social/economy systems** (trade, storage,
whisper, party → guild → vending) and the **long tail of renewal-era subsystem UIs**.

---

## 3. Milestones

| # | Milestone | Definition of done | Exit criteria owner |
|---|---|---|---|
| **M0** | Connectivity | Korangar logs into Hercules_RO, creates a character, walks around Prontera | Packetver resolved; GRFs loading |
| **M1** | Playable solo | Level a novice → first job via combat, NPC quests, shops, kafra save; no crashes in a 2-hour session | All P0 verified against *our* server |
| **M2** | Playable together | Two clients: whisper, trade, party up, share exp, use storage | Trade/storage/whisper/party/emotes shipped |
| **M3** | Server parity | Every feature the Hercules_RO scripts/DB actually use works in-client (P1+P2 audit-driven) | Guild, vending, pets, quest log, status icons |
| **M4** | Better-than-official | **DM interface shipped (E7)** — dice cards, campaign journal, DM console; official client retired; distribution/patching in place | A full Seal Cascade session run natively in Korangar |

---

## 4. Work breakdown structure

Sizes: **S** ≤ ½ day · **M** ≤ 2 days · **L** ≤ 1 week · **XL** > 1 week (split before starting).
Feature tasks in E4–E6 all follow the same slice: *packets (`ragnarok-packets`) → events (`korangar-networking`) → state (`korangar/src/state`) → UI window (`korangar/src/interface`) → verify vs Hercules*.

### E1 — Connectivity & environment *(→ M0)*

Implementation plan: [plans/M0-connectivity.md](plans/M0-connectivity.md)

| ID | Task | Size | Depends on |
|---|---|---|---|
| E1.1 | **Record packet version decision** (see §5-D1 and SOFTWARE_DESIGN §5) | S | — |
| E1.2 | Rebuild Hercules with chosen `PACKETVER` (`./configure --enable-packetver=…`); confirm renewal/pre-renewal mode while at it | S | E1.1 |
| E1.3 | If keeping the official exe usable: obtain matching patched exe (e.g. 2022-04-06 via WARP) + updated translation data | M | E1.1 |
| E1.4 | Copy GRFs (`data`, `rdata`, `renewal2021`, `resources2021`) into WSL (benchmark `/mnt/h` first; expect to copy) and register all four in Korangar's archive settings | S | — |
| E1.5 | Write `sclientinfo.xml` entry for HerculesRO (127.0.0.1 for WSL dev; 192.168.20.60 profile for Windows) | S | E1.2 |
| E1.6 | Disable packet obfuscation + relax security on Hercules for dev (mirror upstream Korangar dev-server settings) | S | E1.2 |
| E1.7 | Build Korangar in WSL (nightly + slangc); confirm WSLg/GPU works, else document Windows-native build | M | — |
| E1.8 | **M0 demo**: login → char create → walk Prontera; record findings | S | E1.2–E1.7 |

### E2 — Asset & data pipeline *(→ M1)*

Implementation plan: [plans/asset-pipeline.md](plans/asset-pipeline.md)

| ID | Task | Size | Depends on |
|---|---|---|---|
| E2.1 | Audit which `System/` lub data Korangar needs vs. what it derives from GRFs (item names/descriptions, skill info) | M | E1.8 |
| E2.2 | Define client-data sync process: Hercules `db/` ↔ client-side item/skill data (script it) | M | E2.1 |
| E2.3 | Decide + implement custom-content packaging (extra GRF vs loose `data/` overlay; load order) | M | E1.4 |
| E2.4 | BGM path wiring; verify audio on our maps | S | E1.8 |
| E2.5 | Translation/localization source decision and import (§5-I4) | M | E2.1 |

### E3 — P0 verification pass *(→ M1)*
| ID | Task | Size | Depends on |
|---|---|---|---|
| E3.1 | Systematically test every ✅/🔶 P0 row in §2 against Hercules_RO; file a defect list | M | E1.8 |
| E3.2 | Fix defects found (budget) | L | E3.1 |
| E3.3 | Verify NPC input (number/text) with real quest scripts | S | E3.1 |
| E3.4 | Verify job change/appearance across our job DB | M | E3.1 |
| E3.5 | Add weight/overweight display | S | E3.1 |
| E3.6 | **M1 demo**: novice → first job, 2-hour stability session | M | E3.2 |

### E4 — Multiplayer core (Tier 1 features) *(→ M2)*

Protocol-safety starter plan: [plans/packet-gap-party-whisper.md](plans/packet-gap-party-whisper.md)

| ID | Task | Size | Depends on |
|---|---|---|---|
| E4.1 | Whisper / private messages (+chat channel routing UI) | M | M1 |
| E4.2 | Emotes | S | M1 |
| E4.3 | Player↔player trade (request/confirm/item+zeny/cancel) | L | M1 |
| E4.4 | Kafra storage window (open/store/retrieve/close) | M | M1 |
| E4.5 | Party: create/invite/leave/kick/options/exp share/party HP overlay | L | M1 |
| E4.6 | Status effect icons (buff/debuff bar) | M | M1 |
| E4.7 | Item identification (magnifier) | S | M1 |
| E4.8 | Minimap markers (party members, quest) | M | E4.5 |
| E4.9 | **M2 demo**: two-client session script | S | E4.1–E4.5 |

### E5 — Guild & economy (Tier 2) *(→ M3)*
| ID | Task | Size | Depends on |
|---|---|---|---|
| E5.1 | Guild core: create/invite/roster/positions/notice/expel/leave | XL → split | M2 |
| E5.2 | Guild skills, emblem upload/display (investigate api-server role) | L | E5.1 |
| E5.3 | Guild storage | M | E5.1, E4.4 |
| E5.4 | Alliance/antagonist + WoE display (castle owner, emperium HP) | L | E5.1 |
| E5.5 | Cart inventory + falcon/warg/mount display states | M | M2 |
| E5.6 | Vending: open shop / browse / purchase | L | E5.5 |
| E5.7 | Pets: tame/hatch/feed/performance/equip | L | M2 |
| E5.8 | Quest log window (accept/track/complete states) | M | M2 |
| E5.9 | Refinement + card compounding UIs | M | M2 |
| E5.10 | Screenshots + chat history QoL | S | M2 |

### E6 — Long tail (Tier 3, audit-driven)
Only schedule what the Hercules_RO content audit (E6.0) shows is actually used.
| ID | Task | Size |
|---|---|---|
| E6.0 | **Audit Hercules_RO scripts/DB** for features actually enabled (instances? BG? bank? RODEX? attendance?) — this gates everything below | M |
| E6.1 | RODEX mail | L |
| E6.2 | Homunculus (incl. AI decision §5-D6) | XL |
| E6.3 | Instances/memorial dungeon UI, battlegrounds, achievements, titles, bank, attendance, marriage, navigation, mercenary, auction, cash shop | per-audit |

### E7 — DM interface & custom features (the payoff) — design: [DM_INTERFACE.md](DM_INTERFACE.md)
Player-facing pieces can start right after M1 (they only need chat + quest data);
the DM console lands alongside M2/M3.

| ID | Task | Size | Depends on |
|---|---|---|---|
| E7.1 | Server-side structured echo: `$dm_client_mode` flag + `[DMJ]{…}` machine-readable lines in dm_campaign scripts (Phase A transport) | M | — (script-only) |
| E7.2 | Player dice cards: parse `@roll`/`@dm check` results → animated d20 result cards (roll/mod/DC/pass/nat-20 flair) | M | M1, E7.1 |
| E7.3 | Native campaign quest journal (IDs 20000–20234, data from `campaign_quest_journal_entries.lua` via build-time codegen) | M | M1 |
| E7.4 | Inspiration token indicator + downed/death-save overlay | S | E7.2 |
| E7.5 | Initiative tracker bar (player + DM reorder view) | M | E7.2 |
| E7.6 | DM check console + rewards drawer (`@dm check/inspire`, `@dmreward`, exp presets) | M | E7.2 |
| E7.7 | DM campaign board + beat director + decision ledger (static data codegen from CAMPAIGN.md/planning lua; `@dmbeat`/`@dmdecide`/`@dmflag`) | L | E7.1 |
| E7.8 | DM encounter panel (spawn palette w/ mob IDs, `@dmscale` slider w/ balance-checker presets, bloodied, manual fallback) | M | E7.7 |
| E7.9 | DM hazard/trap board + scene director (`@dmhazard/trap/symptom`, `@dmscene/cutscene/spotlight/say/secret`) | M | E7.7 |
| E7.10 | DM session HUD (`@dm mode/status`, recap viewer, warp/recall) | S | E7.7 |
| E7.11 | In-world hazard telegraphs (reuse skill-unit rendering) | M | E7.9 |
| E7.12 | Phase B: custom packets for initiative/hazard/encounter state (Hercules plugin + `ragnarok-packets`) — only if Phase A parsing proves limiting | XL | E7.5–E7.9 |
| E7.13 | General QoL parking lot: better `/mi`, native navigation, damage meters, loot filters, replay | — | groom during M3 |

### E8 — Operations & distribution *(→ M4)*
| ID | Task | Size |
|---|---|---|
| E8.1 | Decide single Hercules source of truth (WSL vs `H:\RO\server`) and archive the other (§5-D4) | S |
| E8.2 | DB backup routine for MariaDB (laragon stack) | S |
| E8.3 | Client distribution: build pipeline for Windows binary + assets (replaces PatchClient/`HerculesRO-client.zip`) | M |
| E8.4 | Auto-update story (simple manifest + downloader, or rpatchur-style) | L |
| E8.5 | Optional: control panel (FluxCP-equivalent) on the laragon stack; account registration flow | L |
| E8.6 | Upstream strategy: rebase cadence on vE5li/korangar, PR generic fixes upstream | S, recurring |

### E9 — Quality (continuous)
| ID | Task |
|---|---|
| E9.1 | Per-milestone manual test script (login → … → logout) kept in `docs/` |
| E9.2 | Packet round-trip unit tests for every packet added in E4–E6 |
| E9.3 | Disposable-server integration harness (`athena-start` wrapper) |
| E9.4 | Packet captures from the official 2019 exe as ground truth when implementing Tier 1–2 packets |

---

## 5. Decisions needed & information to gather

### Decisions (blocking)
| ID | Decision | Options | Recommendation | Status |
|---|---|---|---|---|
| **D1** | Packet version | (a) Bump Hercules → 20220406, re-patch official exe if still wanted; (b) implement 20190605 in Korangar | **(a)** — one-line server change; WARP-patchable 2022-04-06 exes exist; (b) is weeks of Rust work | ✅ Decided 2026-07-05: bump Hercules to `20220406` |
| D2 | Renewal vs pre-renewal | Hercules compile flag + matching client data | Confirm current build mode first | ⏳ Open |
| D3 | GRF location | Copy into WSL vs symlink `/mnt/h` | Copy (9P too slow); rerun copy on asset updates | ⏳ Open |
| D4 | Single server source of truth | WSL `~/GitHub/Hercules_RO` vs `H:\RO\server` | WSL copy; archive the Windows one | ⏳ Open |
| D5 | Keep official client alive during transition? | Yes until M3 (ground-truth packet captures, E9.4) / No | Yes until M3 | ⏳ Open |
| D6 | Homunculus user-AI | Reimplement Lua AI hosting vs ship fixed built-in AI vs skip | Defer to E6 audit | ⏳ Open |

### Information to gather (non-blocking, assign early)
| ID | Question | Where to look |
|---|---|---|
| I1 | ~~Which features do Hercules_RO's scripts actually enable?~~ **Answered 2026-07-05** — see §1.2 audit: DM campaign (dedicated epic E7), utility NPCs (standard dialog), barter shops, battlegrounds, seasonal events, hunting missions. Still open: which `conf/` battle features + `db/` customs matter | `conf/import/`, `db/` in Hercules_RO |
| I2 | What is the api-server used for, and does the client need to speak to it (emblems, auth tokens)? | Hercules docs + `conf/api/` |
| I3 | Exactly which client-side data files does Korangar read vs ignore (lub files, txt tables)? | `korangar-loaders`, `ragnarok-formats` source |
| I4 | Translation source for item/skill descriptions (English)? | ROenglishRE-style projects vs current H:\RO System/ files (already partially EN) |
| I5 | Is `passwordencrypt` active on the server, and does Korangar support it? | Hercules login conf + `korangar-networking` login flow |
| I6 | Does Korangar's 20220406 implementation cover renewal stat packets our server sends? | Packet inspector during M0/M1 |
| I7 | GPU/WSLg performance for wgpu — is WSL dev viable or do we need Windows-native builds day one? | E1.7 |

---

## 6. Risk register

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | Packetver mismatch stalls everything until the server is rebuilt | High until E1.2 | High | Rebuild Hercules with `--enable-packetver=20220406`; decision D1 is closed |
| R2 | Korangar pre-alpha gaps larger than cataloged (§2 based on code survey, not runtime) | Medium | High | E3.1 verification pass immediately after M0 |
| R3 | Upstream Korangar moves fast; fork drifts | High | Medium | E8.6 rebase cadence; keep features in separate modules |
| R4 | 20220406 client data (lub/GRF) mismatches our 2019/2021-era assets | Medium | Medium | E1.3/E2.1; the official exe cares more than Korangar does |
| R5 | Solo-dev bandwidth vs XL scope (guild, vending are big) | High | Medium | Strict milestone gating; E6 is audit-gated, not wishlist-driven |
| R6 | WSL GPU/networking friction (WSLg, port forwarding) | Medium | Low | Windows-native client build path documented at E1.7 |
| R7 | `/mnt/h` I/O too slow for iteration | High | Low | D3: copy assets into ext4 |

---

## 7. Operating cadence

- **Board:** treat §4 tables as the backlog; work strictly milestone-by-milestone (M0 → M4).
- **Definition of done** for any feature task: packet tests pass, verified against live Hercules_RO with the manual script, no regression in the prior milestone's script.
- **Weekly:** update this doc's status columns; log decisions in SOFTWARE_DESIGN.md §5/§11.
- **Do first, this week:** E1.2–E1.8 (all are S/M tasks; M0 is achievable in days).
