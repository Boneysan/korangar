use ragnarok_packets::EntityId;

use crate::graphics::PickerTarget;
use crate::world::{Camera, Entity, EntityType};
use crate::{ScreenPosition, ScreenSize};

/// Candidate entity for target cycling.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetCandidate {
    pub entity_id: EntityId,
    pub distance: f32,
}

/// Candidate entity for click selection and overlap resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct ClickCandidate {
    pub entity_id: EntityId,
    pub screen_distance: f32,
    pub camera_depth: f32,
    pub is_direct_hit: bool,
}

/// Hit tolerance in screen pixels around an entity's screen-projected position.
pub const SPRITE_HIT_TOLERANCE_PX: f32 = 30.0;

/// Depth difference threshold below which two entities are considered at the
/// same depth.
pub const DEPTH_EPSILON: f32 = 0.5;

/// Resolves which entity among the candidates should be selected upon a click
/// or hover.
///
/// Rules:
/// 1. Stability: If `current_target` is among the candidates within tolerance,
///    retain it.
/// 2. Screen depth: The entity in front (smaller `camera_depth`) takes
///    precedence over entities behind it.
/// 3. Cursor distance: If two entities have effectively the same depth (within
///    `DEPTH_EPSILON`), the one closer to the cursor in screen space wins.
/// 4. Stable tie-breaker: Entity ID ascending.
pub fn resolve_selection_candidate(mut candidates: Vec<ClickCandidate>, current_target: Option<EntityId>) -> Option<EntityId> {
    if candidates.is_empty() {
        return None;
    }

    // Rule 1: Keep repeated clicks on current target stable if it is still within
    // tolerance
    if let Some(current) = current_target
        && candidates.iter().any(|c| c.entity_id == current)
    {
        return Some(current);
    }

    // Rule 2 & 3 & 4: Sort by depth, then cursor distance, then entity ID
    candidates.sort_by(|a, b| {
        let depth_diff = a.camera_depth - b.camera_depth;
        if depth_diff.abs() > DEPTH_EPSILON {
            a.camera_depth.total_cmp(&b.camera_depth)
        } else {
            a.screen_distance
                .total_cmp(&b.screen_distance)
                .then_with(|| a.entity_id.0.cmp(&b.entity_id.0))
        }
    });

    candidates.first().map(|c| c.entity_id)
}

/// Collects candidates for click selection from the entity list.
/// Clip-space `w` after the view-projection multiply. Behind the camera or on
/// the near plane is off-screen and must not be hovered or clicked.
pub fn clip_is_on_screen(clip_w: f32) -> bool {
    clip_w > 0.0
}

pub fn collect_click_candidates(
    mouse_target: PickerTarget,
    mouse_position: ScreenPosition,
    window_size: ScreenSize,
    camera: &dyn Camera,
    entities: &[Entity],
    local_player_id: Option<EntityId>,
    tolerance_px: f32,
) -> Vec<ClickCandidate> {
    let mut candidates = Vec::new();
    let direct_hit_id = match mouse_target {
        PickerTarget::Entity(id) => Some(id),
        _ => None,
    };

    for entity in entities.iter().skip(1) {
        let entity_id = entity.get_entity_id();
        if Some(entity_id) == local_player_id {
            continue;
        }
        if entity.is_dead() || entity.is_fading() || entity.hides_identity() {
            continue;
        }

        let is_direct = direct_hit_id == Some(entity_id);
        let world_pos = entity.get_position();
        let clip_pos = camera.view_projection_matrix() * world_pos.to_homogeneous();
        if !clip_is_on_screen(clip_pos.w) {
            continue;
        }

        let screen_uv = camera.clip_to_screen_space(clip_pos);
        let entity_screen_pos = ScreenPosition {
            left: screen_uv.x * window_size.width,
            top: screen_uv.y * window_size.height,
        };

        let dx = mouse_position.left - entity_screen_pos.left;
        let dy = mouse_position.top - entity_screen_pos.top;
        let screen_dist = (dx * dx + dy * dy).sqrt();

        if is_direct || screen_dist <= tolerance_px {
            let camera_depth = clip_pos.w;
            candidates.push(ClickCandidate {
                entity_id,
                screen_distance: if is_direct { 0.0 } else { screen_dist },
                camera_depth,
                is_direct_hit: is_direct,
            });
        }
    }

    candidates
}

/// Resolves an effective `PickerTarget` considering GPU picker result, screen
/// cursor position, sprite hit tolerance, screen depth overlap, and target
/// stability.
pub fn resolve_effective_target(
    raw_target: PickerTarget,
    candidates: Vec<ClickCandidate>,
    current_target: Option<EntityId>,
    is_ground_item: bool,
) -> PickerTarget {
    // Ground items take precedence when clicked directly
    if is_ground_item {
        return raw_target;
    }

    if let Some(selected_id) = resolve_selection_candidate(candidates, current_target) {
        PickerTarget::Entity(selected_id)
    } else {
        raw_target
    }
}

/// Checks whether an entity is an eligible hostile target candidate.
///
/// Must be an alive, visible monster (not player, NPC, warp, hidden entity, or
/// dead/fading).
pub fn is_hostile_target_candidate(entity_type: EntityType, is_dead: bool, is_fading: bool, in_view: bool) -> bool {
    entity_type == EntityType::Monster && !is_dead && !is_fading && in_view
}

/// Sorts candidates by distance ascending, with entity ID as a stable
/// tie-breaker.
pub fn sort_target_candidates(candidates: &mut [TargetCandidate]) {
    candidates.sort_by(|a, b| a.distance.total_cmp(&b.distance).then_with(|| a.entity_id.0.cmp(&b.entity_id.0)));
}

/// Cycles to the next target given an ordered list of candidates and the
/// current target.
///
/// Wraps around to the first candidate when reaching the end of the list.
/// If current target is None or not in the candidate list, selects the first
/// candidate.
pub fn cycle_target(candidates: &[TargetCandidate], current_target: Option<EntityId>) -> Option<EntityId> {
    if candidates.is_empty() {
        return None;
    }
    match current_target {
        Some(current) => {
            if let Some(index) = candidates.iter().position(|c| c.entity_id == current) {
                let next_index = (index + 1) % candidates.len();
                Some(candidates[next_index].entity_id)
            } else {
                Some(candidates[0].entity_id)
            }
        }
        None => Some(candidates[0].entity_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(id: u32, distance: f32) -> TargetCandidate {
        TargetCandidate {
            entity_id: EntityId(id),
            distance,
        }
    }

    #[test]
    fn ordering_by_distance_ascending() {
        let mut candidates = vec![cand(1, 12.0), cand(2, 3.5), cand(3, 7.0)];
        sort_target_candidates(&mut candidates);
        assert_eq!(candidates, vec![cand(2, 3.5), cand(3, 7.0), cand(1, 12.0)]);
    }

    #[test]
    fn stable_tie_breaker_by_entity_id() {
        let mut candidates = vec![cand(50, 10.0), cand(10, 10.0), cand(20, 10.0)];
        sort_target_candidates(&mut candidates);
        assert_eq!(candidates, vec![cand(10, 10.0), cand(20, 10.0), cand(50, 10.0)]);
    }

    #[test]
    fn cycle_wraparound() {
        let candidates = vec![cand(10, 5.0), cand(20, 10.0), cand(30, 15.0)];
        let t0 = cycle_target(&candidates, None);
        assert_eq!(t0, Some(EntityId(10)));
        let t1 = cycle_target(&candidates, t0);
        assert_eq!(t1, Some(EntityId(20)));
        let t2 = cycle_target(&candidates, t1);
        assert_eq!(t2, Some(EntityId(30)));
        // Wraps around to the first candidate
        let t3 = cycle_target(&candidates, t2);
        assert_eq!(t3, Some(EntityId(10)));
    }

    #[test]
    fn single_candidate_cycle() {
        let candidates = vec![cand(42, 5.0)];
        assert_eq!(cycle_target(&candidates, None), Some(EntityId(42)));
        assert_eq!(cycle_target(&candidates, Some(EntityId(42))), Some(EntityId(42)));
    }

    #[test]
    fn empty_candidates_returns_none() {
        let candidates = vec![];
        assert_eq!(cycle_target(&candidates, None), None);
        assert_eq!(cycle_target(&candidates, Some(EntityId(42))), None);
    }

    #[test]
    fn filtering_excludes_non_monsters() {
        // Only EntityType::Monster is eligible
        assert!(!is_hostile_target_candidate(EntityType::Player, false, false, true));
        assert!(!is_hostile_target_candidate(EntityType::Npc, false, false, true));
        assert!(!is_hostile_target_candidate(EntityType::Warp, false, false, true));
        assert!(!is_hostile_target_candidate(EntityType::Hidden, false, false, true));
        assert!(is_hostile_target_candidate(EntityType::Monster, false, false, true));
    }

    #[test]
    fn filtering_excludes_dead_and_fading() {
        assert!(!is_hostile_target_candidate(EntityType::Monster, true, false, true));
        assert!(!is_hostile_target_candidate(EntityType::Monster, false, true, true));
        assert!(!is_hostile_target_candidate(EntityType::Monster, true, true, true));
        assert!(is_hostile_target_candidate(EntityType::Monster, false, false, true));
    }

    #[test]
    fn filtering_excludes_out_of_view() {
        assert!(!is_hostile_target_candidate(EntityType::Monster, false, false, false));
        assert!(is_hostile_target_candidate(EntityType::Monster, false, false, true));
    }

    #[test]
    fn test_add_candidate_updates_cycle_deterministically() {
        let mut candidates = vec![cand(1, 10.0), cand(3, 30.0)];
        sort_target_candidates(&mut candidates);
        assert_eq!(cycle_target(&candidates, Some(EntityId(1))), Some(EntityId(3)));

        // Candidate 2 spawns between 1 and 3
        candidates.push(cand(2, 20.0));
        sort_target_candidates(&mut candidates);
        assert_eq!(cycle_target(&candidates, Some(EntityId(1))), Some(EntityId(2)));
        assert_eq!(cycle_target(&candidates, Some(EntityId(2))), Some(EntityId(3)));
    }

    #[test]
    fn test_remove_candidate_recovers_deterministically() {
        let mut candidates = vec![cand(1, 10.0), cand(2, 20.0), cand(3, 30.0)];
        sort_target_candidates(&mut candidates);
        // Current is candidate 2, which then dies
        candidates.retain(|c| c.entity_id != EntityId(2));
        assert_eq!(candidates.len(), 2);
        // When current is missing, fall back to first (closest) remaining target
        assert_eq!(cycle_target(&candidates, Some(EntityId(2))), Some(EntityId(1)));
    }

    #[test]
    fn test_reorder_candidates_updates_cycle_deterministically() {
        let mut candidates = vec![cand(1, 5.0), cand(2, 20.0)];
        sort_target_candidates(&mut candidates);
        assert_eq!(candidates[0].entity_id, EntityId(1));

        // Entities move: candidate 2 moves closer, candidate 1 moves further
        candidates[0].distance = 25.0;
        candidates[1].distance = 10.0;
        sort_target_candidates(&mut candidates);
        assert_eq!(candidates[0].entity_id, EntityId(2));
        assert_eq!(candidates[1].entity_id, EntityId(1));
        assert_eq!(cycle_target(&candidates, None), Some(EntityId(2)));
    }

    fn click_cand(id: u32, screen_dist: f32, depth: f32, direct: bool) -> ClickCandidate {
        ClickCandidate {
            entity_id: EntityId(id),
            screen_distance: screen_dist,
            camera_depth: depth,
            is_direct_hit: direct,
        }
    }

    #[test]
    fn overlap_selects_front_entity() {
        // Monster 1 is in front (depth 10.0), Monster 2 is behind (depth 20.0).
        // Monster 2 is closer to cursor (distance 2.0 vs 15.0), but Monster 1 is in
        // front.
        let candidates = vec![click_cand(2, 2.0, 20.0, false), click_cand(1, 15.0, 10.0, false)];
        let chosen = resolve_selection_candidate(candidates, None);
        assert_eq!(chosen, Some(EntityId(1)));
    }

    #[test]
    fn overlap_depth_tie_selects_closest_to_cursor() {
        // Monster 1 and 2 are at effectively the same depth (10.0 vs 10.2, within
        // DEPTH_EPSILON). Monster 1 is closer to cursor (5.0 vs 18.0).
        let candidates = vec![click_cand(2, 18.0, 10.2, false), click_cand(1, 5.0, 10.0, false)];
        let chosen = resolve_selection_candidate(candidates, None);
        assert_eq!(chosen, Some(EntityId(1)));
    }

    #[test]
    fn repeated_clicks_keep_current_target_stable() {
        // Monster 1 is in front (depth 8.0, distance 12.0).
        // Monster 2 is behind (depth 15.0, distance 4.0), BUT Monster 2 is already the
        // current target! Repeated clicking should keep Monster 2 rather than
        // jumping to Monster 1.
        let candidates = vec![click_cand(1, 12.0, 8.0, false), click_cand(2, 4.0, 15.0, false)];
        let chosen = resolve_selection_candidate(candidates, Some(EntityId(2)));
        assert_eq!(chosen, Some(EntityId(2)));
    }

    #[test]
    fn click_outside_current_target_switches() {
        // Current target is Monster 2, but the click was on Monster 1 only (Monster 2
        // not in candidates).
        let candidates = vec![click_cand(1, 5.0, 10.0, false)];
        let chosen = resolve_selection_candidate(candidates, Some(EntityId(2)));
        assert_eq!(chosen, Some(EntityId(1)));
    }

    #[test]
    fn empty_click_candidates_returns_none() {
        assert_eq!(resolve_selection_candidate(vec![], None), None);
        assert_eq!(resolve_selection_candidate(vec![], Some(EntityId(5))), None);
    }

    #[test]
    fn overlap_after_filtering_hidden_selects_the_visible_player() {
        // Hidden GM was in front (would have been id 2 at depth 5). After
        // `hides_identity` filtering only the visible player remains.
        let candidates = vec![click_cand(1, 12.0, 18.0, false)];
        assert_eq!(resolve_selection_candidate(candidates, None), Some(EntityId(1)));
    }

    #[test]
    fn overlap_two_visible_players_picks_the_front_one() {
        let candidates = vec![click_cand(10, 4.0, 22.0, false), click_cand(11, 9.0, 8.0, false)];
        assert_eq!(resolve_selection_candidate(candidates, None), Some(EntityId(11)));
    }

    #[test]
    fn offscreen_clip_w_is_rejected() {
        assert!(!clip_is_on_screen(0.0));
        assert!(!clip_is_on_screen(-2.0));
        assert!(clip_is_on_screen(0.01));
    }

    #[test]
    fn resolve_effective_target_preserves_ground_items() {
        let candidates = vec![click_cand(1, 5.0, 10.0, false)];
        let ground_item_target = PickerTarget::Entity(EntityId(99));
        // If it was a ground item, it is preserved
        let resolved = resolve_effective_target(ground_item_target, candidates.clone(), None, true);
        assert_eq!(resolved, ground_item_target);

        // If not a ground item, entity candidate is selected
        let resolved2 = resolve_effective_target(PickerTarget::Tile { x: 5, y: 5 }, candidates, None, false);
        assert_eq!(resolved2, PickerTarget::Entity(EntityId(1)));
    }

    #[test]
    fn resolve_effective_target_falls_back_to_tile() {
        let tile = PickerTarget::Tile { x: 12, y: 34 };
        let resolved = resolve_effective_target(tile, vec![], None, false);
        assert_eq!(resolved, tile);
    }
}
