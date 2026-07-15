# Targeted Spec — Dynamic Bestiary Journal (E7.3 + Roadmap)

**Parents**: FEATURE_ROADMAP.md (Dynamic Bestiary Journal), PROJECT_PLAN.md E7.3, DM_INTERFACE.md §9.3, DM_DATA_GUIDE.md, BESTIARY.md + bestiary.json.

**Purpose**: A native, unlockable monster manual that displays accurate stats, weaknesses, skills, and lore for campaign-relevant mobs. Unlocks via kills or DM Lore checks. Replaces/supplements official quest journal for Seal Cascade content.

**Data Sources** (already generated):
- `docs/bestiary.json` — Primary: Id, Name, Lv, Hp, Stats (STR- LUK), Attack, Def, Mdef, MoveSpeed, AttackDelay, Element, Race (inferred), XP, Skills (with Level/Rate/Delay), PhysDPS/MagicDPS.
  - **Export gap closed 2026-07-15** by `tools/extend_bestiary_export.py`
    (no original exporter was ever committed, so this is a fresh, narrowly
    -scoped script — it *merges* into the existing file, leaving every
    already-shipped field, e.g. `PhysDPS`/`MagicDPS`/`DropsCount`
    consumed by `src/dm/loot.rs`, untouched). Parses `Element`/`Race`/
    `Size`/`Mode` straight from `mob_db.conf` and joins `Skills`
    (id/level/rate/delay) from `mob_skill_db.conf` by `SpriteName`.
    Coverage: **Element/Race 1750/1759, Size 1744/1759, Mode 1721/1759,
    Skills 1237/1759** (the ~9-60 gaps left are the known renewal/event
    mobs already absent from other exports). `Element` is written as a
    formatted string (`"Water 1"`, type + level) rather than a nested
    object, matching the existing Rust `Option<String>` field in
    `BestiaryMonster` (`src/dm/data.rs`) so no schema change was needed;
    `Mode`/`Skills` have no Rust field yet (safely ignored by serde until
    one is added — needed to actually *render* the Combat tier). Verified
    against the existing `dm_data_tests` deserialization tests (pass).
    Re-run after `mob_db.conf`/`mob_skill_db.conf` changes, then rebuild.
- **Lore (game data for the Scholar tier — ships in the client, embedded
  like bestiary.json). Combined coverage: 650/1759 bestiary mobs, and
  100% of the 62 mobs actually spawned along the campaign's arc route
  (verified 2026-07-15 by cross-referencing `CAMPAIGN.md`'s arc maps
  against Hercules' real spawn tables in `npc/re/mobs/`):**
  - `docs/mob_lore.json` — 540/1759 mobs, fetched from ragnarok.fandom.com
    by `tools/fetch_mob_lore.py` (CC-BY-SA; attribution kept in `_meta` +
    per-entry `url`). Entries are **short excerpts, capped ~500 chars at
    a sentence boundary** — a blurb for the Scholar-tier panel, not the
    full article; `url` always links the complete source. **Bug fixed
    2026-07-15**: the disambiguation filter was matching the *hatnote*
    text ("For other uses, see X (disambiguation)") that prefixes some
    articles' real content and discarding the whole entry — Nightmare,
    Nine Tail, and Dark Illusion were silently dropped this way. Fetcher
    now strips leading hatnotes before both the filter check and storage.
  - `docs/mob_lore_rml.json` — 151 mob ids from Mitten's fan "Ragnarok
    Monster Lore" encyclopedia (WarpPortal forums), fetched display-clean
    and excerpted (~500 char cap, same rule as above — source posts run
    up to 13k chars uncapped; official-kRO-sourced material for classic
    MVPs) by `tools/fetch_mob_lore_rml.py`. ~49 ids Fandom lacks.
    Fan-authored, no explicit license — shipped in this private
    non-commercial client as a short excerpt with author/source
    attribution retained per entry.
  - `docs/mob_lore_campaign.json` — **49 hand-authored Seal Cascade
    entries** (2026-07-15). First 19: every campaign-referenced mob no
    external source documents (Amon Ra, Tao Gunka, Ifrit, Venatu, the
    Niflheim/banquet dead, the corrected kill-quest targets). Remaining
    30, added the same day via a **zone-prioritized authoring pass**
    (arc route maps → real spawn tables → intersect against still-
    uncovered ids): RSX-0806 and the Wounded Morroc / Incarnation of
    Morroc finale bosses grounded directly in the Arc 7 and Arc 19 vault
    synopses; the six Lighthalzen Bio Lab researchers (Randel, Flamel,
    Chen, Gertie, Alphoccio, Trentini) as a new "Prometheus Project"
    thread consistent with the existing Rekenber/Bio Labs plot; the
    Hugel dragon-nest family, Veins' carnivorous-plant trio, Izlude's
    turtle-dungeon wardens, and Payon/Morroc/Prontera zone flavor. All
    spoiler-calibrated — foreshadows arcs, reveals no twists.
  - `docs/mob_lore_variants.json` — **32-entry elite/tiered-reskin
    inheritance map** (manually verified exact-name matches against
    `mob_db.conf`, not fuzzy matching — an earlier fuzzy-match attempt
    produced wrong hits like "Choco"→"Coco" and was discarded). Covers
    Ragnarok's own tier-naming conventions: `Solid/Swift/Elusive/Furious
    <Base>`, `<Base> Ringleader`, the Lighthalzen research-staff
    job-class prefixes (`Paladin Randel` etc.), and same-name duplicate
    spawn ids (the four Incarnation of Morroc splinters, the Acidus/Ferus
    pairs). Detail pane: if a mob has no direct entry, look up here and
    render the `base_id`'s lore with the `tier` string as a small tag
    ("Elite variant of Stapo.").
  - **Merge priority in the detail pane**: campaign > RML > Fandom >
    variant-inherited. Show a small source line under the lore text.
  - Audit note: the earlier "20 campaign mobs missing lore" list contained
    six phantoms — "Elder" was a word-match false positive, and five were
    off-by-one MobId typos in `quest_db.conf` (Evil Snake Lord→Zipper Bear,
    Galion→Roween, Gazeti→Ice Titan, Siroma→Snowier, Blazer→Lava Golem),
    fixed server-side 2026-07-15 (Hercules `d28ffb666`).
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
