# Playtest adjustment plan — 2026-09-12

> Qwen3 must execute this backlog through
> [`qwen3-playtest-runbook.md`](qwen3-playtest-runbook.md). The runbook owns the
> single `NEXT` pointer, bounded task cards, evidence requirements, and
> non-blocking senior-developer checkpoints. Qwen3 continues automatically from
> each completed or blocked task to the next runnable task. Do not treat this
> broad product plan as one implementation task.

This plan turns the September playtest notes into a prioritized backlog shared
between Korangar, Hercules, and the Windows distribution package. Repeated
reports have been merged. The audit below separates behavior already present in
the repositories, defects confirmed from source, and new work. Runtime reports
still receive a reproduction pass because overweight state can suppress
regeneration, latency can expose both WASD rubber-banding and targeting errors,
and failed skills can come from either client range handling or server rejection.

## Repository audit snapshot

| Classification | Verified state on 2026-09-12 |
|---|---|
| Already present | Ctrl+Q quest journal and collection counts; inventory equipment comparison tooltips; hotbar clearing by dragging an entry back to its source window; click-to-walk floor pickup; Trade One/All; `@save`/`@load`; GM `@skreset`; persistent Zeny; Act I exploration-chest rewards |
| Confirmed defects or gaps | Vendor sell offers are not client-filtered for equipped items; a successful sale clears the buy cart instead of the sell cart; vendor rows lack equipment tooltips; item export lacks complete class eligibility; updater/setup cannot repair one file; quest data lacks route/NPC/monster guidance; no durable late-join quest reconciliation |
| Needs a live reproduction | Blue Potion trade refusal despite no item-db trade restriction; spell failures; internet WASD corrections; dropped-item repickup; sitting recovery at each weight threshold |
| New features | Exact stack quantities; visible unusable-item treatment; Combat chat channel; Tab target cycle; quest HUD and portal routing; player minimap colors; player-accessible respec/checkpoints; retaliation stance |

Experience also needs an explicit balance pass. Hercules currently uses 100% base
and job monster EXP, forces even EXP share for new parties, gives no additional
even-share party bonus, and limits even sharing to a 15-level spread.

## Outcomes

1. Friends can update safely without downloading the full client again.
2. Combat inputs are predictable on both LAN and internet connections.
3. Inventory, vending, and trading never move or sell an unintended item.
4. Players can understand the current campaign objective without asking the DM
   for internal map names.
5. Recovery, weight, respawn, and respec rules support a tabletop-paced
   campaign.
6. Exploration rewards and player identification are visible and consistent.

## Priority order

| Priority | Meaning | Work |
|---|---|---|
| P0 | Data loss, release failure, or core action failure | Updater corruption; equipped-item selling; stale sell cart/duplicate transaction risk; spells that fail; Increase AGI patch |
| P1 | Frequent playtest friction | Internet WASD; targeting; hotbar removal; inventory/trade quantities; quest clarity |
| P2 | Campaign rules and pacing | Monster/party EXP; regeneration; weight; respawn; respec; retaliation; loot reach |
| P3 | Presentation and optional systems | Combat log; player colors; click sounds; cosmetics; world chests |

## Phase 0 — capture reproducible cases

Before changing behavior, record one short case for each report. Capture the
client build hash, server hash, ping, map, character weight percentage, skill or
item ID, and the last relevant server/client log lines.

- [ ] Reproduce each failed spell with the skill name, target, range, SP, and
      server rejection/result packet.
- [ ] Reproduce WASD movement on LAN and over the internet at measured ping and
      packet loss. Compare click-to-move on the same route.
- [ ] Reproduce selling an equipped item and determine whether the server accepts
      it or the client sends the wrong inventory index.
- [ ] Reproduce the Blue Potion trade refusal and record the exact item flags,
      packet response, and UI message.
- [ ] Reproduce immediate pickup after dropping an item. Check whether the client
      issues pickup automatically or the server still considers the previous
      click active.
- [ ] Verify sitting HP/SP regeneration at normal weight and at every overweight
      threshold.
- [ ] Verify Zeny across logout, reconnect, server restart, trade, vending, and
      cart transactions. Hercules stores Zeny in the character database; this
      test checks every client path against that persisted value.
- [ ] Reproduce the report that vendors sell twice and clarify whether it means a
      duplicate purchase, duplicate sale, or cart inventory being included.
      Include the confirmed client defect where sale success clears the buy cart
      instead of the sell cart.
- [ ] Measure one solo and one even-share party monster kill from the before/after
      base and job EXP totals. Repeat inside and outside the 15-level share range;
      compare the result with the monster database reward and active rate flags.

Exit condition: every P0/P1 bug has exact steps and an owner; ambiguous reports
are converted into a concrete case rather than guessed at during implementation.

## Phase 1 — release and safety hotfix

### Increase AGI effect

The Korangar source change is already prepared locally: Increase AGI uses the
classic `EF_INCAGILITY` visuals and sound instead of the Heal animation.

- [ ] Keep the existing source changes and include them in the next Windows
      client build.
- [ ] Test Increase AGI from the skill bar, direct casting, another player, and
      an NPC or item source. Confirm Heal still uses its own animation.
- [ ] Publish the changed executable and manifest through the small patch, then
      test an upgrade from the last friend build.

### Updater corruption and recovery

Observed failure: setup verified all 200 merged files, then reported
`CORRUPT Verify.ps1`. The recovery text encouraged downloading a package half
again, which can lead players to replace far more data than necessary.

The current scripts do not download files. `Update.ps1` copies a separately
downloaded small patch over an installation; `Setup.ps1` merges the separately
downloaded client and Assets halves; `Verify.ps1` only validates their two
manifests. Automatic retry or single-file repair therefore requires a stable
download endpoint or a separately published repair bundle.

- [ ] Compare the distributed `Verify.ps1` byte count and SHA-256 value with the
      manifest and the copy retrieved from the shared Drive folder.
- [ ] Check whether Google Drive, browser security, antivirus, CRLF conversion,
      or updater self-replacement changes the script after download.
- [ ] Reproduce from the exact two uploaded packages and identify which manifest
      disagrees with the final `Verify.ps1`. Ensure shared files are byte-identical
      in both uploaded halves, or list shared verifier files in only one manifest.
- [ ] Preserve manifest provenance while verifying and print the expected hash,
      actual hash, local path, and the package half responsible for a mismatch.
- [ ] For the current Drive workflow, publish a small repair package for client
      scripts. If a stable file endpoint is added later, download to a temporary
      name, verify, retry, and atomically replace the destination.
- [ ] Keep a known-good setup/verify pair outside the files being copied while a
      merge or update is running.
- [ ] Test clean install, one missing small file, one corrupt small file, one
      interrupted GRF, and upgrade from the previous friend build.

Exit condition: a corrupt `Verify.ps1` can be repaired without touching valid
GRFs, the error identifies the responsible package, and the Increase AGI fix
reaches an upgraded client.

### Transaction safety

- [ ] Hide equipped items from the vendor sell list. If the server can still
      receive such a request, reject it there as a second guard.
- [ ] On successful sale, clear the sell cart rather than the buy cart. Prevent a
      stale selection from being submitted twice after the server reply.
- [ ] Add a confirmation for selling refined, slotted, carded, favorite, or
      otherwise valuable equipment.
- [ ] Ensure cart inventory is shown only in an explicitly labeled cart tab and
      is never sold as part of normal inventory selection.
- [ ] Add regression cases for equipped weapon/armor, costume equipment, cart
      items, stackable items, and a normal unequipped item.

Exit condition: no equipped item can be sold, and every sale changes inventory
and Zeny exactly once.

## Phase 2 — combat and input reliability

### Spell execution

- [ ] Build a small matrix from the failed spells: self-targeted, ally-targeted,
      enemy-targeted, ground-targeted, instant, and cast-time skills.
- [ ] Show a brief reason when the server rejects a cast: out of range, invalid
      target, insufficient SP, cooldown, line of sight, or wrong weapon/state.
- [ ] Preserve the selected target after a valid repeated cast so a player can
      cast on the same monster again.
- [ ] Confirm hotbar activation, skill-window activation, and keyboard activation
      send the same skill request.

### WASD movement over the internet

Korangar already throttles held-key requests, sends a longer path for sustained
movement, avoids some redundant corrections, and provides `KORANGAR_WASD_TRACE`.
Start with that trace before changing prediction or reconciliation.

- [ ] Log client input sequence, predicted tile, acknowledged server tile, ping,
      and correction distance without flooding normal logs.
- [ ] Audit whether the current 200 ms throttle and 15-cell held path coalesce
      internet input correctly; retain only the newest safe request where traces
      show redundant paths.
- [ ] Evaluate less-visible reconciliation only after measuring corrections; all
      movement must remain consistent with the authoritative server path.
- [ ] Prevent a held key from resending a stale path after knockback, warp, stun,
      or server correction.
- [ ] Test at 0, 75, 150, and 250 ms latency with jitter and 1–3% packet loss;
      compare WASD with click-to-move.

Exit condition: ordinary movement does not visibly bounce backward at 150 ms,
and forced corrections never let the client cross blocked terrain.

### Targeting

Korangar already has a selected-target frame and attack buffering. Audit their
visibility and persistence before changing them.

- [ ] Add Tab to cycle visible, alive, hostile monsters by distance, with a
      configurable key binding and deterministic wraparound.
- [ ] Strengthen the existing target frame and add a clear ring/outline so the
      selected monster remains obvious in a crowd.
- [ ] Increase click tolerance around monster sprites while resolving overlaps by
      screen depth and distance from the cursor.
- [ ] Keep the current target when clicking its sprite repeatedly and when a cast
      is rejected for range.
- [ ] Add player mouseover identification without making players eligible for the
      hostile Tab cycle.
- [ ] Decide whether “attack when hit” is an opt-in stance. Recommended behavior:
      acquire and attack the aggressor only while idle; never override movement,
      spell targeting, or an existing target.

Exit condition: a player can identify, select, reselect, cycle, and repeatedly
attack the intended monster in a crowded fight.

### Hotbar editing

- [ ] Reproduce the existing clear path: drag an item to Inventory or a skill to
      Skill Tree. Verify it sends `UNBOUND` and survives relogging.
- [ ] Add discoverable removal by dragging off the bar and by a right-click
      **Clear slot** action, since the existing source-window drop target was not
      apparent during play.
- [ ] Persist the cleared slot to Hercules so it remains empty after relogging.
- [ ] Test every hotbar row and distinguish clicking a slot from beginning a drag.

## Phase 3 — inventory, loot, trade, and vendors

### Inventory and trade quantities

- [ ] Replace or augment the existing ground-drop actions **Split half** and
      **Split off 1** with a clearly named quantity dialog. Do not label dropping
      part of a stack as an inventory-only split.
- [ ] Replace the existing **Trade one/all** choice with the same exact quantity
      control. Keep `/trade add <inventory index> [amount]` as a DM/debug path,
      not the normal player workflow.
- [ ] Make keyboard entry, arrow buttons, maximum quantity, cancellation, and
      invalid values behave consistently.
- [ ] Diagnose Blue Potion trading from the server response. Item 505 has no
      trade restriction in the current item database, so refusal is a client or
      transaction-path defect unless runtime data proves an override.
- [ ] Document the GM flow for giving an exact item quantity to another character
      and expose it in the DM command window.

### Floor items

- [ ] Preserve the existing behavior that walks to and picks up a clicked item.
      Add the requested area-loot action: when the player clicks a floor item
      within four cells, collect that item and all other reachable, eligible loot
      within four cells. Show which items are queued and stop at normal ownership,
      inventory, and weight restrictions.
- [ ] Cancel pending pickup when the item disappears, the player selects another
      action, combat begins, or the path fails.
- [ ] Clear pending pickup state before processing a player-initiated drop so the
      dropped item is not immediately collected again.
- [ ] Test two players racing for one item, blocked paths, full inventory, and
      overweight rejection.

### Vendor comparison and eligibility

- [ ] Reuse the existing inventory equipment-comparison tooltip in vendor buy
      rows, then show attack/MATK, defense, slots, refinement,
      weight, required level, allowed classes, and important bonuses.
- [ ] Compare it with the currently equipped item in the matching slot and show
      signed stat differences.
- [ ] Give equipment the same unusable visual state everywhere it appears:
      muted or red-tinted icon, a small blocked/equipment marker, and disabled
      Equip action where applicable. Use it in Inventory, vendor Buy, trade,
      cart/storage, and floor-loot previews without hiding the item.
- [ ] Put the exact reason in the tooltip: **Cannot equip: class**, **level**,
      **job**, **sex**, or **equipment location**. Extend the generated item data
      with Hercules job/upper/gender restrictions; the current export is not
      sufficient to decide every class restriction client-side.
- [ ] Verify buy/sell/cart transactions each execute exactly once.

Exit condition: players can compare equipment, understand restrictions, choose
exact stack quantities, and never trigger an unintended pickup or sale.

## Phase 4 — campaign quest clarity

Use two explicit quest models:

- Normal Ragnarok quests remain character-owned.
- Seal Cascade quest flags are currently synchronized only to eligible members
  who are online when a script advances. Carried quest items remain per-character,
  and turn-in consumes the speaking character's inventory. Add a durable party
  campaign checkpoint plus an explicit Session Board reconciliation action for
  reconnecting and late-joining members.

### Journal and overview

The current campaign's 41 ordinary hunts are item collection and turn-in quests;
its six single-boss quests use server target counters. The UI model should support
both without presenting collection drops as kill credit.

- [ ] Show active campaign quests in the character overview with title, current
      step, completion count, and a shortcut to the full journal.
- [ ] Update objectives immediately from quest add/update/remove packets and from
      party-wide campaign events.
- [ ] Give every objective an explicit type and icon: **Talk**, **Kill**,
      **Collect**, **Explore**, **Interact**, or **DM encounter**. Do not reduce a
      quest to its title and flavor text when structured objective data exists.
- [ ] For kill objectives, show the exact monster name and live count in
      `current / required` form. Also show the recommended map, whether the
      monster is normal, boss-type, or MVP, and whether kills are shared with the
      party.
- [ ] For collection objectives, show the required item and live inventory count,
      every known monster that can satisfy the campaign objective, and the
      recommended map. Keep natural loot and quest-specific bonus drops distinct
      so the player understands why the journal advanced.
- [ ] Label collection counts **You carry**. Display party quest state separately;
      never imply that individual inventories are one shared counter unless the
      server later supplies an authoritative aggregate.
- [ ] For talk, exploration, and interaction objectives, name the NPC or object,
      give a human-readable area, and add coordinates or a map marker when the
      story has revealed the location.
- [ ] Always show the turn-in NPC and return location once an objective is ready.
      A completed hunt should change to **Return to ...**, rather than remaining
      on its old monster instructions.
- [ ] Rewrite **Omens at the Fountain** as explicit current steps rather than a
      static premise. Each branch should say whom to speak to, what clue was
      found, what remains, and where the next lead begins.
- [ ] Show human-readable map and region names, coordinates where useful, and a
      clickable map marker for the next NPC, portal, or hunt zone.
- [ ] For collection objectives, list the monster source and recommended map once
      the quest giver has provided that information.
- [ ] Distinguish optional objectives, required objectives, completed steps, and
      DM-triggered encounter steps.
- [ ] Verify that reconnecting, changing maps, joining late, and a party member
      completing an objective all refresh the journal correctly.

The first vertical slice should include **Field Contract: Rockers and Rumors**
and present information in this form:

```text
Field Contract: Rockers and Rumors
Recommended area: Prontera West Field (prt_fild07)

Collect Grasshopper's Leg       7 / 10  — Rocker
Collect Animal Skin             4 / 10  — Savage Babe
Collect Rocker Doll             1 / 3   — Vocal (boss-type, rare spawn)

Turn in: Quartermaster Wynne — Prontera (156, 191)
You carry: counts shown above
Party quest state: synchronized while members are online
```

The required item names and counts already come from generated campaign data.
Extend that schema from `dm_hunt_db.json` with monster sources, monster rank, and
recommended maps. Put story-step NPCs, coordinates, reveal conditions, and
turn-in locations in a new authoritative guidance dataset; they are not present
in the current generated table. Never copy those facts into UI code by hand.

### Tracked-quest breadcrumb overlay

The journal is the detailed source; the overlay is the at-a-glance route while
playing. A player can track a quest or a single objective from Ctrl+Q, and the
HUD shows one primary route without requiring the journal to remain open.

- [ ] Show a compact chain of the revealed steps: **current objective → next
      destination → turn-in**. Never reveal a hidden story step before its flag
      or dialogue unlocks it.
- [ ] Keep the current objective specific: monster or NPC name, item/kill count,
      and remaining amount. For example, `Vocal — Rocker Doll 1/3` rather than
      only `Rockers and Rumors`.
- [ ] On the current map, show direction, tile distance, and a minimap marker for
      the selected monster area, NPC, object, or coordinate.
- [ ] Across maps, point to the next known portal and show the readable route,
      such as `Prontera → West Field → Labyrinth Forest`. Fall back to the target
      map name when a complete portal route is unavailable; never invent a path.
- [ ] When several objectives are open, let the player choose the active one from
      the overlay or journal. Completed objectives collapse and the next
      incomplete objective becomes active, without overriding a manual choice.
- [ ] Refresh progress, destination, and markers immediately when party progress
      changes, an item is picked up or dropped, a monster is killed, dialogue
      advances, the player changes maps, or the quest completes.
- [ ] Clicking the objective opens its journal entry; clicking the destination
      enables or disables guidance. Provide collapse, hide, scale, opacity, and
      HUD-position controls, and remember them per client.
- [ ] Keep the overlay out of dialogue, target, party, and hotbar interaction
      areas at supported laptop resolutions. It must not capture movement or
      combat clicks outside its visible controls.

### Portal-by-portal quest guidance

Cross-map guidance must continue through every map in the route. The full route
comes from the server's actual warp graph, following the architecture in
[navigation-quest-guiding.md](../specs/navigation-quest-guiding.md), rather than
from a hand-written list of directions.

- [ ] Build a route as ordered legs: current position → current-map portal → next
      map portal → ... → objective. Show the entire readable map sequence in the
      journal and only the current leg on the HUD.
- [ ] Highlight the next portal on the minimap and in the world. The guidance
      arrow and distance must point to that portal, not toward the final objective
      through an impassable map edge.
- [ ] As soon as the player changes maps, mark the previous leg complete and
      guide them to the next portal. On the destination map, switch guidance from
      portals to the exact NPC, object, or monster area.
- [ ] If the player takes a different exit, teleports, uses a Fly Wing, dies,
      respawns, or joins the party on another map, recompute from the new map and
      position without losing the tracked objective.
- [ ] Detect unavailable, one-way, level-gated, quest-gated, and instance-only
      warps. Exclude unusable edges when the client has enough state; otherwise
      label the uncertain step instead of sending the player into a dead end.
- [ ] Include custom DM warps and active instance entrances while they exist, and
      remove those route edges when the session or instance ends.
- [ ] Let the player expand the HUD to see upcoming legs, select a leg to inspect
      it on the world map, pause guidance, or choose another available route.
- [ ] Keep route guidance advisory: it never automatically enters a portal or
      moves the character, and manual movement always remains in control.

Cross-map acceptance case from Prontera to `prt_maze02`:

```text
Destination: Labyrinth Forest 2
Route: Prontera → West Field → Mt. Mjolnir → North Prontera Field
       → Labyrinth Forest 1 → Labyrinth Forest 2

Current leg: West Field — enter the north portal (292, 385)
```

After entering Mt. Mjolnir, the current leg must automatically change to its
east portal. Taking a wrong portal must produce a valid route from the new map,
not directions back to a stale coordinate.

Breadcrumb acceptance case:

```text
Rockers and Rumors
● Vocal — Rocker Doll 1/3
  412 tiles · Prontera West Field
  Route: current map → prt_fild07
  Then: Quartermaster Wynne, Prontera (156, 191)
```

After the third doll is collected, the same overlay must switch to:

```text
Rockers and Rumors — objectives complete
Return to Quartermaster Wynne
Prontera (156, 191)
```

Exit condition: a player can open Ctrl+Q after any Arc 1 interaction and answer
“what do we do next, which monsters or objects count, how far along are we,
where do we go, and who advances it?” without DM knowledge. With the journal
closed, the breadcrumb overlay provides the same immediate next action and route.

## Phase 5 — campaign progression rules

These changes affect balance and should ship together behind documented server
settings so they can be tuned without rebuilding the client.

### Recovery and respawn

- [ ] Verify sitting regeneration first, including the overweight suppression
      rules. Show a visible reason when regeneration is blocked.
- [ ] Add an explicit combat state for recovery. Taking damage, dealing damage,
      or using an offensive skill enters combat; the higher out-of-combat HP/SP
      rate begins after 8 seconds without one of those events. Support and healing
      skills should not restart the timer unless they affect an active combatant.
- [ ] While out of combat and standing, regenerate HP and SP substantially faster
      than the current natural rate. Keep the multiplier in Hercules configuration
      so it can be tuned without rebuilding Korangar.
- [ ] While sitting, restore 25% of maximum HP and 25% of maximum SP on every
      sitting recovery tick. Use a 10-second tick initially, display the active
      recovery state, and ensure the percentage is based on maximum rather than
      current HP/SP.
- [ ] Define stacking explicitly: sitting recovery replaces the standing
      out-of-combat tick instead of adding both rewards on the same tick. Clamp
      both resources at their maximum values.
- [ ] Document and expose the existing GM `@save` and `@load` flow. Add a
      player-accessible save NPC or optional per-session DM checkpoint where
      campaign pacing needs it.
- [ ] On respawn, restore 50% HP/SP immediately and regenerate the remaining 50%
      over 10 seconds. Damage should cancel or pause the recovery to prevent
      combat abuse.
- [ ] Test recovery while standing, sitting, overweight, dead, recently respawned,
      poisoned, in combat, and immediately after combat. Verify HP and SP from
      before/after totals rather than relying only on visual bars.

### Weight and encumbrance

- [ ] Measure ordinary session loot and consumable loads before selecting a new
      capacity.
- [ ] Recommended first pass: substantially raise carrying capacity, warn before
      the penalty threshold, allow attacks while overweight, and reserve severe
      penalties for a higher threshold.
- [ ] Keep item pickup and trading errors explicit when hard capacity is reached.
- [ ] Test regeneration, movement, attacks, skills, death, storage, trade, and
      cart operations at each threshold.

### Skill reallocation

- [ ] Keep GM `@skreset` as the immediate playtest workaround. Add an
      always-available player campaign respec through the DM window or a clearly
      placed NPC.
- [ ] Reset skills safely, preserve valid prerequisite allocation, and refresh
      the hotbar so removed skills cannot remain castable.
- [ ] Decide whether resets are free during playtests and later receive a Zeny or
      cooldown cost. Recommended playtest setting: free and unlimited.

Exit condition: recovery is fast enough for session pacing, encumbrance does not
disable core combat, and players can repair experimental builds without database
editing.

### Monster and party experience

Current server values are `base_exp_rate: 100`, `job_exp_rate: 100`,
`party_default_share: 7`, `party_even_share_bonus: 0`, and
`party_share_level: 15`. Even share is enabled for new parties, but there is no
global monster-rate increase and no extra party-size bonus.

- [ ] Choose explicit playtest multipliers for monster base EXP and job EXP and
      place the overrides in `conf/import/battle.conf` so Hercules updates do not
      erase them. Record the chosen multipliers in the player/DM guide.
- [ ] Make the party increase multiply EXP received, rather than merely dividing
      the unchanged reward. With bonus `B` percent per additional eligible member
      and `N` eligible members, each even share uses multiplier
      `1 + (B / 100) * (N - 1)`. This is equivalent to increasing the total pool
      before division except for Hercules' integer truncation.
- [ ] Decide the party multiplier from expected time-to-level, then configure
      `party_even_share_bonus`. Do not describe even division alone as an EXP
      increase.
- [ ] Decide whether the 15-level party share range fits the campaign. When it
      blocks even share, show a clear message naming the level-range rule.
- [ ] Verify solo and two/three-player kills against database EXP, global rates,
      attacker bonus, level penalty, party pool multiplier, and party division.
      Check both base and job EXP totals before and after each kill.
- [ ] Audit quest/script EXP separately through `quest_exp_rate` and authored
      `DM_PartyExp` awards. If the campaign multiplier applies to all XP given,
      set `quest_exp_rate` to that multiplier and prove an authored 1,000 EXP
      reward changes the character total by the intended amount exactly once.
- [ ] Decide whether the party-size bonus should also increase monster Zeny.
      Hercules currently applies `party_even_share_bonus` to shared Zeny as well
      as base/job EXP; implement an EXP-only setting if that is not desired.
- [ ] Keep Korangar's EXP gain notification and HUD totals consistent with the
      authoritative server result, including a level-up that rolls the bar over.

Exit condition: the selected monster and party EXP multipliers are active after
a server restart, documented, and proven by before/after character totals rather
than inferred from a toast alone.

## Phase 6 — information and presentation

### Chat and feedback

- [ ] Add a Combat chat channel with damage dealt/received, healing, status
      changes, skill failures, experience, and loot. Give each category its own
      toggle to avoid spam.
- [ ] Add restrained UI click sounds. Use one sound for activation and a distinct
      low-volume sound for rejection; provide independent volume and disable
      controls. Do not play a beep every frame while a button is held.

### Minimap and player identity

- [ ] Assign party members stable, distinct minimap colors and show a matching
      color beside their party-list entry.
- [ ] Let each player override colors locally and restore defaults.
- [ ] Show character name/class on mouseover and preserve privacy rules for hidden
      GMs.

### Cosmetics

- [ ] Treat current headgear rendering as the baseline. Inventory remaining hair,
      clothing palette, body-style, and costume gaps before exposing choices.
- [ ] Add cosmetics only after other players can see the same appearance and the
      choice persists across reconnects.

### World chests

Hercules already implements 38 authored Act I exploration chests across five
regions. Each is once per character, grants a persistent Field Note and a
Cartographer's Mark to the party pool when grouped, and awards a regional
cosmetic after all notes in that region. The other hidden achievement chests do
not currently share these campaign rewards.

- [ ] Give each chest a clear visual state: unopened, available, and opened.
- [ ] Explain the existing behavior in the journal or chat: Field Note count,
      personal discovery state, party-pooled mark, regional cosmetic progress,
      and the fact that an authored chest does not reset for that character.
- [ ] Surface the existing `@fieldnotes` and `@marks` information in the DM/player
      UI and verify the regional cosmetic renders and persists.
- [ ] Decide separately whether any of the remaining hidden chests should join
      the Act I reward system. Preserve the current per-character discoveries
      unless a campaign design change explicitly replaces them.

Exit condition: players know what chests are for, can identify party members,
and can separate combat feedback from normal conversation.

## Release gates

Every patch should pass the following before going to friends:

1. Clean install and upgrade install on Windows.
2. Manifest verification, interruption recovery, and small repair-package flow.
3. Two-client internet session covering WASD, targeting, spells, loot, trade,
   vendor sale, death/respawn, and reconnect.
4. Arc 1 party quest run from Wynne through at least one updated objective and
   turn-in, including one late join or reconnect.
5. Solo and party monster EXP totals match the configured multipliers and share
   rules.
6. No new client errors, map-server errors, duplicated transactions, or lost
   inventory/Zeny across restart.

## Suggested patch sequence

1. **Hotfix:** Increase AGI, updater repair, equipped-item sale guard.
2. **Combat patch:** failed-spell feedback, targeting/Tab, hotbar clearing, WASD
   reconciliation.
3. **Inventory patch:** stack quantities, Blue Potion trade diagnosis, floor loot,
   vendor comparison and class eligibility.
4. **Campaign patch:** party quest synchronization, dynamic objectives, overview,
   tracked-quest breadcrumb overlay, locations and markers.
5. **Rules patch:** monster/party EXP, regeneration, respawn, weight, respec,
   optional retaliation.
6. **Presentation patch:** combat log, player/minimap colors, UI sounds, cosmetics,
   and world chests.
