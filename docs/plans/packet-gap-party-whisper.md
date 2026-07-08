# Implementation Plan — Party & Whisper Packet Gap

| | |
|---|---|
| **Status** | Implemented and live-validated (2026-07-08, two clients) |
| **Milestone** | Phase 1 protocol safety, prerequisite for M2 group play |
| **Parent** | [SOFTWARE_DESIGN.md](../SOFTWARE_DESIGN.md) §5.3–§5.4, [FEATURE_ROADMAP.md](../FEATURE_ROADMAP.md) §8.3, [hercules-20220406.md](../protocol/hercules-20220406.md) |
| **Depends on** | M0 connectivity and packet inspector working |

## 1. Scope

Define and register missing party and whisper packet families so Korangar can
frame traffic safely before the first real party session.

This plan started as protocol safety and now tracks the remaining live
validation:
- Packet structs are defined in `ragnarok-packets`.
- Dedicated handlers are registered in `version_20220406.rs`.
- Minimal events and client `PartyState` are available for verification and
  future party frames.
- Slash commands exist for create/invite/accept/reject/leave, party chat, and
  whisper.
- Real party/whisper UI remains later work.

## 2. Why This Comes Early

Korangar frames incoming packets by deserializing the registered packet type.
Generated length fallbacks now protect the stream from unknown known-length
packets, but modeled party packets are still needed for state, UI, and DM tools.
Party traffic matters because the DM campaign is party-locked through
`$dm_active_party`.

Minimum target: every party/whisper packet emitted by a normal Hercules group
session is modeled or safely framed, with the core roster/chat/whisper packets
promoted to real events.

## 3. Packet Families

### Party

Implemented areas:
- Party roster/list.
- Party member HP updates.
- Party member position updates.
- Party member job/level updates.
- Party chat (`0x0108` / `0x0109`).
- Create, invite, accept/reject, leave, and invite-block commands.
- 20220406 main roster/member packets are `0x0AE5` / `0x0AE4`; older
  `0x00FB` roster notes are not correct for this server build.

Still later:
- Real party frames.
- Party leader change and kick UI.
- SP updates if the server/client variant emits them; 2022 main HP packet does
  not include SP.

### Whisper

Implemented areas:
- Client → server whisper send: `0x0096`.
- Server → client whisper receive: `0x09DE`.
- Server → client whisper result: legacy `0x0098` and modern `0x09DF`.

Still later:
- `/r` reply state and whisper history.

DM dependency:
- Verify whether `@dmsecret` sends true whispers or `dispbottom`; if true
  whispers are used, this becomes a DM-interface prerequisite.

## 4. Files Touched

- `ragnarok-packets/src/**`
- `korangar-networking/src/packet_versions/version_20220406.rs`
- `korangar-networking/src/event.rs` if adding minimal observable events
- `korangar-networking/src/lib.rs` if adding outgoing whisper/party send methods
- Focused packet tests in the relevant crate

## 5. Steps

1. Capture ground truth.
   - Start Hercules with packet inspector enabled in Korangar.
   - Form a party using two Korangar clients once possible, or a matching
     `20220406` official client if one is obtained. The existing 2019 official
     client cannot connect to the rebuilt `20220406` server.
   - If the client cannot create parties yet, use Hercules-side commands or a
     temporary script to put two online characters into a party and trigger the
     server's normal update packets.
   - Capture unknown packet headers and surrounding server log context.
   - Repeat for whisper send/receive.

2. Cross-check Hercules packet definitions.
   - Start from [hercules-20220406.md](../protocol/hercules-20220406.md), then
     verify against the local Hercules source.
   - Find `PACKETVER=20220406` packet layouts in Hercules packet DB/source.
   - Record opcode, length, direction, and field layout for each party/whisper
     packet observed.

3. Add packet structs.
   - Implement byte layouts in `ragnarok-packets`.
   - Prefer semantic field names over raw buffers where the layout is known.
   - Use raw/opaque tail fields only as a temporary framing measure and document
     what remains unknown.

4. Add tests.
   - Round-trip fixed-length packets.
   - Parse representative variable-length packets.
   - Include at least one packet with multiple party members or message text.

5. Register packets for `20220406`.
   - Noop-register packets needed only for framing.
   - Add real handlers only for data required by immediate verification.
   - Ensure unknown-packet logs no longer appear for ordinary party/whisper flows.

6. Add minimal outgoing sends.
   - Done for whisper, party chat, create, invite, accept/reject, leave, and
     invite-block toggle.

7. Verify against Hercules.
   - Two accounts online.
   - Create or join a party.
   - Move both characters.
   - Change HP on one character.
   - Send party chat.
   - Send and receive a whisper.

### Live validation result (2026-07-08)

Two Korangar clients (`test` in izlude, `test2` in int_land — deliberately
cross-map) against local Hercules 20220406:

- `/party create SealCascade` → "Party successfully created.", row in `party`
  table, both chars got `party_id`.
- `/party invite test2` by name worked cross-map; invitee saw
  "Party invite from SealCascade. Use /party accept or /party reject."
- `/party accept` → leader saw "test2 accepted the party invite."
- `[Party]` chat delivered in both directions, cross-map.
- Whispers delivered in both directions, cross-map.

Issues found and fixed during validation:
- Server rejections were **silent** in the client. The party-create rejection
  under `basic_skill_check` actually arrives as a skill-fail
  (`ZC_ACK_TOUSESKILL` `0x0110`, skill 1 / cause 0), and other rejections use
  `ZC_MSG` (`0x0291`, msgstringtable id). Both are now promoted to chat-line
  messages (plus `0x09CD` `ZC_MSG_COLOR`); see `message_table_text` /
  `skill_failed_text` in `version_20220406.rs`.
- `basic_skill_check: false` is now set on the server so party/trade/sit are
  not gated behind Novice Basic Skill for the campaign
  (`Hercules_RO/conf/import/battle.conf`).
- `127.0.0.1` is allow-listed in `Hercules_RO/conf/import/socket.conf`: local
  clients reconnecting after a server restart repeatedly tripped the DDoS
  guard, which then blocked logins *and* the map↔char inter-server link.
- `LoginFailedPacket2` (`0x083E`) was modeled as 3 bytes but Hercules sends 26
  (u32 error code + 20-byte block date); fixed, plus two networking-thread
  robustness fixes (stale-task abort, no panic on instant connect failure)
  found while reproducing login failures.

**Open loose end — rejection messages not yet demoed live.** The new
rejection-message path (0x0110 / 0x0291 / 0x09CD → chat lines) is verified
against Hercules source only: packet layouts, field semantics, and the exact
party-create rejection (`clif->skill_fail(sd, 1, USESKILL_FAIL_LEVEL, 4, 0)`,
i.e. skill 1 / cause 0) were checked in `clif.c` / `packets_struct.h`, but the
live end-to-end demo did not happen (the test client was past the drivable
point and keyboard input cannot be injected under WSLg). It will confirm
itself the first time any rejection occurs in play (e.g. a skill without
enough SP). For a deliberate test: set `basic_skill_check: true` in
`Hercules_RO/conf/import/battle.conf`, restart the servers, and
`/party create Foo` should print the red "You need to learn the basic skills
first." line — then set it back to `false` (the campaign setting).

## 6. Acceptance Criteria

- No unknown/unhandled packets during normal two-player party setup, movement,
  HP change, party chat, and whisper.
- Packet inspector can still parse later packets in the same read after each new
  packet family appears.
- Added packet structs have tests.
- Any noop registration is listed in [FEATURE_ROADMAP.md](../FEATURE_ROADMAP.md)
  §8.3 or a follow-up plan with the reason it is still noop.

## 7. Risks

- Party packet layouts may differ across PACKETVER variants with similar names.
- Variable-length string encoding may expose legacy RO encoding edge cases.
- A raw framing-only packet can hide real data needed later; keep those temporary.
- Official-client packet captures are 2019 unless a 2022 exe is obtained, so use
  Hercules 20220406 definitions as authoritative for layouts.
