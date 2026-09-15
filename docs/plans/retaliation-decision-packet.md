# Retaliation Decision Packet (QW-038)

**Status:** Recorded as Human Decision Gate (Awaiting User Sign-off)
**Implementation State:** NOT IMPLEMENTED (Blocked on human approval per QW-038 specification)

---

## 1. Overview & Motivation

In traditional Ragnarok Online clients, when an idle player is attacked by a monster, the client can either remain passive (requiring the player to manually click or Tab-target the aggressor) or optionally acquire the attacking monster to retaliate.

For accessibility and combat convenience, an opt-in auto-retaliation feature has been requested. However, automated combat actions can disrupt kiting, positioning, spell-casting, party coordination, and MVP tactics if not strictly bounded. Therefore, this document establishes the precise behavioral specification and records the formal human decision gate prior to any implementation.

---

## 2. Core Principles & Safety Invariants

If approved, the auto-retaliation mechanism must adhere to the following non-negotiable invariants:

1. **Strictly Opt-In (Default OFF)**:
   - Setting: `auto_retaliate: bool = false` stored in `GameSettings`.
   - Toggleable via the Game Settings interface.
   - Persisted across game sessions via settings serialization.

2. **Idle-Only Trigger**:
   - The local player must be completely idle.
   - Must **never** trigger if:
     - The player is moving (WASD keys held or mouse movement in flight).
     - The player is casting a spell (`active_cast.is_some()`).
     - The player is engaged in dialogue with an NPC or shop window.
     - The player is sitting / resting (`/sit`).

3. **Never Override Existing Intent**:
   - Must **never** override an existing target (`last_skill_target.is_some()` or active combat lock).
   - If the player has already targeted an entity (monster, player, or ground), retaliation is suppressed.

4. **Hostile Monster Source Only**:
   - Must only trigger on attacks originating from `EntityType::Monster`.
   - Never triggers on player damage (PvP/GvG), environmental hazards, or indirect traps unless specifically configured.
   - Target monster must be alive (`!is_dead`), not fading (`!is_fading`), and not hidden (`!is_hidden()`).

5. **User Priority & Immediate Abortion**:
   - Any user input (mouse click, WASD keypress, hotbar activation, ESC, or manual target change) takes immediate, 100% priority and cancels any pending retaliation action without latency.

6. **Range & Pathing Safety**:
   - Retaliation only acquires target within weapon attack range (or direct line of sight up to standard visible range).
   - Never initiates unattended long-distance pathfinding across cliffs, walls, or through dangerous mob packs.

7. **Debounce & Anti-Spam**:
   - Triggered only once per incoming damage packet sequence; does not flood outgoing `Action` packets.

---

## 3. Technical Architecture (Proposed)

### A. Configuration (`korangar-config` / `korangar`)
Add setting to `GameSettings`:
```rust
pub struct GameSettings {
    // ...
    pub auto_retaliate: Cell<bool>, // default: false
}
```

### B. Ingress Damage Signal
In the network packet dispatcher (`handle_network_events` processing `DamageEffect` or `Action(Attack)`):
```rust
if self.client_state.follow(client_state().game_settings().auto_retaliate()) {
    if damage.destination_entity_id == local_player_id {
        self.evaluate_retaliation_candidate(damage.source_entity_id);
    }
}
```

### C. Guard Predicate
```rust
fn evaluate_retaliation_candidate(&mut self, attacker_id: EntityId) {
    if self.is_player_busy() || self.has_active_target() {
        return;
    }
    let Some(attacker) = self.find_entity(attacker_id) else { return; };
    if attacker.get_entity_type() != EntityType::Monster
        || attacker.is_dead()
        || attacker.is_fading()
        || attacker.is_hidden() {
        return;
    }
    if !self.is_within_engagement_range(attacker) {
        return;
    }
    // Set target / queue retaliation action
    self.set_active_target(attacker_id);
}
```

---

## 4. Verification & Testing Strategy

1. **Deterministic Unit Tests**:
   - `retaliate_ignored_when_disabled`: disabled setting ignores damage.
   - `retaliate_ignored_when_moving`: active walk path ignores damage.
   - `retaliate_ignored_when_casting`: active spell cast ignores damage.
   - `retaliate_ignored_when_target_already_set`: existing target is preserved.
   - `retaliate_ignored_for_non_monster`: player/NPC damage does not trigger.
   - `retaliate_aborts_on_wasd_input`: WASD input clears pending retaliation.

2. **Automated Headless Scenario**:
   - Add a scenario where a test mob strikes the player while idle; verify packet sequence `0x0089` (Action::Attack) is emitted only once when enabled, and omitted when disabled.

---

## 5. Human Decision Gate

- **Decision Options**:
  1. **Option A: Approve Implementation**: Implement the opt-in auto-retaliation system according to the above specification in a subsequent task.
  2. **Option B: Reject Implementation**: Retain strict manual-only targeting (Tab-to-target and click-to-attack) to preserve 100% classic client fidelity.

- **Gate Status**: Recorded as pending human sign-off. Implementation will not proceed until explicitly approved by the user.
