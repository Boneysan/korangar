# Targeted Spec — Dice Cards Widget (E7.2)

**Parents**: PROJECT_PLAN.md E7.2, DM_INTERFACE.md §9.3, DM_DATA_GUIDE.md, FEATURE_ROADMAP.md.

**Purpose**: Replace flat chat spam for `@roll` and `@dm check` results with attractive, animated "physical" dice card UI elements. Gives immediate visual/tactile feedback for tabletop rolls.

## Requirements

- Triggered by parsing chat messages (and later [DMJ]).
- Shows: Roll result, modifier, DC, success/fail, natural 20/1 flair.
- Animated (simple physics or tween for "roll" feel).
- Non-intrusive: Appears near hotbar or in dedicated corner, fades after time or on click.
- Supports advantage/disadvantage (show two cards or combined).

## Data Flow (Phase A)

1. Server sends result as normal chat (e.g. "Wynne rolls 17 + 4 vs DC 15 → Success!").
2. Client chat parser (in `dm/parser.rs` or lib.rs ChatMessage handler) detects roll patterns or [DMJ] {"t":"check_result", ...}.
3. Spawns `DiceCard` particle or dedicated HUD element.
4. Renders using existing text + rectangle primitives (or simple 3D quad for "card" look).

## Implementation

**Widget** (`interface/components/dice_card.rs` or in dm/):
- Struct holding roll data.
- `create_layout_info` / `lay_out` for rendering.
- Animation state (rolling → result).
- Flair: Green glow for nat20, red for nat1, checkmark/cross.

**Parsing**:
- Regex or simple string match for common patterns.
- Prefer [DMJ] JSON for structured data (who, stat, roll, mod, dc, success, nat).

**State**:
- Transient list in `dm_state.recent_dice` (bounded, e.g. last 5).
- Or pure particle system for "throw" effect.

**DM Extras**:
- DM can trigger via console.
- History log for recap.

## Visuals

- Card-like rectangle with d20 icon + big number.
- Color by success.
- Optional: Small 3D dice model using existing effect system.

See `DM_DATA_GUIDE.md` for parser patterns and `modern-mechanics.md` for related dice ideas.

**MVP first**: Basic text card on chat detection. Then animation + [DMJ] support.
