# Ragnarok Online: Friends Server Modernization

## Game Design Document

*Version 0.2 | September 2026*

> **DESIGN THESIS:** Preserve Ragnarok Online's identity and systemic depth, remove unnecessary friction, and add tactical clarity, smarter PvE, optional guidance, and modern usability without turning the game into a different MMO.

*Primary scope: cooperative PvE friends server. PvP and War of Emperium are intentionally out of scope for this version.*

# Document Control

| **Field**               | **Value**                                                                                                   |
|-------------------------|-------------------------------------------------------------------------------------------------------------|
| Document                | Ragnarok Online Friends Server Modernization - Game Design Document                                         |
| Version                 | 0.2 - v0.1 baseline plus implementation status refresh (2026-09-24)                                           |
| Primary Audience        | Server owner, designers, scripters, client/UI developers, content builders, playtesters                     |
| Current Scope           | Cooperative PvE, story, exploration, tactical combat, UI/UX, progression, economy, social play              |
| Explicitly Out of Scope | PvP, War of Emperium, competitive ranking systems, cash-shop power                                          |
| Art Direction           | Preserve original Ragnarok art and world identity; new UI/effects should match the existing visual language |
| Design Status           | Design baseline with a refreshed per-section implementation audit. Numeric tuning and live acceptance remain subject to playtesting. |
| Implementation          | Korangar client fork (`korangar/`) + Hercules server fork (`Hercules/`), `PACKETVER=20220406`. First friends playtest 2026-09-05. |
| Location                | `korangar/docs/GDD.md` in [Boneysan/korangar](https://github.com/Boneysan/korangar). Server-side counterpart docs live in [Boneysan/Hercules](https://github.com/Boneysan/Hercules) under `planning/`. |

## Decision Language

| **Label**           | **Meaning**                                                                                 |
|---------------------|---------------------------------------------------------------------------------------------|
| Locked Principle    | A core identity rule. Do not change without revisiting the overall design direction.        |
| Recommended Default | The preferred implementation unless technical or playtest evidence supports another choice. |
| Configurable        | A server or player option that may vary without harming the design.                         |
| Playtest Value      | A number or threshold that should not be treated as final until tested in-game.             |

# Contents

- 1. Executive Summary

- 2. Product Vision and Scope

- 3. Design Pillars and Guardrails

- 4. Core Player Experience and Gameplay Loops

- 5. Tactical Combat Framework

- 6. Monster AI and Encounter Design

- 7. Classes, Stats, Skills, and Build Identity

- 8. Story and Quest Design

- 9. World, Exploration, and Navigation

- 10. User Interface and User Experience

- 11. Loot, Equipment, Cards, and Refinement

- 12. Economy, Vending, and Crafting

- 13. Party and Friends-Server Social Systems

- 14. Death, Recovery, and Difficulty

- 15. Accessibility and Player Options

- 16. Audio, Visual Feedback, and Presentation

- 17. Configuration and Server Administration

- 18. Balance and Playtesting Methodology

- 19. Implementation Roadmap

- 20. Success Criteria and Non-Goals

- Appendix A. Example Player Journey

- Appendix B. Example Tactical Encounter

- Appendix C. Initial Configuration Matrix

- Appendix D. Open Design Questions

- Appendix E. Implementation Status (refreshed 2026-09-24)

# 1. Executive Summary

This project modernizes the original Ragnarok Online experience for a cooperative friends server while preserving the systems and visual identity that make Ragnarok distinctive. The modernization focuses on how the game communicates, controls, guides, and challenges the player rather than replacing the underlying game with a modern action-MMO template.

The server should support three equally valid ways to play: following a centralized story with optional guidance, exploring the world without a prescribed route, and pursuing self-directed goals such as leveling, card hunting, equipment farming, crafting, or helping friends. The game should always be willing to explain where a player could go next, but should rarely dictate where the player must go next.

> **LOCKED PRINCIPLE:** Give players information, not instructions. Give them destinations, not obligations. Give them breadcrumbs, not rails.

## The Modernization Has Three Primary Pillars

| **Pillar**               | **Purpose**                                                                            | **Examples**                                                                                                                  |
|--------------------------|----------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------|
| Tactical PvE Combat      | Make the existing combat model more readable, responsive, positional, and cooperative. | Smarter monster behavior, readable attack tells, better ground targeting, meaningful positioning, responsive input buffering. |
| Adventure and Navigation | Reduce confusion without removing exploration.                                         | World map, breadcrumb routing, monster population regions, optional quest markers, Adventure Guide, searchable journals.      |
| Modern UI/UX             | Expose Ragnarok's depth instead of simplifying it away.                                | Custom HUD, better inventory, skill details, build planner, party frames, combat explanations, scalable UI.                   |

## What This Project Is Not

- It is not a conversion to full action combat.

- It is not a linear main-story MMO where content is locked behind quest chapters.

- It is not a class-homogenization project.

- It is not an attempt to remove grinding, rare drops, unusual builds, or player-directed progression.

- It is not currently a PvP or War of Emperium redesign.

- It is not a loot-treadmill game where each content patch invalidates old equipment.

## Expected Result

A returning Ragnarok player should recognize the game immediately, but it should feel dramatically easier to read and control. A new player should be able to understand where to go, why a combat interaction occurred, and how their build works without maintaining multiple external wiki pages. A group of friends should be able to decide on a goal, navigate there together, fight tactically, pursue useful drops, and change plans freely when something more interesting happens.

# 2. Product Vision and Scope

## 2.1 Product Vision

The intended experience is "Ragnarok Online, fully realized." The original world structure, sprites, classes, equipment concepts, monster identity, cards, stats, cast-time model, autoattack builds, direct player trade, and open-ended progression are treated as assets rather than legacy problems. Modern systems are added only when they improve responsiveness, clarity, cooperation, navigation, accessibility, or tactical depth.

## 2.2 Audience

- Friends who know Ragnarok and want a more polished cooperative version.

- Returning players who remember the world but no longer remember every route, monster location, or formula.

- New players who benefit from optional guidance and integrated explanations.

- Build-focused players who enjoy unusual stat, card, equipment, and class combinations.

- Explorers and collectors who enjoy learning maps, monsters, drops, and rare items.

## 2.3 Current Feature Scope

| **In Scope**                                   | **Deferred / Out of Scope**        |
|------------------------------------------------|------------------------------------|
| Central story with optional quest tracking     | PvP balancing                      |
| Open-world leveling and farming                | War of Emperium                    |
| Tactical PvE combat and smarter monster AI     | Competitive ladders                |
| MVP and boss encounter improvements            | Cash-shop power systems            |
| World map, navigation, journals, search        | Large-scale guild warfare tooling  |
| Modern UI, inventory, party, skill and stat UX | Cross-server matchmaking           |
| Cards, rare drops, equipment, refinement       | Mandatory daily/weekly progression |
| Direct player trade, crafting, small-server economy | Battle pass systems           |
| DM Session mode (opt-in, DM-activated)         | Player vending, market board, auction (decision C2) |
| In-game encyclopedia (§9.5)                    |                                    |

*Added in v0.2 (decision C5):* **DM Session mode** is a distinct, opt-in mode layered on the server. It is activated by the Dungeon Master (`@dm mode on`, `@dm start`) for one party, runs the Seal Cascade campaign through private instances, scripted hazards, skill checks and DM-chosen beat outcomes, and is switched off between sessions. Everything else in this document describes the server outside a session. DM Session content is not the §8 story spine and does not count toward it.

Story completion is **character-specific**, including in DM Session mode. A party can advance its enrolled characters together, but an alternate character on the same account must play through the story independently. General discovery and non-story unlocks are account-wide; story quests, choices, chapters, and story-gated access are not. S10 records typed quest/flag events and replays them through per-character SQL cursors. Live acceptance verifies offline reconnect replay, late joining, same-account alternate isolation until enrollment, and cursor advancement for actor and returning characters. Run identity, rewards, and failure-repair acceptance remain open (§8.7, S10).

*Added in v0.2 (decision C1):* the **in-game encyclopedia** (§9.5) is in scope as a first-class knowledge system, not a convenience.

## 2.4 Art and Content Constraint

Original art is preserved as the visual baseline. New interface assets, telegraphs, icons, markers, map overlays, particles, and added animation frames should imitate the proportions, palette discipline, and readability of the existing game rather than introducing a visibly separate modern art style. The modernization should look like an official evolution of the original client, not a UI skin laid over it.

# 3. Design Pillars and Guardrails

## Preserve Complexity; Remove Friction

Ragnarok's complexity is part of its identity. The goal is to explain and expose systems such as ASPD, cast time, size, race, elements, status effects, Hit/Flee, cards, and equipment interactions rather than flattening them into generic stats.

## Tactical, Not Twitch

Combat should reward movement, preparation, positioning, skill choice, target priority, party coordination, and understanding of monsters. Mechanical execution should matter, but universal dodge-roll and animation-cancel gameplay is not the target.

## Optional Guidance

Players following the story should receive clear routing and objectives. Players who prefer exploration can disable guidance and wander freely.

## The World Remains Relevant

Travel, maps, fields, dungeon entrances, Kafra services, boats, portals, and monster habitats should remain meaningful. Navigation improvements should not turn the world into a teleport menu.

## Classes Stay Asymmetric

Different classes should solve different problems. Not every class needs equivalent solo capability, defense, mobility, healing, or utility.

## Friends Should Be Able to Play Together

The server should avoid systems that punish helping a lower-level friend or joining an inefficient group. Cooperative play is a primary success condition.

## Rare Drops Stay Exciting

Cards and valuable equipment should remain exciting because they are rare and useful, not because the interface creates artificial spectacle.

## No Chore Design

The server should not rely on mandatory daily quests, weekly caps, login pressure, or battle-pass progression to create engagement.

## 3.1 Guardrails - Features to Avoid by Default

- Universal dodge roll or universal iframe button.

- Class kits rebuilt around standardized cooldown rotations.

- Generic stat normalization that makes STR/AGI/VIT/INT/DEX/LUK mostly cosmetic.

- Giant persistent telegraphs covering most of the battlefield.

- Mandatory main-story progression gates for ordinary maps and hunting areas.

- Auction house or market board systems. *(v0.2: vending itself is out of scope for this server's population — decision C2 — so the guardrail is against automated brokering, not in favour of vending.)*

- Frequent gear resets that obsolete cards and equipment every content cycle.

- Exact spawn-coordinate overlays that turn exploration into a debug screen.

- Overly scripted normal monsters that make routine farming exhausting.

# 4. Core Player Experience and Gameplay Loops

## 4.1 Primary Session Loop

1. Choose a goal: story chapter, level target, item, card, monster, map, boss, crafting material, or helping a friend.

2. Use the Adventure Guide or personal knowledge to identify where the goal can be pursued.

3. Travel through the world using ordinary map connections, Kafra services, Warp Portal, boats, or other in-world transportation.

4. Fight monsters using class-specific tools and tactical positioning.

5. Gain experience, loot, monster knowledge, quest progress, crafting materials, or social progress.

6. Reassess. Continue the plan, change targets, follow a rumor, join a friend, or pursue a newly discovered objective.

## 4.2 Self-Directed Progression Loop

The game should support a player saying "I want a Raydric Card" and turning that desire into gameplay without creating a formal quest. Item Search identifies the source, Monster Journal identifies habitats, navigation provides a route, and the map helps the player hunt in the right general area. The player selected the goal; the game merely supplied knowledge and navigation support.

## 4.3 Story-Guided Loop

A player who prefers direction can follow the central story. Story quests intentionally introduce towns, dungeons, systems, and enemy families. The quest tracker explains the immediate objective and can provide breadcrumb routing to the next relevant map or NPC. Story participation should never be required to use ordinary maps unless a location exists specifically as a story instance.

## 4.4 Friends-First Loop

A player should be able to stop whatever they are doing when a friend logs in. Shared destination markers, party routing, forgiving level-range rules, group goals, and optional level synchronization should make "come join us" an easy answer rather than an optimization problem.

# 5. Tactical Combat Framework

## 5.1 Combat Objective

Combat modernization should make decisions more important without making input speed the dominant skill. Players should read monsters, choose targets, position around ground effects, manage distance, exploit class tools, and coordinate with friends. The combat model remains recognizably Ragnarok: targeted attacks, autoattacks, skills, cast time, ASPD, SP, status effects, elements, Hit/Flee, ground skills, and equipment preparation.

## 5.2 Input Responsiveness

| **System**            | **Recommended Default**                     | **Design Purpose**                                                                     |
|-----------------------|---------------------------------------------|----------------------------------------------------------------------------------------|
| Input buffer          | 150-250 ms; playtest value                  | Remember a single near-term intended action so players do not need to mash skills.     |
| Action queue          | One queued action maximum                   | Responsiveness without automation or rotation scripting.                               |
| Client acknowledgment | Immediate visual feedback on accepted input | Reduce the feeling that the player is waiting for the server to notice a button press. |
| Server authority      | Retained                                    | Damage, status, movement validation, and outcomes remain authoritative.                |

Input buffering must never become a multi-step programmable queue. The player chooses every meaningful action.

## 5.3 Movement

- Classic click-to-move remains fully supported.

- Holding the mouse should provide smooth continuous path updates rather than repeated discrete clicks.

- Optional keyboard movement may be added as an alternate input method using the same tile/pathing rules.

- Keyboard movement must not provide faster acceleration, unique collision behavior, animation cancels, or competitive advantages.

- Pathfinding should avoid obvious oscillation, unnecessary backtracking, and getting caught on minor geometry.

> **LOCKED PRINCIPLE:** Movement itself is the universal defensive action. Do not add a universal dodge roll unless the entire combat model is intentionally being redesigned.

## 5.4 Targeting

- Left-click or equivalent selects a monster immediately and clearly.

- Target switching should be reliable and configurable, including nearest hostile and cycle-target actions.

- Ground-target abilities should show actual reachable area and valid placement before cast confirmation.

- Players may choose classic click-cast, press-aim-click, hold-aim-release, or quickcast-at-cursor where technically feasible.

- Target frames should show only information relevant to tactical decisions: HP state, significant status effects, cast/action state, race/size/element if known, and threat/danger cues where appropriate.

## 5.5 Positioning

Positioning should matter primarily through existing Ragnarok concepts: distance, ground effects, traps, knockback, cast safety, monster access, chokepoints, line of approach, and party protection. The project should avoid universal backstab, flanking, wall-collision damage, or high-ground modifiers unless a specific class or monster explicitly uses them.

## 5.6 Ground Control

Ground-target skills should become one of the game's signature tactical systems. Firewall, Safety Wall, Pneuma, traps, Storm Gust, Meteor Storm, Magnus Exorcismus, Land Protector, Quagmire, and similar skills should have precise, readable placement. Their existing strategic identity should be strengthened through better previews, monster reactions, and encounter layouts rather than replaced with cooldown mechanics.

## 5.7 Knockback and Displacement

Knockback is primarily battlefield control. It creates breathing room, breaks formations, removes enemies from dangerous positions, pushes threats away from vulnerable allies, and changes how a room is occupied. Avoid making wall collision a universal high-damage optimization because it would redefine too many existing skills.

## 5.8 Telegraph Philosophy

| **Attack Type**                     | **Telegraph Expectation**                                              |
|-------------------------------------|------------------------------------------------------------------------|
| Ordinary autoattack                 | Animation only; no ground warning.                                     |
| Minor monster skill                 | Animation, sound, or small effect; no mandatory floor marker.          |
| Dangerous interruptible attack      | Distinct wind-up plus optional compact cast/action indicator.          |
| Large area or high-lethality attack | Strong animation cue plus restrained tile/ground indication.           |
| Boss signature mechanic             | Clearly readable through animation, sound, arena state, and UI backup. |

The monster should communicate danger first. UI and floor indicators are backup channels, not substitutes for readable animation.

## 5.9 Combat Information Layers

| **Layer**  | **Audience**       | **Example**                                                                         |
|------------|--------------------|-------------------------------------------------------------------------------------|
| Immediate  | Everyone           | Damage number, critical indicator, "Element Advantage," important status icon.      |
| Contextual | Interested players | Tooltip explaining that the selected monster is Large / Shadow / Demi-Human.        |
| Advanced   | Theorycrafters     | Damage breakdown with base, stat, race, size, element, card, and defense modifiers. |

## 5.10 Autoattacks

Autoattack-focused builds remain first-class gameplay. High-ASPD Knights, critical Assassins, Battle Priests, Falcon Hunters, Blacksmiths, and auto-cast builds should feel powerful and responsive. Improvements should come from synchronized animations, cleaner target switching, hit reactions, proc feedback, sound, and tactical enemy behavior - not from forcing those builds to add arbitrary rotational buttons.

## 5.11 Skills as Tools

The ideal Ragnarok skill is useful because it solves a problem, not because the rotation requires it every eight seconds. Provoke manipulates attention. Pneuma denies ranged attacks. Safety Wall creates a protected tile. Firewall shapes movement. Hide changes threat and positioning. Traps shape space. Lex Aeterna creates a damage opportunity. The redesign should preserve this tool-oriented philosophy.

## 5.12 Tactical Party Interactions

- Frontliners can physically occupy approaches and intercept enemies through threat, body positioning, or class tools.

- Ranged characters benefit from stable firing lanes and controlled enemy movement.

- Ground-control classes shape where monsters can safely travel.

- Support classes create safe windows, remove statuses, protect vulnerable allies, and amplify opportunities.

- Burst classes capitalize on debuffs, exposed targets, isolated threats, or short tactical openings.

These relationships should emerge from class abilities and monster behavior rather than a hard-coded tank/healer/DPS requirement.

## 5.13 Implementation Path (added v0.2)

*Status refreshed 2026-09-24 from code and local tests; live gates remain separate. Each row names the hook that exists today and the size of the gap. Sizes: S ≤ 1 day · M ≤ 1 week · L longer.*

| **§**  | **Design item**                         | **Hook that exists**                                                                                                                                          | **Gap**                                                                                                                                                        | **Size** |
|--------|-----------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 5.8    | Telegraphs for monster area attacks     | Both 20220406 cast events preserve source, target entity, target tile, skill and duration; eligible invariant multi-cell layouts render a footprint during the authoritative cast interval. | Verify ground/entity/moving-target semantics and readability live; level-dependent/unknown layouts intentionally show only the cast bar. | **verify** |
| 5.2    | Single-action input buffer, 150-250 ms  | Walk-into-range chaining remains separate; a 200 ms replaceable slot now accepts attack, pickup, and entity/ground casts during finite local action animations and dispatches after unlock. Entity/item disappearance immediately invalidates a matching queued action with a cancellation toast. | Local tests/build pass; live lock timing and rapid-input acceptance need verification. No queue; newest wins. | **verify** |
| 5.2    | Immediate client acknowledgement        | Skill-fail reasons (`0x0EFE`) and out-of-range chat messages already make refusals explicit.                                                                   | Start the swing/cast animation on send rather than on server echo, and roll back on refusal. Playtest whether it reads as responsive or as desync.           | **M**    |
| 5.4    | Target cycling / nearest hostile        | Tab/Shift+Tab cycles visible living monsters by distance then screen-centre angle; click-to-attack is preserved. | Live-check overlapping targets, despawn/death cleanup and selection readability. | **verify** |
| 5.4    | Monster target frame: race / size / element | Reactive target frame shows live HP and bundled bestiary level/element/race/size when available; selected monsters retain an in-world health bar. | Live-check; add status/cast details only after verified data exists. Account discovery must not gate mechanical facts. | **verify** |
| 5.4    | Skill-range preview on hover            | Footprint renderer already tints red when out of range.                                                                                                       | Draw the range ring around the player while a skill is armed or its hotbar slot is hovered.                                                                    | **S**    |
| 5.9    | "Element advantage" cue on damage       | Damage numbers exist; `attr_fix.conf` is the elemental table; target element is in `bestiary.json`.                                                           | Client computes attacker element vs target element for the *cue only* (server still owns damage). Colour or suffix the number when the multiplier is > 1 or < 1. | **S**    |
| 5.9    | Advanced damage breakdown               | None — the server sends a total.                                                                                                                              | Needs a fork packet carrying the `battle_calc` components. Defer until the immediate/contextual layers are live and someone asks for it.                       | **L**    |
| 5.3    | Hold-mouse continuous path              | Click-to-move.                                                                                                                                                | Re-issue the move destination on a throttle while the button is held; same 200 ms bound WASD uses.                                                            | **S**    |
| 5.10   | Proc feedback for auto-cast builds      | Status tints and looping status effects render on the actor.                                                                                                  | Nothing beyond what is done; verify auto-spell / auto-cast visuals read on Sage and Blacksmith builds during the Phase 2 playtest.                             | verify   |

**Recommended order:** telegraphs → target cycling → monster target frame/chips → input buffer → element cue → hold-mouse → client acknowledgement. The first three have code and local compile/test coverage; live-readability still gates the §6 AI pilot.

# 6. Monster AI and Encounter Design

## 6.1 AI Design Goal

Normal monsters should feel like creatures with recognizable behavior, not miniature raid bosses. AI sophistication increases with area difficulty and monster identity. Early fields teach basic movement and targeting. Midgame areas introduce range, control, statuses, and group interactions. Late areas combine multiple behaviors and require target prioritization and positioning.

## 6.2 Behavior Archetypes

| **Archetype**    | **Behavior**                                                                  | **Tactical Result**                                        |
|------------------|-------------------------------------------------------------------------------|------------------------------------------------------------|
| Aggressor        | Closes quickly and pressures the nearest or highest-threat target.            | Creates frontline pressure.                                |
| Skirmisher       | Approaches, attacks, then repositions.                                        | Punishes static play without requiring complex scripting.  |
| Ranged Keeper    | Attempts to maintain a preferred distance.                                    | Rewards gap closing, line control, and ranged counterplay. |
| Support          | Buffs, heals, cleanses, or empowers nearby allies.                            | Creates target priority.                                   |
| Controller       | Uses slow, silence, knockback, traps, or ground hazards.                      | Changes positioning and party response.                    |
| Coward / Fleeing | Retreats when injured or isolated.                                            | Creates pursuit and containment decisions.                 |
| Protector        | Interposes, taunts, or prioritizes attackers threatening allies.              | Creates small group formations.                            |
| Opportunist      | Changes targets toward low-health, casting, or exposed players within limits. | Makes threat management less binary.                       |

## 6.3 Family Behavior

Monster families should share enough behavior that players learn patterns. Orcs may fight as a crude group: melee Orcs pressure the front, Archers maintain distance, and an Orc Lady or specialist may strengthen nearby allies. Undead may be relentless but predictable. Insects may swarm. Magical creatures may react more strongly to casting. Family behavior should build world identity and reduce the need for unique scripting on every monster.

## 6.4 Monster Reaction to Ground Effects

Not every monster should intelligently avoid every spell. Behavior depends on intelligence and role. A simple Zombie may walk through Firewall. A humanoid Archer may step around it if a reasonable path exists. A boss may intentionally cross a hazard when enraged. This creates tactical variety while preventing ground control from becoming either useless or universally dominant.

## 6.5 Threat and Targeting

Threat should remain understandable but not entirely deterministic. Damage, healing, proximity, specific skills, and role behaviors can contribute. Some monsters may use explicit target rules - for example, a predator may prefer an isolated target, while a ranged enemy may switch away from a protected frontline. Important target changes should have readable feedback.

## 6.6 Encounter Density

Tactical combat fails if every farming pull requires maximum concentration. Most overworld fights should resolve quickly. Tactical complexity should come from composition, density, terrain, elites, and special monsters rather than every individual enemy using a five-step script.

## 6.7 Elite Monsters

Selected maps may contain elite variants with one or two enhanced behaviors, better rewards, and a recognizable visual or name treatment. Elites are optional spikes in attention and can introduce mechanics later used by an MVP or story boss.

## 6.8 MVPs and Bosses

MVPs should be apex expressions of Ragnarok combat: open-world pressure, adds, positioning, statuses, displacement, ground control, target management, and recognizable signature mechanics. Avoid converting every MVP into a long scripted raid encounter. The most memorable encounters should still permit improvisation when another player arrives, someone dies, the boss moves unexpectedly, or the environment changes.

## 6.9 Example Boss Mechanic Template

| **Component**     | **Guideline**                                                                                   |
|-------------------|-------------------------------------------------------------------------------------------------|
| Signature         | One mechanic players immediately associate with the boss.                                       |
| Pressure          | Consistent baseline threat through normal attacks or adds.                                      |
| Movement Check    | Occasional reason to reposition without constant floor dancing.                                 |
| Class Opportunity | At least one mechanic where control, support, interruption, ranged play, or preparation shines. |
| Escalation        | A late-fight change in pace rather than a completely new rule set.                              |
| Recovery          | Short windows where a struggling party can stabilize.                                           |

## 6.10 Implementation Path (added v0.2)

*Mechanics checked against the Hercules fork on 2026-09-21; source implementation status refreshed 2026-09-24. Live acceptance is tracked separately in Appendix E.*

### Archetypes: mostly data, not code

Hercules already has a per-monster **mode bitfield** (`MD_AGGRESSIVE`, `MD_ASSIST`, `MD_TARGETWEAK`, `MD_CHANGETARGET_MELEE/CHASE`, `MD_CASTSENSOR_IDLE/CHASE`, `MD_ANGRY`, `MD_DETECTOR`, `MD_LOOTER`) and a **conditional skill table** (`mob_skill_db.conf`) whose triggers include own HP thresholds, a friend's HP, being targeted by a cast, being attacked at range, the master being attacked, spawn, and after-skill chaining, with targets of self / current target / random / friend / master / a cell around the target. `NPC_RUN` is a real flee skill (ten monsters use it today). The global `monster_ai` bitfield adds chase-ranged-attackers (`0x004`), scatter-on-lost-target (`0x008`) and random skill order (`0x100`).

| **Archetype**   | **Mechanism**                                                                                                                         | **Needs C?**                          |
|-----------------|---------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------|
| Aggressor       | `MD_AGGRESSIVE` + `MD_CHANGETARGET_MELEE`; a heavy attack (`NPC_CRITICALSLASH`, `NPC_COMBOATTACK`) with a visible cast time so §5.8 telegraphs it. | No                                    |
| Support         | `AL_HEAL` / `NPC_*` buffs on `MST_FRIEND` with `MSC_FRIENDHPLTMAXRATE`; `MD_ASSIST`.                                                  | No                                    |
| Controller      | `NPC_STUNATTACK`, `NPC_SLOWCAST`, `NPC_STOP`, ground `NPC_GROUNDATTACK` on `MST_AROUND`; already the most common skill family in the DB. | No                                    |
| Coward / Fleeing| `NPC_RUN` on `MSC_MYHPLTMAXRATE 30`; `monster_ai 0x008` for scatter.                                                                   | No                                    |
| Protector       | `MD_ASSIST` + `MSC_MASTERATTACKED` / `MSC_FRIENDHPLTMAXRATE` triggering a taunt-like `NPC_PROVOCATION` or a reposition.               | No (partial: true "interpose" needs C)|
| Opportunist     | `MD_TARGETWEAK` + `MD_CHANGETARGET_CHASE` + `MSC_CASTTARGETED` (switch to casters).                                                    | No                                    |
| Skirmisher      | After a bounded number of player hits, take one step outward, then resume stock AI.                                                     | **Yes — prototype present:** map+monster profiles count positive player hits, scan at most eight adjacent cells, request one path, and apply a cooldown. Live movement/load acceptance remains open. |
| Ranged Keeper   | Hold a preferred distance.                                                                                                             | **Yes — prototype present:** a map+monster profile takes one legal outward step when a player enters its configured band; cooldown and stock fallback are implemented. Live acceptance remains open. |

§6.4 (reaction to ground effects) needs a path-cost hook. A partial opt-in implementation now reads `AvoidHazards` from map+monster profiles and adds cost for active Fire Wall, Fire Pillar, Meteor, Storm Gust/Vermilion, Quagmire, Hunter traps, and NPC ground/fire attacks. It caches hazard checks inside a 17×17 window around each path origin; behavior outside that window remains stock. Orc Archer on `orcsdun02` opts in; ordinary monsters and undead remain unaffected. Boss inversion and live dense-map/path-load acceptance remain open.

### Family behaviour is a data convention

§6.3 needs no engine feature: it is a naming and review rule for `mob_skill_db` entries — family members share reviewed trigger thresholds and emotes, while role-specific kits remain explicit. `Hercules/planning/monster-family-templates.md` records the Orc-undead template; `Hercules/tools/check_mob_skill_families.py` verifies the shared Orc Zombie / Orc Skeleton skill-state parameters against `db/re/mob_skill_db.conf`. The generator for applying templates to additional family members and the Glast Heim family expansion remain follow-up work; do not treat a passing consistency audit as live AI acceptance.

### Pilot plan (answers Appendix D Q7 as a recommendation)

| **Tier** | **Map**                       | **Why**                                                                                                                                    |
|----------|-------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------|
| Early    | `prt_fild08` (Prontera Field 08)| The §9.3 example map; Poring / Lunatic / Fabre teach Aggressor and Coward with zero lethality.                                              |
| Mid      | `orcsdun01` (Orc Dungeon 1F)    | The §6.3 worked example: Orc Zombie (relentless Aggressor), Orc Skeleton, with Orc Archers on `orcsdun02` for the Ranged Keeper pilot. |
| Late     | `gl_prison` / `gl_knt01` (Glast Heim) | Appendix B is written for it: Raydric Aggressor/Protector pairs, Raydric Archer Keeper, Evil Druid Support.                          |

Pilot order: data-only archetypes on all three maps first (one day of `mob_skill_db` work per map), playtest with §5.8 telegraphs live, then the two C archetypes, then §6.4.

### First boss (Appendix D Q8, recommendation)

Prove the §6.9 template on **Eddga** (`pay_fild11`): a mid-level open-world MVP the group will meet naturally, with a tiny existing kit (Fire Ball, Fire Attack, slaves) that maps cleanly onto Signature (Fire-element ground pressure, telegraphed), Pressure (Wild Rose adds), Movement Check (a ground `NPC_FIREATTACK` area), Class Opportunity (Water-element preparation — the §8.6 elements lesson), Escalation (`MSC_MYHPLTMAXRATE 30` → faster casts), Recovery (a post-escalation pause). It is also already scripted as a DM Session beat, so the campaign version and the open-world version can share the entry.

# 7. Classes, Stats, Skills, and Build Identity

## 7.1 Preserve the Original Stat Model

STR, AGI, VIT, INT, DEX, and LUK remain meaningful build decisions. The modernization should not collapse them into generic Power, Defense, Crit, and Haste ratings. The interface should instead explain their consequences clearly and preview how allocating points affects derived stats and relevant skills.

## 7.2 Build Diversity

- AGI and VIT variants should remain meaningfully different.

- Autoattack, critical, casting, support, hybrid, crafting, and unusual builds remain valid design targets.

- Cards and equipment should enable specialization against monster families, elements, sizes, or situations.

- Niche builds do not need identical efficiency in every map; they need places where their identity matters.

## 7.3 Class Asymmetry

| **Class Family** | **Identity to Preserve / Strengthen**                                                 |
|------------------|---------------------------------------------------------------------------------------|
| Swordman         | Frontline durability, disruption, weapon specialization, physical pressure.           |
| Mage             | Area control, elemental preparation, cast positioning, burst and battlefield shaping. |
| Archer           | Range, traps, target selection, sustained pressure, terrain use.                      |
| Acolyte          | Healing, protection, buffs, status response, undead/demon specialization.             |
| Thief            | Evasion, crit/burst, stealth, disruption, opportunistic target play.                  |
| Merchant         | Combat plus economic identity, crafting, cart, discount/overcharge, equipment support. *(Vending skill exists but the vending economy is out of scope — C2.)* |

## 7.4 Skill Tree UX

Skill trees remain mechanically recognizable but gain better presentation. Every skill should clearly display level scaling, prerequisites, SP cost, range, cast time, delay, element, area, status interactions, and other meaningful mechanics. A preview mode allows players to allocate hypothetical future skill points without committing them.

## 7.5 Stat and Skill Respec

For this friends server, **both stat and skill points are intended to be reallocated for free at any level by ordinary players**. Experimentation should not require a DM, Zeny, a level/class gate, or an empty inventory. A one-point skill refund remains subject to prerequisite safety checks. Group 0 now grants registered `@streset`, `@skreset`, and `@refundskill`; Character Overview has distinct free stats-only and skills-only reset buttons. A focused disposable-server scenario verifies that stat reset restores an allocated Strength point without changing learned SM_BASH, skill reset clears SM_BASH without changing Strength, neither reset changes Zeny, and both reset results persist across relog. That harness uses its group-99 fixture to seed the test skill, so ordinary group-0 command reachability, level/job boundaries, and prerequisite-rejection behavior still need live verification. The priced Reset Girl script remains inactive; the stock Hypnotist is restricted to eligible first jobs below level 50 with no inventory weight.

## 7.6 Integrated Build Planner

The character window should support a planning state. Players choose a target Base/Job level, allocate hypothetical stats and skills, and preview expected HP, SP, Hit, Flee, ASPD, cast-time changes, and other relevant derived values. Planned changes are never applied until explicitly committed through normal progression or a respec system.

## 7.7 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Audited 2026-09-23.*

| **§** | **Design item**                     | **Hook that exists**                                                                                                                  | **Gap**                                                                                                                                                                       | **Size** |
|-------|-------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 7.1   | Derived-stat preview per point      | `stats.rs` allocates and shows raw stats; Hercules `status.c` holds the renewal formulas; `statpoint.txt` the point costs.            | Port HIT / FLEE / ASPD / cast / HP / SP formulas client-side for *preview only*, labelled "estimate"; show the delta on hover of each `+` button.                             | M        |
| 7.4   | Skill detail (delay, prereqs …)    | `tools/export_skill_info.py` already exports per-level SP cost, range, cast/fixed cast, element, target, hits, duration, area, and reagents; `skill_tooltip_text` renders these in hotbar and skill tree. | Add verified cooldown/after-cast delay, status interactions, and skill-tree prerequisites; retain compact tooltips. Formula prose needs server validation. | M |
| 7.4   | Preview allocation (no commit)      | The skill tree already stages **currently available** points in `pending_skill_points`, with Apply, Reset Pending, and Cancel. | Extend that model to hypothetical future Job levels/points without sending packets; distinguish the current allocation mode from the future build planner. | M |
| 7.5   | Free player stat/skill reallocation | Hercules registers and group 0 grants `@streset`, `@skreset`, and `@refundskill`; Character Overview exposes separate free reset actions. Focused disposable-server test verifies distinct stat/skill reset effects, zero Zeny cost, and reset persistence across relog using a group-99 fixture. | Verify group-0 reachability, level/job boundaries, and one-point refund prerequisite rejection. | partial |
| 7.6   | Build planner                       | —                                                                                                                                     | 7.1 + 7.4 preview combined behind a target level slider. Do after both; reuse the stats formulas.                                                                              | L        |
| 7.1   | Creation-screen allocator           | Done (2026-09-03/04): 48-point spread, suggested first-job template, live preview.                                                     | —                                                                                                                                                                             | done     |

# 8. Story and Quest Design

## 8.1 Story as a Spine

The centralized storyline should orient players through the world, introduce systems, and provide a coherent narrative for players who want one. It should not become a mandatory corridor through all content. A player may pause the story indefinitely and continue leveling, exploring, farming, crafting, or helping friends.

*v0.2 (decision C5):* the Seal Cascade campaign is **not** this spine — it is DM Session mode (§2.3), gated on a DM being present. The self-directed spine this section describes does not yet exist; when it is built, the campaign's party-shared quest helpers and hub-NPC markers are reusable.

Each character completes either story in their own right. Party play shares accepted quest progress to participating characters, not to every character on those players' accounts. A completed story chapter never appears automatically on a new character.

## 8.2 Quest Categories

| **Category**        | **Purpose**                                                                           | **Guidance Level**                         |
|---------------------|---------------------------------------------------------------------------------------|--------------------------------------------|
| Main Story          | Narrative spine, region introductions, major unlocks that are story-specific.         | Strong optional tracking and breadcrumbs.  |
| Class / Profession  | Teach class identity, abilities, crafting, or advancement.                            | Clear objectives; moderate guidance.       |
| Regional Side Quest | Worldbuilding, local characters, unusual rewards, map discovery.                      | Discoverable; tracking optional.           |
| Rumor / Lead        | Points toward a place, monster, mystery, or opportunity without becoming a checklist. | Map note or journal lead.                  |
| Hunting Goal        | Player-created objective such as a card or material target.                           | No formal quest completion logic required. |

## 8.3 Quest Marker Philosophy

Quest markers are optional and restrained. Main-story objectives may use a distinct marker. Side quests may use a subtler marker once discovered. The player can disable all markers. Do not cover every town with dozens of icons before the player has interacted with the relevant content.

## 8.4 Breadcrumb Guidance

When a quest is tracked, navigation should answer the next useful question: which map exit, building, or region should I head toward? It should not draw a continuous glowing path across the ground. Once the player reaches the correct map, the objective may narrow to a building, NPC region, or general search area depending on the intended exploration challenge.

## 8.5 Quest Design Rules

- Use kill counts only when the act of hunting itself is meaningful.

- Avoid filler chains whose only purpose is to extend time.

- Use story quests to introduce systems through play rather than long tutorial popups.

- Let quests reveal rumors, maps, NPC services, monster information, and shortcuts.

- Do not make every valuable monster or dungeon require a quest unlock.

- Respect group play: nearby party members should receive reasonable shared progress where the fiction and mechanics support it.

- In DM Session mode, accepted story quest and flag changes reach every character in the campaign party. Online members update immediately; disconnected members catch up on return. A character joining the active party late automatically inherits the party's current campaign quests and story flags, without receiving past rewards. Another character on the same account inherits nothing until it joins that party. See the [DM party quest-sync contract](specs/dm-party-quest-sync.md).

## 8.6 Story-Based System Teaching

| **System**      | **Example Teaching Moment**                                                                                              |
|-----------------|--------------------------------------------------------------------------------------------------------------------------|
| Elements        | A quest sends the player into an area where an elemental weapon or spell advantage is obvious.                           |
| Cards           | A character explains a monster-specific card after the player encounters their first rare card-related clue.             |
| Refinement      | A smith-related story step introduces risk, ores, and safe ranges using a real item.                                     |
| Party Play      | A dungeon story objective encourages a small group and highlights party frames / shared destination tools.               |
| Monster Journal | A researcher asks the player to observe a family of monsters, teaching discovery without forcing completionist behavior. |

## 8.7 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Status refreshed 2026-09-24.*

| **§**  | **Design item**                      | **Hook that exists**                                                                                                                                   | **Gap**                                                                                                                                                                                | **Size** |
|--------|--------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 8.4    | Breadcrumb to the next exit          | Generated JSON graph from loaded static warp scripts; client shortest-hop routing, minimap exit marker, sampled walkable breadcrumbs, a 27-location atlas, hunt/item-source routing, validated `<NAVI>` buttons, and selected-target portal details are present in source. | Expand travel/service coverage and validate coordinates, displayed next exits, and route behavior in a live client. | M |
| 8.4    | Clickable `<NAVI>` links in dialogue | `dialog.rs` parses valid short/full `<NAVI>` targets into labeled route buttons; same-map targets mark their cell and cross-map targets choose a verified graph route. | Live-check focus order, current-map accuracy, destination coordinates, and guidance clearing. Malformed or unreachable targets remain readable and do not replace the active route. | verify |
| 8.3    | Quest markers with a player toggle   | `QuestIcon` particles render server quest effects on NPCs.                                                                                             | A Game Settings toggle (like `show_minimap`) that suppresses them.                                                                                                                       | S        |
| 10.11  | On-HUD quest tracker                 | Track/Untrack, per-character persistence, server hunt counts, and supported map-level routes. Item turn-ins link to Guide item entries and verified drop-source maps. Guide monsters can be saved as separate client-only personal hunt goals, persisted per character, listed in the HUD/log, and routed to verified broad spawn maps. | Add explicit NPC/story objective locations; live-check both server and personal-goal persistence, routes, and reconnect behavior. | M |
| 8.2    | Hunting goals / journal leads        | —                                                                                                                                                      | Client-side pinned list (item / monster / map name), persisted in settings, shown in the tracker; link to the §9.5 entry.                                                                | S-M      |
| 8.5    | Shared progress for stock quests     | `mob.c` calls `quest->update_objective_sub` for nearby party members; `quest.c` checks party ID and active quest. `AREA_SIZE` defaults to 14 cells in `conf/map/battle/client.conf`, overridden to 30 by the active local `conf/import/battle.conf`. | Disposable live scenario verifies mixed ownership, solo/former-party behavior, the exact 30/31-cell boundary, and one credit per character when both party members land hits on the same monster. Simultaneous separate-monster kills remain to be tested. | partial |
| 8.5    | DM Session party story sync          | Party join, quest-log, and map-load hooks reconcile allowlisted campaign state; typed quest/flag events are journaled and replayed with per-character SQL cursors. Replay never grants past rewards. `dm-party-offline-replay` verifies a disconnected member receives a quest start and flag on reconnect, repeated catch-up does not duplicate the active quest notification, and a disposable MariaDB audit confirms the quest/flag plus cleanup events were journaled and that the returning character cursor reached the party journal tail. | Verify multi-transition cursor order, late join, alternate-character isolation, party recreation, reward isolation, and failed-replay repair; add explicit run identity/repair reporting if party migration testing requires it. [Contract](specs/dm-party-quest-sync.md), S10. | M |
| 8.6    | Story-based teaching                 | DM Session mode already teaches through play (skill checks, hazards).                                                                                  | Content for the future self-directed spine; not before Phase 4.                                                                                                                         | L        |

# 9. World, Exploration, and Navigation

## 9.1 Navigation Philosophy

> **LOCKED PRINCIPLE:** Remove confusion, not curiosity. Players may get lost because they chose to explore; they should not remain lost because the game refuses to explain how maps connect.

## 9.2 World Map

The world map should show major regions, towns, known dungeons, transportation routes, and discovered connections. Selecting a destination displays useful information such as suggested level, known monsters, travel route, services, and whether the player has visited the location. The map never acts as unrestricted instant teleportation.

## 9.3 Map Information Panel

The World Map & Route Finder now shows a partial map-information panel for the selected destination, or the current map when no destination is selected. It reports the mean monster level weighted by exported static spawn-record count, indexed static spawn-record/species counts, verified outgoing portal and authored service edges, the next route step (including service action and known fare), account visit-sync state, online party members whose reported map matches, and facility names from the client Towninfo table for any selected map with entries. These are reference summaries, not live population counts or a level recommendation; conditional/scripted spawns and broader NPC/service coverage are not included. Missing spawn data is labeled unavailable rather than inferred.

| **Field**          | **Example**                                                                 |
|--------------------|-----------------------------------------------------------------------------|
| Map                | Prontera Field 08                                                           |
| Suggested Level    | 8-20                                                                        |
| Monster Population | Poring - Very Common; Lunatic - Common; Pupa - Uncommon; Creamy - Rare      |
| Connections        | North -\> Prontera Field 05; East -\> Prontera; South -\> Prontera Field 11 |
| Services / POIs    | South Gate, optional field NPC, Kafra if applicable                         |
| Party Presence     | Party members currently on map                                              |

## 9.4 Monster Population Regions

Players may select a known monster and display broad population regions on a map. Regions indicate high, medium, or low concentration rather than exact coordinates or respawn timers. This tells the player where to hunt while preserving the act of searching and learning the terrain.

## 9.5 Adventure Guide — the In-Game Encyclopedia

*Rewritten in v0.2 (decision C1, 2026-09-21).*

> **LOCKED PRINCIPLE:** A player should never need to leave the client to learn how this server works. Everything a player would look up on an external wiki is available in the game, searchable, and correct for **this** server.

External wikis document official servers. This server runs a Hercules renewal fork with its own drop rates, custom quests, custom NPCs, fork-only packets, and a DM campaign; an external wiki is wrong about it in ways a player cannot detect. The encyclopedia is therefore **generated from the server's own tables**, not transcribed from a wiki, so it stays true as the server changes.

### Coverage

The target is the coverage of a full RO wiki. Categories, with the server-side source of truth for each:

| **Category**        | **Content**                                                                                                          | **Source of truth**                                                                                   | **In-tree today**                                        |
|---------------------|----------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------|----------------------------------------------------------|
| Monsters            | Stats, level, HP, element/race/size, modes, skills used, spawn maps and density, drops with rates, card, MVP status.  | `db/re/mob_db.conf`, `mob_skill_db.conf`, `npc/**/mobs/*.txt` spawn lines                             | `bestiary.v1.json` (1,759; stats/modes/skills/drops; no spawn data) plus legacy `bestiary.json`; `mob_lore.json` (540 flavour) |
| Items               | Every item: stats, script effects in plain language, weight, slots, refinable, who can equip, buy/sell, where it drops. | `db/re/item_db.conf`, `db/item_db2.conf`, `item_combo_db.conf`, `item_group.conf`                     | `items.v1.json` (13,183 incl. item_db2; effects marked untranslated) plus legacy `items.json` (13,182) |
| Cards               | Effect, slot type, source monster, compounding notes, set/combo bonuses.                                             | `item_db.conf` (card type), `item_combo_db.conf`                                                      | `cards.v1.json` (1,012, linked drops; effects untranslated) plus legacy `cards.json` |
| Skills              | Per level: SP, cast time, delay, cooldown, range, area, element, damage formula, status inflicted, prerequisites.     | `db/re/skill_db.conf`, `skill_tree.conf`, `sc_config.conf`                                            | `skills.json` (1,170; costs/range/cast/element/area/reagents already exported and shown in tooltips; delays, prerequisites, verified formulas remain) |
| Jobs / classes      | Job tree, change requirements, stat bonuses per job level, skill trees, base/job EXP tables.                          | `job_db.conf`, `skill_tree.conf`, `exp_group_db.conf`, `statpoint.txt`, jobmaster NPC                 | —                                                        |
| Status effects      | What each buff/debuff does, duration, what cures it, what causes it, icon.                                           | `sc_config.conf`, `skill_db.conf`, `src/map/*.c` literal `sc_start` calls | 700 icon names are searchable; 540 link to verified status IDs/config flags, direct/skill-db links, and 114 literal C call-site references across 75 statuses. Call sites are non-exhaustive navigation evidence, not full effect/source/cure documentation. Exact outcomes, duration/odds, complete sources, interactions, and cures still need verified data. |
| Maps                | Name, region, suggested level, connections (warp graph), monster population, NPCs and services, Kafra, map flags.    | `db/re/map_zone_db.conf`, `npc/**/warps/*.txt`, `npc/**/mobs/*.txt`, `Towninfo`                        | Guide details graph maps with static-spawn mean/counts and verified outgoing exits/routes; atlas adds account visits, danger marker, party presence, and Towninfo facilities for any listed map. Authored services, map flags, conditional spawns remain incomplete. |
| Quests              | Name, giver, steps, rewards, prerequisites, level; custom and campaign quests included.                              | `db/re/quest_db.conf`, `npc/re/quests/**`, `npc/custom/quests/**`, campaign quest IDs 20000-20234    | `quests.v1.json` (3,172 tracked-HEAD names, 468 hunt targets, 842 with related static NPC-script references); Guide searches reference + active server quests, shows active counts, and routes exact NPC cells, known monsters, and supported maps. NPC relations are script references, not verified givers; story steps, rewards, prerequisites, and full custom coverage remain open. |
| NPCs                | Name, map and coordinates, function (shop, quest, service), shop inventory.                                          | `npc/**/*.txt` headers, shop lines                                                                    | `Towninfo` POIs                                          |
| Mechanics           | Stat formulas (HIT, FLEE, ASPD, cast time), elemental table, size/race modifiers, refinement odds, EXP penalty, party share. | `attr_fix.conf`, `size_fix.txt`, `refine_db.conf`, `level_penalty.conf`, `battle/*.conf` values     | —                                                        |
| Server rules        | Effective rates, party level range/bonus, death penalty, autoloot, free stat/skill resets, DM Session mode. | `conf/map/battle/*.conf`, `conf/import/*.conf`, `conf/common/inter-server.conf`, `groups.conf`, this document | — |

### Behaviour

- **One search box, categorised results.** Searching "Hydra" returns the monster, its card, the maps it spawns on, quests that mention it, and a Navigate action (§9.9). Searching "Oridecon" returns the item, every monster that drops it with rates, and the refinement mechanics page.

- **Everything links.** Item → source monster → spawn map → route. Skill → status inflicted → what cures it. Job → skill tree → each skill's page. No result is a dead end.

- **Correct for this server.** Rates, drops, and quest steps reflect the fork's tables, including custom NPCs (warper, healer, jobmaster, stylist) and any future rebalancing. When a value is a client-side estimate rather than a server table (§10.10), it is labelled as such.

- **Reference knowledge is accessible from the start.** A fresh account can search monsters, items, cards, skills, jobs, maps, quests, status effects, mechanics, and server rules as their verified data ships. Build-relevant facts—including stats, effects, requirements, sources, drops/rates, and boss mechanics—do not require a kill or lore check. Unknown fields are labeled “not documented yet.” Account-wide discovery adds journal history, badges, and non-spoiler encounter notes, not a mechanical-information gate. Campaign plot spoilers and DM controls remain separate.

- **Flavour text is attributed and rewritten.** Wiki prose (the fandom extracts in `mob_lore.json` are CC-BY-SA) is reference material for the writer, not player-facing copy. Player-facing lore is authored in the server's voice with attribution kept in the data file.

### Build order

1. **Generator.** Keep the existing reproducible `tools/` JSON exporters aligned with the Hercules tree whenever `db/` or `npc/` changes. Script-only rules and prose need reviewed authored supplements; unsupported fields must say so. Skill export/tooltips and monster/item/card exporters now have drift checks. See the [data contract](specs/encyclopedia-data.md).

2. **Player Guide — shipped partial slice.** The separate player-facing Adventure Guide has an All search plus category filters, versioned references, account discovery badges, source/incomplete-data labels, reciprocal skill/status links, and supported quest/monster/map routes. It remains separate from DM Bestiary reveal controls. This is the first shippable slice for build research, not the completed encyclopedia.

3. **Remaining coverage** in order of playtest demand: verified status outcomes/cures and fuller skill explanations; authored map/NPC and quest giver/story/reward data; mechanics and effective server rules. Status icons already link to server IDs/flags/associated skills, and quest hunt references are searchable, but these are explicitly partial.

4. **Linking and Navigate — partial.** Static graph routes, monster spawn-map links, tracked quest hunt routes, and item/drop-source routes are wired where graph-known data exists; NPC/scripted and conditional locations remain.

## 9.6 Discovery and Reference Access

The friends-server baseline is **Open Database**: all verified reference data is searchable immediately, regardless of encounter history. Account-wide discovery records what the player has experienced without limiting research. General discoveries and non-story access (including future visited places, rumors, and general services) carry across characters on an account; **story quests, chapter completion, choices, and story-gated access do not**. Character build and inventory also remain character state. A later opt-in restrictive mode would need its own owner decision and cannot silently replace this baseline.

## 9.7 Rumors

NPC dialogue, signs, books, quests, and exploration can add Rumors to the Adventure Guide. A rumor is information, not an obligation. Examples: "Adventurers have seen unusual creatures beneath the sea near Izlude" or "Merchants near Morroc are paying well for Ant Jaws." A rumor may add a map note without creating a kill-count quest.

## 9.8 Travel

- Kafra, Warp Portal, boats, airships, and other in-world transportation remain meaningful.

- Discovered services are visible on the world map with their destinations.

- No unrestricted click-to-teleport world map by default.

- *Exception (decision C4, 2026-09-21):* `@partyjump <name>` lets any party member warp to any online party member. It is accepted as a friends-first convenience on this private server and is deliberately unbounded for now; it would need a cooldown or same-map rule before any public use.

- Fast travel may be expanded later for repeated routes, but should preserve class and world-travel identity.

## 9.9 Portal Labels

Hovering a graph-known warp identifies its destination; the next verified exit on the tracked route is labeled “Route portal.” Unknown/unindexed warps retain their existing hover behavior. Code and unit coverage are present; in-world readability and route matching still need live visual acceptance.

## 9.10 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Status refreshed 2026-09-24.*

| **§** | **Design item**                | **Hook that exists**                                                                                                                                  | **Gap**                                                                                                                                                                                    | **Size** |
|-------|--------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 9.2   | World map                      | Generated static-warp graph plus two reviewed Izlude NPC-service edges and in-game atlas/route finder; selected destination details show visit-sync state, known outgoing connections, route hop count, and the next edge's exact map cell/destination/action. Current-map walkable breadcrumb routing and Guide/atlas visited badges are present. The headless live tests traversed Prontera ↔ `prt_fild08` through walk-triggered warps on PACKETVER 20220406 and verified the complete Izlude Sailor → `izlu2dun` → `iz_dun00` route, including the 150-zenny fare. | Broader dynamic/conditional travel coverage, client route following, and visual acceptance remain. No teleport. | M-L |
| 9.3   | Map information panel          | Atlas panel summarizes exported spawn-record-weighted mean level, static spawn-record/species counts, verified outgoing graph edges/next route exit, visit-sync state, online party members on the selected/current map, and Towninfo facility names for any listed map. A map at least 15 mean levels above the character receives a contrasting `!` outline marker. | Add authored NPC-service POIs, broader/labelled conditional-spawn coverage, and low-coverage context; then visually/live-check. Data is static and is not a real-time monster count or entry requirement. | M |
| 9.4   | Population regions             | Static loaded spawn directives are exported as map-level records for 965 monsters; coordinate/spread fields remain deliberately unused. | Add reviewed conditional/scripted coverage and a low/medium/high selected-monster overlay on maps/minimap; never expose exact points. | M |
| 9.6   | Account discovery record       | Hercules stores monster milestones and visited maps in account-keyed SQL ledgers. Account/sequence-bound snapshots and first-kill/map-visit deltas travel in private server messages; Korangar stages complete snapshots and merges them monotonically with queued deltas. Guide badges never gate mechanics. The rebuilt-server `account-discovery-isolation` test now passes for monster and map deltas, same-account alternate-character snapshots, and isolation from a second account. | Verify production migration/startup and DM-ledger separation; add later reviewed encounter milestones. Never gate verified mechanics. | partial |
| 9.7   | Rumors                         | —                                                                                                                                                     | A `rumor` journal category fed by NPC scripts (`callfunc("Journal_AddRumor", id)`) and synced like unlocks. After §9.6.                                                                     | M        |
| 9.9   | Portal labels                  | Graph-known warp entities expose their destination on hover; the next exit on the selected route is prefixed “Route portal.” Unknown edges retain existing hover text. | In-world readability, hover behavior while approaching, and route accent need live visual acceptance.                                                                   | S        |
| 9.8   | Travel                         | Stock; `@partyjump` (C4).                                                                                                                             | —                                                                                                                                                                                          | —        |

# 10. User Interface and User Experience

## 10.1 UI Vision

The UI should use modern interaction standards while retaining Ragnarok's compact visual language. Windows, borders, icons, sounds, item art, and typography should feel compatible with the original client. The modernization is primarily organizational: better information hierarchy, customization, scaling, search, and contextual explanation.

## 10.2 HUD Presets and Edit Mode

- Classic Layout preset for returning players.

- Modern Layout preset emphasizing target, party, navigation, and cleaner spacing.

- HUD Edit Mode allowing drag, scale, show/hide, snap, and anchor options.

- Save profiles such as Exploration, Dungeon, Healer, Farming, and Minimal.

- Out-of-combat fading for combat-only elements.

## 10.3 Player Frame

The player frame shows HP, SP, Base/Job progress, and important status effects without becoming oversized. Buffs and debuffs should support compact icons, timers, optional grouping, and hover explanations. Players can choose numerical, percentage, or bar-focused display.

## 10.4 Target Frame

The target frame shows name, health state, important status effects, known element/race/size, and active cast or major action state. Ordinary monsters remain compact. MVPs and major story bosses may use a larger boss frame with signature mechanic support.

## 10.5 Tactical Combat UI

- Cast bars for the player and important enemy casts.

- Optional skill-range preview on modifier or hover.

- Ground-target footprints for placement skills.

- Distinct but restrained status and interrupt cues.

- Optional target-of-target for players who want deeper party information.

- Critical enemy effects can remain visible even when general effect density is reduced.

## 10.6 Hotbars and Keybinds

Multiple hotbars are supported, but the interface should not imply that every slot must be filled. Players may bind number keys, modifiers, mouse buttons, and custom keys. Separate bars may be used for combat, buffs, equipment, consumables, or social actions. A skill remains usable through the classic interface for players who prefer it.

## 10.7 Equipment Sets

Players may create named equipment sets such as Undead, Demi-Human, Fire Resist, Farming, or Boss. Swaps obey combat restrictions and should not bypass animation locks, status restrictions, or other intended rules. The system exists to reduce inventory friction, not create instant combat exploits.

## 10.8 Character Stats Interface

| **Mode** | **Information**                                                                  |
|----------|----------------------------------------------------------------------------------|
| Simple   | Plain-language description of what the stat improves plus current major effects. |
| Detailed | Derived stat changes for the next point and selected skills affected.            |
| Advanced | Relevant formulas, breakpoints, and exact component values where available.      |

## 10.9 Inventory and Storage

- Search by name.

- Filters for equipment, consumables, cards, materials, quest items, favorites, and recent loot.

- Sort by name, category, weight, quantity, value, or acquisition time.

- Favorite / Do Not Sell / Do Not Drop protections.

- Kafra storage search and filters.

- Clear weight usage display and warnings before meaningful thresholds.

**Explicitly deferred filter semantics:** Quest-item status is not inferred from the broad `Etc` item type; it needs an authoritative server classification. Favorites need consistent favorite metadata across inventory/storage snapshots and relogs. Recent-loot filtering needs a defined acquisition ledger and persistence policy. Until those contracts exist, the implemented filters remain limited to server item types.

## 10.10 Equipment Comparison

Tooltips compare equipped and hovered items using direct stat changes first. An optional context mode estimates performance against the currently selected monster. Advanced comparison can explain why one item performs better through race, size, element, ASPD, defense, or card interactions. Estimates must be clearly labeled if exact server formulas cannot be calculated client-side.

## 10.11 Quest Tracker

Players choose which quests appear. The tracker shows objective text, immediate destination, and maps remaining where appropriate. Clicking an objective opens the relevant map or navigation route. The tracker can be collapsed or hidden entirely.

## 10.12 Minimap

- Zoom and pan where technically practical.

- Layer toggles for portals, NPCs, quests, party members, Kafra, shops, monster regions, and personal markers.

- Clickable personal waypoint placement.

- Party-shared destination and pings.

- Exit labels and tracked-route highlight.

## 10.13 Global Search

*v0.2: the full specification now lives in §9.5.* The Adventure Guide search field becomes the player's in-game reference tool. Results are categorized rather than dumped into one list. The player can move from item -\> source monster -\> map -\> navigation without leaving the client.

## 10.14 Party UI

Party frames show class, HP, SP where appropriate, important statuses, death state, and approximate distance/out-of-map state. Clicking a frame targets that member. Optional healer layout enlarges health/status readability. Party member locations are visible on maps, and "Navigate to Party Member" is available for separated friends.

## 10.15 Chat and Linking

- Tabs for General, Party, Whisper, System, Loot, and future channels.

- Timestamps and filtering.

- Clickable player names and copyable text.

- Item links that show the exact item tooltip, refinement, slots, and cards.

- Links to monster, map, or quest journal entries where supported.

## 10.16 UI Scaling

UI scaling must support modern display resolutions without blurring original pixel art. Use integer or nearest-neighbor scaling for pixel assets where possible. Text, panels, and interaction areas should scale independently enough to remain accessible on high-DPI displays.

## 10.17 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Status refreshed 2026-09-24.*

| **§**  | **Design item**                         | **Hook that exists**                                                                                                                                                  | **Gap**                                                                                                                                                                                | **Size** |
|--------|-----------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 10.2   | HUD Edit Mode + presets                 | Partial: `WindowCache` persists geometry with old-cache migration; Game Settings supports lock/unlock, Classic/Modern layouts, per-character `My Layout`, and reset. | Combat-only fading, adjustable snapping behavior, and visual acceptance remain. | M |
| 10.3   | Status icons with timers                | Text status bar with monograms + timers + descriptions (`status_bar.rs`, M1-010). The GRFs ship `texture/effect/*.tga` icons indexed by `System/stateiconimginfo.lub`. | Load the lub table like `Towninfo`, draw the icon beside the text; verify the table is present in the shipped GRFs first.                                                             | S-M      |
| 10.6   | Keybind remap screen                    | Partial: versioned RON overrides drive main UI/navigation shortcuts, WASD movement, all 27 number-row hotbar slots, and debug-camera movement, four keyboard look directions, and hold-to-accelerate; Game Settings captures chords, rejects reserved/conflicting keys (allowing same-direction contextual camera/movement defaults), resets, and imports/exports `client/keybindings.ron`. Mouse-look remains available. | Remaining fixed aliases, localization, and keyboard-layout/live acceptance. | M |
| 10.6   | Hotbars                                 | Three rows, items and skills, server-stored (done 2026-09-05).                                                                                                        | Optional: a fourth vertical bar for buffs/consumables; mouse-button binds (needs 10.6 remap).                                                                                          | S        |
| 10.7   | Equipment sets                          | `RequestEquipItemPacket` per item; inventory ids known.                                                                                                              | Named sets stored client-side as item ids; "Equip set" loops the packets in slot order and reports what was missing. Server rules still apply per packet.                              | M        |
| 10.8   | Stats interface modes                   | Needs §7.7 formulas.                                                                                                                                                  | Simple / Detailed / Advanced tabs over the same numbers.                                                                                                                                | M        |
| 10.9   | Inventory search / sort / filters       | Inventory has All/Equipped/Gear/Items plus server-item-type Consumables/Etc/Cards/Ammo tabs, search, sort, drag arrangement, and character-persisted item locks/order. Split-stack is server-authoritative. | Quest-item, favorite, and recent-loot semantics explicitly deferred pending authoritative metadata/persistence contracts; remaining live/visual acceptance. | M |
| 10.9   | Storage search                          | Storage has name search plus shared Gear/Items/Consumables/Etc/Cards/Ammo filters. | Remaining live/visual acceptance; semantically unsupported filters remain deferred with inventory. | S |
| 10.10  | Equipment comparison                    | **Partly done:** tooltip shows "— vs equipped —" deltas for ATK / MATK / DEF / slots / refine (`item_stats.rs`).                                                      | Plain-language script bonuses (`Script` field in `items.json` → "+10% vs Demi-Human"); optional "vs current target" using the §5.13 element/race/size chips.                          | S-M      |
| 10.12  | Minimap layers / waypoints / pings      | Blips for player, party, compass, Towninfo POIs, and six expiring party ping kinds. Latest accepted ping has labeled minimap and in-world markers; a selected map route can be shared with the party and accepted into local navigation. Ready checks have nonce-bound start/replies, a 30-second timeout, and one response per roster member. | Layer toggles, personal waypoints, and live mixed-client carrier acceptance. | S / M |
| 10.14  | Party frame click-to-target, distance   | Roster with HP/SP, class, Go-to.                                                                                                                                       | Click a row → `player_target`; show "other map" or tile distance from `party_state`.                                                                                                    | S        |
| 10.15  | Chat: timestamps, item links, tabs      | Public / Party / Whisper channels. `<ITEM>` tags are stripped in `dialog.rs`.                                                                                        | Timestamps S; a Loot/System filter S; `<ITEM>` → hover tooltip via the existing item tooltip S-M; text selection/copy needs a framework primitive M.                                    | S-M      |
| 10.16  | UI scaling                              | Done.                                                                                                                                                                 | —                                                                                                                                                                                      | done     |
| —      | Character delete confirmation (M1-014)  | Character card exposes Delete / Switch; deletion still has a `WarningBanner` and exact-name confirmation before the existing delete event.                                                                                               | Live-check discoverability and Windows-pack item-drop behavior.                             | S        |
| —      | Toast notifications                     | Bounded priority-aware toast queue and HUD display are implemented for quest, level/EXP, item pickup, party membership, and sent/received party pings. Toast text is static, with no motion or flashing. | Wire remaining event sources and verify timing/readability in live UI. | S-M |

# 11. Loot, Equipment, Cards, and Refinement

## 11.1 Loot Philosophy

Loot remains physically connected to monsters and maps. Valuable items should matter because of utility, rarity, build relevance, or crafting demand. The redesign should not replace monster-specific farming with universal token vendors.

## 11.2 Loot Pickup

Recommended default: configurable nearby pickup for ordinary drops, with rare items always receiving a visible ground presence before collection. Weight and inventory rules remain. Auto-loot is a convenience system, not an infinite storage bypass.

*v0.2 (decision C6):* Hercules `@autoloot <percent>`, `@alootid` and `@autoloottype` are granted to every player (group 0). Players should not need the host to enable pickup. The GDD's category/wishlist filter model (§11.3) is a later client-side layer over these commands.

## 11.3 Loot Filters

- Always show Cards.

- Always show selected rare materials and wishlisted items.

- Optional categories for equipment, consumables, quest materials, crafting materials, and common loot.

- Per-item ignore and favorite rules.

- Separate visual and pickup rules where technically feasible.

## 11.4 Rare-Drop Presentation

A rare card drop should use the original card art, a distinctive but restrained sound, and a short notification. Avoid giant beams, full-screen banners, or excessive particle effects. Restraint reinforces the significance of the drop.

## 11.5 Cards

Cards remain a central form of horizontal progression and specialization. The Monster Journal links a monster to its card and the Item Search links the card back to the monster. The interface should show card slotting consequences clearly but should not remove the need for build planning.

## 11.6 Refinement

Refinement retains tension and resource cost. Before an attempt, the UI must clearly show success chance if the server exposes it, required materials, cost, and exact failure consequence. Cash-shop protection or improved RNG is outside the design direction. Risk models may be adjusted for the friends-server population to avoid excessive destructive loss, but the final model is a playtest decision.

## 11.7 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Audited 2026-09-21.*

| **§** | **Design item**            | **Hook that exists**                                                                                                     | **Gap**                                                                                                                                                          | **Size** |
|-------|----------------------------|--------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 11.2  | Player-controlled pickup   | `@autoloot` / `@alootid` / `@autoloottype` now group 0 (C6); Commands window button.                                     | A Loot tab in Game Settings that sends the three commands (rate slider, type checkboxes, per-item list) so nobody types `@` commands.                            | S        |
| 11.3  | Visual loot filters        | `ground_item.rs` renders items with English names; `items.json` `Type` is available.                                     | Client-side: hide/dim label for categories the player unchecked; **cards and wishlisted items always shown**; separate from the pickup rules above.               | S-M      |
| 11.4  | Rare-drop presentation     | Ground items know their id; card items are `Type` card.                                                                  | On a card drop within view: card-art label, one restrained sound cue (§16.5), one toast. No beams.                                                                | S        |
| 11.5  | Cards ↔ monsters           | `cards.json` has `DropsFrom`; DM Bestiary shows it.                                                                      | Encyclopedia (§9.5) category; card tooltip gets a "Drops from →" link.                                                                                            | S        |
| 11.6  | Refinement odds            | `weapon_refine.rs` window; `db/re/refine_db.conf` has success rates per level and material.                              | Export rates into the JSON; show "Success 60% · on failure: item destroyed" before the attempt. Risk model tuning is a playtest value.                            | S        |
| 12.2  | Small-server rates         | All rates stock in `conf/map/battle/{drops,exp}.conf`.                                                                   | Playtest decision; when made, one `conf/import/battle.conf` edit.                                                                                                | S        |

# 12. Economy, Vending, and Crafting

## 12.1 Player Vending — Out of Scope

*Rewritten in v0.2 (decision C2).* Player vending, market search, buying stores and auction are **out of scope** for this server. With a single friends group there is no population to keep shops occupied, so the atmosphere argument for vending does not apply. Loot moves between players through **direct 1-on-1 trade**, which is implemented and must remain solid (drag-item grid, zeny entry, last-second-change highlight, two-client live validation are the open items). Revisit only if the player population grows enough to sustain listings.

## 12.2 Small-Server Economy

A friends server has lower liquidity than a public MMO. Drop rates, crafting requirements, rare-item scarcity, and money sinks should be tuned for a small population rather than copied blindly from official rates. The goal is to preserve excitement without creating weeks of dead-end farming for an item that would normally be supplied by hundreds of players.

## 12.3 Crafting Professions

Blacksmith and Alchemist economic identities should be strengthened. Crafting can support commission orders, crafter signatures, clearer quality or success information, and request boards. The system should make individual friends known for what their characters contribute.

## 12.4 Commission Board

| **Field**          | **Example**                                    |
|--------------------|------------------------------------------------|
| Requested Item     | Fire Claymore                                  |
| Materials          | Provided / Needed                              |
| Desired Refinement | +7                                             |
| Payment            | 350,000z                                       |
| Crafter            | Open request or specific player                |
| Risk Disclosure    | Expected chance and failure result if relevant |

## 12.5 Currency Sinks

Useful sinks may include refinement, crafting fees, transport, cosmetic services, and convenience services. Stat and skill reallocation stay free. Avoid punitive sinks that exist only to slow friends from playing together.

# 13. Party and Friends-Server Social Systems

## 13.1 Friends-First Principle

> **LOCKED PRINCIPLE:** The server should rarely create a mechanical reason for friends to avoid playing together.

## 13.2 Shared Destinations

A party leader or member can propose a destination such as Orc Dungeon, Payon Cave 3F, or a personal marker. Party members accept the route and receive the same breadcrumb guidance. The system coordinates travel without teleporting the party. *(v0.2: `@partyjump` already exists as a regroup teleport — decision C4. Shared destinations remain the intended travel tool; the two coexist.)*

## 13.3 Party Pings

- Go Here

- Danger

- Monster / MVP

- Loot

- Help

- Regroup

Pings should be temporary, readable, and usable on the minimap/world view without filling the screen with permanent markers.

## 13.4 Personal and Shared Hunting Goals

Players can pin goals such as "Raydric Card," "20 Oridecon," or "Explore Glast Heim Churchyard." A party can optionally maintain a shared Tonight's Goals list. These are player-authored objectives, not daily quests, and carry no mandatory reward track.

## 13.5 Level Difference Handling

Exact rules require playtesting. Options include widening party experience ranges, applying diminishing rather than hard cutoffs, or an optional level-sync mode that temporarily reduces an overleveled character while preserving build identity. The system should not allow trivial power-level abuse, but it should favor social play over perfect XP optimization.

## 13.6 Shared Quest Progress

For ordinary kill, collection, or interaction objectives, nearby party members should share progress where doing so does not break story logic. Personal story choices or unique interactions can remain individual.

## 13.7 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Status refreshed 2026-09-24.*

**The transport decision.** Pings, shared destinations, ready checks, and hunting-goal lists need a small channel for ephemeral party state. Bestiary unlocks and rumors need a separate **server-originated** authoritative sync, even if both use the same parser/carrier. The bounded party-chat carrier now handles location pings, shared-route set/accept messages, and nonce-bound ready-check start/reply messages; server-originated discovery sync remains. `[DMJ]` remains separate from player discovery. Keep the carrier contract explicit:

- **Phase A — party chat feasibility gate.** A prototype location ping uses a versioned printable party-chat payload, so stock clients receive a human-readable map/cell fallback. Verify actual packet byte/rate limits, duplicate behavior, sender attribution, and mixed-client usability; move to Phase B if the party-chat carrier cannot meet the contract. Party peers never grant persistent quest/bestiary state.
- **Phase B — a fork packet** (`CZ_/ZC_PARTY_STATE`, ID allocated only after checking the packet registry on both forks) if Phase A cannot meet the contract. Keep the payload model independent of its carrier.

| **§** | **Design item**                | **Hook that exists**                                                    | **Gap**                                                                                                                                       | **Size** |
|-------|--------------------------------|-------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 13.3  | Party pings (6 kinds)          | Minimap buttons send six versioned readable ping intents through ordinary party chat; Ctrl+Alt+G is a remappable shortcut for the Danger ping. Korangar renders a replaceable 15-second same-map marker on the minimap and in-world plus a toast, preserving legacy v1 location pings. A party member can share the selected map/cell route; members see who shared it, choose whether to route locally, and acknowledge acceptance. Nonce-bound ready checks snapshot online members, accept one reply per participant, and expire after 30 seconds. Hercules caps recognized ping/session messages at 128 bytes and one per character per second. | Verify mixed-client behavior and complete live acceptance; rebuild the modified server C source. | M |
| 13.2  | Shared destinations            | Map graph (§8.7).                                                       | Party members can propose a selected map/cell route, accept locally, and acknowledge by matching nonce; remains ephemeral party chat, not a teleport or persistent unlock. | Implemented in client/source; mixed-client live acceptance pending |
| 13.4  | Tonight's Goals                | —                                                                       | Client list, shared over Phase A on change; shown in the tracker.                                                                              | S-M      |
| 13.5  | Level range                    | Effective `conf/import/battle.conf` overrides `party_even_share_bonus: 25`; `conf/common/inter-server.conf` now sets `party_share_level: 30` as a playtest candidate. | Verify under/over-limit, nearby/far members, and total EXP before accepting the wider limit; level sync remains a later decision. | S |
| 13.6  | Shared stock quest progress    | See §8.7.                                                               | Live verification of the existing `mob.c` party iterator; optional independent credit-range config only if needed. | verify |
| —     | Ready check / target marker    | Same transport.                                                         | Ready: one query, N replies, a toast. Marker: entity id + icon, drawn over the entity for the party.                                           | S each   |

# 14. Death, Recovery, and Difficulty

## 14.1 Death Philosophy

Death should create tension and encourage better play without turning technical failure or experimentation into lost evenings. Danger matters; excessive time deletion does not.

## 14.2 Recommended Recovery Model

Use a modest experience penalty with a recoverable component. A player who returns to the dangerous area, defeats enemies, reaches a recovery point, or completes a short recovery condition can regain part of the loss. Exact percentages are playtest values.

## 14.3 Difficulty Communication

Maps and monsters may communicate a suggested level or danger rating. These are warnings, not locks. A lower-level player should still be allowed to enter a dangerous area and discover why the warning existed.

## 14.4 Adaptive Group Pressure

Avoid aggressive hidden scaling that makes character progression feel meaningless. If encounter scaling is ever used, prefer modest party-size adjustments on selected story instances rather than global world scaling. Open-world monsters should largely retain stable identity and power.

## 14.5 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Audited 2026-09-21.*

| **§** | **Design item**           | **Hook that exists**                                                                                         | **Gap**                                                                                                                                                                      | **Size** |
|-------|---------------------------|--------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 14.2  | Recoverable death penalty | `exp.conf`: `death_penalty_type: 1`, 1% base / 1% job. `npc/custom/korangar_death_recovery.txt` measures actual base/job EXP before and after `OnPCDieEvent`, accumulates per-character pending loss, and refunds 50% after 10 same-map monster kills or when login/map-load places the player within 4 cells of the save point. The ledger is cleared before the grant to prevent duplicate refunds. This recovery is source-toggleable and not yet live-server accepted; the engine has no walk-into-radius trigger here, so ordinary movement into save-point range does not currently activate it. |
| 14.3  | Difficulty communication  | Map panel reports the exported static-spawn-record-weighted mean level. On map load, a persistent Game Settings toggle controls a non-blocking warning toast when that mean is at least 15 levels above the character; matching atlas nodes have a contrasting outline and `!`. Missing levels do not guess and travel is never blocked. | Verify warning timing, contrast, marker readability, and accessibility live. Warning is based on static data, not dynamic spawns. | S |
| 14.4  | No hidden scaling         | Done by omission.                                                                                            | Keep it that way; DM Session encounters scale by the DM's hand only.                                                                                                        | —        |

# 15. Accessibility and Player Options

## 15.1 Accessibility Goals

- Remappable controls.

- UI and text scaling.

- Colorblind-safe alternatives for tactical indicators.

- High-contrast option for critical effects and target selection.

- Reduced flashing and reduced screen shake.

- Adjustable combat text size and frequency.

- Alternative ground-target confirmation styles.

- Effect density controls that preserve critical mechanics.

## 15.2 Effect Density

| **Source**                 | **Suggested Player Control**           |
|----------------------------|----------------------------------------|
| Self                       | 0-100% visual density; default 100%    |
| Party                      | 0-100%; default moderate-high          |
| Other players              | 0-100%; default moderate               |
| Critical encounter effects | Always visible option; default enabled |

## 15.3 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Audited 2026-09-21.*

| **§** | **Design item**                      | **Hook that exists**                                                                                                                             | **Gap**                                                                                                                                                              | **Size** |
|-------|--------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 15.1  | Remappable controls                  | See §10.6 (versioned RON binding table and in-Game Settings capture/remap list).                                                                 | Add debug-camera chords; localize labels and verify alternate keyboard layouts.                                                                                       | M        |
| 15.1  | UI / text scaling                    | Interface scaling is shipped.                                                                                                                    | —                                                                                                                                                                    | —        |
| 15.1  | Colourblind-safe / high-contrast     | Built-in High Contrast and Deuteranopia palettes are available for menu/in-game UI; Deuteranopia also changes world status bars (self, allies, monsters, cast), selected-target brackets, target-frame text, cursor/walk indicator, local aim/in-range cues, server cast telegraphs, and party-ping colors. Friend/party status uses color-vision-friendly blue/neutral/vermillion tokens plus explicit status words. | Route remaining critical-effect colors through world-theme cues; complete visual acceptance. | S-M |
| 15.1  | Reduced flashing / screen shake      | Persistent `reduce_motion` suppresses the nonessential Magnum Break camera shake; persistent `reduce_flashing` dims procedural bursts, associated point lights, `.str` skill lights, and classic ACT sprite-effect layers to 55%. Timing and effect lifetime remain unchanged. Older settings default both controls off. | Extend coverage to particle recipes; verify contrast/readability and visual acceptance. | S-M |
| 15.1  | Combat text size and frequency       | Persistent Game Settings controls hide combat text, filter routine damage while retaining criticals/misses, select a status-only mode with no floating numbers, scale floating text small/normal/large, and show one `amount x hit-count` label for a multi-hit damage packet without summing server values. Identical per-hit labels for the same source, target, skill, and critical state merge within an 80 ms presentation window; differing values remain separate. Textual status notices remain visible in status-only mode. | Review varied rapid-packet patterns and complete visual/readability acceptance. | S-M |
| 15.1  | Alternative ground-target confirm    | Persistent Game Settings can choose global quickcast-at-cursor or hold-to-aim-release behavior, and each learned ground/trap skill can override it with Aim + click, Quickcast, Hold + release, or inherit-global modes. Invalid releases cancel. | Verify focus/key-release edge cases and complete usability acceptance. | M |
| 15.2  | Effect density by source             | Every effect attaches to an entity whose relation (self / party / other) is known.                                                               | Three sliders; below a threshold the recipe plays a reduced variant or nothing; "critical encounter" recipes flagged always-on.                                      | M        |

# 16. Audio, Visual Feedback, and Presentation

## 16.1 Preserve Visual Identity

New effects should be readable at sprite scale and should not overwhelm the original artwork. Use animation poses, compact particles, tile accents, shadows, dust, cracks, status icons, and sound cues before resorting to large translucent floor geometry.

## 16.2 Audio Cues

Audio can communicate state without adding screen clutter. Distinct cues are useful for a card drop, a dangerous boss wind-up, a successful interrupt, an important status effect, a party ping, or a completed objective. Repetitive farming sounds should remain pleasant enough for long sessions.

## 16.3 Combat Text

Damage numbers should emphasize criticals, weaknesses, immunities, and important outcomes without turning every hit into a paragraph. Players may reduce or disable floating combat text and rely on the combat log instead.

## 16.4 Out-of-Combat Calm

When the player is exploring a town or field without combat pressure, combat-only HUD elements should be able to fade. Ragnarok's world art should remain the visual focus rather than permanent instrumentation.

## 16.5 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Audited 2026-09-21.*

| **§** | **Design item**        | **Hook that exists**                                                                                                   | **Gap**                                                                                                                                                   | **Size** |
|-------|------------------------|------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 16.1  | Visual identity        | Done in spirit: the classic-effect fidelity programme, sprite-scale footprints, status tints.                          | Keep the restraint rule from §5.13 for telegraphs (cast-time only, layout-sized).                                                                         | —        |
| 16.2  | Audio cues             | `AudioEngine::play_sound_effect` with GRF `wav` assets; server-driven `PlaySoundEffect` events already route through it. | Client-triggered cues: card drop, party ping, quest complete, dangerous cast start (from §5.13 telegraph), successful interrupt. Pick existing GRF sounds first. | S-M      |
| 16.3  | Combat text controls   | See §15.3.                                                                                                             |                                                                                                                                                           | S        |
| 16.4  | Out-of-combat fading   | HUD edit mode (§10.17) once windows have a "combat-only" flag.                                                          | An `in_combat` state (last damage dealt/taken within N s) that fades flagged windows to a set opacity.                                                    | S after 10.2 |
| —     | Toasts                 | See §10.17.                                                                                                            | Shared widget.                                                                                                                                            | S-M      |

# 17. Configuration and Server Administration

## 17.1 Server-Level Configuration

| **Setting**             | **Recommended Baseline**                | **Notes**                                                                    |
|-------------------------|-----------------------------------------|------------------------------------------------------------------------------|
| Knowledge Mode          | Open Database                           | Verified gameplay reference is searchable from the start; account discovery adds history/badges, not mechanical-information gates. |
| Quest Guidance          | Available, opt-in per tracked quest     | Players can disable globally.                                                |
| Exact Spawn Coordinates | Off                                     | Use population regions instead.                                              |
| Auto-Loot               | Configurable categories                 | Respect inventory and weight. *v0.2: `@autoloot`/`@alootid`/`@autoloottype` granted to players (C6); category filters are client work.* |
| Skill respec            | Free at every level                     | Existing player commands; document their use.                               |
| Stat respec             | Free at every level                     | Group 0 has `@streset`; Character Overview has a distinct free reset action. Live verification remains. |
| Party Level Range       | Wider than classic or diminishing model | Tune for friend accessibility.                                               |
| Death Penalty           | Modest and partly recoverable           | Tune through playtests.                                                      |
| PvP / WoE               | Disabled / not part of this design      | Deferred; both stock WoE includes are commented out in `npc/re/scripts_main.conf`. Startup parse and non-WoE guild-script check remain. |

## 17.2 Player-Level Configuration

- HUD layout and scale.

- Classic or modern input preferences.

- Quest markers and breadcrumb intensity.

- Map layers and monster overlays.

- Combat text and damage breakdown depth.

- Ground-targeting style.

- Effect density and accessibility options.

- Auto-loot and loot notification filters.

## 17.3 GM / Designer Tools

The server should include internal tooling that exposes spawn density, monster behavior state, pathing failures, quest state, drop simulation, party-level distribution, and encounter telemetry. Better tools shorten balance iterations and reduce the temptation to solve every issue through raw stat changes.

# 18. Balance and Playtesting Methodology

## 18.1 What to Measure

| **Area**   | **Useful Measures**                                                                          |
|------------|----------------------------------------------------------------------------------------------|
| Combat     | Time-to-kill, damage taken, skill usage, movement frequency, interrupts, deaths, potion use. |
| Navigation | Time spent searching for exits/NPCs, wrong-map transitions, map opens, Navigate usage.       |
| Questing   | Abandon rates, objective confusion, tracker usage, story completion pace.                    |
| Economy    | Item availability, Zeny generation/sinks, trade frequency, crafting material bottlenecks.    |
| Party Play | Level gaps, party duration, shared destination use, XP efficiency, revives/deaths.           |
| UI         | Most-opened panels, tooltip depth usage, common misclicks, scaling preferences.              |

## 18.2 Balance Philosophy

Do not balance only around maximum-efficiency veteran play. This is a friends server, so the primary question is whether systems create interesting decisions and enjoyable cooperation. Some builds and maps may be more efficient than others. Problems occur when one path invalidates almost all alternatives or when a class loses meaningful identity.

## 18.3 Tactical Combat Playtest Questions

- Can players understand why they were hit by a dangerous ability?

- Can players respond through movement or class tools without needing twitch-perfect timing?

- Do normal farming fights remain quick enough?

- Does smarter AI create decisions or merely annoyance?

- Do support and control abilities feel more valuable without becoming mandatory?

- Can players read the battlefield when several characters and monsters are active?

## 18.4 Navigation Playtest Questions

- Can a new player reach a named dungeon without an external website?

- Does breadcrumb guidance feel helpful without making travel automatic?

- Are population regions useful without feeling like exact spawn spoilers?

- Can veteran players disable enough guidance to preserve the classic experience?

# 19. Implementation Roadmap

*Status marks added in v0.2 from the Appendix E audit: **[done]**, **[unseen]** (built, never observed live), **[partial]**, **[open]**, **[descoped]**. Phases were written as sequential; in practice Phase 0, most of Phase 1's plumbing, and pieces of Phases 2 and 4 landed first because a DM-run campaign needed them.*

## Phase 0 - Technical Feasibility and Instrumentation — **done**

- **[done]** Confirm client/server hooks available for UI changes, targeting previews, navigation data, AI behaviors, and input buffering. *Own client fork; three custom packets shipped and versioned on both ends; skill layouts read from `skill_db`.*

- **[done]** Build developer logging for combat actions, pathing, quest state, and map transitions. *`KORANGAR_PACKET_LOG`, packet inspector, `[DMJ]` structured echo, diagnostics collector.*

- **[done]** Define configuration format so experimental values do not require code changes. *Hercules `conf/import/`, Korangar RON settings, DM data as JSON.*

## Phase 1 - Foundational UX — **partial** — *paths in §8.7, §9.10, §10.17*

- **[partial]** World map and map connection data. *The generated graph indexes loaded static warp scripts plus two reviewed Izlude/Byalan Sailor service edges; the regional atlas shows 27 selectable towns and destinations, with reachable connections, account-persistent visited markers, and selected route highlighted. The selected-destination panel shows discovery sync state, outgoing connections, verified route hop count, and the next portal cell or service action/requirement. The current map draws a walkable breadcrumb to the next edge location and recomputes after a map change. The real 150-zenny Izlude ferry transfer to `izlu2dun` passes headless live acceptance; the `izlu2dun` → `iz_dun00` walk warp, full dynamic/conditional travel coverage, client route following, and visual acceptance remain open.*

- **[partial]** Quest tracking and breadcrumb routing. *Track/Untrack persists per character and reconciles with the server. Hercules hunting notifications/progress populate kill objectives; item turn-ins link into Guide item records and known drop-source map routes. Valid short/full `<NAVI>` links create labeled route buttons. NPC/story objectives without explicit location data and live traversal acceptance remain open.*

- **[partial]** Player Adventure Guide. *A separate menu-launched window searches monster/item/card/skill data, graph maps, 151 job names, and active quests. Versioned exports supply inherited Hercules skill trees for 128 job IDs and job-level stat bonus schedules for 147 job IDs; validated skill links show max levels, minimum job levels, and listed prerequisites, while job entries show cumulative STR/AGI/VIT/INT/DEX/LUK gains with their job-level milestones. Item↔monster drop and quest↔item/monster links open related records, and supported map/monster/quest/item details offer routes. Conditional-script effects and the other job IDs without skill-tree/stat rows remain labeled incomplete. Account-synced monster and map-discovery badges are wired; core first-kill/first-visit reconnect and cross-account isolation pass live. Production migration/startup, additional milestones, richer build planning, and visual acceptance remain open.*

- **[partial]** Inventory/storage management. *Inventory and storage share name and server-item-type filters; inventory also has All/Equipped/Gear/Items tabs, sorting, drag arrangement, server-persisted slot order, and per-character do-not-drop/do-not-sell locks. A dedicated server-authoritative split-to-stack action exists. Targeted headless live tests cover split validation/capacity, partial storage transfers, ordering, and real ammo/card item types. Quest-item/favorite/recent-loot categories are deferred pending reliable metadata/history contracts; GUI visual and broader acceptance remain open.*

- **[partial]** Improved target/player frames and UI scaling. *UI scaling and player interaction frame exist; monster selection, HP frame, and available level/element/race/size chips are implemented in code. Status/cast details and live visual acceptance remain open.*

- **[partial]** Party member map location and shared waypoint support. *Party minimap blips with hover names, roster, world HP bars, and `@partyjump` exist. The minimap can send Location, Assist, Danger, Retreat, Ready, and On my way pings through bounded v2 party chat; same-map clients show a replaceable 15-second labeled minimap and in-world marker, and legacy v1 location pings still decode. A route can be proposed in party chat, accepted locally, and acknowledged to clear the proposal. Ready checks snapshot online members and support one response each before a 30-second expiry. Server source contains byte/rate limits for ping and session messages; client source and modified Hercules C have local build coverage. Live mixed-client/server acceptance remains open.*

## Phase 2 - Combat Responsiveness — **partial** — *slices and order in §5.13*

- **[partial]** Single-action input buffering. *A separate one-slot 200 ms animation-lock buffer now handles attack, pickup, entity/ground casts, including self/support hotbar casts; walk-into-range and autoattack state remain separate. Expiry boundaries are unit-tested; live timing/readability and rapid-input behavior remain open.*

- **[partial]** Target switching and selection improvements. *Tab/Shift+Tab monster cycling, click selection, target frame, and selected-monster health bar are implemented in code; live visual acceptance remains open.*

- **[partial — unseen]** Ground-target previews and cast telegraphs. *Real Hercules cell layouts and out-of-range tint exist; cast targets now flow through packets and invariant footprints render during eligible casts. Live visual acceptance remains open.*

- **[done]** Improved cast/action indicators. *Overhead cast bars; fork-only cast cancel; skill-fail reasons.*

- **[done]** Animation and hit-feedback synchronization. *Animation engine phases A-D closed.*

## Phase 3 - Tactical Monster Layer — **partial source pilot** — *data-first path and pilot maps in §6.10*

- **[partial]** Add map+monster scoped profile loading and two starter roles; expand and balance the reusable archetype library.

- **[open]** Pilot on one early, one midgame, and one late-game region.

- **[open]** Add restrained dangerous-attack telegraphs.

- **[open]** Test reactions to ground control and party roles.

- **[open]** Iterate before broad rollout to all monsters.

*Nearest existing asset: the DM hazard/encounter scripts in `Hercules/npc/custom/dm_campaign/shared/` prove the server-side primitives (`DM_HazardArea`, mode-bit stripping, `setcell`) that an AI layer would also use.*

## Phase 4 - Journals and Knowledge Systems — **partial (DM-shaped tools) — scope widened by C1 to the §9.5 encyclopedia** — *paths in §7.7, §9.10*

- **[partial]** Monster Journal with traits and drops. *Versioned bestiary/item/card data, typed client reference API, and a player Guide are implemented and cross-link validated. First-kill account persistence and Guide badges are wired; broader spawn-map data and live account-isolation acceptance remain.*

- **[partial]** Item-to-monster-to-map linking. *Item-to-monster drop links and monster-to-static-map records are exported; item turn-in requirements now open their Guide item entry and expose routes to graph-known maps of verified drop-source monsters. Conditional/scripted spawns, exact NPC/story destinations, and live route acceptance remain open.*

- **[open]** Population overlays.

- **[open]** Rumors and discovery states.

- **[open]** Build planner and advanced combat explanations.

## Phase 5 - Economy and Profession Improvements — **not started / partly descoped**

- **[descoped]** Market/vending search. *Out of scope for this server (C2). Direct trade polish takes its slot.*

- **[open]** Crafting request / commission flow. *DM Loot Generator and `@dmreward` fill this role today.*

- **[partial]** Refinement clarity improvements. *Refine window exists; chance/consequence display not audited.*

- **[open]** Small-population drop-rate and Zeny balancing. *Base/drop/quest rates are stock; the effective import config already gives a 25% even-share party EXP bonus per additional member.*

## Phase 6 - Boss and Story Encounter Pass — **partial (DM Session mode only)**

- **[open]** Upgrade selected MVPs with signature mechanics. *Open-world MVPs are stock; DnD mode suppresses them during sessions.*

- **[partial]** Build story bosses using the tactical rules already taught by normal monsters. *Campaign bosses have adds, beat variants and pulse hazards, but no normal-monster layer teaches those rules first.*

- **[open]** Add optional post-encounter analysis for major fights. *Spec'd as "End-of-Encounter Recap" in the client roadmap.*

## Phase 7 - Polish and Expansion — **partial** — *paths in §15.3, §16.5*

- **[partial]** Accessibility pass. *Persistent Reduce motion suppresses the Magnum Break camera shake; persistent Reduce flashing dims procedural combat bursts and associated point lights without shortening timing or removing sounds. Selected monsters now retain a broken-corner, theme-colored bracket around their overhead health indicator; the Deuteranopia palette uses a distinct yellow accent. Ground/trap skills support global quickcast/hold defaults and per-skill Aim + click, Quickcast, Hold + release, or inherit-global overrides. Show combat text supports off/all/important-only filtering and small/normal/large text; built-in High Contrast and Deuteranopia UI palettes are selectable, and Deuteranopia recolors world self/ally/monster health and cast bars, cursor/walk cue, local aim and incoming cast footprints, and party pings. Friend/party presence and dead-state tags now use blue/neutral/vermillion status colors plus explicit words. Authored effect coverage, remaining effect colors, source-based density, and visual acceptance remain open.*

- **[open]** Effect density and performance tuning.

- **[open]** UI profile polish.

- **[open]** Expanded monster behavior library.

- **[open]** Content pacing and economy rebalance after real player data.

## v0.2 Build Order — the next slices, sequenced (added 2026-09-21)

*Derived from the §5.13-§16.5 implementation paths. Client and server tracks can run in parallel. The [end-to-end improvement plan](plans/gdd-improvement-plan.md) covers all in-scope GDD work; its [Wave 1 next-slices plan](plans/gdd-next-slices.md) records these rows' dependencies, acceptance, and open decisions. Use both before starting a row. Larger rows may need more than one PR.*

| **#** | **Slice**                                                        | **Size** | **Unblocks**                                                        | **Path** |
|-------|------------------------------------------------------------------|----------|---------------------------------------------------------------------|----------|
| 1     | Toast widget                                                     | S-M      | Rare drops, quest complete, level-up, pings, difficulty warnings    | §10.17   |
| 2     | Monster cast telegraphs (carry position through `SkillCast`)     | S        | Everything in §6 feeling fair                                       | §5.13    |
| 3     | Target cycling (Tab) + monster target frame/chips                 | S-M+M    | §5.9 contextual layer, §10.10 vs-target                             | §5.13    |
| 4     | Visible character Delete + typed-name confirm; visible item Drop  | S+S      | Two playtest reports; existing two-step delete stays                | §10.17   |
| 5     | **Map graph generator** from `npc/re/warps/**`                   | M        | Breadcrumbs, portal labels, world map, shared destinations, map panel| §8.7     |
| 6     | `<NAVI>` links + next-exit breadcrumb on the minimap             | S+M      | §8.4, §20.1 "no external guide"                                     | §8.7     |
| 7     | Inventory search / category tabs / sort / lock list              | M        | §10.9; storage reuses it                                            | §10.17   |
| 8     | **Encyclopedia generator** (skills, maps, quests, NPCs, refine)  | M        | §9.5 categories, §7.4 skill tooltips, §11.6 refine odds, §14.3      | §9.5     |
| 9     | Player Adventure Guide window with search (promote DM Bestiary)  | M        | C1 first shippable slice                                            | §9.5     |
| 10    | Single-action input buffer with expiry                           | S-M      | §5.2                                                                | §5.13    |
| 11    | Party transport Phase A + pings + ready check                    | M        | §13.3, §13.2, §13.4; parser may also carry server-originated §9.6 sync | §13.7 |
| 12    | Quest tracker HUD + hunting goals                                | M        | §10.11, §8.2                                                        | §8.7     |
| 13    | Keybinding table + remap screen                                  | M        | §10.6, §15.1                                                        | §10.17   |
| 14    | HUD edit mode: lock, snap, named layouts, combat fade            | M        | §10.2, §16.4                                                        | §10.17   |
| 15    | Accessibility: themes, reduced motion, combat-text controls, density | M    | §15                                                                 | §15.3    |

**Server track (parallel; S8/S9 include C changes):**

| **#** | **Slice**                                                  | **Size** | **Path** |
|-------|------------------------------------------------------------|----------|----------|
| S1    | Remove the stock WoE includes from `scripts_main.conf`     | S        | §17.1    |
| S2    | Repair free player skill reset; enable free player stat reset | S        | §7.7     |
| S3    | Verify effective 25% bonus; test wider inter-server share range | S | §13.7 |
| S4    | Death-recovery script (`OnPCDieEvent` refund)              | S-M      | §14.5    |
| S5    | Map-scoped Fabre Aggressor / Poring Coward source pilot on `prt_fild08` | S | §6.10 |
| S6    | Pilot on `orcsdun01`, then Glast Heim                      | S+S      | §6.10    |
| S7    | Bestiary unlock persistence (account vars + login sync)    | M        | §9.10    |
| S8    | Verify existing nearby-party stock-quest credit; tune only if needed | verify | §8.7 |
| S9    | Skirmisher / Ranged Keeper / hazard-aware pathing (C)      | L        | §6.10    |
| S10   | DM Session party quest sync: per-character event replay    | M        | §8.7     |

Slices 1-4 are client work with no server feature dependency. Slice 5 and slice 8 are the two generators everything in navigation and knowledge hangs off; validate their server revision and coverage before the windows consume them.

## Phase X - Delivered but unplanned

*Added in v0.2. This is where most of the elapsed effort went; see Appendix E "Built Outside the GDD".*

- **[done]** Custom client bring-up, distribution packs, updater, security hardening, Tailscale play.

- **[done]** DM campaign layer (Seal Cascade, `@dm` console, skill checks, instances, hazards).

- **[done]** Headless regression suite and sprite/animation audits.

- **[done]** Playtest 1 fixes: WASD, three hotbar rows with items, party warp, minimap names, quest window, headgear, AMD white-screen.

# 20. Success Criteria and Non-Goals

## 20.1 Success Criteria

- A new player can follow the story and reach major destinations without an external guide.

- A veteran can disable most guidance and retain an open-ended Ragnarok experience.

- Combat feels more responsive without becoming full action combat.

- Monster behavior creates tactical decisions without making routine farming exhausting.

- Players can explain why important combat outcomes occurred.

- Autoattack and unusual builds remain legitimate.

- Classes feel distinct and useful in groups.

- Friends with imperfect level alignment can still play together productively.

- Rare drops and cards remain meaningful goals.

- The UI feels modern in operation but visually compatible with Ragnarok.

## 20.2 Non-Goals

- Perfect balance between every build in every map.

- Eliminating grinding from Ragnarok.

- Removing all reasons to consult community knowledge.

- Making every monster mechanically complex.

- Making every class equally self-sufficient.

- Replacing travel with unrestricted teleports. *(v0.2: `@partyjump` is the one accepted exception, party-scoped — decision C4.)*

- Designing PvP or War of Emperium in this phase.

# Appendix A. Example Player Journey

A level 43 Hunter logs in and sees two friends already online. One is following the main story near Payon; the other wants a Smokie Card. The Hunter opens the party panel and joins them.

1. The group creates a shared goal: Smokie Card. The Adventure Guide identifies Smokie and displays known maps with relative population levels.

2. They select a suitable field and choose Navigate. Each player receives the same breadcrumb route, but they travel normally through the world.

3. On arrival, the minimap displays a broad high-population region because the group has enabled monster overlays.

4. The Hunter places traps near a narrow approach. The Priest chooses a safe tile behind the frontline. A melee friend pulls several monsters through the area.

5. A ranged monster attempts to maintain distance rather than standing still. The group changes position instead of merely repeating attacks.

6. A dangerous monster begins a clearly animated heavy attack. One player moves, another uses a control skill, and the cast is interrupted.

7. After several pulls, the main-story player notices a nearby rumor marker connected to their chapter. The group decides to change plans and investigate it together.

8. The story tracker updates the destination. No one needed to return to a quest hub or finish a daily checklist before changing goals.

This journey demonstrates the desired interaction between self-directed goals, tactical combat, navigation, party tools, and the central story. The game assists every transition without deciding what the group must do next.

# Appendix B. Example Tactical Encounter - Glast Heim Patrol

## Encounter Setup

A party enters a Glast Heim corridor containing two Raydrics, a ranged enemy, and a support-type enemy. The room is narrow with a side alcove. The enemies use reusable behavior archetypes rather than a bespoke scripted sequence.

## Behavior

| **Enemy**     | **Behavior**                                                                                             |
|---------------|----------------------------------------------------------------------------------------------------------|
| Raydric A     | Aggressor. Presses nearest frontline target and occasionally uses a clearly animated heavy sword attack. |
| Raydric B     | Protector. Prefers threats attacking the support enemy and may reposition to block access.               |
| Ranged Enemy  | Maintains preferred distance and changes lane if a Firewall blocks the direct route.                     |
| Support Enemy | Uses a short buff or debuff and attempts to remain behind allies.                                        |

## Possible Player Responses

- The Knight uses Provoke to keep one Raydric from reaching the Wizard.

- The Wizard places Firewall to reshape the ranged enemy's route.

- The Hunter traps the alternate lane created by the Firewall.

- The Priest positions Safety Wall where the frontline can retreat into it.

- The party chooses to burst the support enemy during a short opening rather than simply attacking the nearest target.

- If the heavy attack begins, the target can move, interrupt it with a suitable tool, or accept the hit using defensive preparation.

No universal dodge roll, flanking meter, or prescribed combo is required. The tactical gameplay emerges from movement, class tools, AI, space, and existing Ragnarok mechanics.

# Appendix C. Initial Configuration Matrix

*v0.2 adds a **Current** column — what the forks do today (status refreshed 2026-09-24 from source and local tests) — beside the design baseline, so the gap is visible per row.*

| **Feature**                | **Baseline**                                      | **Type**              | **Current (2026-09-24)**                                                              |
|----------------------------|---------------------------------------------------|-----------------------|---------------------------------------------------------------------------------------|
| PvP / WoE                  | Disabled / deferred                               | Locked for this scope | Both stock WoE includes are commented out; startup parse and non-WoE guild-script check remain. |
| DM Session mode            | Opt-in, DM-activated, one party                   | Decided (C5)          | Implemented: `@dm mode on` / `@dm start`, `DM_SessionAllows` gate at 50 sites.       |
| Main Story Guidance        | Available when tracked                            | Player configurable   | Quest log Track/Untrack/HUD, hunt routes, and item-turn-in links to verified drop-source map routes exist. NPC/story destinations without explicit location data remain open. |
| World Breadcrumbs          | Next-exit guidance                                | Player configurable   | Generated graph, atlas, sampled walkable next-exit route, actionable validated `<NAVI>` links, hunt routes, and item turn-in→Guide→drop-source-map routes exist; NPC/story locations without explicit data and live traversal remain open. |
| Monster Population Overlay | Broad regions only                                | Player configurable   | Partial: static map-level spawn records are exported and Guide maps can route; density overlay and conditional/scripted spawn data remain unbuilt. |
| Knowledge Mode             | Open Database: verified reference data searchable | Decided for friends server | Player Guide offers All and category-filtered monster/item/card/skill/status/map/job/quest searches, with tracked quest names/targets plus active entries and server-synced account monster/visited-map badges; job-level stat schedules are exported for 147 job IDs. Conditional-script effects, complete giver/story quest coverage, and live account-isolation checks remain open. DM Bestiary unlocks remain session-only. |
| Story completion           | Each character completes the story, including DM Session | Decided | Party quest/flag transitions now have typed SQL event rows and per-character replay cursors; snapshot reconciliation remains as a legacy/backfill path. Live reconnect, same-account alternate isolation, and event/run migration remain S10 acceptance work. |
| In-game Encyclopedia       | Full wiki coverage, generated from server tables  | Locked principle (C1) | Monster/item/card/skill exports, 128 inherited job-skill trees with verified skill/prerequisite links, searchable maps/job names/active quests, account monster/visited-map badges, and static spawn-map routes exist. Job-stat bonuses, richer build planning/cross-links, conditional spawns, and live acceptance remain open. |
| Input Buffer               | ~200 ms starting test                             | Playtest value        | Separate one-slot animation-lock buffer added; live timing/readability acceptance remains open. |
| Action Queue               | 1 action                                          | Recommended default   | Both movement chaining and timed animation-lock buffers are single-slot; continuous autoattack remains separate. |
| Universal Dodge            | None                                              | Locked principle      | None. Plan section struck (C3).                                                       |
| Auto-Loot                  | Category filters                                  | Player configurable   | `@autoloot` (drop-rate threshold), `@alootid`, `@autoloottype` — group 0 (C6). No category filters. |
| Party Level Rules          | Friend-friendly widened range / diminishing model | Playtest decision     | Effective import config sets 25% even-share bonus per additional member; inter-server share limit remains 15 levels. |
| Death Penalty              | Modest + partial recovery                         | Playtest decision     | Stock: 1% base / 1% job, no recovery.                                                 |
| Skill respec               | Free, ordinary-player accessible at every level  | Decided               | Group 0 grants registered `@skreset`/`@refundskill`; Character Overview has a distinct free skill-reset action. Focused test confirms reset, zero Zeny change, and relog persistence on a GM fixture; group-0 reachability and prerequisite rejection remain unverified. |
| Stat respec                | Free, ordinary-player accessible at every level  | Decided               | Group 0 grants registered `@streset`; Character Overview has a distinct free stats-only reset action. Focused test confirms the allocation is restored, the skill pool is unchanged, and Zeny/relog state are correct on a GM fixture; group-0 reachability remains unverified. |
| Fast Travel                | In-world systems; no unrestricted map teleport    | Locked principle      | Kafra + warper NPC; `@partyjump` to any online party member, unbounded (C4).          |
| Exact Spawn Coordinates    | Hidden                                            | Recommended default   | Hidden (nothing shows them).                                                          |
| HUD Presets                | Classic + Modern + custom                         | Recommended default   | None. Windows movable/resizable and persisted; no presets or edit mode.               |
| Drop / EXP Rates           | Tuned for small population (§12.2)                | Playtest decision     | Stock 100% / 100% / card 100%.                                                        |
| Keyboard Movement          | Optional, same pathing rules                      | Recommended default   | WASD on by default, click-to-move retained, 200 ms throttle.                          |
| UI Scale                   | Independent of pixel-art scaling                  | Player configurable   | Interface Settings > Scaling (Ctrl+I).                                                |

# Appendix D. Open Design Questions

1. Which exact server/client codebase and tooling constrain the planned UI and AI changes?

2. How far should keyboard movement go while preserving tile/pathing parity with click-to-move?

3. ~~Should monster journal data be character-specific, account-wide, or server-wide?~~ **Answered 2026-09-22: account-wide.**

4. ~~What percentage of item/drop information should be available immediately in Hybrid knowledge mode?~~ **Answered 2026-09-22: all verified build-relevant reference information is available immediately; no Hybrid gate.**

5. Should friends have an optional level-sync system, or is a widened party XP model sufficient?

6. How destructive should high-level refinement remain on a small server?

7. Which three maps should be used as the first tactical-AI pilot areas? *v0.2 recommendation in §6.10: `prt_fild08`, `orcsdun01`, Glast Heim. Not yet decided.*

8. Which MVP should be the first redesigned boss used to prove the tactical-combat framework? *v0.2 recommendation in §6.10: Eddga. Not yet decided.*

9. Should party pings and shared destinations persist across map transitions or expire quickly?

10. How much advanced formula information can the client display accurately from the server implementation?

11. Which original UI windows can be extended safely versus requiring replacement or overlay panels?

12. What level of map/monster data can be generated automatically from server tables versus authored manually? *2026-09-24: the v1 pipeline exports loaded static spawn-map regions (not exact cells or dynamic/conditional spawns); reviewed supplements remain necessary for script-only facts and rumors.*

*Answered or narrowed since v0.1:*

- Q1 — **Answered.** Korangar (Rust, wgpu) client fork and Hercules server fork at `PACKETVER=20220406`. UI is fully owned; custom packets are cheap but must be versioned on both ends.

- Q2 — **Answered.** WASD ships as an optional mode over the same pathfinder with a 200 ms throttle; click-to-move retained. No further keyboard-movement scope is planned.

- Q3–Q4 — **Policy answered; implementation partial.** The player Guide is Open Database by default, with account-wide discovery records independent of reference-field visibility. Monster first-kill and map-visit persistence/badges are wired; live same-account reconnect and cross-account isolation pass. Production migration/startup and richer discovery milestones remain. DM Bestiary unlocks stay separate and session-only.

*New questions raised by the audit:*

13. ~~Is the Seal Cascade DM campaign the §8 story spine, or a separate "DM session" mode?~~ **Answered 2026-09-21: a separate DM Session mode, activated by the DM.** See §2.3 and Appendix E, C5.

14. ~~Should the Adventure Guide rejection (2026-07-05) stand?~~ **Answered 2026-09-21: no — the Adventure Guide becomes a full in-game encyclopedia (§9.5).** See Appendix E, C1.

15. Which of the fork's protocol additions (cast cancel, skill-fail reasons, party invite sender, party SP) should be promoted into the GDD as designed features rather than incidental fixes?

16. ~~What bounds `@partyjump`?~~ **Answered 2026-09-21: nothing, for now.** Accepted as a private-server convenience; revisit later. See Appendix E, C4.

17. Which three playtest-1 gaps get priority for playtest 2: inventory search, a HUD quest tracker, or party pings on the existing `[DMJ]` transport?

HUD edit progress since that refresh: `WindowCache` migrates the legacy geometry cache and persists lock state and per-character named layouts. Game Settings exposes lock/unlock, Classic/Modern presets, a `My Layout` custom slot, reset, and a persisted screen-grid snap cycle (off/8/16/32px). Tests cover grid rounding and cache migration; combat-only fading and visual acceptance remain open.

## Recommended Next Design Deliverables

- Tactical Combat Specification: input states, targeting rules, action buffer, movement, cast states, telegraph taxonomy, AI interfaces. *Next-slice contract: [combat input](specs/gdd-combat-input.md); AI extension: [monster pilot](specs/monster-ai-pilot.md).*

- UI Wireframe Pack: [next-slice HUD/window states](specs/gdd-ui-wireframes.md). World map and build planner need later follow-on wireframes when their slices are scheduled.

- Navigation Data Specification: [map graph and next-exit contract](specs/navigation-quest-guiding.md). Population regions and discovery flags are follow-on slices.

- Monster AI Behavior Library: [profile and C-side pilot contract](specs/monster-ai-pilot.md).

- Pilot Region Design: [`prt_fild08` first-tier pilot](specs/monster-ai-pilot.md), connected to navigation/Guide as those data slices land.

- Encyclopedia Data Generator Specification: [category/source and knowledge-policy contract](specs/encyclopedia-data.md).

- Implementation Status Refresh: [next-slices plan](plans/gdd-next-slices.md) records current-code evidence and live gates; update Appendix E after each playtest.

# Appendix E. Implementation Status (refreshed 2026-09-24)

This section was added in v0.2 after auditing the two forks that implement this design — the **Korangar** Rust client (`korangar/`) and the **Hercules** server (`Hercules/`) — against the code, the commit history, and the first friends playtest (2026-09-05, four players, three remote). Status reflects what exists in the tree, not what the client's own roadmap checkboxes say (those were found to be stale).

*2026-09-22 source corrections:* the active free skill-reset commands supersede the inactive priced NPC; effective import config already sets a 25% party even-share bonus; stock kill objectives already credit nearby party members; current skill-point staging and detailed skill tooltips exist; `player_target.rs` is player-only; and the DM-styled Bestiary/Reveal all controls are reachable through the unguarded Commands window. These corrections are folded into the rows below and the [next-slices plan](plans/gdd-next-slices.md). Live acceptance remains separate from source inspection.

*2026-09-24 implementation/status refresh:* client code includes target cycling and a reactive monster HP/reference-stat frame, eligible cast telegraphs, graph/atlas routing, validated `<NAVI>` actions, spawn/drop-verified quest routes, an open searchable Adventure Guide, account discovery badges, a replaceable 200 ms action buffer, six party ping kinds, shared-route proposals, nonce-bound ready checks, persistent accessibility/combat-text settings, and configurable primary/hotbar/movement bindings. Guide references now include versioned monster/item/card data, skill details, inherited skill trees for 128 jobs, stat schedules for 147 jobs, searchable status-icon names, and cross-links; unknown or conditional mechanics remain explicitly labeled. Hercules source now includes account+mob/map discovery ledgers and snapshots, bounded party ping/session carriers, campaign event replay, death recovery, map-scoped Aggressor/Coward/Skirmisher/RangedKeeper profiles, and opt-in local hazard-weighted A* for the Orc Archer. Source builds and targeted script/data checks pass, but they do not prove configured server startup or live behavior. Latest client workspace library run: 375 Korangar tests passed, 17 ignored; all workspace library test groups passed (the other groups reported 7, 63, 37, 6, 39, 6, 47, 11, and 54 passing). `cargo check --workspace` and formatting checks pass. Full configured-server/database acceptance, traversal and quest-routing playtests, mixed-client behavior, party EXP/reset/death-recovery/discovery/replay acceptance, mid/late AI roster completion, hazard/path/load testing and coverage outside the bounded A* window, Guide/status data completeness, remaining accessibility/keybinding gaps, and visual acceptance remain open. Do not count an implemented source hook as accepted gameplay.

*2026-09-24 continuation status refresh:* the live Izlude route now verifies the real 150-zenny Sailor transfer, map-cache-derived walk, and dungeon-entry warp end to end. Fresh-server checks passed for party-message carrier boundaries, shared quest credit, and account discovery; the discovery isolation scenario now also passes after quest-credit in the same run, and checks selected mob/map records rather than incorrectly requiring a previously used account to have an empty snapshot. A new live `quest-reviewed-brasilis-npc-routes` scenario verifies the real Angelo quest 9030 offer and 9031 turn-in/9032 cooldown branch; its turn-in quest state is set synthetically, so puppy-objective completion is not covered. `cargo test --workspace` passes; the Korangar library reports 411 passed and 17 ignored, and 160 headless scenarios are registered. Core account discovery delta/reconnect/isolation and the authored Izlude route are live-verified. Mixed stock-client party behavior, production discovery migration/startup, party EXP tuning, death-recovery cases, DM replay lifecycle, broader route/quest coverage, AI stress/behavior, remaining accessibility coverage, and visual acceptance remain open. Visual checks remain deliberately deferred.

*2026-09-24 Guide quest-reference follow-up:* `quests.v1.json` now contains related static NPC-script references for 842 tracked quests. The Adventure Guide searches those NPC names/maps and offers exact-cell route actions when the referenced map is in the verified navigation graph. These are quest-state calls found inside statically located NPC blocks—not proven quest-giver relationships—and do not recover conditional dialogue, story steps, rewards, or prerequisites. The exporter deliberately uses tracked `db/quest_db.conf` when that file is modified locally; it does not consume the uncommitted local quest-database edit or live SQL. Latest focused client library run: 382 passed, 17 ignored. Exporter unit tests and supported-data drift checks pass. This improves Guide search/routing coverage but does not close live quest traversal or visual acceptance.

*2026-09-24 map information/difficulty follow-up:* the atlas panel and Guide map details now report static-spawn-record-weighted mean level, indexed spawn-record/species counts, verified exits/routes, account visit-sync, and (in the atlas) online party members reporting that map. Guide map details list up to eight outgoing portal cells and route actions. The atlas now includes facility names from the client Towninfo table for any selected map represented there. Map load presents an optional-to-disable warning when the static mean is at least 15 levels above the character; atlas nodes at that threshold get a contrasting outline and `!`. Missing data is silent and travel remains unrestricted. Tests cover map summaries, route details, POI labels, the 15-level boundary, missing data, and opt-out; the shared threshold is unit-tested. Latest Korangar library run: 385 passed, 17 ignored; workspace check and formatting pass. Authored service POIs, conditional spawns, low-coverage context, and live/visual acceptance remain open.

*2026-09-24 keybinding follow-up:* sustained WASD movement and all 27 number-row hotbar slots now use the versioned remapping table while preserving defaults, F1–F9 aliases, and exact modifier matching. Focused input tests cover remapped hotbar dispatch and retained Shift+1–4 party targeting. At this point in the work, `slangc` was unavailable for a debug-feature build; it has since been installed, and the later debug-camera follow-up below verifies the debug-feature check. Keyboard-layout acceptance, live play, and visual checks remain open.

*2026-09-24 accessibility and ping follow-up:* a multi-hit damage packet now produces one `amount x hit-count` floating label, keeping per-hit server values unaggregated while reducing number density. The latest validated same-map ping also appears above its world tile as a labeled cue; it replaces the prior marker and expires after 15 seconds or map-state clear. Hercules discovery script parsing and the modified C source build pass; full configured-script startup/live-server and visual acceptance remain outstanding.

*2026-09-24 party-session follow-up:* selected navigation routes can be shared with party members over bounded `[KORANGAR-SESSION:v1]` chat messages. A matching acceptance routes locally and broadcasts an acknowledgement; destination state survives map changes, clears on party end or acceptance, and never touches persistent discovery/quest data. Ready checks use the same carrier with nonce-bound start/replies, an online-party snapshot, one response per member, and 30-second expiry. Hercules source size/rate limits cover session messages as well as pings. At the later action-buffer refresh the client library suite is 370 passed, 17 ignored; `cargo check -p korangar` passed. The modified Hercules C sources compile and link; live mixed-client/server acceptance remains open.

*2026-09-24 quest-routing follow-up:* item turn-in requirements in the quest log now offer a Guide action to the exact item/card ID, followed by routes only to graph-known maps present in exported drop-source monster spawn records. Unknown items, missing drop data, and unmapped spawn records produce no speculative routes. The Guide also finds exact numeric item IDs. NPC/story objective locations still require explicit source data; this is not live or visual acceptance.

*2026-09-24 timed-buffer follow-up:* when a queued entity target or ground item disappears, the one-slot action buffer is now cleared immediately and a cancellation toast is shown instead of retaining an invalid action until its next processing tick. Escape now clears both the timed animation buffer and the ordinary walk-chain buffer before opening/closing UI, with a cancellation toast, so a backed-out action cannot fire later. Unit tests cover entity, pickup-item, and ground-cast invalidation matching; latest Korangar library suite at that point: 385 passed, 17 ignored. Live input timing acceptance remains open.

*2026-09-24 timed-buffer verification follow-up:* newest-press replacement now has a direct unit test proving it overwrites the queued action and restarts the 200 ms deadline. The deadline boundary/tick-wrap, invalidation, and refusal tests remain green; `cargo test --workspace` passes with 413 Korangar tests and 17 ignored. Live attack→skill/skill→skill timing, ground-skill movement, and visual cue acceptance remain open.

*2026-09-24 Guide job-data follow-up:* `tools/export_job_skills.py` expands inherited renewal skill trees and validates skill/prerequisite IDs; 128 job skill trees are bundled and linked from Guide job entries with max levels, minimum job levels, and prerequisites. The later job-bonus export also validates `job_db2.txt` schedules against constants and bundles 147 job IDs; the Guide shows cumulative stats and exact milestone levels. Conditional skill-script effects and jobs without source rows remain explicitly incomplete. Guide cross-link/reconciliation tests pass and the supported-data exporter reports no drift.

*2026-09-24 route validation follow-up:* Guide monster/hunt routes and quest-log hunt actions now omit static spawn maps absent from the navigation graph, matching the existing item-turn-in route guard. Tests check the returned hunt routes against the graph; latest client library suite: 371 passed, 17 ignored. This verifies route-data filtering, not live traversal.

*2026-09-24 status-only combat-text follow-up:* Game Settings now cycles All → Important → Status only. Status only suppresses floating damage, miss, and heal values while leaving the existing textual status notices available; the selected mode persists in `game_settings.ron`. Mode behavior and cycling are unit-tested. Visual/readability acceptance remains deferred.

*2026-09-24 Guide job-bonus follow-up:* `tools/export_job_bonuses.py` now validates Hercules `job_db2.txt` level schedules against `constants.conf` and bundles all 147 matching job IDs. Guide job records display cumulative stats and exact job-level milestone lists alongside skill trees; conditional skill-script effects and job IDs without source rows remain explicitly unknown.

*2026-09-24 Guide status-reference follow-up:* a versioned export joins 540 of 700 server icon names to `sc_config.conf` status IDs, raw Buff/Debuff and other server flags, recalculation flags, and the associated `skill_db.conf` skill where present. Guide search now matches status constants and associated skill names; status details link to associated skills, and skill details link back to every associated status icon. Exact effects, per-level duration/odds, complete infliction sources, interactions, and cures remain explicitly undocumented. Name-only icons remain labeled; no mechanics are inferred from icon labels.

*2026-09-24 Guide status skill-field follow-up:* the status exporter now also indexes explicit `StatusChange` fields from `skill_db.conf`, validates the resulting skill IDs/names, and adds searchable two-way Guide links. This fills the three `sc_config` statuses without a direct `Skill` field (Adaptation, Assumptio, Basilica Buff). These records mean the skill database names that status; they are not an exhaustive source list. Focused exporter and Guide-link tests pass; exact outcomes, durations, cures, and all infliction sources remain open.

*2026-09-24 Guide status C-source follow-up:* the exporter now scans Hercules `src/map/*.c` for literal `sc_start`/`sc_start4` calls whose third argument is a concrete `SC_*` identifier. The Guide shows up to eight source file/line references per status; the current source has 114 such references across 75 statuses. Comments, strings, and dynamic `get_sc_type(...)` expressions are excluded. This is explicitly a source-navigation index—not exhaustive attribution, effect explanation, or cure coverage. Exporter fixtures and the Guide status-detail test pass.

*2026-09-24 navigation destination-detail follow-up:* the world atlas now shows the selected destination’s account-visit/sync state, outgoing verified portal count, shortest-route hop count, and next verified exit’s source cell and destination map. Tests cover a graph-backed Prontera→Izlude route and an unavailable route; this confirms reference-data behavior only, not live traversal or dynamic transport coverage.

*2026-09-24 authored-travel follow-up:* the navigation graph generator now merges validated, provenance-bearing NPC service edges. The Guide and atlas identify the Izlude Sailor's conditional 150-zenny Byalan transfer and provide the map route. A live 20220406 scenario follows a map-cache-derived sequence of short walk legs and verifies the real fare, transfer to `izlu2dun`, and final walk warp arrival in `iz_dun00`.

*2026-09-24 server/accessibility follow-up:* party sharing uses level-spread 30 as an explicitly unaccepted S3 candidate; the effective 25% even-share bonus is unchanged. S10 journals typed DM quest/flag changes in SQL and replays them through per-character cursors, verifying state before cursor advancement and never replaying rewards. Latest focused Korangar library run: 375 tests passed, 17 ignored. Persistent Reduce flashing now dims procedural bursts and relevant point lights; authored effect coverage and visual acceptance remain open. Modified Hercules map-server sources compile and link. Live party EXP, early-AI farming/retreat, discovery migration/reconnect, alternate-character isolation, configured full-server startup, and visual acceptance remain open. Source-only Orc Archer RangedKeeper/local hazard-cost pilot (`orcsdun02`) and three-hit Orc Skeleton Skirmisher (`orcsdun01`) profiles now compile; mid/late roster completion, live behavior acceptance, and stress testing remain open.

*2026-09-24 target-outline follow-up:* selected and buffered monsters now receive a broken-corner bracket around their projected enemy health bar, using a configurable world-theme accent; Deuteranopia uses a tested yellow accent, and older world themes deserialize with the default. Focused palette/migration tests, `cargo check --workspace`, and formatting pass. The marker has not been visually inspected; screenshots/playtest acceptance remains deferred.

*2026-09-24 ground-target accessibility follow-up:* Game Settings persists optional quickcast-at-cursor and hold-aim-release modes for ground/trap skills. Hold mode arms on hotbar-key press, samples the cursor target on release of that same key, and takes precedence over quickcast; invalid releases cancel with a toast, clicks do not commit while the key remains held, and keyboard-focus transfer cancels the hold. Otherwise a valid cursor tile/entity cell casts through the existing walk-into-range path and no-target quickcast safely falls back to armed aim-and-click. Legacy settings default both options off. Tests cover persisted defaults/migration and slot-matched release with valid/fizzled target resolution and focus cancellation; at this checkpoint, per-skill overrides were still open (completed in the subsequent follow-up below), along with live timing, focus/key-release hardware edge cases, and visual/usability acceptance.

*2026-09-24 per-skill targeting follow-up:* Learned ground/trap skills now expose a Game Settings cycle for explicit Aim + click, Quickcast, and Hold + release overrides, then return to inherited global behavior. Inherited skills respect the global hold-over-quickcast precedence; explicit overrides take precedence per skill. RON defaults/migration, override serialization, global precedence, cycle reset, and client compilation are covered. Live key/focus timing and visual/usability acceptance remain open.

*2026-09-24 reduced-flashing follow-up:* the persistent Reduce flashing option now dims classic ACT sprite-effect layers to 55% alpha in addition to procedural bursts and `.str`-effect lights. Tests cover alpha scaling; effect timing and lifetime are unchanged. Particle-recipe coverage and visual contrast/readability acceptance remain open.

*2026-09-24 discovery monotonicity follow-up:* a complete but stale account-mob snapshot can no longer downgrade a higher milestone already received by the client; snapshot reconciliation merges existing milestones and queued deltas by maximum tier. A focused regression test passes. This does not replace live SQL migration, account-isolation, or reconnect acceptance.

*2026-09-24 static warp coverage follow-up (checkpoint):* the navigation exporter now parses warp NPC names containing spaces (including all seven previously omitted multiword-named static warps). At that checkpoint the graph had 538 maps and 3,183 static-warp edges with zero unsupported static warp rows; two parser tests cover multiword names, comments, and unknown destination validation. The later authored-travel follow-up adds reviewed service edges.

*2026-09-24 monster-family follow-up:* Hercules now has a written Orc-undead skill template and `tools/check_mob_skill_families.py`, which passes on five shared Orc Zombie/Orc Skeleton skill-state records and their trigger/presentation fields. This prevents drift in that pair's current data; it does not yet generate entries for new family members or add the Glast Heim roster. S6 remains partial and late-map behavior remains gated on prior live acceptance.

*2026-09-24 work-stop checkpoint:* Korangar was pushed through `20eccad3` (`Load Towninfo services for atlas destinations`); the follow-up `641871e2` records this checkpoint. The latest validated client results at that point were 385 Korangar library tests passed (17 ignored), `cargo check --workspace`, `cargo fmt --all -- --check`, and `git diff --check`. Visual checks are intentionally deferred. Hercules remains on `merge/pr6` with pre-existing local edits in four server configuration files, `db/quest_db.conf`, and untracked `libmariadb.so.3`; none were staged or changed. Its Orc-undead consistency checker passes and incremental `make -j1` exits successfully, but a forced clean rebuild is currently blocked during configure because the environment lacks the zlib development headers/library (`zlib.h` and `libz.so`). Do not count that incremental make as a clean rebuild.

*2026-09-24 dependency/build refresh:* `zlib1g-dev` is now installed. A fresh Hercules configure detects zlib; using the checkout's MariaDB headers and existing local `libmariadb.so.3` also passes its MariaDB checks, but configure next stops because PCRE headers are absent. `cargo check --workspace` passes, as do `cargo test -p korangar --lib` (402 passed, 17 ignored) and the headless-tester example tests (13 passed). `cargo test --workspace` currently cannot link the `ragnarok-packets` `pcap` example because `libpcap` development files are absent. No live integration run or clean Hercules build was completed in this refresh. Do not alter the pre-existing Hercules configuration edits or untracked library.

*2026-09-24 HUD snapping follow-up:* Game Settings can now cycle window snapping off/8/16/32px; the window cache persists the choice and defaults older caches to off. Dragged windows snap their actual screen-space top-left (including right/bottom anchored windows), with malformed grid values ignored. Cache-cycle/migration and anchor-coordinate tests pass. The Korangar library suite now passes 404 tests (17 ignored), `korangar-interface` tests pass, workspace check/format/diff checks pass; full workspace native linking and Hercules build remain blocked by missing PCRE/libpcap development packages. Visual position/drag acceptance remains deferred.

*2026-09-24 Escape-buffer cancellation follow-up:* Escape now cancels pending timed and ordinary walk-chain actions before changing UI state and posts a cancellation toast. This closes the case where an animation-buffered action could fire after the player pressed Escape to back out. `cargo fmt --all -- --check`, `cargo check --workspace`, and the Korangar library suite pass (385 passed, 17 ignored); live input acceptance remains open.

*2026-09-24 timed-buffer refusal follow-up:* regular skill refusal and the missing-required-item refusal now each clear any pending timed action and report that cancellation in the toast queue. This makes the GDD's “refusal clears it” rule explicit for both failure packet forms. Focused refusal tests pass; the full Korangar library suite passes (387 passed, 17 ignored), as do workspace check, format, and diff checks. Live input acceptance remains open.

*2026-09-24 Adventure Guide search follow-up:* item and card lookup now accepts exact numeric IDs, Aegis/display names, raw exported effect tags, and linked drop-source monster display/Japanese/sprite names. The All search and category filters use the same indexes. Tests check Oridecon by ID, a Poring source lookup, and the Poring Card ID/effect tag. Raw script tags remain untranslated and are not presented as semantic effect search; Guide usability and live acceptance remain open.

*2026-09-24 pause/resume checkpoint:* the latest Korangar work is committed through `ed9a6091` (`Add per-skill ground targeting overrides`), following `b38744c4` (HUD window grid snapping) and `53a437b4` (native build verification refresh). The working tree was clean before this documentation update. Most recent recorded verification is 405 Korangar library tests passed / 17 ignored, `cargo check --workspace`, `cargo fmt --all -- --check`, and `git diff --check`; visual inspection was deliberately deferred. Do not alter the separate Hercules checkout's pre-existing local config/database edits or untracked `libmariadb.so.3`. Resume first by installing the missing PCRE and libpcap development packages (`libpcre3-dev` and `libpcap-dev` on Ubuntu), then rerun a clean Hercules configure/build and the full client workspace tests. After build gates pass, continue with disposable-server/database acceptance and the live split/transfer/filter checks tracked in the phase plan, followed by mixed-client, reconnect/account-isolation, navigation/quest, and party/server acceptance. Quest Items, Favorites, and Recent Loot filters remain explicitly deferred until they have reliable item metadata and product rules; visual acceptance stays on hold until requested.

*2026-09-24 Adventure Guide monster-skill follow-up:* monster details now list up to eight bundled `mob_skill_db.conf` entries, show verified skill descriptions as links into the skill Guide, and preserve raw rate and delay values with their source. Unmatched skill names remain explicit raw references. Poring skill-link/value tests pass; live Guide usability remains open.

*2026-09-24 atlas Towninfo routing follow-up:* the atlas now offers up to three clickable actions for a selected map's official Towninfo facilities, routing to their exact listed coordinates; nonnegative coordinates are required. The POI coordinate conversion is unit-tested and the Korangar library suite passes (388 passed, 17 ignored), with workspace check, formatting, and diff checks passing. This is a client-data route, not evidence of NPC presence or service availability in a live server; visual and traversal checks are deferred/open.

*2026-09-24 Guide map-service routing follow-up:* map details in the Adventure Guide now include up to eight exact-cell Towninfo facility route actions in addition to outgoing portal routes. They use the selected map's client facility data and the existing navigation destination handler; negative coordinates are omitted. Tests verify route serialization and rejection, and the library suite passes (389 passed, 17 ignored); workspace check, formatting, and diff checks pass. This still does not verify NPC presence, active services, or visual usability.

*2026-09-24 party-ping membership follow-up:* party pings now check the current roster before consuming the shared send cooldown or sending chat. Party-less clicks show a toast and cannot create a false local “sent” marker. Membership is roster-based (a stale party label alone is insufficient) and now has a focused state test. Live server rejection/stock-client and mixed-party acceptance remain open.

*2026-09-24 combat-text density follow-up:* identical per-hit damage labels from the same source/target/skill/critical state now merge for 80 ms, displaying the unchanged per-hit value and combined count; different values or actors stay separate. This is presentation-only and never sums/rewrites server damage. Three focused tests cover matching, key mismatches, and expiry. Full Korangar tests, workspace check, formatting, and diff checks pass (393 library tests passed, 17 ignored); live visual/readability acceptance remains open.

*2026-09-24 debug-camera accessibility follow-up:* debug-camera forward/back/left/right/up movement, four keyboard look directions, and hold-to-accelerate are now in the versioned keybinding table and remappable through Game Settings. Mouse-look remains available; same-direction arrow-look/movement chords are allowed because the active camera suppresses character movement. Releasing a chord modifier also resets acceleration, avoiding a stuck fast speed. Keybinding tests cover remapped look/acceleration dispatch and the camera-mode gate; camera tests cover all look directions and frame-time scaling. `cargo check -p korangar --features debug` and focused debug input tests pass with `slangc` from `~/.local/opt/slang/bin`; visual and keyboard-layout acceptance remain open.

*2026-09-24 personal hunting goal follow-up:* Adventure Guide monster details can add up to five local-only target monsters to a separate Quest Log/HUD section; each character's IDs are stored in `game_settings.ron`, the user can remove targets, and graph-known static spawn regions offer broad map routes. The UI explicitly says these are not server quests and have no tracked kill count because visible death packets do not establish local kill attribution. Tests cover Guide availability, five-goal bound, duplicate rejection, display labeling, logout clearing, settings migration, per-character isolation, and persistence round-trip. Full Korangar library tests pass (397 passed, 17 ignored); `cargo check --workspace` and the debug-feature check pass. Live two-character/reconnect/traversal and visual acceptance remain open.

*2026-09-24 dependencies/build/live-test follow-up:* `libpcre3-dev`, `libpcap-dev`, and `libmariadb-dev` are now installed alongside `zlib1g-dev`. Ubuntu provides `/usr/bin/mariadb_config`, not `mysql_config`, so Hercules configure succeeds with `MYSQL_CONFIG_HOME=/usr/bin/mariadb_config ./configure --enable-packetver=20220406`. A fresh detached worktree configured and completed a from-scratch `make -j2`; the disposable full-server run against the main checkout also configured and built successfully. `cargo test --workspace` passes (Korangar library: 405 passed, 17 ignored), `cargo fmt --all -- --check` passes, every supported-data exporter reports current output, and `PYTHONPATH=tools python3 -m unittest discover -s tools/tests -v` passes all six parser/exporter tests. The disposable MariaDB integration runner freshly passed `storage-persistence` and `inventory-split` (2/2, cleanup audit clean): transfers preserve partial counts and indices; split validation covers valid/invalid/full-slot cases; live Iron Arrow and Poring Card packets report item types 10 and 6. These protocol tests do not exercise rendered GUI filters. The integration runner now validates Hercules checkouts with `git rev-parse`, including linked worktrees, with a regression test. Do not stage the separate Hercules checkout's local config/database changes or `libmariadb.so.3`. At this checkpoint, discovery isolation/reconnect was still open and has since passed for monster/map data; mixed-client pings/routes, quest/campaign recovery and traversal, and AI/path stress remain live gates. Visual acceptance remains deferred.

*Next work, in order:* (1) close Guide/navigation coverage gaps with verified authored NPC/service destinations and actual quest objective/giver routing data, keeping script references labeled as references; (2) continue remaining accessibility and action-buffer gaps, then retain the visual/usability pass for when requested; (3) run party ping/session mixed-client, personal-goal two-character/reconnect/traversal, and S10 campaign replay scenarios against a fully configured server; (4) continue server AI/profile work only after live pilot and path-load acceptance; (5) refresh each GDD row from measured evidence, not source presence alone. The unresolved broad items and their acceptance gates remain tracked in the section tables and `docs/plans/gdd-next-slices.md`.

*2026-09-24 party-carrier live follow-up:* new two-client carrier coverage (now registered as `party-message-carrier`) caught that Hercules was matching `[KORANGAR-PING:*]` against the start of the sender-prefixed party chat, silently bypassing the carrier cap/cooldown. The server now inspects the body after the sender delimiter and measures the 128-byte bound on that body. Against a freshly rebuilt disposable server, the scenario passed: one v2 ping relays, an immediate second ping from that sender is suppressed, a 128-byte body is accepted, a 129-byte body is rejected, and a later ping relays after cooldown. Fixture cleanup is clean. This proves the headless server carrier path only—not the graphical marker, mixed stock-client fallback, reconnect/map changes, or client-applied shared-route/ready-check flows.

*2026-09-24 account-discovery live follow-up:* `account-discovery-isolation` passed against a freshly rebuilt Hercules and disposable MariaDB. It verified a first-kill delta, replay of that mob in a second character's completed snapshot on the same account, and an empty snapshot for a separate account; fixture cleanup was clean. This closed core monster-milestone isolation/reconnect acceptance; map-visit acceptance was still open at this point.

*2026-09-24 account map-discovery follow-up:* live acceptance exposed that Hercules only runs `OnPCLoadMapEvent` on maps explicitly marked with the global `loadevent` mapflag, so ordinary-town/field visits were not recorded. Map transitions now invoke only the dedicated `KorangarDiscovery::OnKorangarMapChange` event; unrelated map scripts remain opt-in. A freshly rebuilt server passed `account-discovery-isolation`, which now verifies first-visit delta delivery, replay of the visited map in a completed same-account alternate-character snapshot, no cross-account map delta/snapshot leakage, plus the existing monster milestone checks. Fixture cleanup was clean. Production migration/startup remains separate acceptance.

*2026-09-24 full headless-suite follow-up:* the 162-scenario ordered run produced 156 passes, five failures, and one expected skip. The five failures reduced to two fixture issues: a second Prontera NPC overlapped the dialogue helper's position search, and discovery could choose the account's current map, for which no map-change event fires. The EXP helper NPC is now outside the dialogue search radius, and discovery chooses an undiscovered map different from the current map. A rebuilt Hercules and client passed all seven focused cases (`dialogue-linear`, `dialogue-choice`, `dialogue-number`, `dialogue-string`, `dialogue-warp`, `quest-reviewed-brasilis-npc-routes`, and `account-discovery-isolation`); packet coverage reported zero unknown/fallback or failed packets. Together this makes all 161 non-skipped scenarios green across the full and focused runs; the full ordered run itself has not yet been repeated after the fixes. Full-run fixture cleanup was clean. The novice skill case remains an expected skip because its active skills are quest-gated.

*2026-09-24 account-discovery migration follow-up:* the discovery schema file was not registered with Hercules' SQL upgrade runner and used a filename/header that its fixed-size index parser cannot recognize. It now uses the standard timestamped filename/header and is listed in `sql-files/upgrades/index.txt`, with an idempotent `sql_updates` marker. Applying it twice to a fresh disposable MariaDB schema created both tables and left one upgrade marker. Existing-production rollout/startup is still unverified; the source-side `OnInit` table creation remains a fallback, not migration evidence.

*2026-09-25 live-server acceptance:* the latest full ordered run against freshly provisioned Hercules and MariaDB passed **162/162 non-skipped scenarios**, with one expected `skills-novice` skip, zero flaky passes/failures/unexpected skips, 176 distinct incoming and 68 outgoing packets, zero unknown/fallback or failed packets, zero unmet skill expectations, clean fixture cleanup, and clean DM journal/replay cursor audits. Archive: [`runs/20260925-014755.log`](../tools/testing/runs/20260925-014755.log). This run includes `account-discovery-isolation` after the long skill sweep and `party-quest-credit`, plus the S4 `death-recovery-save-point` scenario with its logout/relogin persistence assertion. The discovery fixture logs in directly instead of using the paired-venue warp (which flushes pending login snapshots) and waits for both account-ledger end markers before selecting unused mob/map fixtures. This closes the full ordered headless-server registry gate; it does not replace production migration/startup, live graphical playtests, or the separately tracked gameplay/data gaps. Visual acceptance remains deliberately deferred.

*2026-09-25 registry refresh:* after adding `reset-command-behavior`, the first 163-scenario ordered run (`runs/20260925-031223.log`) had one late S4 timeout and a skill-tree packet decode failure; the reset scenario itself passed. The failure did not reproduce in the focused reset→S4 sequence or the next complete ordered run. The fresh full run passed **163/163 non-skipped scenarios**, with one expected `skills-novice` skip, zero flaky passes/failures/unexpected skips, 176 incoming / 68 outgoing packet types, zero unknown/fallback or deserialization failures, zero unmet skill expectations, clean DM journal/replay SQL audits, and clean fixture teardown. It includes the new reset scenario and the S4 logout/relogin refund assertion. Archive: [`runs/20260925-042322.log`](../tools/testing/runs/20260925-042322.log). The headless server registry gate is green at 163 scenarios; production migration/startup, visual acceptance, and separate gameplay/data gaps remain open.

*2026-09-25 death-recovery persistence follow-up:* the S4 scenario now disconnects after observing real base/job EXP loss, reconnects the same character, then verifies the exact measured half-refund on save-point arrival with no duplicate refund. This directly validates persistence of the pending per-character ledger across logout/relogin. Focused rebuilt-server run passed 1/1 with clean fixture teardown (`tools/testing/runs/20260925-014233.scoped`); the later full ordered run also passed this extended scenario. Repeated-death accumulation, zero-loss/exempt maps, party-kill attribution, and the 10-kill threshold remain open.

*2026-09-24 DM campaign offline replay follow-up:* the first live attempt exposed that the headless runner created only Hercules' base schema, omitting the campaign checkpoint/state/event upgrades; the reset command consequently logged missing checkpoint tables and never reached replay. The three campaign SQL upgrades now use Hercules' indexed filename/header convention alongside discovery, the runner applies all four to fresh disposable databases before server startup, and the two client/server scenario `dm-party-offline-replay` passes. It recorded a campaign quest start and story flag while one enrolled party member was offline, verified both after reconnect, and confirmed a repeated catch-up emits no duplicate active-quest notification. Fixture cleanup was clean. This is one reconnect path only: late join, alternate-character isolation, multi-transition order, reward isolation, run recreation, and failed-replay repair remain open.

*2026-09-24 authenticated party-message follow-up:* Korangar now checks each party-ping/session message's claimed sender against the roster name associated with Hercules' authenticated party-chat account ID before applying the structured command. This protects shared-destination changes, ready-check responses, and local ping markers from forged sender prefixes in chat text; destination-accept messages retain the sender in their parsed representation so they receive the same check. Case-insensitive roster matching and wrong-account/name rejection are unit-tested; the full Korangar library suite passes (412 passed, 17 ignored). Live mixed-client/replay behavior and visual acceptance remain open. `zlib1g-dev` is confirmed installed (`pkg-config` reports zlib 1.3); no additional package is needed for this client change.

*2026-09-24 party-session carrier follow-up:* the two-client carrier scenario now also verifies that Hercules relays `[KORANGAR-SESSION:v1]` ready-check traffic, applies the same per-character cooldown across ping and session message kinds, accepts a 128-byte session body, and rejects 129 bytes. It asserts relayed packet account IDs and sender-prefixed text for ping/session carriers, validating the identity premise used by Korangar's structured-message authentication. The disposable server run passed with clean fixture teardown. This proves carrier bounds/relay, not the client's shared-route and ready-check state handling, stock-client presentation, or reconnect/map-change behavior.

*2026-09-24 stock quest-credit follow-up:* disposable live scenario `party-quest-credit` passed against two Korangar clients and the server's Spore hunt objective. A member with the active quest received kill progress, a nearby party member without the quest received none, and after independently adding the quest that member received progress on the next party kill. The fixture positioned the second member from the spawned monster's observed cell and verified credit at exactly 30 cells but not at 31; after both clients left the party, the active quest holder still received solo progress while the former party member did not. A contested-kill check observed both party members land hits on one Spore while each quest advanced exactly once. Finally, both clients started attacks on separate Spore targets before awaiting either result; both died and each active quest advanced twice. Fixture cleanup was clean. **Configuration correction:** the ignored local `conf/import/battle.conf` sets `area_size: 30`, overriding the source default of 14 in `conf/map/battle/client.conf`; the initial failed boundary attempt also measured from the player rather than the monster.

## Status Language

*Status refresh: 2026-09-24. Code-backed summaries below include the Guide's All-category search, active quest data, hunt/item routes, 842 related static NPC-script references with supported exact-cell routes, quest-marker visibility, the remappable party Danger ping, and partial map-information summaries. These NPC references are not verified quest givers. Live server, mixed-client, and visual acceptance remain marked open; visual checks are intentionally deferred.*

| **Label**        | **Meaning**                                                                                              |
|------------------|----------------------------------------------------------------------------------------------------------|
| Done             | Implemented and seen working live (GUI or playtest).                                                     |
| Done (unseen)    | Implemented and test-covered, but never observed on screen. Treat as done pending one live look.         |
| Partial          | A real slice exists; the GDD's full intent does not.                                                     |
| Stock            | Behaviour is unmodified upstream Hercules/RO. Nothing done, nothing broken.                              |
| Not started      | No code, or only a spec/plan document.                                                                   |
| Decided (Cn)     | Was a conflict between a fork decision and this GDD; resolved 2026-09-21 — see the Conflicts table below.  |

## Headline

- **The foundation is real.** A custom Rust client connects to a `PACKETVER=20220406` Hercules fork, four friends have played a session over Tailscale from shipped Windows/macOS packs, and the headless protocol suite now registers 152 scenarios. The last complete-suite evidence remains the earlier 149-scenario run; targeted inventory and party-carrier scenarios have separate disposable-server evidence. Phase 0 and most of the client half of Phase 1 exist.

- **The largest body of finished work is not in this GDD's original scope.** The forks were built around a **DM-run tabletop campaign** ("Seal Cascade": 19 arcs in 4 acts, ~5,600 lines of shared DM script, `@dm` console, d20 skill checks, private party instances, scripted hazards, dice cards, bestiary journal, loot generator). Decided 2026-09-21: this is **DM Session mode**, a separate opt-in mode (§2.3), not the §8 story spine.

- **§6 (Monster AI) has map-scoped source pilots.** `db/re/mob_ai_profile_db.conf` opts Fabre Aggressor and Poring low-health NPC_RUN on `prt_fild08`, Orc Archer RangedKeeper plus local hazard-weighted A* on `orcsdun02`, and Orc Skeleton three-hit Skirmisher on `orcsdun01`; map+monster lookup prevents global species changes. A reviewed Orc-undead family template and a checker now verify five shared skill-state records; the general template generator and Glast Heim family expansion remain open. The C build succeeds, but live farming/retreat/path-load/readability acceptance is outstanding. Hazard coverage beyond the bounded local path window remains open. Do not confuse client target cycling/monster frame with accepted monster AI.

- **Navigation (§9) and knowledge systems (§9.5, §10.13, §11.5) remain partial.** Route graph/atlas with account-persistent visited markers, searchable graph map identifiers with account-persistent visited badges, validated actionable `<NAVI>` links, Guide/static spawn-map routes, server hunt routes, item-turn-in→Guide→verified drop-source map routes, and 842 quest-linked static NPC-script references with exact routes to supported cells exist. The Guide provides All/category search across versioned monster/item/card/skill/status/job/map/quest references, 128 inherited job skill trees, 147 job-level stat schedules, active server quest data, exported job names, and account-persistent monster milestones. NPC references do not establish giver ownership or conditional objective locations. Core account discovery delta, alternate-character reconnect, and cross-account isolation, plus Prontera and Izlude/Sunken Ship route traversal, now pass live checks; production migration/startup, personal-goal reconnect/traversal, conditional spawn coverage, broader transport services, richer build planning/cross-links, and visual acceptance remain open.

- **Combat responsiveness (§5) is further along than expected**: ground-target footprints with real Hercules layouts, propagated cast targets for eligible invariant footprints, target cycling and a reactive monster frame, walk-into-range for every targeting mode, a unit-tested 200 ms timed action buffer with immediate target-loss cancellation, overhead cast bars, hit/animation synchronisation, and a fork-only cast cancel packet all exist. Live feel acceptance remains open.

## Status by Section

### §5 Tactical Combat Framework

| **Item**                                   | **Status**       | **Evidence / Notes**                                                                                                                                                                                                                  |
|--------------------------------------------|------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 5.2 Input buffer (150-250 ms, one action)  | Partial          | `TimedBufferedAction` is a separate 200 ms, newest-wins slot for attack, pickup, and entity/ground casts during finite local attack/skill/pickup animation. Expiry boundary/wrap and target/item disappearance invalidation are tested; Escape clears timed and ordinary walk-chain actions, and skill refusals clear pending timed actions. Queue/expiry/cancel toasts provide client feedback. Live lock timing and rapid-input acceptance remain open. |
| 5.3 Click-to-move                          | Done             | Stock Korangar, live-verified.                                                                                                                                                                                                        |
| 5.3 Keyboard movement                      | Done             | WASD shipped 2026-09-05: optional (Game Settings, default on), camera-relative, same pathfinder, 200 ms packet throttle, disabled while text fields have focus. Meets the "no unique collision / no speed advantage" rule by construction. |
| 5.3 Hold-mouse continuous path             | Not verified     | Not audited.                                                                                                                                                                                                                          |
| 5.3 No universal dodge roll (LOCKED)       | Decided (C3)     | Plan section in `korangar/docs/plans/modern-mechanics.md` §5 struck 2026-09-21. Principle stands.                                                                                                                                       |
| 5.4 Left-click select, target frame        | Partial          | Clicking/cycling selects a monster, keeps its in-world health bar visible, and opens a reactive frame with live HP and available level/element/race/size. Client builds/tests pass; cast/status details and live visual acceptance remain open. |
| 5.4 Target cycling / nearest hostile       | Partial          | Tab/Shift+Tab cycles visible living monsters by distance then screen-centre angle; click-to-attack remains; Ctrl+Shift+Tab retains party support-target cycling. Client builds/tests pass; live acceptance remains open. |
| 5.4 Ground-target preview of reachable area| Done (unseen)    | `world/skill_layout.rs` + `Map::render_skill_footprint` (2026-07-26): real cell shapes from `skill_db` layouts, the 15 hardcoded Hercules unit layouts, direction-dependent walls (Fire Wall, Ice Wall …), red tint when out of range. Never seen on screen. |
| 5.4 Alternate cast styles                  | Partial          | Classic press-then-click remains; an optional hold-ground/trap-hotbar-key → aim → release-to-cast style and quickcast-at-cursor are available, with per-learned-skill Aim + click/Quickcast/Hold + release overrides and inherit-global behavior. Live timing and visual acceptance remain open. |
| 5.5-5.7 Positioning / ground control / KB  | Stock            | Server mechanics untouched, as intended.                                                                                                                                                                                              |
| 5.8 Telegraphs                             | Partial — unseen | Cast packet target data is retained and invariant multi-cell footprints render for eligible casts; unknown or level-dependent layouts show only a cast bar. Live target-semantic and visual acceptance remain open. Campaign scripted hazards also exist (`DM_HazardArea`). |
| 5.9 Immediate layer (damage numbers)       | Done             | Damage/heal numbers, crit attack animation. No "element advantage" cue.                                                                                                                                                               |
| 5.9 Contextual / advanced layers           | Partial          | Monster target frame shows level/element/race/size when bundled data exists. Element advantage cues and damage breakdown are not implemented. |
| 5.10 Autoattack feel                       | Partial          | Animation engine phases A-D closed; hit/attack sync; ReadyFight stance; ranged attacks draw the real ammunition sprite. Proc/status feedback: opt1/opt2 tints, looping status STRs, freeze on stun/sleep.                             |
| *Fork addition — cast cancel*              | Done (unseen)    | `CZ_CANCEL_CAST` (0x0F00) + Hercules delta; right-click/Escape aborts own cast. Not an RO feature; deliberate. Movement still never cancels; casting still roots.                                                                     |
| *Fork addition — skill-fail reasons*       | Done             | `ZC_SKILL_FAIL_REASON` (0x0EFE) names why a skill actually failed; missing-item failures name the item. Directly serves the "players can explain outcomes" success criterion.                                                          |

### §6 Monster AI and Encounter Design

| **Item**                            | **Status**   | **Evidence / Notes**                                                                                                                                                                                                                              |
|-------------------------------------|--------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 6.2 Behaviour archetypes            | Partial      | Map+monster profile hook in `Hercules/src/map/mob.c`; optional `mob_ai_profile_db.conf` adds Fabre Aggressor/Poring low-health Coward on `prt_fild08`, Orc Archer RangedKeeper on `orcsdun02`, and three-hit Orc Skeleton Skirmisher on `orcsdun01`. A reviewed Orc-undead family template and consistency checker cover the shared Zombie/Skeleton skill-state entries. Other species/maps remain unchanged; live acceptance remains. Six of eight archetypes are achievable with mode bits + `mob_skill_db` data alone — see §6.10. |
| 6.3-6.5 Family behaviour, ground reaction, threat | Stock |                                                                                                                                                                                                                                             |
| 6.7 Elites                          | Not started  |                                                                                                                                                                                                                                                   |
| 6.8-6.9 MVPs / boss template        | Partial (campaign only) | Campaign bosses in private instances have adds, phases via `@dmbeat` variants (Dark Lord, Randgris, Beelzebub, Thanatos, Bijou/Maret), and pulse hazards (Ifrit heat, Rift Anchor, Thanatos resonance, Ash Vacuum). DnD mode suppresses stock MVP spawns. Open-world MVPs are stock. |
| *Server-side encounter tooling*     | Done         | `dm_encounters.txt` prices encounters on real HP/DPS (2026-08-18); `@dm hazard`, `@dm spawn`, `@dm stakes`; stealth via `MD_DETECTOR` mode stripping; `dm_traps.txt` helpers. This is the DM's hand doing what §6 wants the AI to do.       |

### §7 Classes, Stats, Skills, and Build Identity

| **Item**                          | **Status**   | **Evidence / Notes**                                                                                                                                          |
|-----------------------------------|--------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 7.1 Original stat model           | Done         | Stock. Character creation now has a 48-point allocator with a suggested first-job spread and a live 8-facing preview (2026-09-03/04).                          |
| 7.1 Derived-stat preview          | Not started  | Stats window allocates points; no next-point preview.                                                                                                         |
| 7.4 Skill tree presentation       | Partial      | Tabs, drag-to-hotbar, staged current-point allocation, and per-level SP/range/cast/element/area/reagent tooltips exist. Cooldown/delay, prerequisites, verified formulas, and future-level planning remain. |
| 7.5 Respec                        | Partial; live check due | Group 0 grants `@streset`, `@skreset`, and `@refundskill`; Character Overview exposes separate free stats/skills reset buttons. Focused test confirms distinct reset effects, no Zeny cost, and relog persistence with a group-99 fixture. Verify group-0 access, level/job scope, and prerequisite rejection. Priced `resetnpc.txt` remains inactive; stock Hypnotist is restricted. |
| 7.6 Build planner                 | Not started  | Rejected-adjacent: the client roadmap deferred build planning to "skill tree search/filter later".                                                            |

### §8 Story and Quest Design

| **Item**                          | **Status**   | **Evidence / Notes**                                                                                                                                                                                                                 |
|-----------------------------------|--------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 8.1 Story spine                   | Not started (self-directed) | Seal Cascade (4 acts, 19 arcs, quest IDs 20000-20234) exists but is **DM Session mode** (C5), gated by `DM_SessionAllows` at 50 sites. No self-directed spine exists.                                                      |
| 8.2 Hunting goals / rumor leads   | Not started  |                                                                                                                                                                                                                                      |
| 8.3 Quest markers                 | Partial      | Standard quest-effect markers on campaign hub and objective NPCs; persistent Game Settings toggle hides/shows them (old settings default to visible). Remaining: distinguish authored main/side markers and live-check. |
| 8.4 Breadcrumb guidance           | Partial      | World map/next-exit routing draws a sampled walkable minimap breadcrumb to the next portal or authored travel-service cell and recomputes after map changes. Valid `<NAVI>` targets render labeled route buttons; the atlas exposes the next edge location/action and Towninfo facility actions route to exact client-listed cells. Identified hunting objectives route only to loaded static spawn maps present in the navigation graph. The Izlude Sailor → `izlu2dun` → `iz_dun00` travel path now passes live headless traversal. NPC/story objective locations, broader route coverage, client route following, and visual acceptance remain open. |
| 8.5 Kill counts only when meaningful | Done      | Hunting contracts fill from **drops** instead of kill counts (2026-08-25); the quest log shows what each contract still wants.                                                                                                       |
| 8.5 Shared party progress         | Built; stock live checks passed | Campaign quest state uses `DM_Party*` helpers. For stock kill objectives, `mob.c` credits nearby members of the killer's party through `quest_update_objective_sub` (`AREA_SIZE`; default 14, active local override 30). Disposable test confirms mixed quest ownership, progress at 30 but not 31 cells, solo credit after leaving the party, one increment per character when both members damage the same monster, and two increments each when the clients attack separate targets concurrently. |
| 8.5 DM Session party quest sync    | Partial — reconnect and late-join paths live-tested | Party-join, quest-log, and map-load hooks replay typed quest/flag transitions from a SQL journal using character-keyed cursors, verified postconditions, and no reward replay. `dm-party-offline-replay` verifies reconnect replay and idempotent repeat catch-up. `dm-party-alternate-character` confirms a second character on the same account has no story quest/flag before joining, receives both on joining, and cleans up through journaled transitions. SQL audits confirm both characters' cursors reach the journal tail. Actor-side quest/flag helpers verify and advance their own cursor after mutation. Run identity, party recreation/reset, reward isolation, and failed-replay repair remain open. |
| 8.6 Story-based system teaching   | Not started  |                                                                                                                                                                                                                                      |
| *Fork addition — skill checks*    | Done         | `@check`: d20 + stat/15 + proficiency + assists vs DC, with server-side consequences (hazard saves, `setcell`, mode bits). Not in the GDD; a candidate for §8.6-style "optional guidance."                                          |

### §9 World, Exploration, and Navigation

| **Item**                          | **Status**   | **Evidence / Notes**                                                                                                                                                                    |
|-----------------------------------|--------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 9.2 World map                     | Partial — code present, live acceptance open | `World Map & Route Finder` is available from the in-game menu. The generated graph currently contains 538 maps and 3,185 directed edges: 3,183 loaded static walk warps and two reviewed Izlude/Byalan NPC services, including the conditional 150-zenny ferry edge. The atlas lays out 27 selectable towns/destinations, filters overview connections through graph reachability, highlights a selected route, and shows account-persistent visited badges. Selected-target details show visit-sync state, outgoing connections, hop count, and the next portal cell or service action/requirement; Towninfo facility actions route to exact client-listed cells. The Izlude ferry and dungeon-entry route pass headless live acceptance; broader travel-service/conditional-route coverage, client route following, and visual acceptance remain open. |
| 9.3 Map information panel         | Partial — source implemented, visual/live acceptance open | World Map & Route Finder shows spawn-record-weighted mean level, static spawn-record/species counts, graph exits and authored NPC travel services with their next-route action/requirement, account visit-sync state, and online party members on the selected/current map. Up to three Towninfo facilities on the selected map have clickable exact-cell route actions; the table's remaining facility names are summarized. No live density count or conditional-spawn coverage. Danger markers and a configurable non-blocking entry warning exist; broader authored NPC/service coverage remains partial. |
| 9.4 Monster population regions    | Partial      | v1 bestiary exports map-level regions from loaded static spawn directives (3,380 records, 965 monsters). No density layer, exact cells, or dynamic/conditional spawns. |
| 9.5 In-game encyclopedia          | Partial — player Guide present | Versioned bestiary/item/card/skill data, graph map identifiers, job names, account-persistent monster milestones, and tracked-HEAD quest references feed a searchable Guide. The Guide has an All search across monsters/items/cards/skills/statuses/maps/jobs/quests plus per-category filters. Item/card lookup includes exact IDs, display/Aegis names, raw exported effect tags, and verified drop-source monster names/aliases; monster detail lists bundled server skill entries and links matched skill IDs to detail pages, and adds monsters to a separate per-character, client-only hunting-goal list. Quest search matches titles/IDs, explicit target monsters/maps, and static NPC script references; 842 quest IDs currently have at least one related NPC script reference and supported graph-known NPC cells offer exact-coordinate routes. Two source-reviewed Brasilis quest stages identify Angelo#br as the offer for quest 9030 and turn-in NPC for quest 9031, with exact source lines and route labels; the exporter rejects stale call-site citations. Live `quest-reviewed-brasilis-npc-routes` verifies the actual offer and turn-in/cooldown script branches; quest 9031 is added synthetically and objective completion is not tested. Other script references remain clues rather than verified giver relationships, and conditional story steps/rewards/prerequisites remain unavailable. Active server quests are merged/labeled; active objectives show progress and link supported monster/map routes. Personal hunt goals are persisted per character and route only to graph-known static spawn regions; kill progress is not claimed. The static quest export deliberately falls back to tracked Hercules HEAD while `db/quest_db.conf` is locally modified and does not read live SQL. Job entries link to skills and show cumulative bonuses/milestones; item↔monster and quest↔item/monster cross-links, route actions, source/incomplete-data labels, account-synced badges, and 700 searchable status-icon names are present. 540 status icons expose linked server status IDs, raw flags/recalculation metadata, direct associated-skill and explicit skill-db `StatusChange` links. Exact status outcomes/durations/cures, exhaustive infliction sources, conditional skill effects, richer build planning, mechanics/server rules, authored effects/spawns, and live sync acceptance remain. DM Bestiary/Reveal all remains separate. |
| 9.6 Account discovery             | Partial; monster + map live acceptance passed | Hercules has account+mob and account+map SQL ledgers with first-kill/first-visit deltas and complete bounded login snapshots; a dedicated map-change event records visits without globally enabling map `loadevent` scripts. Korangar account-binds private server messages and shows Guide badges without hiding facts. Disposable live acceptance passed for monster/map deltas, same-account alternate-character snapshots, and isolation from a second account. The indexed Hercules upgrade now applies idempotently on MariaDB; representative-existing-database rollout/startup and broader encounter milestones remain. DM unlocks stay session-only; `@monster` is server permission-checked. |
| 9.7 Rumors                        | Not started  |                                                                                                                                                                                         |
| 9.8 Travel                        | Stock + one addition | Kafra, warper NPC. `@partyjump <name>` (2026-09-05) lets any party member warp to another online member — accepted unbounded for now (C4).                              |
| 9.9 Portal labels                 | Partial — source implemented, visual/live acceptance open | Hovering a graph-known warp entity shows its destination; the next verified route edge receives a “Route portal” label. Unknown/unindexed warps retain existing hover text. Graph matching and route-highlight tests pass; in-world readability and live route behavior remain unverified. |
| Minimap (base)                    | Done         | Ctrl+Tab; Towninfo POIs (shops, Kafra, guides), player blip, party blips with hover names, compass packet.                                                                              |

### §10 User Interface and User Experience

| **Item**                              | **Status**   | **Evidence / Notes**                                                                                                                                                          |
|---------------------------------------|--------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 10.2 HUD presets / Edit Mode          | Partial      | Versioned `WindowCache` migrates legacy geometry and stores movement lock, named per-character layouts, and optional 8/16/32px screen-grid snapping. Game Settings exposes lock/unlock, Classic/Modern presets, `My Layout`, and reset. Combat-only fading and visual acceptance remain open. |
| 10.3 Player frame                     | Partial      | HP/SP bars, Base/Job EXP + zeny HUD (Alt+Shift+H), overweight colouring. Status bar is a **text** list with timers; real SC icons not implemented.                             |
| 10.4 Target frame                     | Partial      | Reactive monster name/HP frame and available level/element/race/size reference chips are implemented; target cycling skips invalid/dead/hidden entities. Status/cast details and live visual acceptance remain open. |
| 10.5 Cast bars                        | Done         | Overhead cast bars on entities (self and others).                                                                                                                             |
| 10.5 Ground footprints / range preview| Done (unseen)| See §5.4.                                                                                                                                                                     |
| 10.5 Target-of-target                 | Not started  |                                                                                                                                                                               |
| 10.6 Hotbars                          | Partial      | Three rows of nine (1-9, Ctrl+1-9, Alt+1-9; F1–F9 aliases still work); items/potions on the hotbar (2026-09-05). All 27 number-row slots and debug-camera movement/look/acceleration actions use the versioned Game Settings remapping table. Live/alternate-layout acceptance remains open. |
| 10.7 Equipment sets                   | Not started  |                                                                                                                                                                               |
| 10.8 Stats interface modes            | Not started  |                                                                                                                                                                               |
| 10.9 Inventory search/filter/sort/lock| Partial — headless live paths passed; visual acceptance open | Inventory has All/Equipped/Gear/Items/Consumables/Etc/Cards/Ammo tabs, name search, sorting, drag arrangement with server-persisted slot ordering, per-character drop/sale protection, and server-authoritative split-stack. Headless live tests pass valid/invalid/full-capacity split rejection and confirm server item types for stacked Iron Arrows and Poring Card. Quest-item/favorite/recent-loot filters are explicitly deferred pending authoritative metadata/persistence contracts. Visual acceptance remains open. Weight display and thresholds: done. |
| 10.9 Storage                          | Partial — headless live paths passed; visual acceptance open | Kafra storage open/store/retrieve, name search, and shared Gear/Items/Consumables/Etc/Cards/Ammo filters are present. Headless live tests confirm partial store/retrieve counts and stable source/destination indices; visual acceptance remains open. |
| 10.10 Equipment comparison            | Partial      | Tooltip shows "— vs equipped —" deltas (ATK/MATK/DEF/slots/refine). Script bonuses and vs-target context: not started.                                                        |
| 10.11 Quest tracker                   | Partial      | Quest log has Track/Untrack, a HUD summary, and per-character persistence reconciled against the server list. Hunting notifications/progress populate server quest mob counts and link known mobs to static spawn-map routes. Item turn-ins open the matching Guide item/card record and link to graph-known verified drop-source maps. Monster details in the player Guide can add up to five separate character-local personal hunting goals; these display in the quest log/HUD, persist in `game_settings.ron`, and route only to graph-known static spawn regions. They explicitly have no server-tracked progress (the client cannot safely attribute all visible monster deaths). Explicit NPC/story quest locations and live reconnect/two-character/traversal acceptance remain open. |
| 10.12 Minimap layers / waypoints / pings | Partial | Minimap offers six expiring party ping kinds over v2 chat, retains v1 location decoding, authenticates structured ping/session sender names against Hercules' account ID and current party roster, validates map/cells, and draws one same-map marker on the minimap and in-world; Hercules source bounds/rate-limits ping and shared-route/ready-check messages. Graph routing and visited badges are present. Ready checks are nonce-bound with one response per online roster member and 30-second expiry. Personal waypoints, mixed-client/live acceptance, and marker layers remain open. |
| 10.13 Global search                   | Partial — player Guide shipped | Adventure Guide has a combined All search for monster/item/card/skill/status/map/job/quest data and per-category filters. Item/card search indexes exact IDs, display/Aegis names, raw exported effect tags, and verified drop-source monster names/aliases; scripts remain untranslated. Quest search includes target monsters/maps, 842 related NPC script references, and active server entries; graph-known NPC cells offer exact-coordinate route actions. Account discovery badges, clickable item↔monster/quest↔item/monster links, and job→skill cross-links from 128 verified inherited trees are present; job detail includes 147 exported stat-bonus schedules. Status details expose verified IDs, raw server flags/recalculation fields, direct config-associated skills, explicit skill-db `StatusChange` links, and literal C call sites for 75 status types (114 references); these call sites are non-exhaustive source-navigation clues. Exact effects/cures/exhaustive sources remain undocumented where no verified data exists. Script references do not prove quest-giver relationships or disclose conditional story steps, rewards, or prerequisites; those, conditional effects, full build planning, and live acceptance remain open. |
| 10.14 Party UI                        | Done         | Roster window (Alt+Z / Alt+P) with HP + SP bars (server now sends party SP), class (job-refresh bug fixed server-side 2026-09-05), "Go to" button, minimap blips, world ally HP bars. Click-to-target and distance/out-of-map state: not verified. |
| 10.15 Chat                            | Partial      | Public / Party / Whisper channels, `/r`-style commands. No timestamps, no item links (`<ITEM>` stripped), no copy/paste selection, no Loot/System tabs.                        |
| 10.16 UI scaling                      | Done         | Interface Settings > Scaling (Ctrl+I); character-creation screen exposes it directly (playtest T6).                                                                            |
| Character creation / select           | Partial      | Sex, hair, stat allocator, 8-facing preview, headgear on select cards. Delete is now visible and requires typing the exact character name after a warning banner; live acceptance remains open (M1-014). |

### §11 Loot, Equipment, Cards, and Refinement

| **Item**                        | **Status**   | **Evidence / Notes**                                                                                                                                                      |
|---------------------------------|--------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 11.2 Configurable nearby pickup | Partial (C6 applied) | Hercules `@autoloot <percent>` (a drop-rate threshold), `@alootid`, `@autoloottype`, plus a Commands-window button. Granted to group 0 on 2026-09-21. Not yet the §11.3 category/wishlist filter. |
| 11.3 Loot filters               | Not started  |                                                                                                                                                                           |
| 11.4 Rare-drop presentation     | Not started  |                                                                                                                                                                           |
| 11.5 Cards ↔ monster linking    | Partial      | Versioned card and bestiary references power player Guide searches and cross-links; verified drop sources route to known spawn maps. Missing/conditional source coverage and live navigation acceptance remain. |
| 11.6 Refinement                 | Partial      | Weapon-refine window exists and names the item on result. Success chance / failure consequence display not audited. Refine risk model is stock.                            |
| Drop rates                      | Stock        | 100% common / 100% card. No small-server tuning (§12.2).                                                                                                                  |

### §12 Economy, Vending, and Crafting

| **Item**                    | **Status**   | **Evidence / Notes**                                                                                                                                                    |
|-----------------------------|--------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 12.1 Player vending         | Descoped (C2) | Out of scope for this server; §12.1 rewritten. Direct trade is the loot path.                                                                                            |
| 12.2 Small-server tuning    | Not started  | All rates stock.                                                                                                                                                        |
| 12.3-12.4 Crafting / commission board | Not started | Replaced in practice by the DM Loot Generator and `@dmreward`.                                                                                                 |
| 12.5 Sinks                  | Stock        |                                                                                                                                                                         |
| Direct player trade         | Done         | Protocol MVP 2026-07-10; two-client live validation still listed as open.                                                                                               |

### §13 Party and Friends-Server Social Systems

| **Item**                          | **Status**     | **Evidence / Notes**                                                                                                            |
|-----------------------------------|----------------|---------------------------------------------------------------------------------------------------------------------------------|
| 13.2 Shared destinations          | Partial        | Selected map/cell routes are proposed over `[KORANGAR-SESSION:v1]`, shown in the party window, accepted into each recipient's local navigation, and cleared by matching nonce. Hercules source bounds/rate-limits the carrier and compiles; mixed-client live acceptance remains open. |
| 13.3 Party pings                  | Partial        | Location, Assist, Danger, Retreat, Ready, and On my way send `[KORANGAR-PING:v2]` through ordinary party chat; Ctrl+Alt+G is a remappable Danger shortcut; v1 location pings remain readable. Korangar validates version/kind/map/cells, sender sends are limited to one/second, and received cells are bounded to the loaded map before drawing a 15-second labeled same-map marker on the minimap and in-world. Shared destinations and nonce-bound 30-second ready checks use `[KORANGAR-SESSION:v1]`; Hercules source rejects recognized ping/session bodies over 128 bytes and rate-limits each character to one per second. Modified C compiles; live mixed-client/server acceptance remains, and peer text never grants persistent state. |
| 13.4 Hunting goals list           | Not started    |                                                                                                                                 |
| 13.5 Level difference handling    | Partial        | Effective import config gives 25% even-share bonus per extra eligible member; `party_share_level` is set to 30 as a playtest candidate. Under/over-limit, distance, and EXP-total live checks remain. No level-sync. |
| 13.6 Shared quest progress        | Built; live check due | Campaign state shares via helpers; stock kill objectives already credit nearby same-party members. See §8.5. |
| Party warp                        | Done           | `@partyjump`, non-GM.                                                                                                           |
| Party loot                        | Decided        | Stays random (`party_item_share_type: 0`), operator decision 2026-09-05.                                                        |

### §14 Death, Recovery, and Difficulty

| **Item**                      | **Status**   | **Evidence / Notes**                                                                                   |
|-------------------------------|--------------|--------------------------------------------------------------------------------------------------------|
| 14.2 Recoverable death penalty| Partial      | Stock `death_penalty_type: 1`, 1% base / 1% job, plus source-toggleable per-character recovery script (actual loss measured; half refunded after 10 same-map kills or qualifying login/map-load near save point). Live relog, repeated-death, zero-loss, and party-kill validation remains open. |
| 14.3 Difficulty communication | Partial — client warning/marker implemented, live/visual acceptance open | Atlas shows static-spawn-record-weighted mean level; maps at least 15 mean levels above character get a contrasting outline and `!`. Map-load warning toast uses the same threshold and can be disabled in Game Settings. Neither blocks entry. Live timing, contrast, and readability acceptance remain. |
| 14.4 No hidden scaling        | Done         | By omission; campaign instances scale only through DM choice.                                         |

### §15-16 Accessibility, Audio, Presentation

| **Item**                                   | **Status**   | **Evidence / Notes**                                                                                                                                             |
|--------------------------------------------|--------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 15.1 UI/text scaling                       | Done         |                                                                                                                                                                  |
| 15.1 Remappable controls                   | Partial      | Versioned settings and a Game Settings capture list cover primary UI/navigation actions, 27 hotbar slots, normal movement, and debug-camera forward/back/left/right/up, four look directions, and hold-to-accelerate with conflict/reserved-key checks, reset, and RON import/export. Same-direction movement/look bindings may share keys because camera mode suppresses character movement. Fixed compatibility aliases and full keyboard-layout acceptance remain. |
| 15.1 Colourblind / high-contrast / reduced motion / combat-text controls | Partial | Menu/in-game theme selectors provide built-in High Contrast and Deuteranopia palettes; Deuteranopia changes world self/ally/monster health and cast bars, selected-target bracket, target-frame text, cursor/walk cue, local aim/in-range footprints, incoming cast telegraphs, and party-ping colors. Friend/party state uses blue/neutral/vermillion color tags plus explicit status words. Game Settings has persistent Reduce motion, Reduce flashing, quest-marker visibility, global quickcast/hold ground-skill modes with per-skill Aim + click/Quickcast/Hold + release/inherit overrides, Show combat text, all-vs-important/status-only frequency, and small/normal/large floating-text size controls. Hold mode commits only on release of its originating hotbar key and cancels without a valid ground target; global hold takes precedence over global quickcast. Reduced flashing dims procedural and ACT sprite-effect alpha plus associated/skill `.str` point lights to 55%, without shortening durations or removing sounds. Status-only suppresses damage, misses, and healing numbers while preserving textual status notices. Packet-level multi-hit damage appears as one amount/count label; identical per-hit labels from the same source/target/skill/critical state also merge across packets within 80 ms without summing damage. Remaining varying-packet/readability checks, particle/effect coverage, remaining effect colors, source-based density, and visual acceptance remain open. |
| 15.2 Effect density                        | Not started  |                                                                                                                                                                  |
| 16.1 Preserve visual identity              | Done         | Classic effect fidelity programme: `.str` recipes for wizard/persistent units, Hunter traps as real RSM props, status tints with desaturation, alpha-test fix.   |
| 16.2 Audio cues                            | Stock        | BGM/SFX play; no new cue design.                                                                                                                                 |
| 16.3 Combat text options                   | Partial      | Persistent on/off, all-vs-important/status-only filtering, and Small/Normal/Large size controls affect floating damage, miss, and healing numbers. Multi-hit packets use one amount/count label; matching per-hit labels from the same source/target/skill/critical state also merge across packets within 80 ms without summing damage. Varying packet patterns, source-based density, and visual acceptance remain open. |
| 16.4 Out-of-combat fading                  | Not started  |                                                                                                                                                                  |

### §17 Configuration and Server Administration

| **Item**                        | **Status**   | **Evidence / Notes**                                                                                                                                                                                                                                                    |
|---------------------------------|--------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 17.1 Server baseline settings   | Mostly stock | `area_size: 30` (draw distance) overridden; autoloot and free stat/skill reset commands granted to group 0. Both stock WoE includes are commented out; startup parse and confirmation that non-WoE guild scripts still load remain. Knowledge Mode, guidance, party range, and death penalty are otherwise unchanged. |
| 17.2 Player-level configuration | Partial      | WASD toggle, minimap toggle, auto-attack, scaling, graphics/audio. None of the guidance/overlay/density/loot-filter options.                                                                                                                                             |
| 17.3 GM / designer tools        | Done (DM-shaped) | `@dm` console (mode, spawn, warp, recall, hazard, beat, quest, flag, reward, check, encounter, instance, stakes, status, reset, dryrun), `@roll`, DM handbook, session board, Ctrl+O commands window, Ctrl+D dice, Bestiary Journal, Loot Generator, `dev.sh` DB snapshot/restore. **Missing the balance telemetry** the GDD names: spawn density, pathing failures, drop simulation, party-level distribution. |
| Developer logging (Phase 0)     | Done         | `KORANGAR_PACKET_LOG`, packet inspector, diagnostics collector, `Troubleshoot.bat` for white-screen reports, structured `[DMJ]` echo.                                                                                                                                    |

### §18 Balance and Playtesting

| **Item**                | **Status**   | **Evidence / Notes**                                                                                                                                       |
|-------------------------|--------------|------------------------------------------------------------------------------------------------------------------------------------------------------------|
| First playtest          | Done         | 2026-09-05. Findings and 13 sized tasks in `korangar/docs/plans/playtest-2026-09-05.md`; 11 closed within the day.                                         |
| 18.1 Measurement        | Not started  | No telemetry for any of the six areas; feedback is verbal.                                                                                                 |

## Built Outside the GDD

Work that consumed most of the effort to date and that this document did not anticipate. It should be acknowledged so the roadmap stops under-counting it.

| **Area**                       | **What exists**                                                                                                                                                                                                                                                 |
|--------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Custom client                  | Korangar fork: wgpu renderer, GL/WSL and macOS bring-up, animation engine rewrite, effect fidelity, headgear rendering (1,706 of 1,984 view IDs resolve), character creation, 50+ interface windows.                                                             |
| DM campaign layer              | Seal Cascade (19 arcs), `@dm` namespace, DnD mode, private instances per party, party-mirrored dialogue, skill checks, hazards/traps, encounter pricing, storyteller helpers, campaign quest journal, Bestiary Journal + Loot Generator, dice roller.               |
| Protocol additions             | `ZC_SKILL_FAIL_REASON` 0x0EFE, `ZC_PARTY_INVITE_SENDER` 0x0EFF, `CZ_CANCEL_CAST` 0x0F00, party SP broadcast, ammunition broadcast, party-roster job refresh.                                                                                                     |
| Distribution and operations    | One-command Windows (cross-compiled) and macOS packs, 47 MB client-only update zip, download verifier, launcher/troubleshooter, Tailscale reachability, `_create`-only registration, four security audits with remediations, hashed passwords, CI on both forks. |
| Test infrastructure            | 163 registered headless scenarios; the latest full ordered run passed **163 non-skipped scenarios with 1 expected skip and 0 flaky passes/failures/unexpected skips**. It reported 176 distinct incoming / 68 outgoing packets, zero unknown/fallback/deserialization failures and zero unmet skill expectations; inventory, reset-command behavior, party-quest credit, account discovery after all skill sweeps, navigation, 19-arc DM beats / 116 story beats, death recovery including S4 logout/relogin persistence, and observer cases passed. DM journal/replay and alternate-character SQL audits plus fixture cleanup were clean. Archive: `tools/testing/runs/20260925-042322.log`. Focused reset test also passed (`tools/testing/runs/20260925-031003.scoped`). `cargo test --workspace --quiet` passes (Korangar library: 413 passed, 17 ignored; other test groups also green). Production migration/startup, visual acceptance, and separate playtest/data gaps remain open. |
| Content fixes                  | Academy 2F rebuilt from the client's own GAT; teleport-destination audit; 1,156 maps loading.                                                                                                                                                                    |

## Conflicts — Decided 2026-09-21

The audit surfaced six places where a fork decision contradicted this document. All six were decided by the server owner on 2026-09-21. The affected sections have been amended in place; this table is the record.

| **#** | **Conflict**                                                        | **Decision**                                                                                                                                                                                                                                                                                   | **Follow-up**                                                                                                                                                                                       |
|-------|---------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| C1    | Adventure Guide rejected by the client roadmap (2026-07-05)         | **GDD wins, and the scope grows.** Rejection reversed. The Adventure Guide becomes a full **in-game encyclopedia** — everything a player would otherwise look up on iRO Wiki, generated from this server's own data and searchable in the client. See §9.5 (rewritten).                             | Client: build a separate player window over the versioned reference API; keep DM reveal/spawn controls out of its launcher. Add categories, details, cross-links, and source/incomplete-data notes.           |
| C2    | Vending / market search vs fork descope                             | **Fork wins — vending and market search are out of scope** for this server. §12.1 rewritten. Direct player trade is the loot-passing path and must be solid.                                                                                                                                    | Close the open trade items: drag-item grid, zeny field polish, last-second-change highlight, and the two-client live validation.                                                                    |
| C3    | Dodge roll planned in `modern-mechanics.md` §5                      | **GDD wins — no dodge roll.** §5.3 locked principle stands. Plan section struck.                                                                                                                                                                                                                | Done 2026-09-21: `korangar/docs/plans/modern-mechanics.md` §5 replaced with a rejection note.                                                                                                       |
| C4    | `@partyjump` is an unbounded party teleport                         | **Keep as-is for now.** Accepted as a §13.1 friends-first convenience on a private server; abuse potential noted for a public setting. May be bounded later.                                                                                                                                    | §9.8 amended with the exception. Revisit if the group grows or if it starts short-circuiting §9 travel.                                                                                             |
| C5    | Campaign is DM-gated; §8 describes a self-directed spine            | **Campaign is a separate mode.** "DM Session" is activated by the DM (`@dm mode on` / `@dm start`) and is not the §8 story spine. §8 stays as the design for a future self-directed spine; Seal Cascade is not counted against it.                                                              | §2.3 scope amended. Appendix D Q13 answered. Phase 6 rows re-labelled "DM Session mode".                                                                                                            |
| C6    | `@autoloot` gated to group 1                                        | **Grant to players.** `autoloot`, `alootid`, `autoloottype` moved to group 0 alongside `partyjump`.                                                                                                                                                                                             | Done 2026-09-21 in `Hercules/conf/groups.conf`. The §11.3 filter model remains separate future work.                                                                                                |
| C7    | Late-joining campaign party members                                | A character joining the active DM Session party automatically inherits its current campaign quests and story flags, without receiving past rewards. State remains per-character; same-account alternates inherit only after joining.                                                                 | 2026-09-23: recorded in the DM party quest-sync contract. 2026-09-24 live scenarios verify offline replay, late join, and same-account alternate isolation until party enrollment; reward and run-recreation acceptance remain open. |

### Owner decisions — 2026-09-22

- **Account-wide general unlocks; character-specific story:** Persistent non-story discoveries and general access carry across an account's characters. Every character must complete story chapters, choices, and story-gated access independently, including in DM Session mode. Build, inventory, and quest objective counts remain character state.
- **DM group quest sync (2026-09-23; implementation follow-up 2026-09-24):** A character joining the active campaign party automatically inherits its current campaign quests and story flags, without past rewards. A disconnected member catches up on return; another character on the same account inherits nothing until it joins. S10 journals typed events and replays them using character-scoped cursors; live tests verify reconnect, late join, and alternate isolation, including actor/returning cursor advancement. Run recreation, reward isolation, and failed-replay repair remain unaccepted.
- **Open, searchable knowledge:** The player Adventure Guide must let a fresh account research builds from verified server data. Discovery adds history and optional non-spoiler notes; it does not gate mechanics, drops, rates, or item/skill information. Campaign plot spoilers and DM controls stay separate.
- **Free experimentation:** Ordinary players can reset stats and skills separately, free at every level. The current command/permission/client mismatch in §7.5 and S2 must be fixed before this is marked delivered.

### Open playtest item recorded with the decisions

- **Dropping items from the inventory.** Reported by players as not possible. The code path exists (right-click an inventory slot → drop half / one / all, `CZ_ITEM_THROW2` 0x0363) and was live-verified on macOS 2026-07-10, so this is either a discoverability problem — right-click is the only path, the same issue character-delete has (M1-014) — or a Windows-pack regression. Needs one live check on the shipped pack; if it works, add a visible Drop affordance rather than documenting the right-click.


**End of Version 0.2**

*This document is intended to evolve through implementation and playtesting. Core identity principles should remain stable; tuning values and implementation details should change when evidence supports improvement.*
