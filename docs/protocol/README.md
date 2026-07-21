# Protocol References

**Parent hub**: [docs/README.md](../README.md) (start here for the full documentation index).

This directory holds curated protocol notes derived from the local Hercules
server tree. These notes are not the byte-layout source of truth; the Rust
packet definitions in `ragnarok-packets` are authoritative for Korangar, and
Hercules remains authoritative for what the server emits.

| Reference | Purpose |
|---|---|
| [hercules-20220406.md](hercules-20220406.md) | Hercules packet source map, lookup workflow, and current `PACKETVER=20220406` audit findings |
| [packet-length-fallbacks.md](packet-length-fallbacks.md) | Why framing-by-deserialization needs length tables and how `register_length_fallbacks` prevents desyncs |
| [inventory-and-ranged-attacks.md](inventory-and-ranged-attacks.md) | Ammo modeling (stackable+equippable), the `EquipAmmunitionPacket` (0x013C) no-offset index gotcha, and the client-drawn normal-attack arrow projectile |
| [PACKET_EVENTS_CATALOG.md](../PACKET_EVENTS_CATALOG.md) | Complete catalogue of `NetworkEvent` variants, every producing packet + handler, structs, data flows into lib.rs/world/UI, DM usage (chat + quest effects), how to extend |

