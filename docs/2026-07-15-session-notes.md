# Session notes — 2026-07-15

Design pass over the rest of the E7 backlog (specs for everything that
was still design-only), plus a bestiary data pipeline build-out: lore
text for the campaign's mobs and the missing stat/tactical fields the
tiered lore-check reveal needs.

## Specs written

Five new targeted specs close every remaining gap in the E7 roadmap
(`docs/specs/README.md`, `docs/README.md`, and the `PROJECT_PLAN.md` E7
table all updated with links):

- **`campaign-quest-journal.md` (E7.3)** — `quest_db.conf` → embedded JSON
  pipeline (corrects the plan's reference to a
  `campaign_quest_journal_entries.lua` that doesn't exist), journal window
  + HUD tracker, consumes the already-wired `QuestAdded`/`QuestRemoved`/
  `QuestList` events (the last port-back row).
- **`bestiary-unlock-persistence.md` (E7.15, new row)** — client-local
  `client/dm_campaign.ron` now (copies the `window_cache.ron` pattern),
  server-authoritative party-wide `[DMJ]` sync after E7.1. Persists the
  unlock **tier** from day one so Phase 2 needs no format migration.
- **`initiative-encounter-panel.md` (E7.5+E7.8)** — covers both sides,
  including the server scripts that don't exist yet (`@dminitiative`,
  `@dmencounter`, `@dmscale` — idempotent, scaled from DB base stats,
  `@dmbloodied`). Placeholder party-size scaling presets since the
  balance-checker doc the design cited also doesn't exist.
- **`atb-structured-rounds.md` (E7.14, new row, future work)** — a
  DM-toggleable Active Time Battle mode. Phase A: scripted structured
  rounds via `setpcblock`/`SC_STOP`. Phase B: **time dilation of
  Hercules' own `canact_tick`/`canmove_tick`/`attackabletime` gates**
  (~4–6×, tuned to an FF6-style 6–8s bar fill) rather than an imposed
  clock — the engine already runs ATB, this just makes it human-legible.
  Movement stays continuous (WoW-style) but must dilate by the same
  multiplier or kiting trivializes every fight.
- **`proficiency-checks.md` (E7.16, new row, future work)** — bounded-
  accuracy check formula (`d20 + min(base_stat/15, 8) + proficiency`,
  replacing the design's `stat/10` which breaks past 100+ stats),
  proficiency derived from `getskilllv()` on a class-skill mapping table
  (spend job points on Hide, you're trained in Stealth — no new point
  economy), mechanical consequences on success (stealth → real `SC_HIDE`,
  not just narration). **Social checks are stat-by-approach** (argue =
  INT, charm = LUK, intimidate = STR, haggle = INT/LUK) with a per-class
  flavor table so every class has a social angle — Merchant's
  Discount/Overcharge as trained haggling, Rogue's Gangster's Paradise as
  intimidation, etc. Also extended to **lore checks** (stat by mob race)
  feeding the bestiary tier reveal below.
- **`bestiary-journal.md`** — gained the tiered-reveal design: kill = full
  unlock, a lore check unlocks by margin (Identity at DC → race/size/
  element, Combat at +5 → HP/DPS/skills, Scholar at +10 or nat 20 →
  drops/cards/lore), plus a nat-1 misinformation DM toggle.

## Bestiary lore data pipeline

Built from scratch since the lore-check design needs actual text to
reveal. Combined coverage: **650/1759 bestiary mobs**, and **100% of the
62 mobs actually spawned along the campaign's arc route** (verified by
cross-referencing `CAMPAIGN.md`'s arc maps against Hercules' real spawn
tables in `npc/re/mobs/`).

- `docs/mob_lore.json` (540 mobs) — `tools/fetch_mob_lore.py`, MediaWiki
  API against ragnarok.fandom.com (CC-BY-SA, attributed). Found and fixed
  a real bug: the disambiguation filter matched the *hatnote* text ("For
  other uses, see X (disambiguation)") that prefixes some articles'
  real content, silently discarding good entries. Also added a ~500-char
  sentence-bounded excerpt cap — a blurb for the panel, not the article;
  `url` always links the full source.
- `docs/mob_lore_rml.json` (151 mobs) — `tools/fetch_mob_lore_rml.py`,
  scraped from Mitten's fan "Ragnarok Monster Lore" encyclopedia
  (WarpPortal forums; official-kRO-sourced material for classic MVPs).
  Same excerpt cap (source posts ran past 13k chars uncapped).
- `docs/mob_lore_campaign.json` (49 mobs, hand-authored) — every
  campaign-referenced mob no external source documents. First 19: arc
  bosses and story mobs (Amon Ra, Tao Gunka, Ifrit, Venatu, the Niflheim
  dead), grounded in the campaign vault's beat docs. While authoring
  these, found and fixed **five off-by-one MobId typos in
  `Hercules/db/quest_db.conf`** that had silently turned low-level kill
  quests into MVP hunts (e.g. "Mine Dust Medicine" targeted Evil Snake
  Lord instead of Zipper Bear) — pushed as Hercules `d28ffb666`. Remaining
  30: a zone-prioritized pass (arc maps → real spawn tables → intersect
  against still-uncovered ids) covering RSX-0806 and the Wounded Morroc /
  Incarnation of Morroc finale bosses (grounded in the Arc 7 / Arc 19
  vault synopses), a new Lighthalzen "Prometheus Project" thread for the
  six Bio Lab researchers, and the Hugel dragon-nest / Veins flora /
  Izlude warden families.
- `docs/mob_lore_variants.json` (32 entries) — elite/tiered-reskin
  inheritance map, resolved by **exact-name matching against Ragnarok's
  own naming convention** (`Solid/Swift/Elusive/Furious <Base>`, `<Base>
  Ringleader`, job-class prefixes), not fuzzy matching — an earlier fuzzy
  attempt produced wrong hits (`Choco`→`Coco`) and was discarded.

## Bestiary stat/tactical export

`tools/extend_bestiary_export.py` — no original exporter was ever
committed, so this is a fresh script that *merges* into `bestiary.json`
rather than regenerating it, leaving every already-shipped field
(`PhysDPS`/`MagicDPS`/`DropsCount`, consumed by `src/dm/loot.rs`)
untouched. Adds:

- `Element`/`Race`/`Size` from `mob_db.conf` — coverage 3 → **1750/1759**
  (Element/Race), **1744/1759** (Size).
- `Mode` flags (Aggressive/Looter/Assist/Boss/…) — **1721/1759**.
- `Skills` (id/level/rate/delay) joined from `mob_skill_db.conf` by
  `SpriteName` — 0 → **1237/1759**.

Caught a real bug before it shipped: `BestiaryMonster.element` in
`src/dm/data.rs` is `Option<String>`, but the first pass wrote `Element`
as a nested `{Type, Level}` object, which would have broken
deserialization for the already-shipped Bestiary window. Fixed by
formatting as `"Water 1"` (a plain string) instead of changing the Rust
schema. Verified against the existing `dm_data_tests` — both pass.
`Mode`/`Skills` have no Rust struct field yet (safely ignored by serde
until one's added — the next step to actually render the Combat tier).

## Commits

korangar (`agent/platform-connectivity-controls`):
`f360dcd4` (five specs) → `5fb53fd0` (ATB time dilation) → `7fd5c43f`
(tiered reveal + first lore batch) → `007bf302` (lore expansion + fetcher
bug fix) → `eb3c3068` (bestiary exporter extension).
Hercules (`agent/map-teleport-safety`): `d28ffb666` (quest_db MobId fixes).

## Not yet done

- `Mode`/`Skills` Rust struct fields + the actual tiered-reveal UI in the
  Bestiary Journal window (data is ready; the client code that consumes
  it for the Identity/Combat tiers isn't built).
- The ~1100 remaining lore-less bestiary mobs are genuinely outside the
  campaign's route and outside what any source (official, wiki, fan)
  documents — not a near-term priority.
- Everything already listed as "not yet done" in the 2026-07-14 notes
  (live GUI click-through gating the `main` fast-forward, dice cards,
  the rest of the E7 build queue) is still open; this session was design
  + data, not new client code.

## How to resume

Regenerating lore/stat data after Hercules DB changes: re-run
`tools/fetch_mob_lore.py`, `tools/fetch_mob_lore_rml.py`, and
`tools/extend_bestiary_export.py` from `korangar/`, then rebuild — all
four JSON files are embedded at compile time via `include_str!`.

Build order per the now-fully-specced E7 queue: dice cards (E7.2) →
campaign quest journal (E7.3) → bestiary unlock persistence (E7.15) →
initiative/encounter panel (E7.5+E7.8) → proficiency checks (E7.16) → ATB
rounds (E7.14). The manual GUI pass from 2026-07-14 still gates the
`main` fast-forward and hasn't happened yet.
