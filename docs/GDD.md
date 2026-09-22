# Ragnarok Online: Friends Server Modernization

## Game Design Document

*Version 0.2 | September 2026*

> **DESIGN THESIS:** Preserve Ragnarok Online's identity and systemic depth, remove unnecessary friction, and add tactical clarity, smarter PvE, optional guidance, and modern usability without turning the game into a different MMO.

*Primary scope: cooperative PvE friends server. PvP and War of Emperium are intentionally out of scope for this version.*

# Document Control

| **Field**               | **Value**                                                                                                   |
|-------------------------|-------------------------------------------------------------------------------------------------------------|
| Document                | Ragnarok Online Friends Server Modernization - Game Design Document                                         |
| Version                 | 0.2 - v0.1 baseline plus implementation status audit (2026-09-21)                                            |
| Primary Audience        | Server owner, designers, scripters, client/UI developers, content builders, playtesters                     |
| Current Scope           | Cooperative PvE, story, exploration, tactical combat, UI/UX, progression, economy, social play              |
| Explicitly Out of Scope | PvP, War of Emperium, competitive ranking systems, cash-shop power                                          |
| Art Direction           | Preserve original Ragnarok art and world identity; new UI/effects should match the existing visual language |
| Design Status           | Design baseline with a per-section implementation audit. Numeric tuning remains subject to playtesting.     |
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

- Appendix E. Implementation Status (audited 2026-09-21)

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

The intended experience is "Ragnarok Online, fully realized." The original world structure, sprites, classes, equipment concepts, monster identity, cards, stats, cast-time model, autoattack builds, player vending, and open-ended progression are treated as assets rather than legacy problems. Modern systems are added only when they improve responsiveness, clarity, cooperation, navigation, accessibility, or tactical depth.

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
| Player vending, crafting, small-server economy | Battle pass systems                |

*Added in v0.2 (decision C5):* **DM Session mode** is a distinct, opt-in mode layered on the server. It is activated by the Dungeon Master (`@dm mode on`, `@dm start`) for one party, runs the Seal Cascade campaign through private instances, scripted hazards, skill checks and DM-chosen beat outcomes, and is switched off between sessions. Everything else in this document describes the server outside a session. DM Session content is not the §8 story spine and does not count toward it.

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

- Auction-house replacement that erases player vending from towns.

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
| Merchant         | Combat plus economic identity, crafting, vending, equipment support.                  |

## 7.4 Skill Tree UX

Skill trees remain mechanically recognizable but gain better presentation. Every skill should clearly display level scaling, prerequisites, SP cost, range, cast time, delay, element, area, status interactions, and other meaningful mechanics. A preview mode allows players to allocate hypothetical future skill points without committing them.

## 7.5 Stat and Skill Respec

For a friends server, accidental build mistakes should not require abandoning a character. Recommended default: generous early respecs, then an increasing in-game cost or quest/material requirement at higher progression. Respec should be accessible without real-money dependency. The goal is to preserve meaningful builds while eliminating documentation traps.

## 7.6 Integrated Build Planner

The character window should support a planning state. Players choose a target Base/Job level, allocate hypothetical stats and skills, and preview expected HP, SP, Hit, Flee, ASPD, cast-time changes, and other relevant derived values. Planned changes are never applied until explicitly committed through normal progression or a respec system.

# 8. Story and Quest Design

## 8.1 Story as a Spine

The centralized storyline should orient players through the world, introduce systems, and provide a coherent narrative for players who want one. It should not become a mandatory corridor through all content. A player may pause the story indefinitely and continue leveling, exploring, farming, crafting, or helping friends.

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

## 8.6 Story-Based System Teaching

| **System**      | **Example Teaching Moment**                                                                                              |
|-----------------|--------------------------------------------------------------------------------------------------------------------------|
| Elements        | A quest sends the player into an area where an elemental weapon or spell advantage is obvious.                           |
| Cards           | A character explains a monster-specific card after the player encounters their first rare card-related clue.             |
| Refinement      | A smith-related story step introduces risk, ores, and safe ranges using a real item.                                     |
| Party Play      | A dungeon story objective encourages a small group and highlights party frames / shared destination tools.               |
| Monster Journal | A researcher asks the player to observe a family of monsters, teaching discovery without forcing completionist behavior. |

# 9. World, Exploration, and Navigation

## 9.1 Navigation Philosophy

> **LOCKED PRINCIPLE:** Remove confusion, not curiosity. Players may get lost because they chose to explore; they should not remain lost because the game refuses to explain how maps connect.

## 9.2 World Map

The world map should show major regions, towns, known dungeons, transportation routes, and discovered connections. Selecting a destination displays useful information such as suggested level, known monsters, travel route, services, and whether the player has visited the location. The map never acts as unrestricted instant teleportation.

## 9.3 Map Information Panel

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
| Monsters            | Stats, level, HP, element/race/size, modes, skills used, spawn maps and density, drops with rates, card, MVP status.  | `db/re/mob_db.conf`, `mob_skill_db.conf`, `npc/**/mobs/*.txt` spawn lines                             | `bestiary.json` (1,759), `mob_lore.json` (540 flavour)   |
| Items               | Every item: stats, script effects in plain language, weight, slots, refinable, who can equip, buy/sell, where it drops. | `db/re/item_db.conf`, `item_combo_db.conf`, `item_group.conf`                                         | `items.json` (13,182, with `DropsFrom`)                  |
| Cards               | Effect, slot type, source monster, compounding notes, set/combo bonuses.                                             | `item_db.conf` (card type), `item_combo_db.conf`                                                      | `cards.json` (1,012)                                     |
| Skills              | Per level: SP, cast time, delay, cooldown, range, area, element, damage formula, status inflicted, prerequisites.     | `db/re/skill_db.conf`, `skill_tree.conf`, `sc_config.conf`                                            | `skills.json` (1,170 — names/descriptions only so far)   |
| Jobs / classes      | Job tree, change requirements, stat bonuses per job level, skill trees, base/job EXP tables.                          | `job_db.conf`, `skill_tree.conf`, `exp_group_db.conf`, `statpoint.txt`, jobmaster NPC                 | —                                                        |
| Status effects      | What each buff/debuff does, duration, what cures it, what causes it, icon.                                           | `sc_config.conf`, `status.c` tables                                                                   | `status_effects.json` (700)                              |
| Maps                | Name, region, suggested level, connections (warp graph), monster population, NPCs and services, Kafra, map flags.    | `db/re/map_zone_db.conf`, `npc/**/warps/*.txt`, `npc/**/mobs/*.txt`, `Towninfo`                        | minimap POIs only                                        |
| Quests              | Name, giver, steps, rewards, prerequisites, level; custom and campaign quests included.                              | `db/re/quest_db.conf`, `npc/re/quests/**`, `npc/custom/quests/**`, campaign quest IDs 20000-20234    | quest log window only                                    |
| NPCs                | Name, map and coordinates, function (shop, quest, service), shop inventory.                                          | `npc/**/*.txt` headers, shop lines                                                                    | `Towninfo` POIs                                          |
| Mechanics           | Stat formulas (HIT, FLEE, ASPD, cast time), elemental table, size/race modifiers, refinement odds, EXP penalty, party share. | `attr_fix.conf`, `size_fix.txt`, `refine_db.conf`, `level_penalty.conf`, `battle/*.conf` values     | —                                                        |
| Server rules        | This server's rates, party level range, death penalty, autoloot, respec costs, DM Session mode explanation.          | `conf/map/battle/*.conf`, `groups.conf`, this document                                                | —                                                        |

### Behaviour

- **One search box, categorised results.** Searching "Hydra" returns the monster, its card, the maps it spawns on, quests that mention it, and a Navigate action (§9.9). Searching "Oridecon" returns the item, every monster that drops it with rates, and the refinement mechanics page.

- **Everything links.** Item → source monster → spawn map → route. Skill → status inflicted → what cures it. Job → skill tree → each skill's page. No result is a dead end.

- **Correct for this server.** Rates, drops, and quest steps reflect the fork's tables, including custom NPCs (warper, healer, jobmaster, stylist) and any future rebalancing. When a value is a client-side estimate rather than a server table (§10.10), it is labelled as such.

- **Knowledge mode (§9.6) gates detail, not existence.** In Hybrid mode every entry exists and is findable; exact drop rates, MVP mechanics, and campaign monster lore reveal through play. In Open Database mode everything is visible. The server owner sets the mode.

- **Flavour text is attributed and rewritten.** Wiki prose (the fandom extracts in `mob_lore.json` are CC-BY-SA) is reference material for the writer, not player-facing copy. Player-facing lore is authored in the server's voice with attribution kept in the data file.

### Build order

1. **Generator.** Extend the existing `tools/` JSON exports into one build step that emits every category above from the Hercules tree, so the encyclopedia is rebuilt whenever `db/` or `npc/` changes. Skills and maps are the largest gaps.

2. **Player journal.** Promote the DM Bestiary window into a player-visible Adventure Guide with a search field over monsters, items and cards — the three categories whose data already exists. This is the first shippable slice.

3. **Remaining categories** in order of playtest demand: skills and jobs (build questions), maps and quests (navigation, §8.4), status effects, mechanics, server rules.

4. **Linking and Navigate**, once the map graph from Phase 1 exists.

## 9.6 Discovery Modes

| **Mode**           | **Behavior**                                                                          | **Recommended Use**                                                              |
|--------------------|---------------------------------------------------------------------------------------|----------------------------------------------------------------------------------|
| Discovery Mode     | Monster details, drops, and some locations reveal through encounters and exploration. | Best when discovery is part of the group's desired experience.                   |
| Open Database Mode | Most reference data is searchable immediately.                                        | Best for veteran groups who already use external databases and want convenience. |
| Hybrid             | Basic location and identity visible; exact drops/mechanics reveal through play.       | Recommended default.                                                             |

## 9.7 Rumors

NPC dialogue, signs, books, quests, and exploration can add Rumors to the Adventure Guide. A rumor is information, not an obligation. Examples: "Adventurers have seen unusual creatures beneath the sea near Izlude" or "Merchants near Morroc are paying well for Ant Jaws." A rumor may add a map note without creating a kill-count quest.

## 9.8 Travel

- Kafra, Warp Portal, boats, airships, and other in-world transportation remain meaningful.

- Discovered services are visible on the world map with their destinations.

- No unrestricted click-to-teleport world map by default.

- *Exception (decision C4, 2026-09-21):* `@partyjump <name>` lets any party member warp to any online party member. It is accepted as a friends-first convenience on this private server and is deliberately unbounded for now; it would need a cooldown or same-map rule before any public use.

- Fast travel may be expanded later for repeated routes, but should preserve class and world-travel identity.

## 9.9 Portal Labels

Hovering or approaching a known exit should identify its destination. If it lies on the tracked route, it receives a subtle route indicator. Unknown portals may remain less descriptive to preserve discovery.

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

Useful sinks may include reasonable respec costs, refinement, crafting fees, transport, cosmetic services, and convenience services. Avoid punitive sinks that exist only to slow friends from playing together.

# 13. Party and Friends-Server Social Systems

## 13.1 Friends-First Principle

> **LOCKED PRINCIPLE:** The server should rarely create a mechanical reason for friends to avoid playing together.

## 13.2 Shared Destinations

A party leader or member can propose a destination such as Orc Dungeon, Payon Cave 3F, or a personal marker. Party members accept the route and receive the same breadcrumb guidance. The system coordinates travel without teleporting the party.

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

# 14. Death, Recovery, and Difficulty

## 14.1 Death Philosophy

Death should create tension and encourage better play without turning technical failure or experimentation into lost evenings. Danger matters; excessive time deletion does not.

## 14.2 Recommended Recovery Model

Use a modest experience penalty with a recoverable component. A player who returns to the dangerous area, defeats enemies, reaches a recovery point, or completes a short recovery condition can regain part of the loss. Exact percentages are playtest values.

## 14.3 Difficulty Communication

Maps and monsters may communicate a suggested level or danger rating. These are warnings, not locks. A lower-level player should still be allowed to enter a dangerous area and discover why the warning existed.

## 14.4 Adaptive Group Pressure

Avoid aggressive hidden scaling that makes character progression feel meaningless. If encounter scaling is ever used, prefer modest party-size adjustments on selected story instances rather than global world scaling. Open-world monsters should largely retain stable identity and power.

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

# 16. Audio, Visual Feedback, and Presentation

## 16.1 Preserve Visual Identity

New effects should be readable at sprite scale and should not overwhelm the original artwork. Use animation poses, compact particles, tile accents, shadows, dust, cracks, status icons, and sound cues before resorting to large translucent floor geometry.

## 16.2 Audio Cues

Audio can communicate state without adding screen clutter. Distinct cues are useful for a card drop, a dangerous boss wind-up, a successful interrupt, an important status effect, a party ping, or a completed objective. Repetitive farming sounds should remain pleasant enough for long sessions.

## 16.3 Combat Text

Damage numbers should emphasize criticals, weaknesses, immunities, and important outcomes without turning every hit into a paragraph. Players may reduce or disable floating combat text and rely on the combat log instead.

## 16.4 Out-of-Combat Calm

When the player is exploring a town or field without combat pressure, combat-only HUD elements should be able to fade. Ragnarok's world art should remain the visual focus rather than permanent instrumentation.

# 17. Configuration and Server Administration

## 17.1 Server-Level Configuration

| **Setting**             | **Recommended Baseline**                | **Notes**                                                                    |
|-------------------------|-----------------------------------------|------------------------------------------------------------------------------|
| Knowledge Mode          | Hybrid                                  | Basic monster/map info available; deeper details discovered or configurable. |
| Quest Guidance          | Available, opt-in per tracked quest     | Players can disable globally.                                                |
| Exact Spawn Coordinates | Off                                     | Use population regions instead.                                              |
| Auto-Loot               | Configurable categories                 | Respect inventory and weight.                                                |
| Respec                  | Generous early; in-game cost later      | Avoid real-money dependency.                                                 |
| Party Level Range       | Wider than classic or diminishing model | Tune for friend accessibility.                                               |
| Death Penalty           | Modest and partly recoverable           | Tune through playtests.                                                      |
| PvP / WoE               | Disabled / not part of v0.1 design      | Deferred.                                                                    |

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
| Economy    | Item availability, Zeny generation/sinks, vending prices, crafting material bottlenecks.     |
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

## Phase 1 - Foundational UX — **partial**

- **[open]** World map and map connection data.

- **[partial]** Quest tracking and breadcrumb routing. *Quest log window exists (Ctrl+Q); no tracker HUD, no routing.*

- **[open]** Basic Adventure Guide search. *Confirmed and widened 2026-09-21 (C1): full in-game encyclopedia, §9.5. Seed is the DM Bestiary + in-tree JSON.*

- **[partial]** Inventory/storage search and item protections. *Weight display done; search, sort, filters, locks not started.*

- **[done]** Improved target/player frames and UI scaling. *Basic frames; scaling done. Status icons and target detail still open.*

- **[partial]** Party member map location and shared waypoint support. *Party minimap blips with hover names, roster, world HP bars, `@partyjump` done. Waypoints not started.*

## Phase 2 - Combat Responsiveness — **partial**

- **[partial]** Single-action input buffering. *Walk-into-range chaining only; no timed buffer.*

- **[open]** Target switching and selection improvements.

- **[unseen]** Ground-target previews. *Real Hercules cell layouts, out-of-range tint.*

- **[done]** Improved cast/action indicators. *Overhead cast bars; fork-only cast cancel; skill-fail reasons.*

- **[done]** Animation and hit-feedback synchronization. *Animation engine phases A-D closed.*

## Phase 3 - Tactical Monster Layer — **not started**

- **[open]** Implement reusable behavior archetypes.

- **[open]** Pilot on one early, one midgame, and one late-game region.

- **[open]** Add restrained dangerous-attack telegraphs.

- **[open]** Test reactions to ground control and party roles.

- **[open]** Iterate before broad rollout to all monsters.

*Nearest existing asset: the DM hazard/encounter scripts in `Hercules/npc/custom/dm_campaign/shared/` prove the server-side primitives (`DM_HazardArea`, mode-bit stripping, `setcell`) that an AI layer would also use.*

## Phase 4 - Journals and Knowledge Systems — **partial (DM-only) — scope widened by C1 to the §9.5 encyclopedia**

- **[partial]** Monster Journal with locations, traits, and drops. *Bestiary Journal exists as a DM window with per-account unlock persistence; not a player-facing journal.*

- **[partial]** Item-to-monster-to-map linking. *Data present in `docs/*.json`; only the DM windows read it.*

- **[open]** Population overlays.

- **[open]** Rumors and discovery states.

- **[open]** Build planner and advanced combat explanations.

## Phase 5 - Economy and Profession Improvements — **not started / partly descoped**

- **[descoped]** Market/vending search. *Out of scope for this server (C2). Direct trade polish takes its slot.*

- **[open]** Crafting request / commission flow. *DM Loot Generator and `@dmreward` fill this role today.*

- **[partial]** Refinement clarity improvements. *Refine window exists; chance/consequence display not audited.*

- **[open]** Small-population drop-rate and Zeny balancing. *All rates stock.*

## Phase 6 - Boss and Story Encounter Pass — **partial (DM Session mode only)**

- **[open]** Upgrade selected MVPs with signature mechanics. *Open-world MVPs are stock; DnD mode suppresses them during sessions.*

- **[partial]** Build story bosses using the tactical rules already taught by normal monsters. *Campaign bosses have adds, beat variants and pulse hazards, but no normal-monster layer teaches those rules first.*

- **[open]** Add optional post-encounter analysis for major fights. *Spec'd as "End-of-Encounter Recap" in the client roadmap.*

## Phase 7 - Polish and Expansion — **not started**

- **[open]** Accessibility pass.

- **[open]** Effect density and performance tuning.

- **[open]** UI profile polish.

- **[open]** Expanded monster behavior library.

- **[open]** Content pacing and economy rebalance after real player data.

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

- Replacing travel with unrestricted teleports.

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

| **Feature**                | **Baseline**                                      | **Type**              |
|----------------------------|---------------------------------------------------|-----------------------|
| PvP / WoE                  | Disabled / deferred                               | Locked for v0.1 scope |
| Main Story Guidance        | Available when tracked                            | Player configurable   |
| World Breadcrumbs          | Next-exit guidance                                | Player configurable   |
| Monster Population Overlay | Broad regions only                                | Player configurable   |
| Knowledge Mode             | Hybrid                                            | Server configurable   |
| Input Buffer               | ~200 ms starting test                             | Playtest value        |
| Action Queue               | 1 action                                          | Recommended default   |
| Universal Dodge            | None                                              | Locked principle      |
| Auto-Loot                  | Category filters                                  | Player configurable   |
| Party Level Rules          | Friend-friendly widened range / diminishing model | Playtest decision     |
| Death Penalty              | Modest + partial recovery                         | Playtest decision     |
| Respec                     | Free/cheap early, in-game cost later              | Recommended default   |
| Fast Travel                | In-world systems; no unrestricted map teleport    | Locked principle      |
| Exact Spawn Coordinates    | Hidden                                            | Recommended default   |
| HUD Presets                | Classic + Modern + custom                         | Recommended default   |

# Appendix D. Open Design Questions

1. Which exact server/client codebase and tooling constrain the planned UI and AI changes?

2. How far should keyboard movement go while preserving tile/pathing parity with click-to-move?

3. Should monster journal data be character-specific, account-wide, or server-wide?

4. What percentage of item/drop information should be available immediately in Hybrid knowledge mode? *v0.2: sharpened by §9.5 — every entry exists in Hybrid; the question is which fields reveal through play.*

5. Should friends have an optional level-sync system, or is a widened party XP model sufficient?

6. How destructive should high-level refinement remain on a small server?

7. Which three maps should be used as the first tactical-AI pilot areas?

8. Which MVP should be the first redesigned boss used to prove the tactical-combat framework?

9. Should party pings and shared destinations persist across map transitions or expire quickly?

10. How much advanced formula information can the client display accurately from the server implementation?

11. Which original UI windows can be extended safely versus requiring replacement or overlay panels?

12. What level of map/monster data can be generated automatically from server tables versus authored manually? *v0.2 note: `bestiary.json`, `items.json`, `cards.json`, `skills.json`, `status_effects.json` are already generated in-tree from Hercules DBs; the open part is authoring population regions and rumors.*

*Answered or narrowed since v0.1:*

- Q1 — **Answered.** Korangar (Rust, wgpu) client fork and Hercules server fork at `PACKETVER=20220406`. UI is fully owned; custom packets are cheap but must be versioned on both ends.

- Q2 — **Answered.** WASD ships as an optional mode over the same pathfinder with a 200 ms throttle; click-to-move retained. No further keyboard-movement scope is planned.

- Q3 — **Narrowed.** The DM Bestiary persists unlocks **per account** (`#bestiary_unlock_<id>`). Whether a player-facing journal should be account- or server-wide is still open.

*New questions raised by the audit:*

13. ~~Is the Seal Cascade DM campaign the §8 story spine, or a separate "DM session" mode?~~ **Answered 2026-09-21: a separate DM Session mode, activated by the DM.** See §2.3 and Appendix E, C5.

14. ~~Should the Adventure Guide rejection (2026-07-05) stand?~~ **Answered 2026-09-21: no — the Adventure Guide becomes a full in-game encyclopedia (§9.5).** See Appendix E, C1.

15. Which of the fork's protocol additions (cast cancel, skill-fail reasons, party invite sender, party SP) should be promoted into the GDD as designed features rather than incidental fixes?

16. ~~What bounds `@partyjump`?~~ **Answered 2026-09-21: nothing, for now.** Accepted as a private-server convenience; revisit later. See Appendix E, C4.

17. Which three playtest-1 gaps get priority for playtest 2: inventory search, a HUD quest tracker, or party pings on the existing `[DMJ]` transport?

## Recommended Next Design Deliverables

- Tactical Combat Specification: input states, targeting rules, action buffer, movement, cast states, telegraph taxonomy, AI interfaces.

- UI Wireframe Pack: HUD presets, target/party frames, Adventure Guide, world map, monster journal, inventory, build planner.

- Navigation Data Specification: map graph, portal metadata, breadcrumbs, POIs, monster population regions, discovery flags.

- Monster AI Behavior Library: reusable states, priorities, pathing responses, and authoring parameters.

- Pilot Region Design: one complete region implemented end-to-end to validate combat, UI, quests, and navigation together.

- Implementation Status Refresh: re-run the Appendix E audit after each playtest and move rows, rather than re-auditing from scratch.

# Appendix E. Implementation Status (audited 2026-09-21)

This section was added in v0.2 after auditing the two forks that implement this design — the **Korangar** Rust client (`korangar/`) and the **Hercules** server (`Hercules/`) — against the code, the commit history, and the first friends playtest (2026-09-05, four players, three remote). Status reflects what exists in the tree, not what the client's own roadmap checkboxes say (those were found to be stale).

## Status Language

| **Label**        | **Meaning**                                                                                              |
|------------------|----------------------------------------------------------------------------------------------------------|
| Done             | Implemented and seen working live (GUI or playtest).                                                     |
| Done (unseen)    | Implemented and test-covered, but never observed on screen. Treat as done pending one live look.         |
| Partial          | A real slice exists; the GDD's full intent does not.                                                     |
| Stock            | Behaviour is unmodified upstream Hercules/RO. Nothing done, nothing broken.                              |
| Not started      | No code, or only a spec/plan document.                                                                   |
| Decided (Cn)     | Was a conflict between a fork decision and this GDD; resolved 2026-09-21 — see the Conflicts table below.  |

## Headline

- **The foundation is real.** A custom Rust client connects to a `PACKETVER=20220406` Hercules fork, four friends have played a session over Tailscale from shipped Windows/macOS packs, and a 149-scenario headless regression suite gates the protocol. Phase 0 and most of the client half of Phase 1 exist.

- **The largest body of finished work is not in this GDD's original scope.** The forks were built around a **DM-run tabletop campaign** ("Seal Cascade": 19 arcs in 4 acts, ~5,600 lines of shared DM script, `@dm` console, d20 skill checks, private party instances, scripted hazards, dice cards, bestiary journal, loot generator). Decided 2026-09-21: this is **DM Session mode**, a separate opt-in mode (§2.3), not the §8 story spine.

- **Nothing in §6 (Monster AI) has been started.** `mob.c` is upstream. Tactical encounter behaviour exists only as DM-scripted hazards inside campaign instances.

- **Navigation (§9) and knowledge systems (§9.5, §10.13, §11.5) are the widest gap.** The client roadmap's 2026-07-05 rejection of an in-client database was reversed on 2026-09-21 and the scope widened: a full in-game encyclopedia generated from the server's own tables (§9.5).

- **Combat responsiveness (§5) is further along than expected**: ground-target footprints with real Hercules layouts, walk-into-range for every targeting mode, overhead cast bars, hit/animation synchronisation, and a fork-only cast cancel packet all exist. The single-action input buffer of §5.2 does not.

## Status by Section

### §5 Tactical Combat Framework

| **Item**                                   | **Status**       | **Evidence / Notes**                                                                                                                                                                                                                  |
|--------------------------------------------|------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 5.2 Input buffer (150-250 ms, one action)  | Partial          | `BufferedAction` in `korangar/lib.rs` chains *walk-into-range → act* for attack, pickup, entity cast and ground cast. It is not a timed buffer that absorbs a press during animation lock. Immediate client acknowledgement is not implemented. |
| 5.3 Click-to-move                          | Done             | Stock Korangar, live-verified.                                                                                                                                                                                                        |
| 5.3 Keyboard movement                      | Done             | WASD shipped 2026-09-05: optional (Game Settings, default on), camera-relative, same pathfinder, 200 ms packet throttle, disabled while text fields have focus. Meets the "no unique collision / no speed advantage" rule by construction. |
| 5.3 Hold-mouse continuous path             | Not verified     | Not audited.                                                                                                                                                                                                                          |
| 5.3 No universal dodge roll (LOCKED)       | Decided (C3)     | Plan section in `korangar/docs/plans/modern-mechanics.md` §5 struck 2026-09-21. Principle stands.                                                                                                                                       |
| 5.4 Left-click select, target frame        | Done             | `player_target.rs` window. Shows name/HP; race/size/element and cast state not shown.                                                                                                                                                 |
| 5.4 Target cycling / nearest hostile       | Not started      |                                                                                                                                                                                                                                       |
| 5.4 Ground-target preview of reachable area| Done (unseen)    | `world/skill_layout.rs` + `Map::render_skill_footprint` (2026-07-26): real cell shapes from `skill_db` layouts, the 15 hardcoded Hercules unit layouts, direction-dependent walls (Fire Wall, Ice Wall …), red tint when out of range. Never seen on screen. |
| 5.4 Alternate cast styles                  | Not started      | Classic press-then-click only.                                                                                                                                                                                                        |
| 5.5-5.7 Positioning / ground control / KB  | Stock            | Server mechanics untouched, as intended.                                                                                                                                                                                              |
| 5.8 Telegraphs                             | Not started      | No client decals/floor markers. Server-side scripted hazards exist only in campaign instances (`DM_HazardArea`). Decal pass is listed under the client's aspirational graphics program.                                                |
| 5.9 Immediate layer (damage numbers)       | Done             | Damage/heal numbers, crit attack animation. No "element advantage" cue.                                                                                                                                                               |
| 5.9 Contextual / advanced layers           | Not started      | No monster race/size/element tooltip, no damage breakdown.                                                                                                                                                                            |
| 5.10 Autoattack feel                       | Partial          | Animation engine phases A-D closed; hit/attack sync; ReadyFight stance; ranged attacks draw the real ammunition sprite. Proc/status feedback: opt1/opt2 tints, looping status STRs, freeze on stun/sleep.                             |
| *Fork addition — cast cancel*              | Done (unseen)    | `CZ_CANCEL_CAST` (0x0F00) + Hercules delta; right-click/Escape aborts own cast. Not an RO feature; deliberate. Movement still never cancels; casting still roots.                                                                     |
| *Fork addition — skill-fail reasons*       | Done             | `ZC_SKILL_FAIL_REASON` (0x0EFE) names why a skill actually failed; missing-item failures name the item. Directly serves the "players can explain outcomes" success criterion.                                                          |

### §6 Monster AI and Encounter Design

| **Item**                            | **Status**   | **Evidence / Notes**                                                                                                                                                                                                                              |
|-------------------------------------|--------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 6.2 Behaviour archetypes            | Not started  | No `mob.c` / AI changes in the fork.                                                                                                                                                                                                              |
| 6.3-6.5 Family behaviour, ground reaction, threat | Stock |                                                                                                                                                                                                                                             |
| 6.7 Elites                          | Not started  |                                                                                                                                                                                                                                                   |
| 6.8-6.9 MVPs / boss template        | Partial (campaign only) | Campaign bosses in private instances have adds, phases via `@dmbeat` variants (Dark Lord, Randgris, Beelzebub, Thanatos, Bijou/Maret), and pulse hazards (Ifrit heat, Rift Anchor, Thanatos resonance, Ash Vacuum). DnD mode suppresses stock MVP spawns. Open-world MVPs are stock. |
| *Server-side encounter tooling*     | Done         | `dm_encounters.txt` prices encounters on real HP/DPS (2026-08-18); `@dm hazard`, `@dm spawn`, `@dm stakes`; stealth via `MD_DETECTOR` mode stripping; `dm_traps.txt` helpers. This is the DM's hand doing what §6 wants the AI to do.       |

### §7 Classes, Stats, Skills, and Build Identity

| **Item**                          | **Status**   | **Evidence / Notes**                                                                                                                                          |
|-----------------------------------|--------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 7.1 Original stat model           | Done         | Stock. Character creation now has a 48-point allocator with a suggested first-job spread and a live 8-facing preview (2026-09-03/04).                          |
| 7.1 Derived-stat preview          | Not started  | Stats window allocates points; no next-point preview.                                                                                                         |
| 7.4 Skill tree presentation       | Partial      | Tabs + drag-to-hotbar exist. No per-skill cast time / SP / range / delay / element display; no preview allocation.                                             |
| 7.5 Respec                        | Stock custom | `npc/custom/resetnpc.txt`: 5,000z stats, 5,000z skills, 9,000z both, flat at every level. Not the "generous early, costlier later" curve.                     |
| 7.6 Build planner                 | Not started  | Rejected-adjacent: the client roadmap deferred build planning to "skill tree search/filter later".                                                            |

### §8 Story and Quest Design

| **Item**                          | **Status**   | **Evidence / Notes**                                                                                                                                                                                                                 |
|-----------------------------------|--------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 8.1 Story spine                   | Not started (self-directed) | Seal Cascade (4 acts, 19 arcs, quest IDs 20000-20234) exists but is **DM Session mode** (C5), gated by `DM_SessionAllows` at 50 sites. No self-directed spine exists.                                                      |
| 8.2 Hunting goals / rumor leads   | Not started  |                                                                                                                                                                                                                                      |
| 8.3 Quest markers                 | Partial      | Standard quest-effect markers on campaign hub and objective NPCs. No player toggle.                                                                                                                                                  |
| 8.4 Breadcrumb guidance           | Not started  | Spec exists (`docs/specs/navigation-quest-guiding.md`). `<NAVI>` tags in NPC dialogue are stripped, not clickable.                                                                                                                   |
| 8.5 Kill counts only when meaningful | Done      | Hunting contracts fill from **drops** instead of kill counts (2026-08-25); the quest log shows what each contract still wants.                                                                                                       |
| 8.5 Shared party progress         | Done (campaign) | Campaign quest starts, completions, erasures and flags apply to every online party member; quest-giver dialogue is mirrored to the party as `[Party Story]`. Stock quests remain per-character.                                    |
| 8.6 Story-based system teaching   | Not started  |                                                                                                                                                                                                                                      |
| *Fork addition — skill checks*    | Done         | `@check`: d20 + stat/15 + proficiency + assists vs DC, with server-side consequences (hazard saves, `setcell`, mode bits). Not in the GDD; a candidate for §8.6-style "optional guidance."                                          |

### §9 World, Exploration, and Navigation

| **Item**                          | **Status**   | **Evidence / Notes**                                                                                                                                                                    |
|-----------------------------------|--------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 9.2 World map                     | Not started  |                                                                                                                                                                                         |
| 9.3 Map information panel         | Not started  |                                                                                                                                                                                         |
| 9.4 Monster population regions    | Not started  | Data exists (`docs/bestiary.json` carries spawn maps) but nothing renders it.                                                                                                           |
| 9.5 In-game encyclopedia          | Decided (C1) — Partial data, no UI | Rejection of 2026-07-05 reversed and scope widened to full wiki coverage. Data exists for monsters/items/cards/status; skills, jobs, maps, quests, NPCs, mechanics need a generator. No player UI. |
| 9.6 Discovery modes               | Partial      | Bestiary Journal (DM window) has unlock persistence via `#bestiary_unlock_<id>` account variables — a Discovery Mode for monster lore. No server-level Knowledge Mode switch.           |
| 9.7 Rumors                        | Not started  |                                                                                                                                                                                         |
| 9.8 Travel                        | Stock + one addition | Kafra, warper NPC. `@partyjump <name>` (2026-09-05) lets any party member warp to another online member — accepted unbounded for now (C4).                              |
| 9.9 Portal labels                 | Not started  |                                                                                                                                                                                         |
| Minimap (base)                    | Done         | Ctrl+Tab; Towninfo POIs (shops, Kafra, guides), player blip, party blips with hover names, compass packet.                                                                              |

### §10 User Interface and User Experience

| **Item**                              | **Status**   | **Evidence / Notes**                                                                                                                                                          |
|---------------------------------------|--------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 10.2 HUD presets / Edit Mode          | Not started  | Spec: `docs/specs/hud-edit-mode.md`. Windows are movable/resizable and layout persists, but there is no edit mode, presets, or combat fading.                                 |
| 10.3 Player frame                     | Partial      | HP/SP bars, Base/Job EXP + zeny HUD (Alt+Shift+H), overweight colouring. Status bar is a **text** list with timers; real SC icons not implemented.                             |
| 10.4 Target frame                     | Partial      | Name + HP. No statuses, element/race/size, cast state, or boss variant.                                                                                                       |
| 10.5 Cast bars                        | Done         | Overhead cast bars on entities (self and others).                                                                                                                             |
| 10.5 Ground footprints / range preview| Done (unseen)| See §5.4.                                                                                                                                                                     |
| 10.5 Target-of-target                 | Not started  |                                                                                                                                                                               |
| 10.6 Hotbars                          | Done         | Three rows of nine (1-9, Ctrl+1-9, Alt+1-9; F-keys still work); items/potions on the hotbar (2026-09-05). Keybind remap screen: not started.                                 |
| 10.7 Equipment sets                   | Not started  |                                                                                                                                                                               |
| 10.8 Stats interface modes            | Not started  |                                                                                                                                                                               |
| 10.9 Inventory search/filter/sort/lock| Not started  | `inventory.rs` is a bare grid with drag-and-drop. Weight display and thresholds: done.                                                                                        |
| 10.9 Storage                          | Partial      | Kafra storage open/store/retrieve works; no search/filters.                                                                                                                   |
| 10.10 Equipment comparison            | Not started  |                                                                                                                                                                               |
| 10.11 Quest tracker                   | Partial      | Quest log window (Ctrl+Q, shipped 2026-08-25, visible only since 2026-09-05 — it had been bound to close-window too). No on-HUD tracker, no per-quest tracking toggle.         |
| 10.12 Minimap layers / waypoints / pings | Not started | Base minimap done (see §9).                                                                                                                                                  |
| 10.13 Global search                   | Decided (C1) — Not started | See §9.5.                                                                                                                                                       |
| 10.14 Party UI                        | Done         | Roster window (Alt+Z / Alt+P) with HP + SP bars (server now sends party SP), class (job-refresh bug fixed server-side 2026-09-05), "Go to" button, minimap blips, world ally HP bars. Click-to-target and distance/out-of-map state: not verified. |
| 10.15 Chat                            | Partial      | Public / Party / Whisper channels, `/r`-style commands. No timestamps, no item links (`<ITEM>` stripped), no copy/paste selection, no Loot/System tabs.                        |
| 10.16 UI scaling                      | Done         | Interface Settings > Scaling (Ctrl+I); character-creation screen exposes it directly (playtest T6).                                                                            |
| Character creation / select           | Done         | Sex, hair, stat allocator, 8-facing preview, headgear on select cards. Delete still lacks a confirmation guard (M1-014).                                                       |

### §11 Loot, Equipment, Cards, and Refinement

| **Item**                        | **Status**   | **Evidence / Notes**                                                                                                                                                      |
|---------------------------------|--------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 11.2 Configurable nearby pickup | Partial (C6 applied) | Hercules `@autoloot <percent>` (a drop-rate threshold), `@alootid`, `@autoloottype`, plus a Commands-window button. Granted to group 0 on 2026-09-21. Not yet the §11.3 category/wishlist filter. |
| 11.3 Loot filters               | Not started  |                                                                                                                                                                           |
| 11.4 Rare-drop presentation     | Not started  |                                                                                                                                                                           |
| 11.5 Cards ↔ monster linking    | Partial      | `docs/cards.json` / `bestiary.json` link cards and monsters; only the DM Bestiary window consumes them. Becomes an encyclopedia category (§9.5).                          |
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
| 13.2 Shared destinations          | Not started    |                                                                                                                                 |
| 13.3 Party pings                  | Not started    | The intended transport (`[DMJ]` structured echo, Phase A) exists and carries DM state; ping/ready-check/markers not built on it. |
| 13.4 Hunting goals list           | Not started    |                                                                                                                                 |
| 13.5 Level difference handling    | Stock          | `party.conf` untouched; no level-sync.                                                                                          |
| 13.6 Shared quest progress        | Done (campaign)| See §8.5.                                                                                                                       |
| Party warp                        | Done           | `@partyjump`, non-GM.                                                                                                           |
| Party loot                        | Decided        | Stays random (`party_item_share_type: 0`), operator decision 2026-09-05.                                                        |

### §14 Death, Recovery, and Difficulty

| **Item**                      | **Status**   | **Evidence / Notes**                                                                                   |
|-------------------------------|--------------|--------------------------------------------------------------------------------------------------------|
| 14.2 Recoverable death penalty| Stock        | `death_penalty_type: 1`, 1% base / 1% job. No recovery mechanic.                                       |
| 14.3 Difficulty communication | Not started  | Server-side only: `dm_encounters.txt` tiers encounters on real HP/DPS for the DM.                       |
| 14.4 No hidden scaling        | Done         | By omission; campaign instances scale only through DM choice.                                         |

### §15-16 Accessibility, Audio, Presentation

| **Item**                                   | **Status**   | **Evidence / Notes**                                                                                                                                             |
|--------------------------------------------|--------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 15.1 UI/text scaling                       | Done         |                                                                                                                                                                  |
| 15.1 Remappable controls                   | Not started  |                                                                                                                                                                  |
| 15.1 Colourblind / high-contrast / reduced motion / combat-text controls | Not started | No accessibility settings exist.                                                                                                              |
| 15.2 Effect density                        | Not started  |                                                                                                                                                                  |
| 16.1 Preserve visual identity              | Done         | Classic effect fidelity programme: `.str` recipes for wizard/persistent units, Hunter traps as real RSM props, status tints with desaturation, alpha-test fix.   |
| 16.2 Audio cues                            | Stock        | BGM/SFX play; no new cue design.                                                                                                                                 |
| 16.3 Combat text options                   | Not started  |                                                                                                                                                                  |
| 16.4 Out-of-combat fading                  | Not started  |                                                                                                                                                                  |

### §17 Configuration and Server Administration

| **Item**                        | **Status**   | **Evidence / Notes**                                                                                                                                                                                                                                                    |
|---------------------------------|--------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 17.1 Server baseline settings   | Mostly stock | `area_size: 30` (draw distance) overridden; autoloot commands granted to group 0 (2026-09-21). Knowledge Mode, guidance, respec curve, party range, death penalty: unchanged. PvP/WoE: the stock WoE NPC set (`npc/scripts_woe.conf`, `npc/re/scripts_woe.conf`) is still included by `scripts_main.conf`; the custom `woe_controller`/battleground scripts are not loaded. §2.3 says WoE is out of scope — remove the includes. |
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
| Test infrastructure            | 149-scenario headless client (148 pass / 1 expected skip), skill-expectation generator from `skill_db`, observer-parity harness, `playtest-sprite-audit` (1.65 M composed facings swept), 323 client unit tests.                                                  |
| Content fixes                  | Academy 2F rebuilt from the client's own GAT; teleport-destination audit; 1,156 maps loading.                                                                                                                                                                    |

## Conflicts — Decided 2026-09-21

The audit surfaced six places where a fork decision contradicted this document. All six were decided by the server owner on 2026-09-21. The affected sections have been amended in place; this table is the record.

| **#** | **Conflict**                                                        | **Decision**                                                                                                                                                                                                                                                                                   | **Follow-up**                                                                                                                                                                                       |
|-------|---------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| C1    | Adventure Guide rejected by the client roadmap (2026-07-05)         | **GDD wins, and the scope grows.** Rejection reversed. The Adventure Guide becomes a full **in-game encyclopedia** — everything a player would otherwise look up on iRO Wiki, generated from this server's own data and searchable in the client. See §9.5 (rewritten).                             | Client: promote the DM Bestiary into a player window, add search, build the category browser over the in-tree JSON + server tables. Update `FEATURE_ROADMAP.md` "Considered and rejected".           |
| C2    | Vending / market search vs fork descope                             | **Fork wins — vending and market search are out of scope** for this server. §12.1 rewritten. Direct player trade is the loot-passing path and must be solid.                                                                                                                                    | Close the open trade items: drag-item grid, zeny field polish, last-second-change highlight, and the two-client live validation.                                                                    |
| C3    | Dodge roll planned in `modern-mechanics.md` §5                      | **GDD wins — no dodge roll.** §5.3 locked principle stands. Plan section struck.                                                                                                                                                                                                                | Done 2026-09-21: `korangar/docs/plans/modern-mechanics.md` §5 replaced with a rejection note.                                                                                                       |
| C4    | `@partyjump` is an unbounded party teleport                         | **Keep as-is for now.** Accepted as a §13.1 friends-first convenience on a private server; abuse potential noted for a public setting. May be bounded later.                                                                                                                                    | §9.8 amended with the exception. Revisit if the group grows or if it starts short-circuiting §9 travel.                                                                                             |
| C5    | Campaign is DM-gated; §8 describes a self-directed spine            | **Campaign is a separate mode.** "DM Session" is activated by the DM (`@dm mode on` / `@dm start`) and is not the §8 story spine. §8 stays as the design for a future self-directed spine; Seal Cascade is not counted against it.                                                              | §2.3 scope amended. Appendix D Q13 answered. Phase 6 rows re-labelled "DM Session mode".                                                                                                            |
| C6    | `@autoloot` gated to group 1                                        | **Grant to players.** `autoloot`, `alootid`, `autoloottype` moved to group 0 alongside `partyjump`.                                                                                                                                                                                             | Done 2026-09-21 in `Hercules/conf/groups.conf`. The §11.3 filter model remains separate future work.                                                                                                |

### Open playtest item recorded with the decisions

- **Dropping items from the inventory.** Reported by players as not possible. The code path exists (right-click an inventory slot → drop half / one / all, `CZ_ITEM_THROW2` 0x0363) and was live-verified on macOS 2026-07-10, so this is either a discoverability problem — right-click is the only path, the same issue character-delete has (M1-014) — or a Windows-pack regression. Needs one live check on the shipped pack; if it works, add a visible Drop affordance rather than documenting the right-click.


**End of Version 0.2**

*This document is intended to evolve through implementation and playtesting. Core identity principles should remain stable; tuning values and implementation details should change when evidence supports improvement.*
