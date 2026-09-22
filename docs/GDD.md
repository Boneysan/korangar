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

*Grounded in the audited code on 2026-09-21. Each row names the hook that exists today and the size of the gap. Sizes: S ≤ 1 day · M ≤ 1 week · L longer.*

| **§**  | **Design item**                         | **Hook that exists**                                                                                                                                          | **Gap**                                                                                                                                                        | **Size** |
|--------|-----------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 5.8    | Telegraphs for monster area attacks     | Hercules broadcasts every cast (`ZC_USESKILL_ACK` → `UseSkillSuccessPacket`) with skill id, target entity, **ground position** and cast time; the client already draws the cast bar from it. `Map::render_skill_footprint` draws any skill's real cell layout. | The networking layer drops `position`/`destination_entity` when it emits `NetworkEvent::SkillCast`. Carry them through and draw the footprint at the target for `cast_ms`. Restraint rule: only for skills whose layout is larger than one cell, and only while casting. | **S**    |
| 5.2    | Single-action input buffer, 150-250 ms  | `BufferedAction` (`lib.rs`) holds one pending attack / pickup / cast and fires when the actor walks into range.                                              | Add an expiry tick and a second trigger: the end of the local attack or skill animation. One slot, newest wins, expired input is dropped. No queue.            | **S-M**  |
| 5.2    | Immediate client acknowledgement        | Skill-fail reasons (`0x0EFE`) and out-of-range chat messages already make refusals explicit.                                                                   | Start the swing/cast animation on send rather than on server echo, and roll back on refusal. Playtest whether it reads as responsive or as desync.           | **M**    |
| 5.4    | Target cycling / nearest hostile        | Entity list with positions and hostility; `player_target.rs` frame.                                                                                           | Tab = nearest hostile not already targeted, Shift+Tab = previous; sort by distance then screen-centre angle. Client only.                                       | **S**    |
| 5.4    | Target frame: race / size / element     | `bestiary.json` carries element, race, size, modes for all 1,759 monsters and is already compiled into the client for the DM Bestiary.                        | Look the target's mob id up and render three chips; hide fields per §9.6 knowledge mode. Client only.                                                          | **S**    |
| 5.4    | Skill-range preview on hover            | Footprint renderer already tints red when out of range.                                                                                                       | Draw the range ring around the player while a skill is armed or its hotbar slot is hovered.                                                                    | **S**    |
| 5.9    | "Element advantage" cue on damage       | Damage numbers exist; `attr_fix.conf` is the elemental table; target element is in `bestiary.json`.                                                           | Client computes attacker element vs target element for the *cue only* (server still owns damage). Colour or suffix the number when the multiplier is > 1 or < 1. | **S**    |
| 5.9    | Advanced damage breakdown               | None — the server sends a total.                                                                                                                              | Needs a fork packet carrying the `battle_calc` components. Defer until the immediate/contextual layers are live and someone asks for it.                       | **L**    |
| 5.3    | Hold-mouse continuous path              | Click-to-move.                                                                                                                                                | Re-issue the move destination on a throttle while the button is held; same 200 ms bound WASD uses.                                                            | **S**    |
| 5.10   | Proc feedback for auto-cast builds      | Status tints and looping status effects render on the actor.                                                                                                  | Nothing beyond what is done; verify auto-spell / auto-cast visuals read on Sage and Blacksmith builds during the Phase 2 playtest.                             | verify   |

**Recommended order:** telegraphs → target cycling → target-frame chips → input buffer → element cue → hold-mouse → client acknowledgement. The first three are a day each, need no server change, and together are what makes §6 monsters *feel* fair before any AI work starts.

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

*What the Hercules fork already provides, checked 2026-09-21 against `src/map/status.h`, `src/map/mob.h`, `conf/map/battle/monster.conf` and `db/re/mob_skill_db.conf`.*

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
| Skirmisher      | Attack, then step away and return.                                                                                                     | **Yes** — `NPC_RUN` flees, it does not kite. A small `mob_ai` state: after N hits, path to a cell at distance d, resume. |
| Ranged Keeper   | Hold a preferred distance.                                                                                                             | **Yes** — a keep-distance check in `mob_ai_sub_hard` when the target closes within d.  |

§6.4 (reaction to ground effects) is also C: a per-monster "hazard awareness" mode bit consulted by the path cost function so a Zombie ignores Fire Wall and an Orc Archer routes around it when a path exists. Bosses invert it while enraged.

### Family behaviour is a data convention

§6.3 needs no engine feature: it is a naming and review rule for `mob_skill_db` entries — every Orc shares the same trigger thresholds and emotes, every Undead the same relentless profile. Keep the per-family template in `Hercules/planning/` and generate the per-monster entries from it so the family stays consistent when tuned.

### Pilot plan (answers Appendix D Q7 as a recommendation)

| **Tier** | **Map**                       | **Why**                                                                                                                                    |
|----------|-------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------|
| Early    | `prt_fild08` (Prontera Field 08)| The §9.3 example map; Poring / Lunatic / Fabre teach Aggressor and Coward with zero lethality.                                              |
| Mid      | `orcsdun01` (Orc Dungeon 1F)    | The §6.3 worked example: Orc Zombie (relentless Aggressor), Orc Skeleton, with `orc_fild` archers for Ranged Keeper once the C slice lands. |
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

For a friends server, accidental build mistakes should not require abandoning a character. Recommended default: generous early respecs, then an increasing in-game cost or quest/material requirement at higher progression. Respec should be accessible without real-money dependency. The goal is to preserve meaningful builds while eliminating documentation traps.

## 7.6 Integrated Build Planner

The character window should support a planning state. Players choose a target Base/Job level, allocate hypothetical stats and skills, and preview expected HP, SP, Hit, Flee, ASPD, cast-time changes, and other relevant derived values. Planned changes are never applied until explicitly committed through normal progression or a respec system.

## 7.7 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Audited 2026-09-21.*

| **§** | **Design item**                     | **Hook that exists**                                                                                                                  | **Gap**                                                                                                                                                                       | **Size** |
|-------|-------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 7.1   | Derived-stat preview per point      | `stats.rs` allocates and shows raw stats; Hercules `status.c` holds the renewal formulas; `statpoint.txt` the point costs.            | Port HIT / FLEE / ASPD / cast / HP / SP formulas client-side for *preview only*, labelled "estimate"; show the delta on hover of each `+` button.                             | M        |
| 7.4   | Skill detail (SP, cast, range …)    | Tooltip shows name + `Lv x/max` (`skill_info.rs`). `skills.json` carries only name/description/max level. `skill_db.conf` has everything per level. | Extend the JSON generator with per-level SpCost, CastTime, AfterCastActDelay, Range, Element, SkillType, Hit, StatusChange, prerequisites (from `skill_tree.conf`); render in the tooltip. | M        |
| 7.4   | Preview allocation (no commit)      | Skill tree has tabs, rank-up buttons and drag-to-hotbar (`skill_tree/`).                                                             | A "plan" toggle that holds hypothetical points locally; Commit replays the existing rank-up packets; Discard drops them.                                                        | M        |
| 7.5   | Respec: generous early, costly late | `npc/custom/resetnpc.txt`, flat 5,000z / 5,000z / 9,000z.                                                                             | Price by `BaseLevel`: free below 50, 10k at 50-79, 50k at 80-98, 200k at 99+. Script edit only.                                                                                | S        |
| 7.6   | Build planner                       | —                                                                                                                                     | 7.1 + 7.4 preview combined behind a target level slider. Do after both; reuse the stats formulas.                                                                              | L        |
| 7.1   | Creation-screen allocator           | Done (2026-09-03/04): 48-point spread, suggested first-job template, live preview.                                                     | —                                                                                                                                                                             | done     |

# 8. Story and Quest Design

## 8.1 Story as a Spine

The centralized storyline should orient players through the world, introduce systems, and provide a coherent narrative for players who want one. It should not become a mandatory corridor through all content. A player may pause the story indefinitely and continue leveling, exploring, farming, crafting, or helping friends.

*v0.2 (decision C5):* the Seal Cascade campaign is **not** this spine — it is DM Session mode (§2.3), gated on a DM being present. The self-directed spine this section describes does not yet exist; when it is built, the campaign's party-shared quest helpers and hub-NPC markers are reusable.

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

## 8.7 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Audited 2026-09-21.*

| **§**  | **Design item**                      | **Hook that exists**                                                                                                                                   | **Gap**                                                                                                                                                                                | **Size** |
|--------|--------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 8.4    | Breadcrumb to the next exit          | Every warp is a `warp` script line in `Hercules/npc/re/warps/**` (source map + cell → destination map + cell). The minimap draws blips.                | Generate a **map graph** JSON from those lines; client runs a shortest-path over maps and marks the exit cell on the minimap plus an edge arrow. This one artefact also unblocks §9.2, §9.9 and §13.2. | M        |
| 8.4    | Clickable `<NAVI>` links in dialogue | `dialog.rs` parses `<NAVI>[label]<INFO>map,x,y</INFO></NAVI>` — and strips it.                                                                         | Render the label as a button; same map → drop a minimap marker and walk; other map → feed the route above.                                                                              | S        |
| 8.3    | Quest markers with a player toggle   | `QuestIcon` particles render server quest effects on NPCs.                                                                                             | A Game Settings toggle (like `show_minimap`) that suppresses them.                                                                                                                       | S        |
| 10.11  | On-HUD quest tracker                 | `quest_log.rs` builds rows with remaining objectives (Ctrl+Q).                                                                                         | A small HUD window showing the *tracked* subset (checkbox per quest, persisted); click → open the log or route.                                                                         | M        |
| 8.2    | Hunting goals / journal leads        | —                                                                                                                                                      | Client-side pinned list (item / monster / map name), persisted in settings, shown in the tracker; link to the §9.5 entry.                                                                | S-M      |
| 8.5    | Shared progress for stock quests     | Campaign quests share via `DM_Party*` helpers. Stock hunting quests count only for the killer (`quest_update_objective` in `quest.c`).                   | Server delta: on kill, also update the objective for party members within `party_share_range` on the same map. Keep behind a battle-conf key.                                           | M        |
| 8.6    | Story-based teaching                 | DM Session mode already teaches through play (skill checks, hazards).                                                                                  | Content for the future self-directed spine; not before Phase 4.                                                                                                                         | L        |

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

## 9.10 Implementation Path (added v0.2)

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Audited 2026-09-21.*

| **§** | **Design item**                | **Hook that exists**                                                                                                                                  | **Gap**                                                                                                                                                                                    | **Size** |
|-------|--------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 9.2   | World map                      | Map graph (§8.7) once generated; the client already loads per-map minimap bitmaps; `Towninfo` gives town POIs.                                        | A window laying out region nodes from the graph (hand-authored positions for ~60 towns/fields, generated for the rest), with visited flags persisted client-side. No teleport.               | M-L      |
| 9.3   | Map information panel          | Encyclopedia maps category (§9.5).                                                                                                                    | Suggested level = mean spawn level; population = spawn counts; connections = graph edges; party presence = `party_state` map names.                                                         | M        |
| 9.4   | Population regions             | Spawn lines carry map, cell and spread (`x,y,xs,ys`) — **not yet exported**; `bestiary.json` has no spawn data.                                       | Generator emits per-map spawn rectangles; minimap draws low/medium/high shading for the selected monster. Never exact points.                                                               | M        |
| 9.6   | Knowledge mode                 | `DmCampaignState.bestiary_unlocked` records kills **for the session only** — not persisted, not server-side (`specs/bestiary-unlock-persistence.md` is the plan). | Server: `#bestiary_<id>` account vars set on kill via `OnNPCKillEvent`, synced at login through a `[DMJ]`-style echo (§13.7). Client: parse and persist. Config key for the mode.          | M        |
| 9.7   | Rumors                         | —                                                                                                                                                     | A `rumor` journal category fed by NPC scripts (`callfunc("Journal_AddRumor", id)`) and synced like unlocks. After §9.6.                                                                     | M        |
| 9.9   | Portal labels                  | Map graph.                                                                                                                                            | Hover/approach a warp cell → tooltip with the destination map name; tracked-route portals get an accent.                                                                                   | S        |
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

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Audited 2026-09-21.*

| **§**  | **Design item**                         | **Hook that exists**                                                                                                                                                  | **Gap**                                                                                                                                                                                | **Size** |
|--------|-----------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 10.2   | HUD Edit Mode + presets                 | Every window is movable/resizable and its anchor/size persists (`WindowCache`). Spec: `specs/hud-edit-mode.md`.                                                       | Lock/unlock, snap-to-edge, named layouts (= multiple `WindowCache` files: Classic, Modern, custom), a "reset layout" action. Combat fading in §16.5.                                    | M        |
| 10.3   | Status icons with timers                | Text status bar with monograms + timers + descriptions (`status_bar.rs`, M1-010). The GRFs ship `texture/effect/*.tga` icons indexed by `System/stateiconimginfo.lub`. | Load the lub table like `Towninfo`, draw the icon beside the text; verify the table is present in the shipped GRFs first.                                                             | S-M      |
| 10.6   | Keybind remap screen                    | Chords are hardcoded in `input/mod.rs` (~65 `KeyCode` sites).                                                                                                          | Extract a `Binding → Action` table into settings (RON), drive `input/mod.rs` from it, add a settings tab with conflict detection and profile import/export.                            | M        |
| 10.6   | Hotbars                                 | Three rows, items and skills, server-stored (done 2026-09-05).                                                                                                        | Optional: a fourth vertical bar for buffs/consumables; mouse-button binds (needs 10.6 remap).                                                                                          | S        |
| 10.7   | Equipment sets                          | `RequestEquipItemPacket` per item; inventory ids known.                                                                                                              | Named sets stored client-side as item ids; "Equip set" loops the packets in slot order and reports what was missing. Server rules still apply per packet.                              | M        |
| 10.8   | Stats interface modes                   | Needs §7.7 formulas.                                                                                                                                                  | Simple / Detailed / Advanced tabs over the same numbers.                                                                                                                                | M        |
| 10.9   | Inventory search / sort / filters       | `inventory.rs` is a bare grid; `items.json` has `Type` and names compiled in; drag-and-drop exists.                                                                   | Search box, category tabs from `Type`, sort menu; **Do-Not-Drop / Do-Not-Sell** as a client-side locked-id list checked in `DropItem` and the sell cart.                              | M        |
| 10.9   | Storage search                          | `storage.rs` uses the same item grid.                                                                                                                                  | Reuse the inventory search component.                                                                                                                                                  | S        |
| 10.10  | Equipment comparison                    | **Partly done:** tooltip shows "— vs equipped —" deltas for ATK / MATK / DEF / slots / refine (`item_stats.rs`).                                                      | Plain-language script bonuses (`Script` field in `items.json` → "+10% vs Demi-Human"); optional "vs current target" using the §5.13 element/race/size chips.                          | S-M      |
| 10.12  | Minimap layers / waypoints / pings      | Blips for player, party, compass, Towninfo POIs; hover names.                                                                                                         | Layer toggles per blip class; click-to-place personal waypoint (client); party pings ride §13.7's transport.                                                                            | S / M    |
| 10.14  | Party frame click-to-target, distance   | Roster with HP/SP, class, Go-to.                                                                                                                                       | Click a row → `player_target`; show "other map" or tile distance from `party_state`.                                                                                                    | S        |
| 10.15  | Chat: timestamps, item links, tabs      | Public / Party / Whisper channels. `<ITEM>` tags are stripped in `dialog.rs`.                                                                                        | Timestamps S; a Loot/System filter S; `<ITEM>` → hover tooltip via the existing item tooltip S-M; text selection/copy needs a framework primitive M.                                    | S-M      |
| 10.16  | UI scaling                              | Done.                                                                                                                                                                 | —                                                                                                                                                                                      | done     |
| —      | Character delete confirmation (M1-014)  | Right-click menu fires `DeleteCharacter` directly.                                                                                                                    | Visible Delete button + typed-name confirm. Same fix shape as the item-drop discoverability report.                                                                                    | S        |
| —      | Toast notifications                     | —                                                                                                                                                                     | One slide-in widget reused by §11.4 (rare drop), quest complete, level up, party ping. Build once, early.                                                                              | S-M      |

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

Useful sinks may include reasonable respec costs, refinement, crafting fees, transport, cosmetic services, and convenience services. Avoid punitive sinks that exist only to slow friends from playing together.

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

*Same method as §5.13: the hook that exists today, the gap, and a size (S ≤ 1 day · M ≤ 1 week · L longer). Audited 2026-09-21.*

**The transport decision.** Pings, shared destinations, ready checks, hunting-goal lists, bestiary sync and rumors all need a small channel for structured party state. Nothing in the client parses structured messages today — `[DMJ]` is emitted by server scripts but `DmCampaignState` is populated only from local kill observations. Build the channel once:

- **Phase A — party chat with a prefix.** Send `\x01PS{json}` as ordinary party chat; every client parses and suppresses it; unknown prefixes fall through as text so a stock client still sees something readable. No server change, works tonight. Risk: flood limits (`ip_rules` is tuned) and 150-byte message caps — keep payloads tiny (ping: kind, map, x, y).
- **Phase B — a fork packet** (`CZ_/ZC_PARTY_STATE`, next free id below `0x0F00`) when Phase A's limits bite. Same client parser, different carrier.

| **§** | **Design item**                | **Hook that exists**                                                    | **Gap**                                                                                                                                       | **Size** |
|-------|--------------------------------|-------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------|----------|
| 13.3  | Party pings (6 kinds)          | Minimap blips; party chat.                                              | Phase A transport + a timed blip on minimap and in-world; Ctrl+click on the minimap or a radial on the party frame to send.                    | M        |
| 13.2  | Shared destinations            | Map graph (§8.7).                                                       | Leader proposes `{map}`; accept → each client runs its own route. Payload is one map name.                                                     | S after graph |
| 13.4  | Tonight's Goals                | —                                                                       | Client list, shared over Phase A on change; shown in the tracker.                                                                              | S-M      |
| 13.5  | Level range                    | `conf/map/battle/party.conf` stock.                                     | Set `party_even_share_bonus` and widen the share level range; playtest whether a level-sync mode is still wanted. Config only.                 | S        |
| 13.6  | Shared stock quest progress    | See §8.7.                                                               | Server delta behind a config key.                                                                                                             | M        |
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
| 14.2  | Recoverable death penalty | `exp.conf`: `death_penalty_type: 1`, 1% base / 1% job. Hercules has `OnPCDieEvent` / `OnPCKillEvent` hooks. | Script: on death store the lost EXP in a char variable; on the next N kills on the same map (or touching the save point) refund half. Percentages are playtest values.       | S-M      |
| 14.3  | Difficulty communication  | Encyclopedia maps category (§9.5); spawn levels.                                                             | "Suggested level" on the map panel and a colour on the world map node; a one-line warning toast on entering a map ≥ 15 levels above the player. Warnings, never locks.       | S after generator |
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
| 15.1  | Remappable controls                  | See §10.17 (hardcoded chords).                                                                                                                   | Binding table + remap screen.                                                                                                                                        | M        |
| 15.1  | UI / text scaling                    | Done.                                                                                                                                            | —                                                                                                                                                                    | done     |
| 15.1  | Colourblind-safe / high-contrast     | A theme system exists: three named theme slots in `InterfaceSettings` (`menu_theme`, `in_game_theme`, `world_theme`), `InterfaceTheme::load`, a Theme Inspector window. | Ship "High Contrast" and "Deuteranopia" themes; make telegraphs, target outline and party-frame colours read from the world theme rather than constants.             | S-M      |
| 15.1  | Reduced flashing / screen shake      | Effects are recipe-driven (`skill_recipe.rs`, `unit_recipe.rs`).                                                                                  | A `reduced_motion` setting; recipes tagged `flash` / `shake` are skipped or damped.                                                                                  | S-M      |
| 15.1  | Combat text size and frequency       | `DamageNumber` renders at a hardcoded `FontSize(16.0)`; crit colour only.                                                                        | Size and "show: all / crits+status / none" settings; merge rapid multi-hits into one number.                                                                         | S        |
| 15.1  | Alternative ground-target confirm    | Press-then-click only.                                                                                                                           | Add hold-aim-release and quickcast-at-cursor per skill or globally (§5.4).                                                                                            | M        |
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
| Knowledge Mode          | Hybrid                                  | Basic monster/map info available; deeper details discovered or configurable. *v0.2: gates §9.5 encyclopedia detail, never existence.* |
| Quest Guidance          | Available, opt-in per tracked quest     | Players can disable globally.                                                |
| Exact Spawn Coordinates | Off                                     | Use population regions instead.                                              |
| Auto-Loot               | Configurable categories                 | Respect inventory and weight. *v0.2: `@autoloot`/`@alootid`/`@autoloottype` granted to players (C6); category filters are client work.* |
| Respec                  | Generous early; in-game cost later      | Avoid real-money dependency.                                                 |
| Party Level Range       | Wider than classic or diminishing model | Tune for friend accessibility.                                               |
| Death Penalty           | Modest and partly recoverable           | Tune through playtests.                                                      |
| PvP / WoE               | Disabled / not part of this design      | Deferred. *v0.2: the stock WoE NPC set is still loaded via `npc/scripts_woe.conf` — remove the includes.* |

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

- **[open]** World map and map connection data.

- **[partial]** Quest tracking and breadcrumb routing. *Quest log window exists (Ctrl+Q); no tracker HUD, no routing.*

- **[open]** Basic Adventure Guide search. *Confirmed and widened 2026-09-21 (C1): full in-game encyclopedia, §9.5. Seed is the DM Bestiary + in-tree JSON.*

- **[partial]** Inventory/storage search and item protections. *Weight display done; search, sort, filters, locks not started.*

- **[done]** Improved target/player frames and UI scaling. *Basic frames; scaling done. Status icons and target detail still open.*

- **[partial]** Party member map location and shared waypoint support. *Party minimap blips with hover names, roster, world HP bars, `@partyjump` done. Waypoints not started.*

## Phase 2 - Combat Responsiveness — **partial** — *slices and order in §5.13*

- **[partial]** Single-action input buffering. *Walk-into-range chaining only; no timed buffer.*

- **[open]** Target switching and selection improvements.

- **[unseen]** Ground-target previews. *Real Hercules cell layouts, out-of-range tint.*

- **[done]** Improved cast/action indicators. *Overhead cast bars; fork-only cast cancel; skill-fail reasons.*

- **[done]** Animation and hit-feedback synchronization. *Animation engine phases A-D closed.*

## Phase 3 - Tactical Monster Layer — **not started** — *data-first path and pilot maps in §6.10*

- **[open]** Implement reusable behavior archetypes.

- **[open]** Pilot on one early, one midgame, and one late-game region.

- **[open]** Add restrained dangerous-attack telegraphs.

- **[open]** Test reactions to ground control and party roles.

- **[open]** Iterate before broad rollout to all monsters.

*Nearest existing asset: the DM hazard/encounter scripts in `Hercules/npc/custom/dm_campaign/shared/` prove the server-side primitives (`DM_HazardArea`, mode-bit stripping, `setcell`) that an AI layer would also use.*

## Phase 4 - Journals and Knowledge Systems — **partial (DM-only) — scope widened by C1 to the §9.5 encyclopedia** — *paths in §7.7, §9.10*

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

## Phase 7 - Polish and Expansion — **not started** — *paths in §15.3, §16.5*

- **[open]** Accessibility pass.

- **[open]** Effect density and performance tuning.

- **[open]** UI profile polish.

- **[open]** Expanded monster behavior library.

- **[open]** Content pacing and economy rebalance after real player data.

## v0.2 Build Order — the next slices, sequenced (added 2026-09-21)

*Derived from the §5.13-§16.5 implementation paths. Ordered by dependency first, then cost. Client and server tracks can run in parallel. Each row is one PR-sized slice; sizes as in the section tables.*

| **#** | **Slice**                                                        | **Size** | **Unblocks**                                                        | **Path** |
|-------|------------------------------------------------------------------|----------|---------------------------------------------------------------------|----------|
| 1     | Toast widget                                                     | S-M      | Rare drops, quest complete, level-up, pings, difficulty warnings    | §10.17   |
| 2     | Monster cast telegraphs (carry position through `SkillCast`)     | S        | Everything in §6 feeling fair                                       | §5.13    |
| 3     | Target cycling (Tab) + race/size/element chips on the target frame| S+S     | §5.9 contextual layer, §10.10 vs-target                             | §5.13    |
| 4     | Character-delete confirm + visible item-drop affordance          | S+S      | Two playtest reports                                                | §10.17   |
| 5     | **Map graph generator** from `npc/re/warps/**`                   | M        | Breadcrumbs, portal labels, world map, shared destinations, map panel| §8.7     |
| 6     | `<NAVI>` links + next-exit breadcrumb on the minimap             | S+M      | §8.4, §20.1 "no external guide"                                     | §8.7     |
| 7     | Inventory search / category tabs / sort / lock list              | M        | §10.9; storage reuses it                                            | §10.17   |
| 8     | **Encyclopedia generator** (skills, maps, quests, NPCs, refine)  | M        | §9.5 categories, §7.4 skill tooltips, §11.6 refine odds, §14.3      | §9.5     |
| 9     | Player Adventure Guide window with search (promote DM Bestiary)  | M        | C1 first shippable slice                                            | §9.5     |
| 10    | Single-action input buffer with expiry                           | S-M      | §5.2                                                                | §5.13    |
| 11    | Party transport Phase A + pings + ready check                    | M        | §13.3, §13.2, §13.4, §9.6 sync                                      | §13.7    |
| 12    | Quest tracker HUD + hunting goals                                | M        | §10.11, §8.2                                                        | §8.7     |
| 13    | Keybinding table + remap screen                                  | M        | §10.6, §15.1                                                        | §10.17   |
| 14    | HUD edit mode: lock, snap, named layouts, combat fade            | M        | §10.2, §16.4                                                        | §10.17   |
| 15    | Accessibility: themes, reduced motion, combat-text controls, density | M    | §15                                                                 | §15.3    |

**Server track (parallel, data and script only):**

| **#** | **Slice**                                                  | **Size** | **Path** |
|-------|------------------------------------------------------------|----------|----------|
| S1    | Remove the stock WoE includes from `scripts_main.conf`     | S        | §17.1    |
| S2    | Respec price curve in `resetnpc.txt`                       | S        | §7.7     |
| S3    | `party.conf` share range and even-share bonus              | S        | §13.7    |
| S4    | Death-recovery script (`OnPCDieEvent` refund)              | S-M      | §14.5    |
| S5    | AI archetype pilot on `prt_fild08` via `mob_skill_db`      | S        | §6.10    |
| S6    | Pilot on `orcsdun01`, then Glast Heim                      | S+S      | §6.10    |
| S7    | Bestiary unlock persistence (account vars + login sync)    | M        | §9.10    |
| S8    | Shared stock-quest progress for party members (C, config-gated) | M   | §8.7     |
| S9    | Skirmisher / Ranged Keeper / hazard-aware pathing (C)      | L        | §6.10    |

Slices 1-4 are a week of client work with no server dependency and change how the game *feels* immediately. Slice 5 and slice 8 are the two generators everything in navigation and knowledge hangs off; do them before any of the windows that consume them.

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

*v0.2 adds a **Current** column — what the forks do today (audited 2026-09-21) — beside the design baseline, so the gap is visible per row.*

| **Feature**                | **Baseline**                                      | **Type**              | **Current (2026-09-21)**                                                              |
|----------------------------|---------------------------------------------------|-----------------------|---------------------------------------------------------------------------------------|
| PvP / WoE                  | Disabled / deferred                               | Locked for this scope | Stock WoE NPC scripts still loaded; no PvP maps used. Remove the includes.            |
| DM Session mode            | Opt-in, DM-activated, one party                   | Decided (C5)          | Implemented: `@dm mode on` / `@dm start`, `DM_SessionAllows` gate at 50 sites.       |
| Main Story Guidance        | Available when tracked                            | Player configurable   | Quest log window only; no tracker, no toggle.                                        |
| World Breadcrumbs          | Next-exit guidance                                | Player configurable   | Not started. `<NAVI>` dialogue tags stripped rather than followed.                    |
| Monster Population Overlay | Broad regions only                                | Player configurable   | Not started. Spawn data not yet exported from the server tree.                        |
| Knowledge Mode             | Hybrid                                            | Server configurable   | No switch. DM Bestiary unlocks are session-only, client-side.                        |
| In-game Encyclopedia       | Full wiki coverage, generated from server tables  | Locked principle (C1) | Data for monsters/items/cards/status; no player UI; skills/maps/quests need generator.|
| Input Buffer               | ~200 ms starting test                             | Playtest value        | Not implemented; walk-into-range chaining only.                                       |
| Action Queue               | 1 action                                          | Recommended default   | Not implemented.                                                                      |
| Universal Dodge            | None                                              | Locked principle      | None. Plan section struck (C3).                                                       |
| Auto-Loot                  | Category filters                                  | Player configurable   | `@autoloot` (drop-rate threshold), `@alootid`, `@autoloottype` — group 0 (C6). No category filters. |
| Party Level Rules          | Friend-friendly widened range / diminishing model | Playtest decision     | Stock `party.conf`.                                                                   |
| Death Penalty              | Modest + partial recovery                         | Playtest decision     | Stock: 1% base / 1% job, no recovery.                                                 |
| Respec                     | Free/cheap early, in-game cost later              | Recommended default   | Flat 5,000z stats / 5,000z skills / 9,000z both (`resetnpc.txt`).                    |
| Fast Travel                | In-world systems; no unrestricted map teleport    | Locked principle      | Kafra + warper NPC; `@partyjump` to any online party member, unbounded (C4).          |
| Exact Spawn Coordinates    | Hidden                                            | Recommended default   | Hidden (nothing shows them).                                                          |
| HUD Presets                | Classic + Modern + custom                         | Recommended default   | None. Windows movable/resizable and persisted; no presets or edit mode.               |
| Drop / EXP Rates           | Tuned for small population (§12.2)                | Playtest decision     | Stock 100% / 100% / card 100%.                                                        |
| Keyboard Movement          | Optional, same pathing rules                      | Recommended default   | WASD on by default, click-to-move retained, 200 ms throttle.                          |
| UI Scale                   | Independent of pixel-art scaling                  | Player configurable   | Interface Settings > Scaling (Ctrl+I).                                                |

# Appendix D. Open Design Questions

1. Which exact server/client codebase and tooling constrain the planned UI and AI changes?

2. How far should keyboard movement go while preserving tile/pathing parity with click-to-move?

3. Should monster journal data be character-specific, account-wide, or server-wide?

4. What percentage of item/drop information should be available immediately in Hybrid knowledge mode? *v0.2: sharpened by §9.5 — every entry exists in Hybrid; the question is which fields reveal through play.*

5. Should friends have an optional level-sync system, or is a widened party XP model sufficient?

6. How destructive should high-level refinement remain on a small server?

7. Which three maps should be used as the first tactical-AI pilot areas? *v0.2 recommendation in §6.10: `prt_fild08`, `orcsdun01`, Glast Heim. Not yet decided.*

8. Which MVP should be the first redesigned boss used to prove the tactical-combat framework? *v0.2 recommendation in §6.10: Eddga. Not yet decided.*

9. Should party pings and shared destinations persist across map transitions or expire quickly?

10. How much advanced formula information can the client display accurately from the server implementation?

11. Which original UI windows can be extended safely versus requiring replacement or overlay panels?

12. What level of map/monster data can be generated automatically from server tables versus authored manually? *v0.2 note: `bestiary.json`, `items.json`, `cards.json`, `skills.json`, `status_effects.json` are already generated in-tree from Hercules DBs; the open part is authoring population regions and rumors.*

*Answered or narrowed since v0.1:*

- Q1 — **Answered.** Korangar (Rust, wgpu) client fork and Hercules server fork at `PACKETVER=20220406`. UI is fully owned; custom packets are cheap but must be versioned on both ends.

- Q2 — **Answered.** WASD ships as an optional mode over the same pathfinder with a 200 ms throttle; click-to-move retained. No further keyboard-movement scope is planned.

- Q3 — **Narrowed.** The DM Bestiary tracks unlocks per session, client-side; the spec proposes per-account server variables. Whether a player-facing journal should be account- or server-wide is still open.

*New questions raised by the audit:*

13. ~~Is the Seal Cascade DM campaign the §8 story spine, or a separate "DM session" mode?~~ **Answered 2026-09-21: a separate DM Session mode, activated by the DM.** See §2.3 and Appendix E, C5.

14. ~~Should the Adventure Guide rejection (2026-07-05) stand?~~ **Answered 2026-09-21: no — the Adventure Guide becomes a full in-game encyclopedia (§9.5).** See Appendix E, C1.

15. Which of the fork's protocol additions (cast cancel, skill-fail reasons, party invite sender, party SP) should be promoted into the GDD as designed features rather than incidental fixes?

16. ~~What bounds `@partyjump`?~~ **Answered 2026-09-21: nothing, for now.** Accepted as a private-server convenience; revisit later. See Appendix E, C4.

17. Which three playtest-1 gaps get priority for playtest 2: inventory search, a HUD quest tracker, or party pings on the existing `[DMJ]` transport?

## Recommended Next Design Deliverables

- Tactical Combat Specification: input states, targeting rules, action buffer, movement, cast states, telegraph taxonomy, AI interfaces. *v0.2: §5.13 and §6.10 are the seed; the remaining spec work is the telegraph taxonomy per skill layout size and the two C-side AI behaviours.*

- UI Wireframe Pack: HUD presets, target/party frames, Adventure Guide, world map, monster journal, inventory, build planner.

- Navigation Data Specification: map graph, portal metadata, breadcrumbs, POIs, monster population regions, discovery flags.

- Monster AI Behavior Library: reusable states, priorities, pathing responses, and authoring parameters.

- Pilot Region Design: one complete region implemented end-to-end to validate combat, UI, quests, and navigation together.

- Encyclopedia Data Generator Specification: one build step from the Hercules tree to the §9.5 category files, with the field list per category and the knowledge-mode visibility flag per field.

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
| 5.8 Telegraphs                             | Not started (S)  | No client decals. But the cast packet already carries the ground position and the footprint renderer exists — see §5.13. Scripted hazards exist in campaign instances (`DM_HazardArea`).                                          |
| 5.9 Immediate layer (damage numbers)       | Done             | Damage/heal numbers, crit attack animation. No "element advantage" cue.                                                                                                                                                               |
| 5.9 Contextual / advanced layers           | Not started      | No monster race/size/element tooltip, no damage breakdown.                                                                                                                                                                            |
| 5.10 Autoattack feel                       | Partial          | Animation engine phases A-D closed; hit/attack sync; ReadyFight stance; ranged attacks draw the real ammunition sprite. Proc/status feedback: opt1/opt2 tints, looping status STRs, freeze on stun/sleep.                             |
| *Fork addition — cast cancel*              | Done (unseen)    | `CZ_CANCEL_CAST` (0x0F00) + Hercules delta; right-click/Escape aborts own cast. Not an RO feature; deliberate. Movement still never cancels; casting still roots.                                                                     |
| *Fork addition — skill-fail reasons*       | Done             | `ZC_SKILL_FAIL_REASON` (0x0EFE) names why a skill actually failed; missing-item failures name the item. Directly serves the "players can explain outcomes" success criterion.                                                          |

### §6 Monster AI and Encounter Design

| **Item**                            | **Status**   | **Evidence / Notes**                                                                                                                                                                                                                              |
|-------------------------------------|--------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 6.2 Behaviour archetypes            | Not started  | No `mob.c` / AI changes in the fork. Six of eight archetypes are achievable with mode bits + `mob_skill_db` data alone — see §6.10. |
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
| 9.4 Monster population regions    | Not started  | Spawn data is in Hercules `npc/re/mobs/**` spawn lines and is **not** exported yet; `bestiary.json` has stats only.                                                                    |
| 9.5 In-game encyclopedia          | Decided (C1) — Partial data, no UI | Rejection of 2026-07-05 reversed and scope widened to full wiki coverage. Data exists for monsters/items/cards/status; skills, jobs, maps, quests, NPCs, mechanics need a generator. No player UI. |
| 9.6 Discovery modes               | Partial      | Bestiary Journal (DM window) unlocks entries on observed kills, **session-only and client-side**; the server-variable persistence is a spec (`specs/bestiary-unlock-persistence.md`), not code. No Knowledge Mode switch. |
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
| 10.10 Equipment comparison            | Partial      | Tooltip shows "— vs equipped —" deltas (ATK/MATK/DEF/slots/refine). Script bonuses and vs-target context: not started.                                                        |
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
| 13.3 Party pings                  | Not started    | No structured party transport exists in the client yet (the server's `[DMJ]` echo is not parsed). Transport decision in §13.7.  |
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
| 15.1 Colourblind / high-contrast / reduced motion / combat-text controls | Not started | No accessibility settings; a named-theme system exists to build the colour modes on (§15.3).                                                 |
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
