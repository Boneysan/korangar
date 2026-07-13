# Testing Strategy, Verification & Test Run Results

This document serves as the canonical reference for testing the **Korangar Client** and **Hercules Server Emulator** project. It includes instructions to run tests, explanations of specialized verification binaries, and results from recent test runs.

---

## 1. Automated Unit Testing (Rust Workspace)
The Korangar Rust client codebase consists of several modular crates. Automated unit testing verifies deserialization, packet lengths, collision logic, container structures, and byte-parsing.

### How to Run:
From the `korangar` root directory:
```bash
cargo test --workspace --all-features
```

### Test Execution Results (Run on 2026-07-11):
All unit tests in the workspace passed successfully:
* **`ragnarok-bytes`**: 47 passed, 0 failed.
* **`ragnarok-formats`**: 11 passed, 0 failed.
* **`ragnarok-macros`**: 0 passed (proc-macro skeleton).
* **`ragnarok-packets`**: 36 passed, 0 failed (recently expanded to include friend list layout verification).
* **`korangar-debug`**: 2 doc-tests compile-fails verified (ensures ring buffer panic behavior is correct).
* **Overall**: All 96 unit tests and compile-fail assertions passed cleanly.

---

## 2. Korangar Client-Specific Testing & Diagnostics
Because Korangar is a graphical client utilizing the `wgpu` rendering engine and a custom UI framework, testing requires real-time runtime diagnostics and environment checks:

### A. In-Game Debug & Diagnostic Windows
Building the client with the `debug` feature enables built-in developers' inspectors:
```bash
# Run with debug capabilities enabled
cargo run --release --bin korangar --features debug
```
While running, press **`Ctrl+O`** to open the GM/DM Debug Panel. Key diagnostic windows include:
* **Packet Inspector**: Pretty-prints every incoming and outgoing packet to verify structural alignment and payload deserialization.
* **ClientState Inspector**: Walks the entire reactive client-state tree to debug UI window bindings and player stat updates.
* **Frame Profiler**: Monitors render-pass frame times, loading latencies, and thread-pool behaviors.
* **Render Options & Theme Inspector**: Allows engineers to test different MSAA modes, texture clamping, and UI layout scales on the fly.

### B. Network Protocol Hex Logging
If you need to analyze raw bytes to debug a framing mismatch or custom packet registration:
```bash
KORANGAR_PACKET_LOG=1 cargo run --release --bin korangar
```
This dumps hex streams of incoming and outgoing TCP packets to stdout.

### C. Cross-Platform Graphics & Renderer Testing
Because the client runs on both macOS (Metal backend) and Windows/WSL2/Linux (Vulkan, DX12, OpenGL backends), rendering logic must be validated across different drivers:
* **macOS**: Runs Metal natively. No special flags are required.
* **WSL2 / Linux**: Uses OpenGL translations. Shaders must be tested against fallback paths since WSL OpenGL lacks `TEXTURE_BINDING_ARRAY` and non-uniform indexing.
* **MSAA Resolve Safety**: MSAA resolve causes black screens under OpenGL (WSLg's Mesa driver). The engine automatically detects this and forces `Msaa::Off` to safeguard rendering. Always run visual tests on native Vulkan/Metal backends to ensure MSAA functions as expected there.

---

## 3. How to Test the `ragnarok-packets` Crate
Whenever you add new packet structures or modify existing network protocols in the `ragnarok-packets` crate, you must implement the following three types of tests inside [lib.rs](../../ragnarok-packets/src/lib.rs) or [position.rs](../../ragnarok-packets/src/position.rs):

### A. Serialization & Deserialization Layout Tests (Round-Trip Tests)
* **Verify Serialization**: Test that constructed Rust packet structs serialize into the exact expected byte sequence for the network protocol.
  * *Example*: `add_friend_packet_matches_20220406_layout` asserts that `AddFriendPacket` serializes to `[0x02, 0x02, ...]` followed by the 24-byte padded name.
* **Verify Deserialization**: Test that raw byte payloads captured from the server parse back into correct Rust structs.
  * *Example*: `friend_list_packet_matches_20220406_layout` parses a variable-length repeating array stream of `Friend` entries, confirming that fields (`account_id`, `character_id`, `name`) extract correctly.
* **Assert Struct Sizes**: Use `FixedByteSize` assertions to ensure structure lengths (e.g. `CharacterInformation`, `ShopItemInformation`) match Hercules expectations.

### B. Coordinate Bit-Packing & Math Logic Tests
* Ensure bit-packed coordinate types (`WorldPosition` and `WorldPosition2`) undergo round-trip testing to verify that bitmasking and shifting operations do not corrupt coordinates.

### C. Framing & Stream Fallback Tests
* Test the `PacketHandler` behavior in [handler.rs](../../ragnarok-packets/src/handler.rs) to verify that fixed-length, variable-length, and cut-off (incomplete) packet streams are correctly handled and framed.

---

## 4. Server-Side Campaign & Script Validation (Hercules)
When developing custom tabletop scenarios, bestiaries, or NPC scripts, validate script syntax and load integrity prior to spawning the server.

### A. Campaign Script Check (`check-campaign.sh`)
This tool boots the Hercules map-server using the `--run-once` flag to load all configured NPC/scripts and checks for errors (such as case typos, missing labels, or invalid parameters).
* **How to Run**:
  ```bash
  cd ../../Hercules
  ./tools/check-campaign.sh
  ```
* **Execution Results (Run on 2026-07-11)**:
  * **Result**: **OK — campaign loaded clean** (`30 dm_campaign include lines`, `0 errors`).

### B. Interface Validation (`validateinterfaces.py`)
This script parses the Hercules C source code functions and structures to ensure that declared HPM (Hercules Plugin Manager) interfaces are fully synchronized with their C macros.
* **How to Run**:
  ```bash
  cd ../../Hercules/tools
  python3 validateinterfaces.py
  ```

### C. SQL Schema Linting (`validate_sql-files.py`)
Lints the SQL database schemas in the `sql-files/` directory using SQLFluff.
* **How to Run**:
  ```bash
  cd ../../Hercules
  python3 tools/validate_sql-files.py
  ```

---

## 5. Map Walkability & Asset Audits (Korangar)
Because Korangar relies on custom map warping for Tabletop / DM features, bad coordinates can cause the client to spawn in void/unwalkable cells.

### A. Focused Campaign Beats Warp Safety Audit
Scans the custom Campaign beats scripts against GAT terrain data to verify that all party teleport destinations are walkable.
* **How to Run**:
  ```bash
  cd ../../korangar
  cargo run --release --bin map-asset-audit -- \
    ../../Hercules/conf/map/maps.conf \
    data.grf rdata.grf \
    ../../Hercules/npc/custom/dm_campaign/shared/dm_beats.txt
  ```
* **Execution Results (Run on 2026-07-11)**:
  * **Static teleport destinations checked**: 59
  * **Unsafe static teleport destinations**: 0 (all warp destinations mapped to valid, walkable terrain coordinates).

---

## 6. Live Verification / Integration Testing

### A. Automated: Headless Client
The headless tester (`korangar-networking/examples/headless-tester.rs`) automates protocol-level integration testing against a live server — it shares `ragnarok-packets` and `korangar-networking` with the graphical client, so packet-mapping bugs it finds (and fixes for them) apply to the main client directly.
* **How to Run** (server must be up):
  ```bash
  cargo run --example headless-tester -p korangar-networking
  ```
* **Smoke test result (2026-07-11)**: PASS — login → character select → map load → chat round-trip, exit 0; failure paths (bad credentials, missing character, dead server) exit 1.
* **Full scenario catalog**: [headless_test_plan.md](headless_test_plan.md) (10 phases: session, GM bootstrap, movement, combat, skill sweep, items, dialogue, multi-client social, DM campaign commands, protocol coverage).
* **Bug documentation & port-back workflow**: [headless_findings.md](headless_findings.md) — every failed scenario gets an entry classifying the layer (shared crate / client / server) before the fix lands.
* **Graphical-client handoff matrix**: the “Expanded-suite graphical-client handoff” section in [headless_findings.md](headless_findings.md) records what is shared automatically, what still needs UI verification, and what remains blocked.

The expanded runner currently registers 91 scenarios across phases 1–9. Phase
8 requires a pre-provisioned, non-GM `headless2` account with a `HeadlessTwo`
character; automatic Hercules `_M` registration may be disabled in the local
login configuration. Phase 9 covers the Seal Cascade dice and DM command
contracts.

### B. Manual: Graphical Client
Since Korangar is a graphical client, manual integration testing is used to verify the actual game loop against a live Hercules server.

### Running the Live Environment:
1. **Start the Hercules Servers**:
   ```bash
   cd ../../Hercules
   ./athena-start start
   ```
   *Expects listeners on TCP ports `6900` (login), `6121` (char), and `5121` (map).*
2. **Launch the Korangar Client**:
   * **macOS**: `cargo run --release --bin korangar` from the `korangar/` directory.
   * **WSL2**: `./run-wsl.sh` from the root of `korangar/` (sets forced OpenGL passthrough variables to work around WSLg Vulkan limitations).

### Live Verification Checklist:
Perform the following functional checks during feature updates (see [M1-p0-verification.md](../../docs/plans/M1-p0-verification.md)):
- [x] **Login/Session**: Valid login credentials flow (`korangar`/`korangar`), character select display, character creation, map load, clean logout.
- [x] **Movement & Controls**: Click-to-move (~10+ tiles), Sit/Stand toggling (Home key on macOS).
- [x] **Combat & Status**: Melee attack against Porings (displays correct damage numbers, chaser tracking), skill damage numbers, hotbar casting, buff/debuff bar timeouts.
- [x] **Economy**: Inventory drag & drop, item split, NPC shop buying/selling in English (utilizes compilation of 13k+ names from [items.json](../../docs/items.json)), item identification (Magnifiers), and Kafra storage grids.
- [x] **NPC Interaction**: Dialogue options, warp transitions, and dialogue numeric/string inputs.
- [x] **Skill menus**: Teleport destination selection/cancel and Whitesmith Upgrade Weapon selection were live-verified on macOS 2026-07-12; refine failure, success acknowledgement, and the refinement visual effect rendered correctly.
- [x] **Repair Weapon core flow**: live-verified on macOS 2026-07-12 — the cast completed, the selection window offered Sword, clicking it repaired the item, and chat displayed `Repair succeeded for Sword.` Headless success and cancellation scenarios also pass. Window resize/move and graphical Cancel remain presentation checks.
- [x] **Universal window resizing**: live-verified on macOS 2026-07-12. All current windows resize horizontally and vertically. The interface component default is resizable, stored height is preserved instead of being overwritten by content layout, edge deltas are axis-specific, and future windows inherit two-axis resizing unless they deliberately constrain their dimensions.
- [x] **Account-independent skills and sprites**: every selected character rebuilds its skill layout from that character's job and requests the corresponding SPR/ACT resources through the global asset loaders. Logout and character selection clear learned skills and hotbar bindings so account-specific state cannot leak into the next login; valid decoded assets remain globally cached. The runtime skill paths are relative to the loaders' `data\\sprite\\` root and have regression coverage.

### C. Skill Icon Asset Audit

Run from the nested client directory so `client/game_archives.ron` and the GRFs
resolve exactly as they do for the graphical client:

```bash
cd korangar
cargo run --release --bin skill-asset-audit
```

The audit evaluates the patched Lua skill catalog, takes the union of every
player job's visible skill-tree layout, and verifies that each skill resolves a
loadable SPR/ACT pair. The archive list must include `renewal2021.grf` and
`resources2021.grf` in addition to `data.grf` and `rdata.grf`.

**Result (2026-07-12):** 1,007 visible skills checked; 0 missing icon assets.
