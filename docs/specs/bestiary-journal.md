# Targeted Spec — Dynamic Bestiary Journal (E7.3 + Roadmap)

**Parents**: FEATURE_ROADMAP.md (Dynamic Bestiary Journal), PROJECT_PLAN.md E7.3, DM_INTERFACE.md §9.3, DM_DATA_GUIDE.md, BESTIARY.md + bestiary.json.

**Purpose**: A native, unlockable monster manual that displays accurate stats, weaknesses, skills, and lore for campaign-relevant mobs. Unlocks via kills or DM Lore checks. Replaces/supplements official quest journal for Seal Cascade content.

**Data Sources** (already generated):
- `docs/bestiary.json` — Primary: Id, Name, Lv, Hp, Stats (STR- LUK), Attack, Def, Mdef, MoveSpeed, AttackDelay, Element, Race (inferred), XP, Skills (with Level/Rate/Delay), PhysDPS/MagicDPS.
  - **Known export gaps (audit 2026-07-15)**: Element/Race/Size present for only 3/1759 entries and Skills empty for all — the exporter never read those `mob_db.conf` fields / `mob_skill_db.conf`. Mode flags (Aggressive/Looter/…) never exported. Extend + regenerate before building the lore-check Identity/Combat tiers.
- **Lore (game data for the Scholar tier — ships in the client, embedded
  like bestiary.json):**
  - `docs/mob_lore.json` — 533/1759 mobs, fetched from ragnarok.fandom.com
    by `tools/fetch_mob_lore.py` (CC-BY-SA; attribution kept in `_meta` +
    per-entry `url`).
  - `docs/mob_lore_rml.json` — 151 mob ids from Mitten's fan "Ragnarok
    Monster Lore" encyclopedia (WarpPortal forums), fetched display-clean
    by `tools/fetch_mob_lore_rml.py`. Deeper than the Fandom intros
    (Atroce 3.8k chars; official-kRO-sourced material for classic MVPs);
    ~49 ids Fandom lacks. Fan-authored, no explicit license — shipped in
    this private non-commercial client with author/source attribution
    retained per entry.
  - `docs/mob_lore_campaign.json` — 19 hand-authored Seal Cascade entries
    (2026-07-15) covering every campaign-referenced mob no external source
    documents (Amon Ra, Tao Gunka, Ifrit, Venatu, Celia, the Niflheim/
    banquet dead, the corrected kill-quest targets). Grounded in the
    campaign vault's beat docs; spoiler-calibrated (foreshadows arcs,
    reveals no twists).
  - **Merge priority in the detail pane**: campaign > RML > Fandom. Show a
    small source line under the lore text.
  - Audit note: the earlier "20 campaign mobs missing lore" list contained
    six phantoms — "Elder" was a word-match false positive, and five were
    off-by-one MobId typos in `quest_db.conf` (Evil Snake Lord→Zipper Bear,
    Galion→Roween, Gazeti→Ice Titan, Siroma→Snowier, Blazer→Lava Golem),
    fixed server-side 2026-07-15.
- Cross with `cards.json` for "associated cards".
- `items.json` for drop-related lore if needed.
- Static campaign data from `Hercules_RO/npc/custom/dm_campaign/` + planning docs (arcs, which mobs appear).

## Architecture

**Client-side** (`src/dm/` isolated):
- `DmBestiary` state in `ClientState.dm_state`:
  ```rust
  pub struct DmBestiary {
      unlocked: HashSet<u32>,           // mob IDs
      entries: HashMap<u32, BestiaryEntry>, // loaded from JSON + unlocks
  }
  ```
- Load `bestiary.json` at startup (or lazy on first open). Filter to campaign mobs (e.g., IDs from quest data or hardcoded list for arcs 0-4).
- Unlock logic:
  - On `NetworkEvent::RemoveEntity` (death) or health update to 0 → if entity matches bestiary mob, unlock.
  - Or via [DMJ] echo from server "lore_check" or DM command.
  - DM can force-unlock via console.

**UI** (`interface/windows/dm/bestiary_journal.rs` or extend existing quest journal):
- Tab or separate window: "Bestiary".
- List view (by arc/level) + detail view.
- Detail panel:
  - Header: Name, Lv, Sprite (reuse existing sprite loading).
  - Stats table: HP, STR/AGI/VIT/INT/DEX/LUK, ATK, DEF, MDEF, Speed, Element.
  - Weaknesses: Infer from Element + Race (e.g., Fire weak to Water).
  - Skills: List with estimated DPS contribution.
  - Drops: Link to items.json or show common drops.
  - Lore: Static text from campaign planning (or [DMJ]).
- Unlock states: Locked entries show "???" + silhouette. Unlock animation on first discovery.
- Search/filter by level, element, arc.

**Integration**:
- Reuse `item_box`/`skill_box` patterns for drops/skills.
- Reactive via RustState paths on `dm_state.bestiary`.
- In-world: QuestEffect or custom particles on "lore stones" or specific mobs trigger unlock prompt.

## Data Flow

1. Build-time (optional): `build.rs` or `tools/merge_bestiary.py` merges `bestiary.json` + campaign mob list → `src/dm/data/bestiary.ron` (or const).
2. Runtime load → `DmBestiary`.
3. Network events → unlock entries.
4. UI binds to unlocked set + entry data.
5. DM console can query/edit (for testing).

## Implementation Steps (MVP)

1. Add `dm_state.bestiary` to `ClientState` (hidden).
2. Loader: `src/dm/data/bestiary.rs` — load JSON (or RON), filter campaign-relevant.
3. Unlock handler in `lib.rs` (on entity death or specific [DMJ]).
4. Basic window: List of unlocked + detail view (text + stats table).
5. Wire to quest journal or hotkey.
6. Polish: Sprite rendering, inferred weaknesses, search.

## Tiered reveal — lore checks (design settled 2026-07-15)

Unlocks are **tiered, not binary**. A kill unlocks everything (you learned
by fighting); a passed lore check (`proficiency-checks.md` formula, DC from
mob level + boss bump) unlocks by margin — knowledge *before* the fight,
ordered by what's actionable in RO combat:

| Result | Tier unlocked | Fields shown |
|---|---|---|
| Meet DC | **Identity** | true name, level, **race / size / element** (the "what do I hit it with" triad) |
| Beat by 5+ | **Combat** | + HP pool, attack range, DPS estimates, notable skills (self-destruct, status infliction) |
| Beat by 10+ or nat 20 | **Scholar** | + drops, cards, lore/flavor text |
| Kill | **Full** | everything (equals Scholar) |
| Fail | — | no change |
| Nat 1 | — | *optional DM toggle*: entry shows one plausibly wrong fact (e.g. wrong element), flagged DM-side |

State accordingly becomes a tier map, not a set:
`unlocked: HashMap<u32, UnlockTier>` (`Identity < Combat < Scholar/Full`;
upgrades only, never downgrades — nat-1 misinformation is a display
overlay, not a stored tier). The detail pane renders locked fields of a
partially-unlocked entry as `???` per-section rather than hiding the whole
entry. **Bake the tier into the persistence format from day one** — see
`bestiary-unlock-persistence.md`.

Who rolls: stat-by-mob-race, mirroring the social-check approach system —
see the lore-check table in `proficiency-checks.md`. DM flow: click a `???`
entry in the DM bestiary → "Call lore check" → emits
`@dm check party lore <mob_id>`.

## DM-Specific Features

- **DM overrides**: DM window to "reveal" entries for party or force stats.
- **Balancing tie-in**: Display estimated party DPS vs mob (from bestiary PhysDPS).
- **Card synergy**: Show "Drops X Card" from cards.json.

## Risks & Gotchas

- Data sync: Re-run parsers when Hercules mob DB changes.
- Performance: 1700+ entries — load lazily or pre-filter to ~100-200 campaign mobs.
- Client vs server: Bestiary is read-only reference; server is truth for current HP.
- Rebase: All in `src/dm/`.

## Acceptance Criteria

- Journal opens with static + dynamic unlocks.
- Accurate stats from bestiary.json displayed.
- Unlock on kill (or DM command).
- Searchable/filterable by level/element/arc.
- Works in party (unlocks per-character or shared via [DMJ]?).

See `DM_DATA_GUIDE.md` for loading patterns and `BESTIARY.md` for data schema.

**Next**: After E7.2 dice cards, this is high-value for immersion and planning.
