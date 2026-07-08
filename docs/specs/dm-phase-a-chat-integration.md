# Targeted Spec — DM Phase A Chat Integration (Parser + Command Emitter)

**Parents**: [DM_INTERFACE.md](../DM_INTERFACE.md) §9.3, [DM_CLIENT_IMPLEMENTATION.md](../DM_CLIENT_IMPLEMENTATION.md) §4, [PACKET_EVENTS_CATALOG.md](../PACKET_EVENTS_CATALOG.md) (Chat family + GlobalMessagePacket), [DM_SERVER_FUNCTIONS.md](../DM_SERVER_FUNCTIONS.md), [FEATURE_ROADMAP.md](../FEATURE_ROADMAP.md) E7.1–E7.3.

**Goal**: Implement the **zero-packet-change** transport for DM features. All commands emitted as ordinary chat; results consumed from `ChatMessage` events + `QuestEffectPacket`. Structured `[DMJ]` echoes make parsing reliable.

This is the highest-leverage starting point for E7 (dice cards, journal, inspiration, basic trackers) because it works immediately after M1 using existing `NetworkEvent::ChatMessage`, `send_chat_message`, `AddQuestEffect`, and quest packets.

## 1. Transport Contract (Phase A)

**Client → Server (Commands)**:
- Use existing `networking_system.send_chat_message(player_name, "@dm ...")`.
- Internally builds `GlobalMessagePacket { message: "Player : @dm check ..." }` (header 0x00F3, `length_remaining_off_by_one`).
- Confirmed in `korangar-networking/src/lib.rs:send_chat_message`.

**Server → Client (Feedback)**:
- Human-readable text arrives as `NetworkEvent::ChatMessage { text, color }`.
- When `$dm_client_mode` (script flag), parallel machine-readable line:
  ```
  [DMJ]{"t":"check_result","player":"Wynne","stat":"str","roll":18,"mod":4,"dc":15,"success":true,"nat":null,"timestamp":...}
  ```
- Quest/hazard visuals via existing `QuestEffectPacket` (0x0446) — `AddQuestEffect` / `RemoveQuestEffect`.
- Other state (initiative, flags, beats) via `[DMJ]` or future packets.

Parser must:
- Detect and consume `[DMJ]` lines (do not show raw in normal chat, or rewrite nicely).
- Feed `dm_state`.
- Still allow raw chat to flow for normal play + DM narrative.

## 2. Recommended Code Placement

**New module** (per DM_CLIENT_IMPLEMENTATION.md):
- `korangar/src/dm/parser.rs`
- `korangar/src/dm/commands.rs` (emitter)
- `korangar/src/dm/state.rs` (DmState)

**Integration points in core (keep tiny)**:
- `korangar/src/lib.rs` in `handle_network_events` for `ChatMessage` and `AddQuestEffect`.
- `ClientState` gets `dm_state: DmState` field.
- Optional: dedicated DM chat log or filter.

## 3. Command Emitter (dm/commands.rs)

```rust
use crate::networking::NetworkingSystem;

pub struct DmCommandEmitter<'a> {
    networking: &'a mut NetworkingSystem<...>,
    player_name: String,
}

impl DmCommandEmitter<'_> {
    pub fn send(&mut self, cmd: &str) -> Result<(), NotConnectedError> {
        // cmd can be "@dm check Wynne str 15" or full
        self.networking.send_chat_message(&self.player_name, cmd)
    }

    // Convenience
    pub fn check(&mut self, target: &str, stat: &str, dc: u8, adv: Option<&str>) {
        let mut c = format!("@dm check {} {} {}", target, stat, dc);
        if let Some(a) = adv { c.push_str(&format!(" {}", a)); }
        let _ = self.send(&c);
    }

    pub fn hazard(&mut self, name: &str, x: i16, y: i16, map: &str) {
        let _ = self.send(&format!("@dmhazard {} {} {} {}", name, x, y, map));
    }

    // ... beat, decide, inspire, reward, etc.
}
```

Usage from a future DM window:
```rust
if let Some(emitter) = self.dm_emitter.as_mut() {
    emitter.check("Wynne", "dex", 18, Some("adv"));
}
```

Store the emitter or player_name + &mut networking when the DM window is active.

## 4. Incoming Parser (dm/parser.rs)

```rust
use serde::Deserialize;
use crate::state::{ChatMessage, MessageColor};
use super::state::{DmState, CheckResult, ...};

#[derive(Deserialize, Debug)]
#[serde(tag = "t")]
enum DmjMessage {
    #[serde(rename = "check_result")]
    CheckResult {
        player: String,
        stat: String,
        roll: i32,
        #[serde(rename = "mod")] modifier: i32,
        dc: i32,
        success: bool,
        nat: Option<String>,  // "20" or "1"
        // ...
    },
    #[serde(rename = "initiative")]
    Initiative { order: Vec<String> },
    #[serde(rename = "hazard")]
    Hazard { id: String, active: bool, x: i16, y: i16, /* ... */ },
    // beats, flags, downed, etc.
}

pub fn try_parse_dmj(text: &str) -> Option<DmjMessage> {
    text.strip_prefix("[DMJ]").and_then(|j| serde_json::from_str(j.trim()).ok())
}

/// Call this from lib.rs on every ChatMessage before (or instead of) pushing raw.
pub fn process_incoming_chat(text: String, color: MessageColor, dm_state: &mut DmState, chat_log: &mut Vec<ChatMessage>) {
    if let Some(dmj) = try_parse_dmj(&text) {
        match dmj {
            DmjMessage::CheckResult { player, roll, modifier, dc, success, nat, .. } => {
                let result = CheckResult { player, roll, modifier, dc, success, nat };
                dm_state.record_check(result.clone());
                // Optional: push a nicer formatted message instead of raw
                chat_log.push(ChatMessage::new(
                    format!("🎲 {} rolled {}+{} vs {} → {}", result.player, result.roll, result.modifier, result.dc, if result.success {"PASS"} else {"FAIL"}),
                    MessageColor::Information
                ));
            }
            DmjMessage::Initiative { order } => {
                dm_state.set_initiative(order);
            }
            DmjMessage::Hazard { id, active, x, y, .. } => {
                if active {
                    dm_state.add_or_update_hazard(id, x, y);
                } else {
                    dm_state.remove_hazard(&id);
                }
            }
            // ...
        }
        return; // suppress raw [DMJ] from normal chat if desired
    }

    // Normal or human DM text
    chat_log.push(ChatMessage::new(text, color));

    // Heuristic fallback parsing for non-[DMJ] servers (optional, brittle)
    if text.contains("rolled") && text.contains("vs DC") {
        // parse and call dm_state.record_check(...)
    }
}
```

Also handle `QuestEffectPacket` directly for hazards (already does `particle_holder.add_quest_icon`).

## 5. Integration in lib.rs (handle_network_events)

Around the `ChatMessage` arm (currently ~line 1578):

```rust
NetworkEvent::ChatMessage { text, color } => {
    let chat_messages = self.client_state.follow_mut(client_state().chat_messages());
    let dm_state = self.client_state.follow_mut(client_state().dm_state());  // after adding field

    crate::dm::parser::process_incoming_chat(text, color, dm_state, chat_messages);
}
```

For quest effects (already present):

```rust
NetworkEvent::AddQuestEffect { quest_effect } => {
    if let Some(map) = &self.map {
        self.particle_holder.add_quest_icon(&self.texture_loader, map, quest_effect);
    }
    if let Some(dm) = ... {
        dm.ingest_quest_effect(quest_effect);  // position + effect type for hazards
    }
}
```

Clear DM state on map change / disconnect (like `status_effects.clear()`).

Add tick for any timers in DmState (inspired by status_effects).

## 6. DmState Sketch (dm/state.rs)

```rust
#[derive(Default, RustState, StateElement)]
pub struct DmState {
    pub enabled: bool,
    pub active_party: Vec<String>,
    pub initiative_order: Vec<InitiativeEntry>,
    pub last_checks: VecDeque<CheckResult>,   // bounded for dice cards
    pub hazards: HashMap<String, Hazard>,     // id -> {pos, type, ttl?}
    pub flags: HashMap<String, bool>,
    pub current_beat: Option<u32>,
    pub inspiration: u8,
    pub downed_players: HashMap<String, DownedInfo>,
}

impl DmState {
    pub fn record_check(&mut self, r: CheckResult) { self.last_checks.push_front(r); self.last_checks.truncate(12); }
    pub fn tick(&mut self) { /* hazard ttls etc. */ }
    // ...
}
```

Add to `ClientState`:
```rust
#[hidden_element]
dm_state: DmState,
```

Provide path accessor: `client_state().dm_state()`

Expose via `client_state!` macro if used elsewhere.

## 7. UI Consumers (Phase A examples)

- **Dice cards**: Bind a window or overlay to `dm_state.last_checks`. Render animated cards on new entry. Trigger from chat parse or direct.
- **Hazard telegraphs**: `dm_state.hazards` + `QuestEffect` positions → custom particles or markers in world render / particle_holder.
- **Journal**: Use quest packets + generated campaign data (E7.3).
- **Basic initiative bar**: `dm_state.initiative_order` (even if populated from parsed text initially).

Windows live in `interface/windows/dm/`. Open via `self.interface.open_window(DmCheckConsole::new(client_state().dm_state()))` from menu or hotkey (DM-only).

## 8. Server-Side Prerequisite (E7.1)

In `Hercules_RO/npc/custom/dm_campaign/shared/*.txt` (or dm_console):

- Add `$dm_client_mode` flag (set via `@dm mode client` or always on for the group).
- On key outputs (checks, initiative, hazards, beats):
  ```c
  if ($dm_client_mode) {
      dispbottom sprintf("[DMJ]%s", json);
  }
  dispbottom "Human readable...";   // or announce
  ```

Use a simple json builder or `json_encode` if available in the Hercules build.

## 9. Testing & Edge Cases

- Mixed chat: normal player chat, @commands, [DMJ], narrative `@dmsay`.
- Long variable-length messages.
- Multiple rapid checks (deque bound).
- Map change clears transient DM state.
- GM vs player: emitter only available when appropriate (or server rejects).
- Packet log + inspector must still show raw packets.
- Fallback heuristic for servers without [DMJ] (nice-to-have).

Manual test script (E9): login → party → `@dm check` → observe card + clean chat.

## 10. Next Steps After This Spec

1. Implement `dm/` skeleton + basic parser/emitter for checks.
2. Wire the two integration points in lib.rs.
3. Add `DmState` field + clear logic.
4. Simple dice card widget (can be a floating element or new window).
5. Server script change for at least check results.
6. Then layer on journal, hazards (QuestEffect already works), initiative.
7. Later: promote party packets (see companion spec) for richer roster data.

This delivers immediate value for players (nice dice feedback) and DMs (structured data) with almost no risk to the base client.

Update `DM_CLIENT_IMPLEMENTATION.md`, `PACKET_EVENTS_CATALOG.md` (chat section), and E7 tasks when landed.