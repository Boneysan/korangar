# Implementation Plan — Party & Whisper Packet Gap

| | |
|---|---|
| **Status** | Draft implementation plan |
| **Milestone** | Phase 1 protocol safety, prerequisite for M2 group play |
| **Parent** | [SOFTWARE_DESIGN.md](../SOFTWARE_DESIGN.md) §5.3–§5.4, [FEATURE_ROADMAP.md](../FEATURE_ROADMAP.md) §8.3, [hercules-20220406.md](../protocol/hercules-20220406.md) |
| **Depends on** | M0 connectivity and packet inspector working |

## 1. Scope

Define and register missing party and whisper packet families so Korangar can
frame traffic safely before the first real party session.

This plan is primarily protocol safety:
- Define packet structs in `ragnarok-packets`.
- Register handlers or noops in `version_20220406.rs`.
- Add minimal events only where needed for verification.
- Build real party/whisper UI later.

## 2. Why This Comes Early

Korangar frames incoming packets by deserializing the registered packet type. If
Hercules sends an unregistered packet header, the client cannot know its length
and drops the rest of that TCP read. Party traffic is a live risk because the DM
campaign is party-locked through `$dm_active_party`.

Minimum target: every party/whisper packet emitted by a normal Hercules group
session is at least defined and noop-registered.

## 3. Packet Families

### Party

Known missing areas:
- Party roster/list.
- Party member HP/SP updates.
- Party member position updates.
- Party chat (`0x0108` / `0x0109`).
- Invite, leave, kick, leader, and option updates not already covered.
- 20220406 main roster/member packets are `0x0AE5` / `0x0AE4`; older
  `0x00FB` roster notes are not correct for this server build.

Already mentioned in the noop backlog:
- `PartyInvitePacket`
- `UpdatePartyInvitationStatePacket`

### Whisper

Known missing areas:
- Client → server whisper send: `0x0096`.
- Server → client whisper receive/result: `0x09DE` / `0x0098` for
  `PACKETVER=20220406`.
- `/r` reply state and whisper history are UI follow-ups.

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

6. Add minimal outgoing sends if needed.
   - Whisper send can be a simple networking method before chat UI is polished.
   - Party create/invite can wait unless needed for reproducible verification.

7. Verify against Hercules.
   - Two accounts online.
   - Create or join a party.
   - Move both characters.
   - Change HP on one character.
   - Send party chat.
   - Send and receive a whisper.

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
