# WASD Movement Protocol — Phase 0 Reproduction Template

## Problem Statement

Reported: Internet/WAN connection exhibits WASD movement issues that don't appear on LAN.
Symptoms include rubber-banding, input delay, and targeting errors.

## Test Environment

| Parameter | Value |
|-----------|-------|
| Client Build | korangar HEAD (commit hash) |
| Server Build | Hercules hercules-2025.09 branch |
| Map | midgard 1 (or chosen test map) |
| Character Weight | N/A (test with normal and overweight) |

## Test Routes

Create identical paths for LAN vs Internet comparison:

### Route A: Short Straight Line
```
Start: (150, 150)
End:   (160, 150)
Distance: 10 tiles straight east
Expected time: ~2-3 seconds walking
```

### Route B: L-Shaped Turn
```
Start: (150, 150)
Waypoint: (155, 150) -> (155, 155)
End:   (160, 155)
Distance: 10 tiles (5 east + 5 south)
Expected time: ~3-4 seconds walking
```

### Route C: Zigzag
```
Start: (150, 150)
Path: (152,150) -> (152,152) -> (154,152) -> (154,154) -> (156,154)
End:   (156, 154)
Distance: ~8 tiles
Expected time: ~3 seconds walking
```

## Test Matrix

| Test | Network | Ping | Packet Loss | Description |
|------|---------|------|-------------|-------------|
| T1 | LAN | <5ms | 0% | Baseline control |
| T2 | LAN | <5ms | 0% | With simulated jitter |
| T3 | Internet | ~50ms | 0% | WAN baseline |
| T4 | Internet | ~100ms | 0% | Higher latency |
| T5 | Internet | ~50ms | 2% | Low packet loss |
| T6 | Internet | ~100ms | 5% | High packet loss |

## Measurement Tools

### Client-Side
- Enable `KORANGAR_PACKET_LOG` to capture movement packets
- Note the exact client timestamp when keys are pressed
- Record client-side position updates

### Server-Side
- Hercules debug logging for `map_move` calls
- Record server tick when move packets arrive
- Track position correction events

## Test Procedure

### WASD Movement (Holding Keys)

1. Position character at route start
2. Press and hold movement key(s) to navigate the route
3. Release key upon reaching end marker
4. Note:
   - Total time from key press to arrival
   - Whether target is reached exactly
   - Any visible stutter/rubber-banding

### Click-to-Move (Single Target)

1. Position character at route start
2. Right-click on target cell at route end
3. Observe the walk path displayed
4. Note:
   - Path selection (does it match expected?)
   - Arrival accuracy
   - Any deviation mid-walk

## Expected Behavior

| Action | Expected Result |
|--------|-----------------|
| Hold WASD key | Smooth continuous movement toward direction |
| Click-to-move | Straight line or shortest path to target |
| Key release mid-walk | Immediate stop at current position |
| Rapid direction change | New direction takes effect immediately |

## Actual Failure Patterns to Document

| Symptom | Possible Cause |
|---------|----------------|
| Rubber-banding back | Client predicted, server rejected |
| Movement delay | Network latency + prediction |
| Walking through walls | Range check timing issue |
| Stopping short/overshooting | Pathfinding vs actual movement |
| Direction not changing | Input buffering issue |

## Packet Structure Reference

### CZ_MOVE_PLAYER (0x0073)
```
struct CZ_MOVE_PLAYER {
    int16 packetType;  // 0x0073
    uint8 x, y;        // Target cell coordinates
} __attribute__((packed));
```

### ZC_MOVE_RESULT / ZC_STOPMOVE
Server responses indicating move success/failure.

## Analysis Criteria

A movement issue is **confirmed** when:

1. **Consistent deviation**: Route completion accuracy < 90% across 5 trials
2. **Latency correlation**: Movement quality degrades as ping increases (comparing T1 vs T3/T4)
3. **Packet loss correlation**: Quality degrades with packet loss (comparing T3 vs T5/T6)

## References

- `korangar/src/network/client/packet.rs` — Movement packet handling
- `Hercules/src/map/clif.c` — `clif_parse_MoveStart`, `clif_parse_MoveStop`
- `Hercules/src/map/unit.cpp` — Movement processing logic
