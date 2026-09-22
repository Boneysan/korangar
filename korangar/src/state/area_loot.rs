//! Area-loot queue: click one floor item within four cells, then pick it and
//! every other reachable eligible pile in the same radius.
//!
//! Ownership, path, inventory, and weight here are **client filters**. The
//! server remains authoritative on the actual pickup.

#![allow(dead_code)]

use korangar_interface::element::StateElement;
use ragnarok_packets::{EntityId, TilePosition};
use rust_state::RustState;

/// Chebyshev radius from the player for area loot (playtest: four cells).
pub const AREA_LOOT_RANGE: u16 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloorLootCandidate {
    pub entity_id: EntityId,
    pub tile: TilePosition,
    /// False while another player still owns the pile.
    pub can_loot: bool,
    /// False if the client has already seen the pile vanish.
    pub present: bool,
    pub weight: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaLootCancel {
    Disappeared,
    ManualAction,
    Combat,
    PathFailure,
    MapChange,
    PlayerDrop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaLootError {
    ClickedMissing,
    ClickedOutOfRange,
    ClickedUnreachable,
    ClickedNotOwned,
    ClickedVanished,
    InventoryFull,
    Overweight,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, RustState, StateElement)]
pub struct AreaLootQueue {
    queued: Vec<EntityId>,
}

impl AreaLootQueue {
    pub fn queued(&self) -> &[EntityId] {
        &self.queued
    }

    pub fn is_empty(&self) -> bool {
        self.queued.is_empty()
    }

    pub fn clear(&mut self) {
        self.queued.clear();
    }

    pub fn cancel(&mut self, _reason: AreaLootCancel) {
        self.clear();
    }

    /// Dropped piles must not join the current queue (QW-046).
    pub fn ignore_new_ground_item(&self, entity_id: EntityId) -> bool {
        !self.queued.contains(&entity_id)
    }

    pub fn on_item_vanished(&mut self, entity_id: EntityId) {
        self.queued.retain(|id| *id != entity_id);
    }

    pub fn pop_front(&mut self) -> Option<EntityId> {
        if self.queued.is_empty() {
            None
        } else {
            Some(self.queued.remove(0))
        }
    }

    /// Replace the queue from a click. Existing queued ids are discarded.
    pub fn start_from_click(
        &mut self,
        player: TilePosition,
        clicked: EntityId,
        items: &[FloorLootCandidate],
        walkable: impl Fn(TilePosition) -> bool,
        inventory_slots_free: u32,
        remaining_weight: u32,
    ) -> Result<&[EntityId], AreaLootError> {
        let queued = build_area_loot_queue(player, clicked, items, walkable, inventory_slots_free, remaining_weight)?;
        self.queued = queued;
        Ok(self.queued())
    }
}

fn chebyshev(a: TilePosition, b: TilePosition) -> u16 {
    a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
}

/// Deterministic queue: clicked item first, then others by distance then
/// entity id. Ineligible piles are omitted; inventory/weight stop the fill.
pub fn build_area_loot_queue(
    player: TilePosition,
    clicked: EntityId,
    items: &[FloorLootCandidate],
    walkable: impl Fn(TilePosition) -> bool,
    inventory_slots_free: u32,
    remaining_weight: u32,
) -> Result<Vec<EntityId>, AreaLootError> {
    let clicked_item = items
        .iter()
        .find(|item| item.entity_id == clicked)
        .copied()
        .ok_or(AreaLootError::ClickedMissing)?;

    if !clicked_item.present {
        return Err(AreaLootError::ClickedVanished);
    }
    if !clicked_item.can_loot {
        return Err(AreaLootError::ClickedNotOwned);
    }
    if chebyshev(player, clicked_item.tile) > AREA_LOOT_RANGE {
        return Err(AreaLootError::ClickedOutOfRange);
    }
    if !walkable(clicked_item.tile) {
        return Err(AreaLootError::ClickedUnreachable);
    }
    if inventory_slots_free == 0 {
        return Err(AreaLootError::InventoryFull);
    }

    let mut others: Vec<&FloorLootCandidate> = items
        .iter()
        .filter(|item| item.entity_id != clicked)
        .filter(|item| item.present)
        .filter(|item| item.can_loot)
        .filter(|item| chebyshev(player, item.tile) <= AREA_LOOT_RANGE)
        .filter(|item| walkable(item.tile))
        .collect();
    others.sort_by_key(|item| (chebyshev(player, item.tile), item.entity_id.0));

    let mut queued = Vec::new();
    let mut slots = inventory_slots_free;
    let mut weight_left = remaining_weight;

    let mut consider = |item: &FloorLootCandidate| {
        if slots == 0 {
            return;
        }
        if item.weight > weight_left {
            return;
        }
        queued.push(item.entity_id);
        slots -= 1;
        weight_left = weight_left.saturating_sub(item.weight);
    };

    consider(&clicked_item);
    for item in others {
        consider(item);
    }

    if queued.is_empty() {
        if clicked_item.weight > remaining_weight {
            return Err(AreaLootError::Overweight);
        }
        return Err(AreaLootError::InventoryFull);
    }
    Ok(queued)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(x: u16, y: u16) -> TilePosition {
        TilePosition { x, y }
    }

    fn item(id: u32, x: u16, y: u16) -> FloorLootCandidate {
        FloorLootCandidate {
            entity_id: EntityId(id),
            tile: tile(x, y),
            can_loot: true,
            present: true,
            weight: 10,
        }
    }

    fn always(_: TilePosition) -> bool {
        true
    }

    #[test]
    fn range_includes_four_cells_and_excludes_five() {
        let player = tile(10, 10);
        let inside = item(1, 14, 10);
        let edge = item(2, 14, 14);
        let outside = item(3, 15, 10);
        let walkable = always;
        assert_eq!(
            build_area_loot_queue(player, EntityId(1), &[inside, edge, outside], walkable, 10, 1000).unwrap(),
            vec![EntityId(1), EntityId(2)]
        );
        assert_eq!(
            build_area_loot_queue(player, EntityId(3), &[inside, edge, outside], walkable, 10, 1000),
            Err(AreaLootError::ClickedOutOfRange)
        );
    }

    #[test]
    fn ownership_skips_foreign_piles_and_rejects_foreign_click() {
        let player = tile(10, 10);
        let ours = item(1, 10, 10);
        let mut theirs = item(2, 11, 10);
        theirs.can_loot = false;
        let walkable = always;
        assert_eq!(
            build_area_loot_queue(player, EntityId(1), &[ours, theirs], walkable, 10, 1000).unwrap(),
            vec![EntityId(1)]
        );
        assert_eq!(
            build_area_loot_queue(player, EntityId(2), &[ours, theirs], walkable, 10, 1000),
            Err(AreaLootError::ClickedNotOwned)
        );
    }

    #[test]
    fn unreachable_cells_are_omitted() {
        let player = tile(10, 10);
        let a = item(1, 10, 10);
        let blocked = item(2, 12, 10);
        let walkable = |pos: TilePosition| pos != blocked.tile;
        assert_eq!(
            build_area_loot_queue(player, EntityId(1), &[a, blocked], &walkable, 10, 1000).unwrap(),
            vec![EntityId(1)]
        );
        assert_eq!(
            build_area_loot_queue(player, EntityId(2), &[a, blocked], walkable, 10, 1000),
            Err(AreaLootError::ClickedUnreachable)
        );
    }

    #[test]
    fn vanished_items_are_omitted() {
        let player = tile(10, 10);
        let a = item(1, 10, 10);
        let mut gone = item(2, 11, 10);
        gone.present = false;
        let walkable = always;
        assert_eq!(
            build_area_loot_queue(player, EntityId(1), &[a, gone], walkable, 10, 1000).unwrap(),
            vec![EntityId(1)]
        );
        assert_eq!(
            build_area_loot_queue(player, EntityId(2), &[a, gone], walkable, 10, 1000),
            Err(AreaLootError::ClickedVanished)
        );
    }

    #[test]
    fn full_inventory_rejects_the_click() {
        let player = tile(10, 10);
        let a = item(1, 10, 10);
        let walkable = always;
        assert_eq!(
            build_area_loot_queue(player, EntityId(1), &[a], walkable, 0, 1000),
            Err(AreaLootError::InventoryFull)
        );
    }

    #[test]
    fn overweight_items_are_skipped_and_do_not_block_lighter_ones() {
        let player = tile(10, 10);
        let mut heavy = item(1, 10, 10);
        heavy.weight = 80;
        let mut light = item(2, 11, 10);
        light.weight = 10;
        let walkable = always;
        assert_eq!(
            build_area_loot_queue(player, EntityId(1), &[heavy, light], walkable, 5, 50),
            Ok(vec![EntityId(2)])
        );
        assert_eq!(
            build_area_loot_queue(player, EntityId(1), &[heavy], walkable, 5, 50),
            Err(AreaLootError::Overweight)
        );
    }

    #[test]
    fn two_player_race_orders_by_distance_then_entity_id() {
        let player = tile(10, 10);
        let near_high = item(9, 11, 10);
        let near_low = item(3, 11, 10);
        let far = item(4, 13, 10);
        let walkable = always;
        let shuffled = [far, near_high, near_low];
        let ordered = build_area_loot_queue(player, EntityId(3), &shuffled, walkable, 10, 1000).unwrap();
        assert_eq!(ordered, vec![EntityId(3), EntityId(9), EntityId(4)]);
        let again = build_area_loot_queue(player, EntityId(3), &[near_low, far, near_high], walkable, 10, 1000).unwrap();
        assert_eq!(ordered, again);
    }

    #[test]
    fn queue_state_pops_in_order() {
        let mut queue = AreaLootQueue::default();
        let player = tile(0, 0);
        queue
            .start_from_click(player, EntityId(1), &[item(1, 0, 0), item(2, 1, 0)], |_| true, 10, 1000)
            .unwrap();
        assert_eq!(queue.pop_front(), Some(EntityId(1)));
        assert_eq!(queue.pop_front(), Some(EntityId(2)));
        assert!(queue.is_empty());
    }

    fn filled_queue() -> AreaLootQueue {
        let mut queue = AreaLootQueue::default();
        queue
            .start_from_click(tile(0, 0), EntityId(1), &[item(1, 0, 0), item(2, 1, 0)], always, 10, 1000)
            .unwrap();
        queue
    }

    #[test]
    fn cancel_disappearance_drops_only_that_id() {
        let mut queue = filled_queue();
        queue.on_item_vanished(EntityId(1));
        assert_eq!(queue.queued(), &[EntityId(2)]);
        queue.on_item_vanished(EntityId(2));
        assert!(queue.is_empty());
    }

    #[test]
    fn cancel_manual_action_combat_path_map_and_drop_clear_all() {
        for reason in [
            AreaLootCancel::ManualAction,
            AreaLootCancel::Combat,
            AreaLootCancel::PathFailure,
            AreaLootCancel::MapChange,
            AreaLootCancel::PlayerDrop,
        ] {
            let mut queue = filled_queue();
            queue.cancel(reason);
            assert!(queue.is_empty(), "{reason:?} should clear the queue");
        }
    }

    #[test]
    fn player_drop_is_not_automatically_queued() {
        let queue = filled_queue();
        let dropped = EntityId(99);
        assert!(queue.ignore_new_ground_item(dropped));
        assert!(!queue.queued().contains(&dropped));
    }
}
