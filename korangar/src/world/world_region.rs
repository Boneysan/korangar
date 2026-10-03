//! Regional classification for the Ragnarok Online world.

use serde::{Deserialize, Serialize};

/// Major geographic and political regions of the Ragnarok Online world.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum WorldRegion {
    /// Rune-Midgarts Kingdom (Prontera, Geffen, Payon, Morroc, Izlude, Alberta,
    /// Al De Baran, Lutie, Comodo, Umbala).
    RuneMidgarts,
    /// Republic of Schwarzwald (Juno, Einbroch, Einbech, Lighthalzen, Hugel).
    Schwarzwald,
    /// Arunafeltz States (Rachel, Veins).
    Arunafeltz,
    /// Global Project and outlying lands (Amatsu, Gonryun, Louyang, Ayothaya,
    /// Brasilis, Jawaii, Lasagna).
    GlobalProject,
    /// Dimensional Gorge and foreign worlds (Ash Vacuum Midgard Camp, El
    /// Dicastes, Manuk, Splendide).
    DimensionalGorge,
    /// Prominent landmarks, dungeons, and interior complexes.
    DungeonLandmark,
}

impl WorldRegion {
    /// Human-readable name of the region.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::RuneMidgarts => "Rune-Midgarts Kingdom",
            Self::Schwarzwald => "Republic of Schwarzwald",
            Self::Arunafeltz => "Arunafeltz States",
            Self::GlobalProject => "Global Project",
            Self::DimensionalGorge => "Dimensional Gorge",
            Self::DungeonLandmark => "Dungeons & Landmarks",
        }
    }

    /// Descriptive summary of the region's geography and culture.
    #[allow(dead_code)]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::RuneMidgarts => "The ancient royal kingdom governed from Prontera, spanning central Midgard.",
            Self::Schwarzwald => "A technologically advanced republic dominated by the Rekenber Corporation and airship networks.",
            Self::Arunafeltz => "A devout theocracy centered in Rachel, dedicated to the goddess Freya.",
            Self::GlobalProject => "Far-flung foreign continents, islands, and cultural territories reached primarily by maritime vessels.",
            Self::DimensionalGorge => "Extradimensional expedition zones, the Ash Vacuum, and otherworldly sapient civilizations.",
            Self::DungeonLandmark => "Multi-level subterranean labyrinths, towers, and dangerous historic ruins.",
        }
    }
}

/// Map a known atlas map or town map name to its geographic region.
pub fn map_region(map_name: &str) -> Option<WorldRegion> {
    let normalized = map_name.strip_suffix(".gat").unwrap_or(map_name);
    match normalized {
        // Rune-Midgarts Kingdom
        "prontera" | "geffen" | "payon" | "morocc" | "izlude" | "alberta" | "aldebaran" | "xmas" | "comodo" | "umbala" | "prt_fild08"
        | "prt_fild01" | "prt_fild05" | "prt_fild07" | "gef_fild00" | "pay_fild01" | "pay_fild08" | "moc_fild07" | "cmd_fild04" => {
            Some(WorldRegion::RuneMidgarts)
        }

        // Republic of Schwarzwald
        "yuno" | "einbroch" | "einbech" | "lighthalzen" | "hugel" | "ein_fild04" | "yuno_fild01" | "hu_fild01" | "lhz_fild01" => {
            Some(WorldRegion::Schwarzwald)
        }

        // Arunafeltz States
        "rachel" | "veins" | "ra_fild01" | "ra_fild12" | "ve_fild01" => Some(WorldRegion::Arunafeltz),

        // Global Project
        "amatsu" | "gonryun" | "louyang" | "ayothaya" | "brasilis" | "jawaii" | "lasagna" | "malangdo" | "ama_fild01" | "gon_fild01"
        | "lou_fild01" | "ayo_fild01" | "bra_fild01" => Some(WorldRegion::GlobalProject),

        // Dimensional Gorge / Ash Vacuum
        "mid_camp" | "dicastes01" | "dicastes02" | "manuk" | "splendide" | "mora" | "eclage" | "moc_para01" => {
            Some(WorldRegion::DimensionalGorge)
        }

        // Dungeons & Landmarks
        "c_tower1" | "c_tower2" | "c_tower3" | "c_tower4" | "ama_dun01" | "ama_dun02" | "ama_dun03" | "izlu2dun" | "iz_dun00"
        | "iz_dun01" | "iz_dun02" | "iz_dun03" | "iz_dun04" | "orcsdun01" | "orcsdun02" | "glast_01" => Some(WorldRegion::DungeonLandmark),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{WorldRegion, map_region};

    #[test]
    fn map_region_identifies_major_towns_and_regions() {
        assert_eq!(map_region("prontera"), Some(WorldRegion::RuneMidgarts));
        assert_eq!(map_region("prontera.gat"), Some(WorldRegion::RuneMidgarts));
        assert_eq!(map_region("yuno"), Some(WorldRegion::Schwarzwald));
        assert_eq!(map_region("rachel"), Some(WorldRegion::Arunafeltz));
        assert_eq!(map_region("amatsu"), Some(WorldRegion::GlobalProject));
        assert_eq!(map_region("malangdo"), Some(WorldRegion::GlobalProject));
        assert_eq!(map_region("mid_camp"), Some(WorldRegion::DimensionalGorge));
        assert_eq!(map_region("c_tower1"), Some(WorldRegion::DungeonLandmark));
        assert_eq!(map_region("izlu2dun"), Some(WorldRegion::DungeonLandmark));
        assert_eq!(map_region("nonexistent_map"), None);
    }

    #[test]
    fn world_region_metadata_is_non_empty() {
        for region in [
            WorldRegion::RuneMidgarts,
            WorldRegion::Schwarzwald,
            WorldRegion::Arunafeltz,
            WorldRegion::GlobalProject,
            WorldRegion::DimensionalGorge,
            WorldRegion::DungeonLandmark,
        ] {
            assert!(!region.name().is_empty());
            assert!(!region.description().is_empty());
        }
    }
}
