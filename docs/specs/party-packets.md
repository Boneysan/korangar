# Targeted Spec — Party Packet Promotion (Critical for DM)

**Parent docs**: [FEATURE_ROADMAP.md](../FEATURE_ROADMAP.md) §8.3 (High priority), [plans/packet-gap-party-whisper.md](../plans/packet-gap-party-whisper.md), [PACKET_EVENTS_CATALOG.md](../PACKET_EVENTS_CATALOG.md), [DM_CLIENT_IMPLEMENTATION.md](../DM_CLIENT_IMPLEMENTATION.md), [DM_INTERFACE.md](../DM_INTERFACE.md) (party frames, initiative, shared state for DM).

**Why this spec**: The Seal Cascade campaign is **party-locked** (`$dm_active_party`, `DM_RequireDM`). Without framing + basic data for party roster, HP/SP/position updates, and chat, many DM tools (initiative tracker, party frames, downed overlays, shared pings) are blocked or risky. Length fallbacks protect framing today, but we need modeled packets for events/state/UI.

**Scope (Phase 1 safety + minimal events)**:
- Define missing modern party packets in `ragnarok-packets`.
- Register (at minimum noop + unknown logging; promote key ones to events).
- Add minimal `NetworkEvent` variants for roster/HP/position.
- No full UI yet (party frames come later); focus on data availability for DM state.
- Whisper as companion (used by `@dmsecret` potentially).

**Out of scope for this slice**: Full guild, full party options/config, rich UI frames (post this spec).

**Verification target**: Form/join party on Hercules (PACKETVER 20190605 / client 20220406), see no unknown packets in inspector, receive roster + HP updates + position + party chat.

## 1. Packet Layouts (from Hercules_RO for 20220406 main)

Use these for `ragnarok-packets` structs. Lengths from `packets2022_len_main.h` (or generated lengths_20220406.rs).

### Modern Roster / Member (preferred for 20220406)
**0x0AE4** `ZC_ADD_MEMBER_TO_GROUP` / partymemberinfo — **fixed 89 bytes**

From `src/map/packets_struct.h`:
```c
struct PACKET_ZC_ADD_MEMBER_TO_GROUP {
    int16 packetType;           // 0x0AE4
    uint32 AID;
#if PACKETVER >= 20171207
    uint32 GID;
#endif
    uint32 leader;
#if PACKETVER_MAIN_NUM >= 20170524 || ... 
    int16 class;
    int16 baseLevel;
#endif
    int16 x;
    int16 y;
    uint8 offline;
    char partyName[NAME_LENGTH];      // 24
    char playerName[NAME_LENGTH];     // 24
    char mapName[MAP_NAME_LENGTH_EXT];
    int8 sharePickup;
    int8 shareLoot;
} __attribute__((packed));
```

**0x0AE5** `ZC_GROUP_LIST` / partyinfo — **variable length**

```c
struct PACKET_ZC_GROUP_LIST_SUB {
    uint32 AID;
#if PACKETVER >= 20171207
    uint32 GID;
#endif
    char playerName[NAME_LENGTH];
    char mapName[MAP_NAME_LENGTH_EXT];
    uint8 leader;
    uint8 offline;
#if ... modern
    int16 class;
    int16 baseLevel;
#endif
};

struct PACKET_ZC_GROUP_LIST {
    int16 packetType;   // 0x0AE5
    int16 packetLen;
    char partyName[NAME_LENGTH];
    struct PACKET_ZC_GROUP_LIST_SUB members[];
} __attribute__((packed));
```

### Invite / State
- `PartyInvitePacket` (0x02C6) — already defined (party_id + party_name). Currently noop.
- `UpdatePartyInvitationStatePacket` (0x02C9) — already defined, noop.

### Older fallbacks (for compatibility)
There are conditional aliases (0x0A43 etc.), but for our server focus on 0x0AE* + 0x02C6.

### Whisper (companion)
**CZ → server (send)**: `0x0096` `CZ_WHISPER` (variable)
- Typically: packetLen, target name[24], message...

**ZC ← server (receive)**: `0x09DE` `ZC_WHISPER` (variable, modern)
```c
struct PACKET_ZC_WHISPER {
    int16 PacketType;     // 0x09DE
    int16 PacketLength;
    uint32 senderGID;
    char sender[NAME_LENGTH];
    uint8 isAdmin;
    char message[];
} __attribute__((packed));
```

Result ack: `0x0098` (small result code).

See hercules-20220406.md for lookup.

## 2. Proposed Additions to ragnarok-packets

Add to `ragnarok-packets/src/lib.rs` (near other social packets):

```rust
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(RustState, StateElement))]
#[header(0x0AE4)]
pub struct PartyMemberInfoPacket {  // or ZcAddMemberToGroup
    pub account_id: AccountId,
    #[cfg_attr(..., conditional)] pub character_id: CharacterId, // GID
    pub leader: u32,  // or bool
    // class, base_level conditional on PACKETVER but for now include or use versioned
    pub position: TilePosition,  // x,y
    pub offline: u8,
    #[length(24)] pub party_name: String,
    #[length(24)] pub player_name: String,
    #[length(16)] pub map_name: String,  // adjust
    pub share_pickup: u8,
    pub share_loot: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(RustState, StateElement))]
#[header(0x0AE5)]
#[variable_length]
pub struct PartyListPacket {  // ZcGroupList
    #[length(24)] pub party_name: String,
    #[repeating_remaining]
    pub members: Vec<PartyMemberInfo>,  // define sub-struct matching SUB
}

#[derive(Debug, Clone, ByteConvertable, ...)]
pub struct PartyMemberInfo { ... } // the SUB struct

// Whisper
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[header(0x0096)]
#[variable_length]
pub struct WhisperSendPacket {
    #[length(24)] pub target: String,
    #[length_remaining] pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[header(0x09DE)]
#[variable_length]
pub struct WhisperReceivePacket {
    pub sender_account_id: AccountId,  // or GID
    #[length(24)] pub sender_name: String,
    pub is_admin: u8,
    #[length_remaining] pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[header(0x0098)]
pub struct WhisperResultPacket {
    pub result: u8,  // 0=success etc.
}
```

Add `PartyMember` or reuse/extend `Friend` style if possible. Use `#[cfg]` or version handling only if needed; for now model the 20220406 shapes.

Add `PartyChatPacket` if `0x0109` (ZC_NOTIFY_CHAT_PARTY) is variable: account_id + message.

## 3. NetworkEvent Additions (korangar-networking/src/event.rs)

```rust
// In enum NetworkEvent
PartyList {
    party_name: String,
    members: Vec<PartyMemberData>,
},
PartyMemberAdded { member: PartyMemberData },
PartyMemberRemoved { account_id: AccountId },
PartyMemberUpdate { /* hp, pos, map, etc. */ },
PartyChatMessage { account_id: AccountId, text: String },

WhisperReceived {
    sender_id: AccountId,
    sender_name: String,
    is_admin: bool,
    message: String,
},
WhisperResult { success: bool },
```

Define `PartyMemberData { account_id, char_id, name, map, position: Option<TilePosition>, leader: bool, online: bool, job: JobId, level: u16, hp: Option<(u32,u32)> , ... }`

Keep minimal at first.

## 4. Handler Registration (version_20220406.rs)

In `register_map_server_packets`:

```rust
// Party
packet_handler.register(|packet: PartyListPacket| { /* build event */ NetworkEvent::PartyList { ... } })?;
packet_handler.register(|packet: PartyMemberInfoPacket| NetworkEvent::PartyMemberAdded { ... })?;

// Existing noops → promote:
packet_handler.register(|_: UpdatePartyInvitationStatePacket| NoNetworkEvents)?; // or event
packet_handler.register(|packet: PartyInvitePacket| NetworkEvent::PartyInvite { ... })?;

// Whisper
packet_handler.register(|packet: WhisperReceivePacket| NetworkEvent::WhisperReceived { ... })?;
packet_handler.register(|packet: WhisperResultPacket| ... )?;

// Client send side later
```

For packets that are only framing today (like some member HP updates), start with `register` that emits if data is useful, else keep noop + comment.

Update `register_length_fallbacks` will ignore once registered.

## 5. Consumption in korangar/src/lib.rs

In the big match:

```rust
NetworkEvent::PartyList { .. } => {
    // Update dm_state or a new party_state
    self.client_state.follow_mut(client_state().party_list()).replace(...);
},
// Similarly for updates, chat (can feed into general ChatMessage or dedicated)
```

For DM: route to `dm::parser` or directly to `dm_state.party`.

Clear on map disconnect / party leave.

## 6. Outgoing (NetworkingSystem)

Add methods:

```rust
pub fn create_party(&mut self) -> ... { /* if needed */ }
pub fn invite_to_party(&mut self, name: String) ...
pub fn leave_party(&mut self) ...
pub fn send_party_chat(&mut self, text: String) { /* use specific packet if 0x0108 exists, else Global? */ }
pub fn send_whisper(&mut self, target: String, text: String) { send WhisperSendPacket }
```

Most party management can be via `@commands` initially for DM, but proper packets unlock native frames.

## 7. Implementation Steps & Tests

1. Add structs + derives + tests (roundtrip fixed + var length with 2+ members) in ragnarok-packets.
2. Add to `SupportedPacketVersion` handling if needed (single version now).
3. Register in version_20220406 (promote the invite one at minimum).
4. Add minimal events + From impls if using appear-style.
5. Handle in lib.rs (at least log or DM state update).
6. Add send methods.
7. Packet history / inspector should render them nicely (via Packet derive).
8. Live test: two clients, form party via script or native, move, change HP (server config), party chat. Verify no desync + events fire.
9. Update FEATURE_ROADMAP backlog, PACKET_CATALOG, this spec.

Add unit tests in the packets crate.

## 8. Risks & Notes

- Variable length + repeating members: use `#[repeating_remaining]` carefully; test with 1 and 4 members.
- Name encoding: UTF-8 handling (korangar has unicode feature).
- HP/SP updates may come in separate packets (0x0AE6?); capture live.
- Position updates for party members on map (for minimap / in-world).
- For DM `@dmsecret`: verify if it uses real whisper (0x09DE) or just dispbottom/chat. If whisper, this unblocks it.
- Older packet variants: model the ones our server actually emits.
- Once modeled, full party frames (HP bars, etc.) and shared DM state become possible.

## 9. Cross-Refs

- Capture workflow: hercules-20220406.md
- Lengths: tools/generate... + lengths_20220406.rs
- DM usage: DM_CLIENT_IMPLEMENTATION.md §5 (shared party coordination)
- UI later: party frames share rendering with initiative/downed.

Promote these before first real multi-player DM session.

This spec is ready to execute once M0 is solid and a capture session is run.