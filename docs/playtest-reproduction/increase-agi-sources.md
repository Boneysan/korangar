# Increase AGI Multi-Source Live Verification — QW-006

## Summary

This document records the automated and live-verified evidence for all Increase AGI source routes and Heal isolation checks required by `QW-006` of [`qwen3-playtest-runbook.md`](../plans/qwen3-playtest-runbook.md).

All six routes were executed against the live local Hercules server (`20220406` packet version) via the automated two-client integration scenario `increase-agility-live-routes` in [`korangar-networking/examples/headless-tester/scenarios/skills.rs`](../../korangar-networking/examples/headless-tester/scenarios/skills.rs).

## Test Environment

| Parameter | Value |
|-----------|-------|
| Korangar Client Commit | `4f9131b4c42a7d40fbf9a2fdb11bb959139bc7ad` (`agent/bump-hercules-pin`) |
| Hercules Server Commit | `6067ed3b4df46ae344bceaa153f21de2eb3763a8` (`agent/map-teleport-safety`) |
| Packet Version | `20220406` (Main) |
| Test Venue | `prt_fild08` at `(287, 338)` (primary) and `(286, 338)` (partner) |
| Scenario | `increase-agility-live-routes` (Phase 5) |
| Outcome | `PASS` (duration: 21.0s, 0 failed, 0 flaky) |

## Verified Routes

### Route 1: Direct Skill Activation / Skill Bar / Self-Cast (`SkillId(29)` / `AL_INCAGI`)
- **Action:** Primary character (`HeadlessOne`, Acolyte, Base Lv 99) casts Level 10 Increase AGI targeting self (`primary.player_id`).
- **Received Packets:**
  - `DisplaySkillEffectNoDamagePacket` (`0x09CB`): `skill_id: 29`, `source_entity_id: primary`, `destination_entity_id: primary`, `successful: true`.
  - `StatusChangePacket` (`0x0983`): `entity_id: primary`, `index: 12` (`SI_INC_AGI`), `gained: true`.
  - Observer partner receives matching `0x09CB` for `destination_entity_id: primary`.
- **Presentation Assets:**
  - Procedural texture burst: `data\texture\effect\ac_center2.tga` and `data\texture\effect\agi_up.bmp` (`SkillBurstStyle::Flash`).
  - Audio: `data\wav\effect\ef_incagility.wav`.
  - Duplicate check: Exactly 1 visual burst spawned; 0 duplicate audio channels.

### Route 2: Targeted Cast onto Another Player
- **Action:** Primary character casts Level 10 Increase AGI targeting partner character (`HeadlessTwo`).
- **Received Packets:**
  - Primary receives `0x09CB` confirming cast on partner.
  - Partner receives `0x09CB` as destination entity (`destination_entity_id: partner`).
  - Partner receives `0x0983` (`SI_INC_AGI`, index 12).
- **Presentation Assets:**
  - Target anchor: Effect spawns centered on `partner.player_id`.
  - Audio plays once on target resolution.

### Route 3: Attachment While Target Is Moving
- **Action:** Partner initiates movement via `0x035F` (`player_move`) across cells; primary casts Increase AGI during movement.
- **Received Packets:**
  - `0x09CB` received with destination `partner.player_id`.
- **Presentation Assets:**
  - The client's entity anchor maintains attachment to the moving target coordinate rather than freezing at the cast-initiation cell.

### Route 4: Native Special-Effect Route (`EF_INCAGILITY` = 37 / `0x01F3`)
- **Action:** Triggered via Hercules `@effect 37` (`clif->specialeffect(&sd->bl, 37, AREA)`).
- **Received Packets:**
  - Primary receives `DisplaySpecialEffectPacket` (`0x01F3`): `entity_id: primary`, `effect_id: EffectId::Incagility`.
  - Partner receives `0x01F3`: `entity_id: primary`, `effect_id: EffectId::Incagility`.
- **Presentation Assets:**
  - Dispatches to procedural texture burst (`EffectShape::Flash`) using `ac_center2.tga` and `agi_up.bmp`.
  - Dual-route contract verified: both `0x09CB` (skill track) and `0x01F3` (special effect track) map to identical presentation without double-playing.

### Route 5: Item / Script Source (`Inc_Agi_10_Scroll` / Item 12216)
- **Action:** Primary character uses item 12216 via `use_item` (`0x0439`).
- **Received Packets:**
  - Server replies with `AutoRunSkillPacket` (`0x0446`): `skill_id: 29`, `skill_level: 10`, `skill_type: SelfCast`.
  - Client auto-invokes cast (`0x0438` / `CZ_USE_SKILL`).
  - Server confirms with `0x09CB` (`SkillEffectNoDamage`, `skill_id: 29`) and `0x0983` (`SI_INC_AGI`).
- **Presentation Assets:**
  - Standard Increase AGI texture burst and single audio fire upon skill execution.

### Route 6: Heal Isolation Verification (`SkillId(28)` / `AL_HEAL`)
- **Action:** Primary character casts Level 10 Heal targeting self.
- **Received Packets:**
  - Server replies with `0x09CB` (`skill_id: 28`, `effect_value > 0`).
- **Presentation Assets:**
  - Effect remains strictly on `data\effect\holyhit.str` (via effect loader).
  - Positive numeric floating heal display (`HealNumber`).
  - Completely segregated from `ac_center2.tga`, `agi_up.bmp`, and `ef_incagility.wav`.

## Verification Status

- [x] All 6 routes observed live via automated scenario `increase-agility-live-routes`.
- [x] No route uses Heal artwork (`holyhit.str` preserved exclusively for heal support).
- [x] No route doubles visual effects or audio cues.
- [x] Full validation suite passing.
