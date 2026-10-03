use serde::{Deserialize, Serialize};

use crate::dm::reference_data::reference_data;

/// Qualitative population density for broad spawn regions.
///
/// GDD §9.4 and §9.10 explicitly require broad qualitative density indicators
/// (Low, Medium, High) while forbidding disclosure of exact randomized spawn
/// cells or respawn timers.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PopulationDensity {
    Low,
    Medium,
    High,
}

impl PopulationDensity {
    #[allow(dead_code)]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub const fn label(&self) -> &'static str {
        match self {
            Self::Low => "Low population",
            Self::Medium => "Medium population",
            Self::High => "High population",
        }
    }

    pub const fn color_rgba(&self) -> (u8, u8, u8, u8) {
        match self {
            Self::High => (255, 180, 50, 65),
            Self::Medium => (60, 200, 220, 55),
            Self::Low => (120, 160, 230, 45),
        }
    }

    pub const fn outline_rgba(&self) -> (u8, u8, u8, u8) {
        match self {
            Self::High => (255, 200, 70, 180),
            Self::Medium => (80, 220, 240, 160),
            Self::Low => (140, 180, 250, 140),
        }
    }
}

/// A broad population rectangle on a map.
///
/// Does not reveal exact runtime cells or respawn timers.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BroadSpawnRectangle {
    pub monster_id: u32,
    pub monster_name: String,
    pub map: String,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub density: PopulationDensity,
    pub is_map_wide: bool,
}

impl BroadSpawnRectangle {
    #[allow(dead_code)]
    pub fn tooltip_text(&self) -> String {
        if self.is_map_wide {
            format!("{} (Map-wide • {})", self.monster_name, self.density.label())
        } else {
            format!("{} ({})", self.monster_name, self.density.label())
        }
    }
}

/// Query broad spawn rectangles for a given map and optional target monster.
///
/// If `monster_id` is specified, returns only broad spawn rectangles for that
/// monster. If `monster_id` is None, returns broad spawn rectangles for all
/// monsters with static spawns on the map. Low-coverage maps or maps without
/// static spawns return an empty `Vec`.
pub fn broad_spawn_rectangles_for_map(map_name: &str, monster_id: Option<u32>) -> Vec<BroadSpawnRectangle> {
    let clean_map = map_name.strip_suffix(".gat").unwrap_or(map_name).to_ascii_lowercase();
    let reference = reference_data();

    let mut rectangles = Vec::new();

    let mut check_monster = |monster: &crate::dm::reference_data::ReferenceMonster| {
        for region in &monster.spawn_regions {
            let region_map = region.map.strip_suffix(".gat").unwrap_or(&region.map).to_ascii_lowercase();
            if region_map != clean_map {
                continue;
            }
            for placement in &region.placements {
                let is_map_wide = placement.random_map_cell || (placement.x == 0 && placement.y == 0);
                let (x, y, width, height, density) = if is_map_wide {
                    let density = if placement.amount >= 20 {
                        PopulationDensity::High
                    } else if placement.amount >= 6 {
                        PopulationDensity::Medium
                    } else {
                        PopulationDensity::Low
                    };
                    (0, 0, 0, 0, density)
                } else {
                    let min_x = (placement.x - placement.x_spread).max(0) as u16;
                    let min_y = (placement.y - placement.y_spread).max(0) as u16;
                    let width = (2 * placement.x_spread + 1).max(1) as u16;
                    let height = (2 * placement.y_spread + 1).max(1) as u16;
                    let density = if placement.amount >= 15 {
                        PopulationDensity::High
                    } else if placement.amount >= 5 {
                        PopulationDensity::Medium
                    } else {
                        PopulationDensity::Low
                    };
                    (min_x, min_y, width, height, density)
                };

                rectangles.push(BroadSpawnRectangle {
                    monster_id: monster.id,
                    monster_name: monster.name.clone(),
                    map: clean_map.clone(),
                    x,
                    y,
                    width,
                    height,
                    density,
                    is_map_wide,
                });
            }
        }
    };

    if let Some(id) = monster_id {
        if let Some(monster) = reference.monster_by_id(id) {
            check_monster(monster);
        }
    } else {
        for monster in &reference.monsters {
            check_monster(monster);
        }
    }

    rectangles.sort_by(|a, b| {
        a.is_map_wide
            .cmp(&b.is_map_wide)
            .then_with(|| a.monster_id.cmp(&b.monster_id))
            .then_with(|| a.x.cmp(&b.x))
            .then_with(|| a.y.cmp(&b.y))
    });

    rectangles
}

#[cfg(test)]
mod tests {
    use super::{PopulationDensity, broad_spawn_rectangles_for_map};

    #[test]
    fn pilot_map_prt_fild08_broad_spawn_regions() {
        let all_regions = broad_spawn_rectangles_for_map("prt_fild08", None);
        assert!(!all_regions.is_empty(), "prt_fild08 must have static spawn regions");

        // prt_fild08 has random map-wide spawns
        assert!(all_regions.iter().all(|r| r.is_map_wide));

        // Poring (1002) is present on prt_fild08 with high density (amount = 30)
        let poring = all_regions.iter().find(|r| r.monster_id == 1002).expect("Poring on prt_fild08");
        assert_eq!(poring.density, PopulationDensity::High);
        assert_eq!(poring.monster_name, "Poring");
        assert!(poring.tooltip_text().contains("Map-wide • High population"));

        // Filtering by monster_id=1002 returns only Poring regions
        let poring_only = broad_spawn_rectangles_for_map("prt_fild08", Some(1002));
        assert!(!poring_only.is_empty());
        assert!(poring_only.iter().all(|r| r.monster_id == 1002));
    }

    #[test]
    fn pilot_map_prt_maze01_localized_spawn_rectangles() {
        let regions = broad_spawn_rectangles_for_map("prt_maze01", None);
        assert!(!regions.is_empty(), "prt_maze01 must have static spawn regions");

        // prt_maze01 has localized spawns with spreads
        let localized: Vec<_> = regions.iter().filter(|r| !r.is_map_wide).collect();
        assert!(!localized.is_empty(), "prt_maze01 must contain localized regions");

        // Poring placement: x=179, y=20, xs=21, ys=21
        // min_x = 179 - 21 = 158, min_y = 20 - 21 -> 0 (clamped), width = 2*21+1 = 43,
        // height = 43
        let poring = localized.iter().find(|r| r.monster_id == 1002).expect("localized Poring in maze");
        assert_eq!(poring.x, 158);
        assert_eq!(poring.y, 0);
        assert_eq!(poring.width, 43);
        assert_eq!(poring.height, 43);
        assert_eq!(poring.density, PopulationDensity::Medium); // amount 5 in localized placement

        // Fabre placement: x=99, y=20, xs=21, ys=21
        // min_x = 99 - 21 = 78, min_y = 0, width = 43, height = 43
        let fabre = localized.iter().find(|r| r.monster_id == 1007).expect("localized Fabre in maze");
        assert_eq!(fabre.x, 78);
        assert_eq!(fabre.y, 0);
        assert_eq!(fabre.width, 43);
        assert_eq!(fabre.height, 43);

        // Exact runtime spawn cells or timers are NOT exposed
        assert!(!poring.tooltip_text().contains("timer"));
        assert!(!poring.tooltip_text().contains("179, 20"));
    }

    #[test]
    fn pilot_map_izlude_cleaner_and_non_combat_towns() {
        let izlude_regions = broad_spawn_rectangles_for_map("izlude", None);
        // izlude has only the single city cleaner Wild Rose (ID 1261)
        assert_eq!(izlude_regions.len(), 1);
        assert_eq!(izlude_regions[0].monster_id, 1261);
        assert_eq!(izlude_regions[0].density, PopulationDensity::Low);

        // Searching for hunting targets (e.g. Poring) on izlude returns empty
        assert!(broad_spawn_rectangles_for_map("izlude", Some(1002)).is_empty());

        // Pure non-combat maps like Morroc or indoor maps return completely empty
        assert!(broad_spawn_rectangles_for_map("morroc", None).is_empty());
        assert!(broad_spawn_rectangles_for_map("prt_in", None).is_empty());
    }

    #[test]
    fn population_density_labels_and_colors() {
        assert_eq!(PopulationDensity::High.as_str(), "high");
        assert_eq!(PopulationDensity::Medium.as_str(), "medium");
        assert_eq!(PopulationDensity::Low.as_str(), "low");

        assert_eq!(PopulationDensity::High.label(), "High population");
        assert_eq!(PopulationDensity::Medium.label(), "Medium population");
        assert_eq!(PopulationDensity::Low.label(), "Low population");

        let (r_high, _, _, a_high) = PopulationDensity::High.color_rgba();
        let (_r_med, _, _, a_med) = PopulationDensity::Medium.color_rgba();
        let (r_low, _, _, a_low) = PopulationDensity::Low.color_rgba();
        assert!(r_high > r_low);
        assert!(a_high > 0 && a_med > 0 && a_low > 0);
    }
}
