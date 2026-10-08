use std::collections::HashSet;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, OnceLock};

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::TilePosition;
use rust_state::State;

use crate::PlayerPathExt;
use crate::graphics::{Color, CornerDiameter, ShadowPadding, Texture};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::{FontSize, GameFileLoader, ImageType, OverflowBehavior, TextureLoader};
use crate::renderer::LayoutExt;
use crate::state::discovery::DiscoveryState;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state, this_player};
use crate::world::{Library, NavigationEdge, TownPoi, WorldRegion, is_dangerous_map_level, map_region, navigation_graph, route_edges};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoadKind {
    Walk,
    Transport,
}

#[derive(Clone, Copy, Debug)]
pub struct AtlasRoad {
    pub from: usize,
    pub to: usize,
    pub kind: RoadKind,
}

#[derive(Clone, Copy)]
pub struct AtlasLocation {
    pub map: &'static str,
    pub label: &'static str,
    #[allow(dead_code)]
    pub region: WorldRegion,
    pub x: f32,
    pub y: f32,
    pub tile: TilePosition,
}

// Positions are a readable regional overview rather than in-game coordinates.
pub const LOCATIONS: &[AtlasLocation] = &[
    AtlasLocation {
        map: "yuno",
        label: "Juno",
        region: WorldRegion::Schwarzwald,
        x: 0.48,
        y: 0.12,
        tile: TilePosition { x: 157, y: 123 },
    },
    AtlasLocation {
        map: "rachel",
        label: "Rachel",
        region: WorldRegion::Arunafeltz,
        x: 0.69,
        y: 0.12,
        tile: TilePosition { x: 120, y: 120 },
    },
    AtlasLocation {
        map: "hugel",
        label: "Hugel",
        region: WorldRegion::Schwarzwald,
        x: 0.84,
        y: 0.27,
        tile: TilePosition { x: 96, y: 145 },
    },
    AtlasLocation {
        map: "aldebaran",
        label: "Al De Baran",
        region: WorldRegion::RuneMidgarts,
        x: 0.39,
        y: 0.27,
        tile: TilePosition { x: 140, y: 131 },
    },
    AtlasLocation {
        map: "geffen",
        label: "Geffen",
        region: WorldRegion::RuneMidgarts,
        x: 0.20,
        y: 0.34,
        tile: TilePosition { x: 119, y: 59 },
    },
    AtlasLocation {
        map: "prontera",
        label: "Prontera",
        region: WorldRegion::RuneMidgarts,
        x: 0.48,
        y: 0.43,
        tile: TilePosition { x: 155, y: 183 },
    },
    AtlasLocation {
        map: "payon",
        label: "Payon",
        region: WorldRegion::RuneMidgarts,
        x: 0.76,
        y: 0.39,
        tile: TilePosition { x: 160, y: 120 },
    },
    AtlasLocation {
        map: "morocc",
        label: "Morroc",
        region: WorldRegion::RuneMidgarts,
        x: 0.31,
        y: 0.57,
        tile: TilePosition { x: 156, y: 97 },
    },
    AtlasLocation {
        map: "izlude",
        label: "Izlude",
        region: WorldRegion::RuneMidgarts,
        x: 0.55,
        y: 0.60,
        tile: TilePosition { x: 128, y: 146 },
    },
    AtlasLocation {
        map: "alberta",
        label: "Alberta",
        region: WorldRegion::RuneMidgarts,
        x: 0.68,
        y: 0.58,
        tile: TilePosition { x: 28, y: 234 },
    },
    AtlasLocation {
        map: "amatsu",
        label: "Amatsu",
        region: WorldRegion::GlobalProject,
        x: 0.88,
        y: 0.52,
        tile: TilePosition { x: 198, y: 84 },
    },
    AtlasLocation {
        map: "comodo",
        label: "Comodo",
        region: WorldRegion::RuneMidgarts,
        x: 0.17,
        y: 0.72,
        tile: TilePosition { x: 184, y: 151 },
    },
    AtlasLocation {
        map: "umbala",
        label: "Umbala",
        region: WorldRegion::RuneMidgarts,
        x: 0.31,
        y: 0.78,
        tile: TilePosition { x: 97, y: 153 },
    },
    AtlasLocation {
        map: "jawaii",
        label: "Jawaii",
        region: WorldRegion::GlobalProject,
        x: 0.49,
        y: 0.82,
        tile: TilePosition { x: 251, y: 132 },
    },
    AtlasLocation {
        map: "louyang",
        label: "Louyang",
        region: WorldRegion::GlobalProject,
        x: 0.71,
        y: 0.76,
        tile: TilePosition { x: 217, y: 100 },
    },
    AtlasLocation {
        map: "gonryun",
        label: "Gonryun",
        region: WorldRegion::GlobalProject,
        x: 0.84,
        y: 0.68,
        tile: TilePosition { x: 160, y: 120 },
    },
    AtlasLocation {
        map: "ayothaya",
        label: "Ayothaya",
        region: WorldRegion::GlobalProject,
        x: 0.88,
        y: 0.84,
        tile: TilePosition { x: 208, y: 166 },
    },
    AtlasLocation {
        map: "brasilis",
        label: "Brasilis",
        region: WorldRegion::GlobalProject,
        x: 0.10,
        y: 0.88,
        tile: TilePosition { x: 196, y: 217 },
    },
    AtlasLocation {
        map: "einbroch",
        label: "Einbroch",
        region: WorldRegion::Schwarzwald,
        x: 0.57,
        y: 0.22,
        tile: TilePosition { x: 64, y: 200 },
    },
    AtlasLocation {
        map: "einbech",
        label: "Einbech",
        region: WorldRegion::Schwarzwald,
        x: 0.66,
        y: 0.29,
        tile: TilePosition { x: 63, y: 35 },
    },
    AtlasLocation {
        map: "lighthalzen",
        label: "Lighthalzen",
        region: WorldRegion::Schwarzwald,
        x: 0.55,
        y: 0.34,
        tile: TilePosition { x: 158, y: 92 },
    },
    AtlasLocation {
        map: "lasagna",
        label: "Lasagna",
        region: WorldRegion::GlobalProject,
        x: 0.96,
        y: 0.36,
        tile: TilePosition { x: 193, y: 182 },
    },
    AtlasLocation {
        map: "dicastes01",
        label: "El Dicastes",
        region: WorldRegion::DimensionalGorge,
        x: 0.80,
        y: 0.16,
        tile: TilePosition { x: 198, y: 187 },
    },
    AtlasLocation {
        map: "xmas",
        label: "Lutie",
        region: WorldRegion::RuneMidgarts,
        x: 0.31,
        y: 0.19,
        tile: TilePosition { x: 147, y: 134 },
    },
    AtlasLocation {
        map: "mid_camp",
        label: "Midgard Camp",
        region: WorldRegion::DimensionalGorge,
        x: 0.39,
        y: 0.69,
        tile: TilePosition { x: 180, y: 240 },
    },
    AtlasLocation {
        map: "c_tower1",
        label: "Clock Tower",
        region: WorldRegion::DungeonLandmark,
        x: 0.28,
        y: 0.27,
        tile: TilePosition { x: 235, y: 218 },
    },
    AtlasLocation {
        map: "ama_dun01",
        label: "Amatsu Cave",
        region: WorldRegion::DungeonLandmark,
        x: 0.93,
        y: 0.59,
        tile: TilePosition { x: 54, y: 107 },
    },
];

// Authored regional roads keep the overview legible, explicitly classifying
// overland walk roads and in-world transport connections (ferries, airships,
// sleighs, dimensional rifts). Actual route highlighting and the minimap
// guidance use the generated, verified warp graph.
pub const ROADS: &[AtlasRoad] = &[
    AtlasRoad {
        from: 0,
        to: 18,
        kind: RoadKind::Transport,
    }, // Juno <-> Einbroch (airship)
    AtlasRoad {
        from: 0,
        to: 22,
        kind: RoadKind::Transport,
    }, // Juno <-> El Dicastes
    AtlasRoad {
        from: 1,
        to: 22,
        kind: RoadKind::Transport,
    }, // Rachel <-> El Dicastes
    AtlasRoad {
        from: 2,
        to: 22,
        kind: RoadKind::Transport,
    }, // Hugel <-> El Dicastes
    AtlasRoad {
        from: 18,
        to: 19,
        kind: RoadKind::Walk,
    }, // Einbroch <-> Einbech
    AtlasRoad {
        from: 18,
        to: 9,
        kind: RoadKind::Transport,
    }, // Einbroch <-> Alberta (airship)
    AtlasRoad {
        from: 19,
        to: 20,
        kind: RoadKind::Walk,
    }, // Einbech <-> Lighthalzen
    AtlasRoad {
        from: 20,
        to: 9,
        kind: RoadKind::Transport,
    }, // Lighthalzen <-> Alberta (airship)
    AtlasRoad {
        from: 3,
        to: 4,
        kind: RoadKind::Walk,
    }, // Al De Baran <-> Geffen
    AtlasRoad {
        from: 3,
        to: 5,
        kind: RoadKind::Walk,
    }, // Al De Baran <-> Prontera
    AtlasRoad {
        from: 3,
        to: 23,
        kind: RoadKind::Transport,
    }, // Al De Baran <-> Lutie (Santa sleigh)
    AtlasRoad {
        from: 4,
        to: 5,
        kind: RoadKind::Walk,
    }, // Geffen <-> Prontera
    AtlasRoad {
        from: 4,
        to: 7,
        kind: RoadKind::Walk,
    }, // Geffen <-> Morroc
    AtlasRoad {
        from: 4,
        to: 23,
        kind: RoadKind::Transport,
    }, // Geffen <-> Lutie
    AtlasRoad {
        from: 5,
        to: 6,
        kind: RoadKind::Walk,
    }, // Prontera <-> Payon
    AtlasRoad {
        from: 5,
        to: 7,
        kind: RoadKind::Walk,
    }, // Prontera <-> Morroc
    AtlasRoad {
        from: 5,
        to: 8,
        kind: RoadKind::Walk,
    }, // Prontera <-> Izlude
    AtlasRoad {
        from: 5,
        to: 24,
        kind: RoadKind::Transport,
    }, // Prontera <-> Midgard Camp (gorge)
    AtlasRoad {
        from: 6,
        to: 8,
        kind: RoadKind::Walk,
    }, // Payon <-> Izlude
    AtlasRoad {
        from: 6,
        to: 9,
        kind: RoadKind::Walk,
    }, // Payon <-> Alberta
    AtlasRoad {
        from: 6,
        to: 15,
        kind: RoadKind::Transport,
    }, // Payon <-> Gonryun
    AtlasRoad {
        from: 7,
        to: 12,
        kind: RoadKind::Walk,
    }, // Morroc <-> Umbala
    AtlasRoad {
        from: 7,
        to: 24,
        kind: RoadKind::Transport,
    }, // Morroc <-> Midgard Camp
    AtlasRoad {
        from: 8,
        to: 9,
        kind: RoadKind::Walk,
    }, // Izlude <-> Alberta
    AtlasRoad {
        from: 8,
        to: 13,
        kind: RoadKind::Transport,
    }, // Izlude <-> Jawaii (ferry)
    AtlasRoad {
        from: 9,
        to: 10,
        kind: RoadKind::Transport,
    }, // Alberta <-> Amatsu (voyage)
    AtlasRoad {
        from: 9,
        to: 13,
        kind: RoadKind::Transport,
    }, // Alberta <-> Jawaii (ferry)
    AtlasRoad {
        from: 10,
        to: 15,
        kind: RoadKind::Transport,
    }, // Amatsu <-> Gonryun
    AtlasRoad {
        from: 10,
        to: 26,
        kind: RoadKind::Walk,
    }, // Amatsu <-> Amatsu Cave
    AtlasRoad {
        from: 11,
        to: 12,
        kind: RoadKind::Walk,
    }, // Comodo <-> Umbala
    AtlasRoad {
        from: 12,
        to: 17,
        kind: RoadKind::Transport,
    }, // Umbala <-> Brasilis
    AtlasRoad {
        from: 13,
        to: 14,
        kind: RoadKind::Transport,
    }, // Jawaii <-> Louyang
    AtlasRoad {
        from: 14,
        to: 15,
        kind: RoadKind::Transport,
    }, // Louyang <-> Gonryun
    AtlasRoad {
        from: 14,
        to: 16,
        kind: RoadKind::Transport,
    }, // Louyang <-> Ayothaya
];

/// Official worldview sheets are 1280×1024. `worldviewdata_table.lub` stores
/// each map's click box as the top-left origin rectangle on that sheet.
pub const WORLD_MAP_ART_WIDTH: f32 = 1280.0;
pub const WORLD_MAP_ART_HEIGHT: f32 = 1024.0;

const PAINTED_ART_VIEW_HEIGHT: f32 = 620.0;
const PAINTED_PAGE_ROW_HEIGHT: f32 = 28.0;
const DETAIL_BAND_HEIGHT: f32 = 150.0;
const PAINTED_ATLAS_HEIGHT: f32 = PAINTED_ART_VIEW_HEIGHT + PAINTED_PAGE_ROW_HEIGHT + DETAIL_BAND_HEIGHT;

/// One official world-map painting. The names are the sheet's atlas towns.
/// "localizing 01/02" in `World_List` are the file's own placeholders.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldMapPage {
    Midgard,
    Amatsu,
    Brasilis,
    Dimension,
    Lutie,
    Lasagna,
}

impl WorldMapPage {
    pub const ALL: [Self; 6] = [
        Self::Midgard,
        Self::Amatsu,
        Self::Brasilis,
        Self::Dimension,
        Self::Lutie,
        Self::Lasagna,
    ];

    const fn index(self) -> usize {
        match self {
            Self::Midgard => 0,
            Self::Amatsu => 1,
            Self::Brasilis => 2,
            Self::Dimension => 3,
            Self::Lutie => 4,
            Self::Lasagna => 5,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Midgard => "Midgard",
            Self::Amatsu => "Amatsu",
            Self::Brasilis => "Brasilis",
            Self::Dimension => "Dimension",
            Self::Lutie => "Lutie",
            Self::Lasagna => "Lasagna",
        }
    }

    const fn as_u8(self) -> u8 {
        self.index() as u8 + 1
    }

    fn from_u8(value: u8) -> Option<Self> {
        Self::ALL.get(usize::from(value.wrapping_sub(1))).copied()
    }
}

/// Paths relative to `data\texture\`, in `WorldMapPage::ALL` order.
/// Midgard, the Amatsu sheet, the Brasilis sheet, the dimension sheet,
/// Midgard North, and Far-Star. Crack of Dimension has none of the atlas towns.
const WORLD_MAP_SHEET_PATHS: [&str; 6] = [
    "유저인터페이스\\worldmap.jpg",
    "유저인터페이스\\worldmap_localizing1.bmp",
    "유저인터페이스\\worldmap_localizing2.bmp",
    "유저인터페이스\\worldmap_dimension.bmp",
    "유저인터페이스\\midgard_north.jpg",
    "유저인터페이스\\pasta.jpg",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldMapHotspot {
    pub map: &'static str,
    pub page: WorldMapPage,
    pub x1: u16,
    pub y1: u16,
    pub x2: u16,
    pub y2: u16,
}

/// Click boxes copied from `worldviewdata_table.lub` (`worldtable_*`, fields
/// 3–6). Prontera, Izlude, Yuno, Morocc, Geffen, and Alberta were cropped from
/// `worldmap.jpg` and land on those towns' icons.
pub const WORLD_MAP_HOTSPOTS: &[WorldMapHotspot] = &[
    WorldMapHotspot {
        map: "hugel",
        page: WorldMapPage::Midgard,
        x1: 871,
        y1: 0,
        x2: 927,
        y2: 57,
    },
    WorldMapHotspot {
        map: "yuno",
        page: WorldMapPage::Midgard,
        x1: 644,
        y1: 80,
        x2: 706,
        y2: 136,
    },
    WorldMapHotspot {
        map: "rachel",
        page: WorldMapPage::Midgard,
        x1: 224,
        y1: 292,
        x2: 282,
        y2: 351,
    },
    WorldMapHotspot {
        map: "lighthalzen",
        page: WorldMapPage::Midgard,
        x1: 401,
        y1: 292,
        x2: 459,
        y2: 346,
    },
    WorldMapHotspot {
        map: "einbech",
        page: WorldMapPage::Midgard,
        x1: 576,
        y1: 245,
        x2: 635,
        y2: 291,
    },
    WorldMapHotspot {
        map: "einbroch",
        page: WorldMapPage::Midgard,
        x1: 518,
        y1: 235,
        x2: 575,
        y2: 291,
    },
    WorldMapHotspot {
        map: "aldebaran",
        page: WorldMapPage::Midgard,
        x1: 812,
        y1: 292,
        x2: 870,
        y2: 351,
    },
    WorldMapHotspot {
        map: "geffen",
        page: WorldMapPage::Midgard,
        x1: 576,
        y1: 528,
        x2: 635,
        y2: 586,
    },
    WorldMapHotspot {
        map: "prontera",
        page: WorldMapPage::Midgard,
        x1: 812,
        y1: 587,
        x2: 870,
        y2: 643,
    },
    WorldMapHotspot {
        map: "izlude",
        page: WorldMapPage::Midgard,
        x1: 871,
        y1: 644,
        x2: 902,
        y2: 676,
    },
    WorldMapHotspot {
        map: "umbala",
        page: WorldMapPage::Midgard,
        x1: 308,
        y1: 660,
        x2: 356,
        y2: 716,
    },
    WorldMapHotspot {
        map: "comodo",
        page: WorldMapPage::Midgard,
        x1: 297,
        y1: 835,
        x2: 356,
        y2: 893,
    },
    WorldMapHotspot {
        map: "morocc",
        page: WorldMapPage::Midgard,
        x1: 606,
        y1: 845,
        x2: 655,
        y2: 893,
    },
    WorldMapHotspot {
        map: "payon",
        page: WorldMapPage::Midgard,
        x1: 967,
        y1: 746,
        x2: 1013,
        y2: 803,
    },
    WorldMapHotspot {
        map: "alberta",
        page: WorldMapPage::Midgard,
        x1: 1059,
        y1: 913,
        x2: 1113,
        y2: 964,
    },
    WorldMapHotspot {
        map: "c_tower1",
        page: WorldMapPage::Midgard,
        x1: 912,
        y1: 354,
        x2: 989,
        y2: 387,
    },
    WorldMapHotspot {
        map: "amatsu",
        page: WorldMapPage::Amatsu,
        x1: 282,
        y1: 258,
        x2: 366,
        y2: 343,
    },
    WorldMapHotspot {
        map: "gonryun",
        page: WorldMapPage::Amatsu,
        x1: 907,
        y1: 172,
        x2: 991,
        y2: 255,
    },
    WorldMapHotspot {
        map: "louyang",
        page: WorldMapPage::Amatsu,
        x1: 277,
        y1: 675,
        x2: 361,
        y2: 760,
    },
    WorldMapHotspot {
        map: "ayothaya",
        page: WorldMapPage::Amatsu,
        x1: 829,
        y1: 727,
        x2: 912,
        y2: 810,
    },
    WorldMapHotspot {
        map: "ama_dun01",
        page: WorldMapPage::Amatsu,
        x1: 420,
        y1: 310,
        x2: 514,
        y2: 344,
    },
    WorldMapHotspot {
        map: "brasilis",
        page: WorldMapPage::Brasilis,
        x1: 925,
        y1: 257,
        x2: 1009,
        y2: 342,
    },
    WorldMapHotspot {
        map: "mid_camp",
        page: WorldMapPage::Dimension,
        x1: 571,
        y1: 546,
        x2: 659,
        y2: 633,
    },
    WorldMapHotspot {
        map: "dicastes01",
        page: WorldMapPage::Dimension,
        x1: 977,
        y1: 543,
        x2: 1067,
        y2: 633,
    },
    WorldMapHotspot {
        map: "xmas",
        page: WorldMapPage::Lutie,
        x1: 795,
        y1: 904,
        x2: 854,
        y2: 963,
    },
    WorldMapHotspot {
        map: "lasagna",
        page: WorldMapPage::Lasagna,
        x1: 401,
        y1: 375,
        x2: 576,
        y2: 547,
    },
];

pub fn world_map_hotspot(map: &str) -> Option<&'static WorldMapHotspot> {
    WORLD_MAP_HOTSPOTS.iter().find(|hotspot| hotspot.map.eq_ignore_ascii_case(map))
}

/// `chosen` is 0 until the player picks a sheet. Then the pick stays for this
/// window. Reopening the window starts again from the current map's sheet.
pub fn displayed_world_map_page(chosen: u8, current_map: &str) -> WorldMapPage {
    if let Some(page) = WorldMapPage::from_u8(chosen) {
        return page;
    }
    world_map_hotspot(current_map)
        .map(|hotspot| hotspot.page)
        .unwrap_or(WorldMapPage::Midgard)
}

pub fn fit_world_map_art(viewport: Area) -> Area {
    let scale = (viewport.width / WORLD_MAP_ART_WIDTH).min(viewport.height / WORLD_MAP_ART_HEIGHT);
    let width = WORLD_MAP_ART_WIDTH * scale;
    let height = WORLD_MAP_ART_HEIGHT * scale;
    Area {
        left: viewport.left + (viewport.width - width) * 0.5,
        top: viewport.top + (viewport.height - height) * 0.5,
        width,
        height,
    }
}

pub fn world_map_hotspot_area(art: Area, hotspot: &WorldMapHotspot) -> Area {
    let scale_x = art.width / WORLD_MAP_ART_WIDTH;
    let scale_y = art.height / WORLD_MAP_ART_HEIGHT;
    Area {
        left: art.left + f32::from(hotspot.x1) * scale_x,
        top: art.top + f32::from(hotspot.y1) * scale_y,
        width: f32::from(hotspot.x2 - hotspot.x1) * scale_x,
        height: f32::from(hotspot.y2 - hotspot.y1) * scale_y,
    }
}

pub fn world_map_label_area(art: Area, hotspot_area: Area) -> Area {
    let height = 14.0;
    let width = hotspot_area.width.max(64.0);
    let max_left = (art.left + art.width - width).max(art.left);
    let left = (hotspot_area.left + hotspot_area.width * 0.5 - width * 0.5).clamp(art.left, max_left);
    let above = hotspot_area.top - height;
    let top = if above >= art.top {
        above
    } else {
        (hotspot_area.top + hotspot_area.height).min((art.top + art.height - height).max(art.top))
    };
    Area { left, top, width, height }
}

struct WorldMapSheets {
    pages: [Option<Arc<Texture>>; 6],
}

impl WorldMapSheets {
    fn load(game_file_loader: &GameFileLoader, texture_loader: &TextureLoader) -> Self {
        let pages = std::array::from_fn(|index| load_world_map_sheet(game_file_loader, texture_loader, WORLD_MAP_SHEET_PATHS[index]));
        Self { pages }
    }

    fn any(&self) -> bool {
        self.pages.iter().any(Option::is_some)
    }

    fn get(&self, page: WorldMapPage) -> Option<Arc<Texture>> {
        self.pages[page.index()].clone()
    }
}

fn load_world_map_sheet(game_file_loader: &GameFileLoader, texture_loader: &TextureLoader, relative: &str) -> Option<Arc<Texture>> {
    // A miss becomes the placeholder image, which would paint the wrong map.
    let full = format!("data\\texture\\{relative}");
    if !game_file_loader.file_exists(&full) && !game_file_loader.file_exists(&full.to_lowercase()) {
        return None;
    }
    texture_loader.get_or_load(relative, ImageType::Color).ok()
}

struct PageSelect {
    page: u8,
    chosen: Arc<AtomicU8>,
}

impl ClickHandler<ClientState> for PageSelect {
    fn handle_click(&self, _: &State<ClientState>, _: &mut EventQueue<ClientState>) {
        self.chosen.store(self.page, Ordering::Relaxed);
    }
}

struct PageChip {
    label: &'static str,
    select: PageSelect,
}

pub(crate) struct RouteClick {
    pub(crate) map: &'static str,
    pub(crate) position: TilePosition,
}

impl RouteClick {
    pub(crate) fn event(&self) -> InputEvent {
        InputEvent::SetNavigationDestination {
            map_name: self.map.to_owned(),
            x: self.position.x,
            y: self.position.y,
        }
    }
}

pub(crate) struct FacilityRouteClick {
    pub(crate) map_name: String,
    pub(crate) position: TilePosition,
}

impl FacilityRouteClick {
    pub(crate) fn event(&self) -> InputEvent {
        InputEvent::SetNavigationDestination {
            map_name: self.map_name.clone(),
            x: self.position.x,
            y: self.position.y,
        }
    }
}

impl ClickHandler<ClientState> for FacilityRouteClick {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(self.event());
    }
}

struct FacilityRoute {
    name: String,
    click: FacilityRouteClick,
}

fn town_poi_tile(poi: &TownPoi) -> Option<TilePosition> {
    Some(TilePosition {
        x: poi.x.try_into().ok()?,
        y: poi.y.try_into().ok()?,
    })
}

impl ClickHandler<ClientState> for RouteClick {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(self.event());
    }
}

struct AtlasNode {
    location: AtlasLocation,
    click: RouteClick,
}

struct AtlasView {
    library: Arc<Library>,
    sheets: WorldMapSheets,
    /// 0 follows the current map's sheet. A page click stores that sheet until
    /// the window is closed.
    chosen_page: Arc<AtomicU8>,
    page_chips: Vec<PageChip>,
    nodes: Vec<AtlasNode>,
    facility_routes: Vec<FacilityRoute>,
    details: [String; 6],
    dangerous_maps: HashSet<String>,
    /// Normalized map names that have an online party member who is not you.
    party_dot_maps: Vec<String>,
}

impl AtlasView {
    fn new(library: Arc<Library>, sheets: WorldMapSheets) -> Self {
        Self {
            library,
            sheets,
            chosen_page: Arc::new(AtomicU8::new(0)),
            page_chips: Vec::new(),
            nodes: LOCATIONS
                .iter()
                .copied()
                .map(|location| AtlasNode {
                    location,
                    click: RouteClick {
                        map: location.map,
                        position: location.tile,
                    },
                })
                .collect(),
            facility_routes: Vec::new(),
            details: destination_detail_lines("", None, None, 0, None, &[], &[], None, &[]),
            dangerous_maps: HashSet::new(),
            party_dot_maps: Vec::new(),
        }
    }
}

impl Element<ClientState> for AtlasView {
    type LayoutInfo = Area;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        let minimap_path = client_state().minimap();
        let minimap = state.get(&minimap_path);
        let current_map = minimap.map_name();
        let target = minimap.navigation_target().map(|target| target.map_name.as_str());
        let discovery_path = client_state().discovery();
        let discovery = state.get(&discovery_path);
        let party_path = client_state().party_state();
        let party = state.get(&party_path);
        let route = target.and_then(|destination| route_edges(current_map, destination));
        let selected_map = target.unwrap_or(current_map);
        let outgoing_exits = navigation_graph()
            .edges
            .iter()
            .filter(|edge| edge.from.map.eq_ignore_ascii_case(selected_map))
            .count();
        let visit_state = if discovery.visited_map(selected_map) {
            "visited"
        } else if discovery.map_snapshot_complete() {
            "not yet visited"
        } else {
            "discovery sync pending"
        };
        let party_report = party_map_report(
            &party
                .members()
                .iter()
                .map(|member| PartyMapInput {
                    name: member.name(),
                    map_name: member.map_name(),
                    online: member.online(),
                    is_local: party.is_local(member.account_id()),
                })
                .collect::<Vec<_>>(),
            selected_map,
        );
        self.party_dot_maps = party_report.dot_maps;
        let pois = self.library.town_pois(selected_map);
        self.facility_routes = pois
            .iter()
            .filter_map(|poi| {
                let position = town_poi_tile(poi)?;
                Some(FacilityRoute {
                    name: format!("Route: {}", poi.name),
                    click: FacilityRouteClick {
                        map_name: selected_map.to_owned(),
                        position,
                    },
                })
            })
            .take(3)
            .collect();
        let town_pois = pois.iter().map(|poi| poi.name.clone()).collect::<Vec<_>>();
        let reference = crate::dm::reference_data::reference_data();
        self.details = destination_detail_lines(
            current_map,
            target,
            route.as_deref(),
            outgoing_exits,
            Some(visit_state),
            &party_report.here,
            &party_report.elsewhere,
            reference.map_spawn_details(selected_map),
            &town_pois,
        );
        let player_level = state
            .try_get(&this_player().base_level())
            .copied()
            .map(|level| level.min(u16::MAX as usize) as u16);
        self.dangerous_maps = player_level.map_or_else(HashSet::new, |player_level| {
            LOCATIONS
                .iter()
                .filter(|location| {
                    reference
                        .map_spawn_summary(location.map)
                        .is_some_and(|(_, mean_level, _)| is_dangerous_map_level(mean_level, player_level))
                })
                .map(|location| location.map.to_ascii_lowercase())
                .collect()
        });
        let chosen = Arc::clone(&self.chosen_page);
        self.page_chips = WorldMapPage::ALL
            .into_iter()
            .map(|page| PageChip {
                label: page.label(),
                select: PageSelect {
                    page: page.as_u8(),
                    chosen: Arc::clone(&chosen),
                },
            })
            .collect();
        let height = if self.sheets.any() { PAINTED_ATLAS_HEIGHT } else { 520.0 };
        with_single_resolver(resolvers, |resolver| resolver.with_height(height))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        area: &Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let minimap_path = client_state().minimap();
        let minimap = state.get(&minimap_path);
        let discovery_path = client_state().discovery();
        let discovery = state.get(&discovery_path);
        let current_map = minimap.map_name();
        let target = minimap.navigation_target().map(|target| target.map_name.as_str());
        let route = target.and_then(|destination| route_edges(current_map, destination));
        let mut route_maps = Vec::new();
        if let Some(edges) = &route {
            for edge in edges {
                route_maps.push(edge.from.map.as_str());
                route_maps.push(edge.to.map.as_str());
            }
        }

        layout.add_rectangle(
            *area,
            CornerDiameter::uniform(10.0),
            Color::rgb_u8(15, 27, 42),
            Color::rgba_u8(0, 0, 0, 120),
            ShadowPadding::uniform(3.0),
        );

        let painted = self.sheets.any();
        if painted {
            let page = displayed_world_map_page(self.chosen_page.load(Ordering::Relaxed), current_map);
            self.draw_painted_sheet(layout, area, page, current_map, target, &route_maps, discovery);
        }

        if !painted {
            let node_width = 94.0;
            let node_height = 28.0;
            let point = |location: AtlasLocation| {
                (
                    area.left + location.x * (area.width - node_width - 8.0),
                    area.top + location.y * (area.height - node_height - 160.0),
                )
            };
            for road in available_roads() {
                let (Some(left), Some(right)) = (self.nodes.get(road.from), self.nodes.get(road.to)) else {
                    continue;
                };
                let (x1, y1) = point(left.location);
                let (x2, y2) = point(right.location);
                let color = match road.kind {
                    RoadKind::Walk => Color::rgb_u8(67, 100, 121),
                    RoadKind::Transport => Color::rgb_u8(82, 134, 184),
                };
                draw_dotted_connection(
                    layout,
                    x1 + node_width / 2.0,
                    y1 + node_height / 2.0,
                    x2 + node_width / 2.0,
                    y2 + node_height / 2.0,
                    color,
                );
            }

            // Mark route legs by linking the visible atlas locations found in the
            // computed route. The exact walkable line for the current map appears
            // as breadcrumbs on its minimap.
            let mut route_nodes: Vec<_> = self
                .nodes
                .iter()
                .filter(|node| route_maps.iter().any(|map| map.eq_ignore_ascii_case(node.location.map)))
                .collect();
            route_nodes.sort_by_key(|node| {
                route_maps
                    .iter()
                    .position(|map| map.eq_ignore_ascii_case(node.location.map))
                    .unwrap_or(usize::MAX)
            });
            for pair in route_nodes.windows(2) {
                let (x1, y1) = point(pair[0].location);
                let (x2, y2) = point(pair[1].location);
                draw_dotted_connection(
                    layout,
                    x1 + node_width / 2.0,
                    y1 + node_height / 2.0,
                    x2 + node_width / 2.0,
                    y2 + node_height / 2.0,
                    Color::rgb_u8(255, 204, 84),
                );
            }

            for node in &self.nodes {
                let (center_x, center_y) = point(node.location);
                let node_area = Area {
                    left: center_x,
                    top: center_y,
                    width: node_width,
                    height: node_height,
                };
                let is_current = current_map.eq_ignore_ascii_case(node.location.map);
                let is_target = target.is_some_and(|map| map.eq_ignore_ascii_case(node.location.map));
                let in_route = route_maps.iter().any(|map| map.eq_ignore_ascii_case(node.location.map));
                let is_dangerous = self.dangerous_maps.contains(node.location.map);
                let is_hovered = node_area.check().run(layout);
                let color = if is_target {
                    Color::rgb_u8(122, 79, 25)
                } else if is_current {
                    Color::rgb_u8(32, 94, 108)
                } else if in_route {
                    Color::rgb_u8(77, 71, 42)
                } else if is_hovered {
                    Color::rgb_u8(58, 78, 96)
                } else {
                    Color::rgb_u8(34, 49, 66)
                };
                layout.add_rectangle(
                    node_area,
                    CornerDiameter::uniform(7.0),
                    color,
                    if is_dangerous {
                        Color::rgb_u8(255, 126, 92)
                    } else {
                        Color::rgba_u8(0, 0, 0, 150)
                    },
                    ShadowPadding::uniform(2.0),
                );
                let visit_marker = if is_dangerous {
                    "!"
                } else if discovery.visited_map(node.location.map) {
                    // The game font (NotoSans) has no check mark; "✓" drew as a
                    // missing-glyph box. A bullet reads as "been here".
                    "•"
                } else if discovery.map_snapshot_complete() {
                    "·"
                } else {
                    "?"
                };
                layout.add_text(
                    node_area,
                    visit_marker,
                    FontSize(12.0),
                    Color::rgb_u8(145, 218, 173),
                    Color::WHITE,
                    HorizontalAlignment::Left { offset: 4.0, border: 0.0 },
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );
                layout.add_text(
                    Area {
                        left: node_area.left + 15.0,
                        width: node_area.width - 15.0,
                        ..node_area
                    },
                    node.location.label,
                    FontSize(12.0),
                    Color::rgb_u8(239, 235, 215),
                    Color::WHITE,
                    HorizontalAlignment::Center { offset: -4.0, border: 3.0 },
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );
                // A party member on this town, visible without pressing Navigate.
                if self.party_dot_maps.iter().any(|map| map.eq_ignore_ascii_case(node.location.map)) {
                    layout.add_rectangle(
                        Area {
                            left: node_area.left + node_area.width - 12.0,
                            top: node_area.top + 4.0,
                            width: 8.0,
                            height: 8.0,
                        },
                        CornerDiameter::uniform(4.0),
                        Color::rgb_u8(80, 220, 120),
                        Color::rgba_u8(0, 0, 0, 0),
                        ShadowPadding::uniform(0.0),
                    );
                }
                if is_hovered {
                    layout.register_click_handler(MouseButton::Left, &node.click);
                }
            }
        }

        for (index, line) in self.details.iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            if index == 4 && !self.facility_routes.is_empty() {
                let row = Area {
                    left: area.left + 12.0,
                    top: area.top + area.height - 144.0 + index as f32 * 18.0,
                    width: area.width - 24.0,
                    height: 18.0,
                };
                layout.add_text(
                    Area { width: 78.0, ..row },
                    "Facilities:",
                    FontSize(12.0),
                    Color::rgb_u8(195, 205, 214),
                    Color::WHITE,
                    HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );
                let route_width = ((row.width - 82.0) / self.facility_routes.len() as f32).min(150.0);
                for (route_index, facility) in self.facility_routes.iter().enumerate() {
                    let route_area = Area {
                        left: row.left + 80.0 + route_width * route_index as f32,
                        top: row.top,
                        width: route_width,
                        height: row.height,
                    };
                    let hovered = route_area.check().run(layout);
                    layout.add_rectangle(
                        route_area,
                        CornerDiameter::uniform(4.0),
                        if hovered {
                            Color::rgb_u8(58, 78, 96)
                        } else {
                            Color::rgb_u8(34, 49, 66)
                        },
                        Color::rgba_u8(0, 0, 0, 80),
                        ShadowPadding::uniform(0.0),
                    );
                    layout.add_text(
                        route_area,
                        &facility.name,
                        FontSize(11.0),
                        Color::rgb_u8(239, 235, 215),
                        Color::WHITE,
                        HorizontalAlignment::Center { offset: 0.0, border: 3.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );
                    if hovered {
                        layout.register_click_handler(MouseButton::Left, &facility.click);
                    }
                }
                continue;
            }
            layout.add_text(
                Area {
                    left: area.left + 12.0,
                    top: area.top + area.height - 144.0 + index as f32 * 18.0,
                    width: area.width - 24.0,
                    height: 18.0,
                },
                line,
                FontSize(12.0),
                Color::rgb_u8(195, 205, 214),
                Color::WHITE,
                HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
        }

        let status = if target.is_some() && route.is_none() {
            "No verified portal route from this map. Choose a reachable location."
        } else if painted {
            "Click a town to plot a route. Gold is the route on this sheet. Click does not teleport."
        } else if target.is_some() {
            // target only borrows minimap state, so display a static prompt here;
            // the exact destination is also shown beneath the map window.
            "Gold: route  •  Cyan: overland  •  Blue: in-world transport  •  Cyan dots: walkable trail"
        } else {
            "Select a town or destination to plot a route. Click Clear Route to cancel."
        };
        layout.add_text(
            Area {
                left: area.left + 12.0,
                top: area.top + area.height - 27.0,
                width: area.width - 24.0,
                height: 20.0,
            },
            status,
            FontSize(12.0),
            Color::rgb_u8(195, 205, 214),
            Color::WHITE,
            HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );
    }
}

impl AtlasView {
    fn draw_painted_sheet<'a>(
        &'a self,
        layout: &mut WindowLayout<'a, ClientState>,
        area: &Area,
        page: WorldMapPage,
        current_map: &str,
        target: Option<&str>,
        route_maps: &[&str],
        discovery: &DiscoveryState,
    ) {
        let art_height = (area.height - PAINTED_PAGE_ROW_HEIGHT - DETAIL_BAND_HEIGHT).max(1.0);
        let viewport = Area {
            left: area.left + 8.0,
            top: area.top + 4.0,
            width: (area.width - 16.0).max(1.0),
            height: (art_height - 8.0).max(1.0),
        };
        let art = fit_world_map_art(viewport);
        if let Some(texture) = self.sheets.get(page) {
            layout.add_texture(art, texture, Color::WHITE, true);
        } else {
            layout.add_text(
                viewport,
                "This map sheet is not in the client archives.",
                FontSize(14.0),
                Color::rgb_u8(195, 205, 214),
                Color::WHITE,
                HorizontalAlignment::Center { offset: 0.0, border: 8.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
        }

        for road in available_roads() {
            let (Some(from), Some(to)) = (self.nodes.get(road.from), self.nodes.get(road.to)) else {
                continue;
            };
            let (Some(from_box), Some(to_box)) = (world_map_hotspot(from.location.map), world_map_hotspot(to.location.map)) else {
                continue;
            };
            if from_box.page != page || to_box.page != page {
                continue;
            }
            let from_area = world_map_hotspot_area(art, from_box);
            let to_area = world_map_hotspot_area(art, to_box);
            let color = match road.kind {
                RoadKind::Walk => Color::rgb_u8(67, 100, 121),
                RoadKind::Transport => Color::rgb_u8(82, 134, 184),
            };
            draw_dotted_connection(
                layout,
                from_area.left + from_area.width * 0.5,
                from_area.top + from_area.height * 0.5,
                to_area.left + to_area.width * 0.5,
                to_area.top + to_area.height * 0.5,
                color,
            );
        }

        let mut route_nodes: Vec<_> = self
            .nodes
            .iter()
            .filter(|node| {
                route_maps.iter().any(|map| map.eq_ignore_ascii_case(node.location.map))
                    && world_map_hotspot(node.location.map).is_some_and(|hotspot| hotspot.page == page)
            })
            .collect();
        route_nodes.sort_by_key(|node| {
            route_maps
                .iter()
                .position(|map| map.eq_ignore_ascii_case(node.location.map))
                .unwrap_or(usize::MAX)
        });
        for pair in route_nodes.windows(2) {
            let (Some(from_box), Some(to_box)) = (world_map_hotspot(pair[0].location.map), world_map_hotspot(pair[1].location.map)) else {
                continue;
            };
            let from_area = world_map_hotspot_area(art, from_box);
            let to_area = world_map_hotspot_area(art, to_box);
            draw_dotted_connection(
                layout,
                from_area.left + from_area.width * 0.5,
                from_area.top + from_area.height * 0.5,
                to_area.left + to_area.width * 0.5,
                to_area.top + to_area.height * 0.5,
                Color::rgb_u8(255, 204, 84),
            );
        }

        for node in &self.nodes {
            let Some(hotspot) = world_map_hotspot(node.location.map) else {
                continue;
            };
            if hotspot.page != page {
                continue;
            }
            let hotspot_area = world_map_hotspot_area(art, hotspot);
            let label_area = world_map_label_area(art, hotspot_area);
            let click_area = union_area(hotspot_area, label_area);
            let is_current = current_map.eq_ignore_ascii_case(node.location.map);
            let is_target = target.is_some_and(|map| map.eq_ignore_ascii_case(node.location.map));
            let in_route = route_maps.iter().any(|map| map.eq_ignore_ascii_case(node.location.map));
            let is_dangerous = self.dangerous_maps.contains(node.location.map);
            let is_hovered = click_area.check().run(layout);
            let border = if is_target {
                Some(Color::rgb_u8(255, 204, 84))
            } else if is_current {
                Some(Color::rgb_u8(120, 220, 230))
            } else if is_dangerous {
                Some(Color::rgb_u8(255, 126, 92))
            } else if in_route {
                Some(Color::rgb_u8(196, 164, 84))
            } else if is_hovered {
                Some(Color::rgb_u8(230, 230, 220))
            } else {
                None
            };
            if let Some(color) = border {
                add_area_border(layout, hotspot_area, color, 2.0);
            }
            layout.add_rectangle(
                label_area,
                CornerDiameter::uniform(3.0),
                Color::rgba_u8(15, 27, 42, 190),
                Color::rgba_u8(0, 0, 0, 0),
                ShadowPadding::uniform(0.0),
            );
            let visit_marker = if is_dangerous {
                "!"
            } else if discovery.visited_map(node.location.map) {
                "•"
            } else if discovery.map_snapshot_complete() {
                "·"
            } else {
                "?"
            };
            layout.add_text(
                label_area,
                visit_marker,
                FontSize(11.0),
                Color::rgb_u8(145, 218, 173),
                Color::WHITE,
                HorizontalAlignment::Left { offset: 3.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
            layout.add_text(
                Area {
                    left: label_area.left + 12.0,
                    width: (label_area.width - 12.0).max(1.0),
                    ..label_area
                },
                node.location.label,
                FontSize(11.0),
                Color::rgb_u8(239, 235, 215),
                Color::WHITE,
                HorizontalAlignment::Center { offset: 0.0, border: 2.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
            if self.party_dot_maps.iter().any(|map| map.eq_ignore_ascii_case(node.location.map)) {
                layout.add_rectangle(
                    Area {
                        left: hotspot_area.left + hotspot_area.width - 8.0,
                        top: hotspot_area.top,
                        width: 8.0,
                        height: 8.0,
                    },
                    CornerDiameter::uniform(4.0),
                    Color::rgb_u8(80, 220, 120),
                    Color::rgba_u8(0, 0, 0, 0),
                    ShadowPadding::uniform(0.0),
                );
            }
            if is_hovered {
                layout.register_click_handler(MouseButton::Left, &node.click);
            }
        }

        let off_sheet: Vec<_> = self
            .nodes
            .iter()
            .filter(|node| world_map_hotspot(node.location.map).is_none())
            .collect();
        let chip_count = (self.page_chips.len() + off_sheet.len()).max(1) as f32;
        let row = Area {
            left: area.left + 8.0,
            top: area.top + art_height,
            width: (area.width - 16.0).max(1.0),
            height: PAINTED_PAGE_ROW_HEIGHT - 2.0,
        };
        let chip_width = (row.width / chip_count).min(120.0);
        for (index, chip) in self.page_chips.iter().enumerate() {
            let chip_area = Area {
                left: row.left + chip_width * index as f32,
                top: row.top,
                width: chip_width - 4.0,
                height: row.height,
            };
            let active = WorldMapPage::from_u8(chip.select.page) == Some(page);
            let hovered = chip_area.check().run(layout);
            layout.add_rectangle(
                chip_area,
                CornerDiameter::uniform(4.0),
                if active || hovered {
                    Color::rgb_u8(58, 96, 112)
                } else {
                    Color::rgb_u8(34, 49, 66)
                },
                Color::rgba_u8(0, 0, 0, 80),
                ShadowPadding::uniform(0.0),
            );
            layout.add_text(
                chip_area,
                chip.label,
                FontSize(11.0),
                Color::rgb_u8(239, 235, 215),
                Color::WHITE,
                HorizontalAlignment::Center { offset: 0.0, border: 2.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
            if hovered {
                layout.register_click_handler(MouseButton::Left, &chip.select);
            }
        }
        for (index, node) in off_sheet.iter().enumerate() {
            let chip_area = Area {
                left: row.left + chip_width * (self.page_chips.len() + index) as f32,
                top: row.top,
                width: chip_width - 4.0,
                height: row.height,
            };
            let hovered = chip_area.check().run(layout);
            layout.add_rectangle(
                chip_area,
                CornerDiameter::uniform(4.0),
                if hovered {
                    Color::rgb_u8(58, 78, 96)
                } else {
                    Color::rgb_u8(34, 49, 66)
                },
                Color::rgba_u8(0, 0, 0, 80),
                ShadowPadding::uniform(0.0),
            );
            layout.add_text(
                chip_area,
                node.location.label,
                FontSize(11.0),
                Color::rgb_u8(239, 235, 215),
                Color::WHITE,
                HorizontalAlignment::Center { offset: 0.0, border: 2.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
            if hovered {
                layout.register_click_handler(MouseButton::Left, &node.click);
            }
        }
    }
}

fn union_area(first: Area, second: Area) -> Area {
    let left = first.left.min(second.left);
    let top = first.top.min(second.top);
    let right = (first.left + first.width).max(second.left + second.width);
    let bottom = (first.top + first.height).max(second.top + second.height);
    Area {
        left,
        top,
        width: right - left,
        height: bottom - top,
    }
}

fn add_area_border(layout: &mut WindowLayout<'_, ClientState>, area: Area, color: Color, thickness: f32) {
    let edges = [
        Area {
            left: area.left,
            top: area.top,
            width: area.width,
            height: thickness,
        },
        Area {
            left: area.left,
            top: area.top + area.height - thickness,
            width: area.width,
            height: thickness,
        },
        Area {
            left: area.left,
            top: area.top,
            width: thickness,
            height: area.height,
        },
        Area {
            left: area.left + area.width - thickness,
            top: area.top,
            width: thickness,
            height: area.height,
        },
    ];
    for edge in edges {
        layout.add_rectangle(
            edge,
            CornerDiameter::uniform(0.0),
            color,
            Color::rgba_u8(0, 0, 0, 0),
            ShadowPadding::uniform(0.0),
        );
    }
}

fn available_roads() -> &'static [AtlasRoad] {
    static AVAILABLE: OnceLock<Vec<AtlasRoad>> = OnceLock::new();
    AVAILABLE.get_or_init(|| {
        ROADS
            .iter()
            .copied()
            .filter(|road| route_edges(LOCATIONS[road.from].map, LOCATIONS[road.to].map).is_some())
            .collect()
    })
}

#[allow(clippy::too_many_arguments)] // one input per displayed fact
fn destination_detail_lines(
    current_map: &str,
    target_map: Option<&str>,
    route: Option<&[&'static NavigationEdge]>,
    outgoing_exits: usize,
    visit_state: Option<&str>,
    party_members_here: &[String],
    party_elsewhere: &[String],
    population: Option<crate::dm::reference_data::MapSpawnDetails>,
    town_pois: &[String],
) -> [String; 6] {
    let selected_map = target_map.unwrap_or(current_map);
    let visited = visit_state.unwrap_or("discovery sync pending");
    let region_info = map_region(selected_map)
        .map(|region| format!(" • {}", region.name()))
        .unwrap_or_default();
    let map_heading = match target_map {
        Some(target) => format!("Destination: {target} • {visited}{region_info}"),
        None => format!("Current map: {selected_map} • {visited}{region_info}"),
    };
    let (level_line, population_line) = population.map_or_else(
        || {
            (
                "Suggested level: unavailable (low-coverage or non-combat map)".to_owned(),
                "Static population: no verified spawn records (low coverage)".to_owned(),
            )
        },
        |details| {
            let range_str = if details.min_level != details.max_level {
                format!(" (range: {}–{})", details.min_level, details.max_level)
            } else {
                String::new()
            };
            (
                format!("Suggested level: ~{}{range_str} (static-spawn mean)", details.mean_level),
                format!(
                    "Static population: {} spawn records • {} species",
                    details.records, details.species
                ),
            )
        },
    );
    let party_line = party_presence_line(party_members_here, party_elsewhere);
    let poi_line = if town_pois.is_empty() {
        "Towninfo facilities: none listed for this map".to_owned()
    } else {
        let shown = town_pois.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
        if town_pois.len() > 3 {
            format!("Towninfo facilities: {shown}, +{} more", town_pois.len() - 3)
        } else {
            format!("Towninfo facilities: {shown}")
        }
    };
    let (route_summary, next_exit) = match (target_map, route) {
        (None, _) => (
            format!("Verified outgoing connections: {outgoing_exits}"),
            "choose a destination to plot a route".to_owned(),
        ),
        (Some(target), None) => (
            "No verified route".to_owned(),
            format!("No portal route from {current_map} to {target}"),
        ),
        (Some(target), Some([])) => ("Already at destination".to_owned(), format!("you are on {target}")),
        (Some(_), Some(edges)) => {
            let edge = edges[0];
            let transport_count = edges.iter().filter(|e| e.kind == "npc_service").count();
            let walk_count = edges.len() - transport_count;
            let leg_composition = if transport_count > 0 {
                format!(
                    "{} travel legs ({} walk, {} transport)",
                    edges.len(),
                    walk_count,
                    transport_count
                )
            } else {
                format!("{} travel legs", edges.len())
            };
            let edge_action = if edge.kind == "npc_service" {
                let action = edge.action.as_deref().unwrap_or("use the listed travel service");
                let requirements = edge
                    .requirements
                    .as_deref()
                    .map(|requirements| format!(" • {requirements}"))
                    .unwrap_or_default();
                let availability = if edge.availability == "conditional" { " (conditional)" } else { "" };
                format!(
                    "next: service at {} ({}, {}) — {action}{availability}{requirements} › {}",
                    edge.from.map, edge.from.x, edge.from.y, edge.to.map
                )
            } else {
                format!(
                    "next: portal at {} ({}, {}) › {}",
                    edge.from.map, edge.from.x, edge.from.y, edge.to.map
                )
            };
            (format!("{leg_composition} • {outgoing_exits} connections"), edge_action)
        }
    };
    [
        map_heading,
        level_line,
        population_line,
        party_line,
        poi_line,
        format!("Connections: {route_summary} • {next_exit}"),
    ]
}

fn normalized_map_name(map_name: &str) -> &str {
    let name = map_name.trim();
    name.strip_suffix(".gat").or_else(|| name.strip_suffix(".GAT")).unwrap_or(name)
}

struct PartyMapInput<'a> {
    name: &'a str,
    map_name: &'a str,
    online: bool,
    is_local: bool,
}

struct PartyMapReport {
    here: Vec<String>,
    elsewhere: Vec<String>,
    /// Lowercased map names that should show a party dot on the atlas.
    dot_maps: Vec<String>,
}

/// Who is on the selected map, who is on some other map, and which atlas
/// towns get a dot. The local player is listed in "here" when the server
/// reports them on the selected map, and never gets a dot or an elsewhere row.
fn party_map_report(members: &[PartyMapInput<'_>], selected_map: &str) -> PartyMapReport {
    let mut here = Vec::new();
    let mut elsewhere = Vec::new();
    let mut dot_maps = Vec::new();
    for member in members {
        if !member.online {
            continue;
        }
        let map = normalized_map_name(member.map_name);
        if map.is_empty() {
            continue;
        }
        if map.eq_ignore_ascii_case(selected_map) {
            here.push(member.name.to_owned());
        } else if !member.is_local {
            elsewhere.push(format!("{} ({map})", member.name));
        }
        if !member.is_local {
            let key = map.to_ascii_lowercase();
            if !dot_maps.iter().any(|existing| existing == &key) {
                dot_maps.push(key);
            }
        }
    }
    if elsewhere.len() > 4 {
        let extra = elsewhere.len() - 4;
        elsewhere.truncate(4);
        elsewhere.push(format!("+{extra} more"));
    }
    PartyMapReport { here, elsewhere, dot_maps }
}

fn party_presence_line(here: &[String], elsewhere: &[String]) -> String {
    let here_text = if here.is_empty() {
        "no online members reporting this map".to_owned()
    } else {
        here.join(", ")
    };
    if elsewhere.is_empty() {
        format!("Party here: {here_text}")
    } else {
        format!("Party here: {here_text} | Elsewhere: {}", elsewhere.join(", "))
    }
}

fn draw_dotted_connection(layout: &mut WindowLayout<'_, ClientState>, x1: f32, y1: f32, x2: f32, y2: f32, color: Color) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let distance = (dx * dx + dy * dy).sqrt();
    let count = (distance / 9.0).ceil() as usize;
    for index in 1..count {
        let ratio = index as f32 / count as f32;
        let x = x1 + dx * ratio;
        let y = y1 + dy * ratio;
        layout.add_rectangle(
            Area {
                left: x - 2.0,
                top: y - 2.0,
                width: 4.0,
                height: 4.0,
            },
            CornerDiameter::uniform(2.0),
            color,
            Color::rgba_u8(0, 0, 0, 0),
            ShadowPadding::uniform(0.0),
        );
    }
}

#[allow(dead_code)]
impl AtlasLocation {
    pub const fn region(&self) -> WorldRegion {
        self.region
    }
}

pub struct MapsWindow {
    library: Arc<Library>,
    sheets: WorldMapSheets,
}

impl MapsWindow {
    pub fn new(library: Arc<Library>, game_file_loader: &GameFileLoader, texture_loader: &TextureLoader) -> Self {
        Self {
            library,
            sheets: WorldMapSheets::load(game_file_loader, texture_loader),
        }
    }
}

impl CustomWindow<ClientState> for MapsWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Maps)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "World Map & Route Finder",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                AtlasView::new(self.library, self.sheets),
                button! {
                    text: "Clear Route",
                    tooltip: "Clear the current destination and breadcrumb trail",
                    event: InputEvent::ClearNavigationDestination,
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use korangar_interface::event::{Event, EventQueue};
    use korangar_interface::layout::area::Area;
    use ragnarok_packets::TilePosition;

    use super::{
        FacilityRouteClick, LOCATIONS, PartyMapInput, ROADS, RoadKind, RouteClick, WORLD_MAP_ART_HEIGHT, WORLD_MAP_ART_WIDTH,
        WORLD_MAP_HOTSPOTS, WorldMapPage, available_roads, destination_detail_lines, displayed_world_map_page, fit_world_map_art,
        normalized_map_name, party_map_report, town_poi_tile, world_map_hotspot, world_map_hotspot_area, world_map_label_area,
    };
    use crate::input::InputEvent;
    use crate::state::discovery::DiscoveryState;
    use crate::world::{TownPoi, TownPoiKind, is_dangerous_map_level, map_region, navigation_graph, route_edges};

    #[test]
    fn party_members_show_on_the_atlas_without_a_route() {
        let members = [
            PartyMapInput {
                name: "Local",
                map_name: "prontera.gat",
                online: true,
                is_local: true,
            },
            PartyMapInput {
                name: "Alice",
                map_name: "prontera.gat",
                online: true,
                is_local: false,
            },
            PartyMapInput {
                name: "Bob",
                map_name: "izlude.GAT",
                online: true,
                is_local: false,
            },
            PartyMapInput {
                name: "Carol",
                map_name: "geffen",
                online: false,
                is_local: false,
            },
            PartyMapInput {
                name: "Dana",
                map_name: "  ",
                online: true,
                is_local: false,
            },
        ];
        let report = party_map_report(&members, "prontera");
        assert_eq!(report.here, vec!["Local".to_owned(), "Alice".to_owned()]);
        assert_eq!(report.elsewhere, vec!["Bob (izlude)".to_owned()]);
        assert_eq!(report.dot_maps, vec!["prontera".to_owned(), "izlude".to_owned()]);

        let line = super::party_presence_line(&report.here, &report.elsewhere);
        assert_eq!(line, "Party here: Local, Alice | Elsewhere: Bob (izlude)");

        let alone = party_map_report(&members[..1], "prontera");
        assert!(alone.dot_maps.is_empty());
        assert_eq!(alone.here, vec!["Local".to_owned()]);
        assert!(alone.elsewhere.is_empty());
    }

    #[test]
    fn atlas_poi_routes_accept_only_nonnegative_client_coordinates() {
        let poi = TownPoi {
            name: "Kafra Employee".to_owned(),
            x: 156,
            y: 191,
            kind: TownPoiKind::Kafra,
        };
        assert_eq!(town_poi_tile(&poi), Some(TilePosition { x: 156, y: 191 }));

        let invalid = TownPoi { x: -1, ..poi };
        assert_eq!(town_poi_tile(&invalid), None);
    }

    #[test]
    fn atlas_destination_details_use_verified_route_edges_and_visit_state() {
        let route = route_edges("prontera", "izlude").expect("known route");
        let graph = navigation_graph();
        let outgoing = graph
            .edges
            .iter()
            .filter(|edge| edge.from.map.eq_ignore_ascii_case("izlude"))
            .count();
        let party = vec!["Alice".to_owned()];
        let pois = vec!["Kafra Employee".to_owned(), "Tool Dealer".to_owned()];
        let population = crate::dm::reference_data::reference_data().map_spawn_details("izlude");
        let lines = destination_detail_lines(
            "prontera",
            Some("izlude"),
            Some(&route),
            outgoing,
            Some("visited"),
            &party,
            &[],
            population,
            &pois,
        );

        assert!(lines[0].contains("Destination: izlude • visited"));
        assert!(lines[0].contains("Rune-Midgarts Kingdom"));
        assert!(lines[1].contains("Suggested level:"));
        assert!(lines[2].contains("Static population:"));
        assert!(lines[3].contains("Alice"));
        assert!(lines[4].contains("Kafra Employee"));
        assert!(lines[5].contains(&format!("{} travel legs", route.len())));
        assert!(lines[5].contains(&route[0].from.map));
        assert!(lines[5].contains(&route[0].to.map));
    }

    #[test]
    fn atlas_destination_details_explain_conditional_npc_service_legs() {
        let route = route_edges("izlude", "iz_dun00").expect("authored conditional ferry route");
        let lines = destination_detail_lines("izlude", Some("iz_dun00"), Some(&route), 2, Some("visited"), &[], &[], None, &[
        ]);

        assert!(lines[5].contains("2 travel legs (1 walk, 1 transport)"));
        assert!(lines[5].contains("service at izlude (197, 205)"));
        assert!(lines[5].contains("choose Byalan Island"));
        assert!(lines[5].contains("conditional"));
        assert!(lines[5].contains("150 zeny"));
        assert!(lines[5].contains("izlu2dun"));
    }

    #[test]
    fn atlas_destination_details_do_not_claim_routes_when_graph_has_none() {
        let lines = destination_detail_lines("unknown_map", Some("unknown_destination"), None, 0, None, &[], &[], None, &[]);
        assert!(lines[0].contains("discovery sync pending"));
        assert!(lines[1].contains("unavailable"));
        assert!(lines[2].contains("no verified spawn records"));
        assert!(lines[4].contains("none listed for this map"));
        assert!(lines[5].contains("No verified route"));

        let empty = destination_detail_lines("prontera", None, None, 3, Some("visited"), &[], &[], None, &[]);
        assert!(empty[0].contains("Current map: prontera • visited"));
        assert!(empty[0].contains("Rune-Midgarts Kingdom"));
        assert!(empty[4].contains("none listed"));
        assert!(empty[5].contains("3"));
        assert_eq!(normalized_map_name("izlude.gat"), "izlude");
    }

    #[test]
    fn visited_account_vs_unvisited_account() {
        let mut discovery = DiscoveryState::default();
        discovery.set_account_id(1001);

        // Before server delta or snapshot sync, status is unvisited and sync pending.
        assert!(!discovery.visited_map("prontera"));
        assert!(!discovery.map_snapshot_complete());
        let sync_pending = destination_detail_lines("geffen", Some("prontera"), None, 4, None, &[], &[], None, &[]);
        assert!(sync_pending[0].contains("discovery sync pending"));

        // Receive private server discovery line marking prontera visited.
        let recognized = discovery.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:1001:prontera]", Some(1001));
        assert!(recognized, "server map discovery protocol line must be recognized");
        assert!(discovery.visited_map("prontera"));
        assert!(!discovery.visited_map("geffen"));

        let visited_details = destination_detail_lines(
            "geffen",
            Some("prontera"),
            None,
            4,
            Some(if discovery.visited_map("prontera") {
                "visited"
            } else {
                "not yet visited"
            }),
            &[],
            &[],
            None,
            &[],
        );
        assert!(visited_details[0].contains("Destination: prontera • visited • Rune-Midgarts Kingdom"));

        let unvisited_details = destination_detail_lines(
            "prontera",
            Some("geffen"),
            None,
            4,
            Some(if discovery.visited_map("geffen") {
                "visited"
            } else {
                "not yet visited"
            }),
            &[],
            &[],
            None,
            &[],
        );
        assert!(unvisited_details[0].contains("Destination: geffen • not yet visited • Rune-Midgarts Kingdom"));
    }

    #[test]
    fn two_characters_on_same_account_share_visit_history() {
        let mut discovery = DiscoveryState::default();
        discovery.set_account_id(1001);

        // Character 1 (e.g. Swordsman) visits Prontera, Izlude, and Geffen.
        assert!(discovery.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:1001:prontera]", Some(1001)));
        assert!(discovery.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:1001:izlude]", Some(1001)));
        assert!(discovery.receive_server_line("[KORANGAR-MAP-DISCOVERY:v1:visited:1001:geffen]", Some(1001)));
        assert_eq!(discovery.visited_map_count(), 3);

        // Player switches to Character 2 (e.g. Mage) on the same account (account_id
        // remains 1001).
        discovery.set_account_id(1001);
        assert_eq!(discovery.visited_map_count(), 3, "same account must retain all visited maps");
        assert!(discovery.visited_map("prontera"));
        assert!(discovery.visited_map("izlude"));
        assert!(discovery.visited_map("geffen"));

        // Player switches to an alternate account (account_id: 1002).
        discovery.set_account_id(1002);
        assert_eq!(discovery.visited_map_count(), 0, "different account must reset visited maps");
        assert!(!discovery.visited_map("prontera"));
    }

    #[test]
    fn conditional_in_world_transport_routes() {
        let graph = navigation_graph();

        // 1. Izlude -> Byalan Island: conditional ferry service
        let byalan_route = route_edges("izlude", "iz_dun00").expect("ferry route to Byalan");
        assert_eq!(byalan_route.len(), 2);
        assert_eq!(byalan_route[0].id, "service-izlude-byalan-ferry");
        assert_eq!(byalan_route[0].kind, "npc_service");
        assert_eq!(byalan_route[0].availability, "conditional");
        assert_eq!(byalan_route[0].requirements.as_deref(), Some("Costs 150 zeny."));
        assert_eq!(byalan_route[1].kind, "walk_warp");

        // 2. Return from Byalan Island: free sailor
        let return_route = route_edges("izlu2dun", "izlude").expect("return sailor route");
        assert_eq!(return_route.len(), 1);
        assert_eq!(return_route[0].id, "service-byalan-return-sailor");
        assert_eq!(return_route[0].kind, "npc_service");
        assert_eq!(return_route[0].availability, "always");
        assert_eq!(return_route[0].requirements.as_deref(), Some("No fare."));

        // 3. Cat Fleet to Malangdo
        let cat_route = route_edges("izlude", "malangdo").expect("cat fleet route");
        assert_eq!(cat_route.len(), 1);
        assert_eq!(cat_route[0].kind, "npc_service");
        assert_eq!(cat_route[0].availability, "conditional");
        assert!(cat_route[0].requirements.as_deref().unwrap().contains("1000 zeny"));

        // Verify graph maps exist
        assert!(graph.maps.iter().any(|m| m == "izlude"));
        assert!(graph.maps.iter().any(|m| m == "izlu2dun"));
        assert!(graph.maps.iter().any(|m| m == "malangdo"));
    }

    #[test]
    fn unreachable_routes_return_none_and_never_hallucinate() {
        assert!(route_edges("prontera", "nonexistent_map").is_none());
        assert!(route_edges("nonexistent_map", "prontera").is_none());

        let lines = destination_detail_lines(
            "prontera",
            Some("nonexistent_map"),
            None,
            4,
            Some("not yet visited"),
            &[],
            &[],
            None,
            &[],
        );
        assert!(lines[5].contains("No verified route"));
        assert!(lines[5].contains("No portal route from prontera to nonexistent_map"));
    }

    #[test]
    fn official_world_map_boxes_cover_every_atlas_town_except_jawaii() {
        let prontera = world_map_hotspot("prontera").expect("prontera box");
        assert_eq!((prontera.x1, prontera.y1, prontera.x2, prontera.y2), (812, 587, 870, 643));
        assert_eq!(prontera.page, WorldMapPage::Midgard);

        let izlude = world_map_hotspot("IZLUDE").expect("izlude box");
        assert_eq!((izlude.x1, izlude.y1, izlude.x2, izlude.y2), (871, 644, 902, 676));

        assert!(world_map_hotspot("jawaii").is_none());
        for location in LOCATIONS {
            let hotspot = world_map_hotspot(location.map);
            if location.map == "jawaii" {
                assert!(hotspot.is_none());
                continue;
            }
            let hotspot = hotspot.expect(location.map);
            assert!(hotspot.x2 > hotspot.x1 && hotspot.y2 > hotspot.y1, "{}", location.map);
            assert!(f32::from(hotspot.x2) <= WORLD_MAP_ART_WIDTH, "{}", location.map);
            assert!(f32::from(hotspot.y2) <= WORLD_MAP_ART_HEIGHT, "{}", location.map);
            assert_eq!(
                WORLD_MAP_HOTSPOTS.iter().filter(|entry| entry.map == location.map).count(),
                1,
                "{}",
                location.map
            );
        }
    }

    #[test]
    fn a_painted_hotspot_scales_with_the_fitted_sheet() {
        let viewport = Area {
            left: 10.0,
            top: 20.0,
            width: 640.0,
            height: 512.0,
        };
        let art = fit_world_map_art(viewport);
        assert!((art.width - 640.0).abs() < 0.01);
        assert!((art.height - 512.0).abs() < 0.01);
        assert!((art.width / art.height - WORLD_MAP_ART_WIDTH / WORLD_MAP_ART_HEIGHT).abs() < 0.001);

        let prontera = world_map_hotspot("prontera").expect("prontera");
        let hotspot = world_map_hotspot_area(art, prontera);
        let center_x = hotspot.left + hotspot.width * 0.5;
        let center_y = hotspot.top + hotspot.height * 0.5;
        // Official center is ((812+870)/2, (587+643)/2) = (841, 615), at half scale.
        assert!((center_x - (10.0 + 841.0 * 0.5)).abs() < 0.01);
        assert!((center_y - (20.0 + 615.0 * 0.5)).abs() < 0.01);

        let label = world_map_label_area(art, hotspot);
        assert!(label.top >= art.top);
        assert!(label.left >= art.left);
        assert!(label.left + label.width <= art.left + art.width + 0.01);
    }

    #[test]
    fn the_open_sheet_follows_the_current_map_until_the_player_picks_one() {
        assert_eq!(displayed_world_map_page(0, "amatsu"), WorldMapPage::Amatsu);
        assert_eq!(displayed_world_map_page(0, "jawaii"), WorldMapPage::Midgard);
        assert_eq!(displayed_world_map_page(0, "prontera"), WorldMapPage::Midgard);
        assert_eq!(
            displayed_world_map_page(WorldMapPage::Lutie.as_u8(), "prontera"),
            WorldMapPage::Lutie
        );
    }

    #[test]
    fn route_selection_safety_guarantee_no_teleportation() {
        let node_click = RouteClick {
            map: "prontera",
            position: TilePosition { x: 155, y: 183 },
        };
        let event = node_click.event();

        let facility_click = FacilityRouteClick {
            map_name: "prontera".to_owned(),
            position: TilePosition { x: 156, y: 191 },
        };
        let facility_event = facility_click.event();

        let mut queue = EventQueue::default();
        queue.queue(event);
        queue.queue(facility_event);

        let events: Vec<_> = queue.drain().collect();
        assert_eq!(events.len(), 2);

        // Destination click dispatches ONLY client-side SetNavigationDestination event
        // (never teleports)
        match &events[0] {
            Event::Application {
                custom_event: InputEvent::SetNavigationDestination { map_name, x, y },
            } => {
                assert_eq!(map_name, "prontera");
                assert_eq!(*x, 155);
                assert_eq!(*y, 183);
            }
            _ => panic!("expected SetNavigationDestination"),
        }

        // Facility click dispatches ONLY client-side SetNavigationDestination event
        match &events[1] {
            Event::Application {
                custom_event: InputEvent::SetNavigationDestination { map_name, x, y },
            } => {
                assert_eq!(map_name, "prontera");
                assert_eq!(*x, 156);
                assert_eq!(*y, 191);
            }
            _ => panic!("expected SetNavigationDestination"),
        }
    }

    #[test]
    fn atlas_roads_classification_and_region_coverage() {
        // Every atlas location must have a valid regional assignment matching
        // map_region
        for location in LOCATIONS {
            assert_eq!(
                location.region(),
                map_region(location.map).expect("atlas location must have a known region"),
                "location {} mismatch",
                location.map
            );
        }

        // Roads must contain both overland walk roads and in-world transport
        // connections
        let walk_roads = ROADS.iter().filter(|r| r.kind == RoadKind::Walk).count();
        let transport_roads = ROADS.iter().filter(|r| r.kind == RoadKind::Transport).count();
        assert!(walk_roads >= 10, "expected at least 10 overland walk roads, got {walk_roads}");
        assert!(
            transport_roads >= 10,
            "expected at least 10 in-world transport routes, got {transport_roads}"
        );

        // Every road that is marked available must have a verified route in the
        // navigation graph
        let available = available_roads();
        assert!(!available.is_empty(), "available roads must not be empty");
        for road in available {
            let from_map = LOCATIONS[road.from].map;
            let to_map = LOCATIONS[road.to].map;
            assert!(
                route_edges(from_map, to_map).is_some(),
                "road between {from_map} and {to_map} must have a verified route"
            );
        }
    }

    #[test]
    fn map_information_spawn_range_and_population_summary() {
        let reference = crate::dm::reference_data::reference_data();
        let details = reference
            .map_spawn_details("prt_fild08")
            .expect("prt_fild08 must have static spawn records");
        assert!(details.records > 0);
        assert!(details.species > 0);
        assert!(details.min_level <= details.max_level);

        let lines = destination_detail_lines(
            "prontera",
            Some("prt_fild08"),
            None,
            4,
            Some("visited"),
            &[],
            &[],
            Some(details),
            &[],
        );
        let expected_range = if details.min_level != details.max_level {
            format!("(range: {}–{})", details.min_level, details.max_level)
        } else {
            String::new()
        };
        assert!(lines[1].contains(&format!("Suggested level: ~{}", details.mean_level)));
        if !expected_range.is_empty() {
            assert!(lines[1].contains(&expected_range));
        }
        assert!(lines[1].contains("(static-spawn mean)"));
        assert!(lines[2].contains(&format!(
            "Static population: {} spawn records • {} species",
            details.records, details.species
        )));
    }

    #[test]
    fn map_information_low_coverage_labeling() {
        let lines = destination_detail_lines("prontera", Some("unknown_map"), None, 0, None, &[], &[], None, &[]);
        assert_eq!(lines[1], "Suggested level: unavailable (low-coverage or non-combat map)");
        assert_eq!(lines[2], "Static population: no verified spawn records (low coverage)");
    }

    #[test]
    fn map_information_danger_guidance_and_entry_warning() {
        assert!(is_dangerous_map_level(35, 20));
        assert!(!is_dangerous_map_level(34, 20));
        assert!(!is_dangerous_map_level(20, 20));

        let warning = crate::map_difficulty_warning("orcsdun02", Some(35), Some(20), true).expect("expected danger warning");
        assert!(warning.contains("orcsdun02 averages level 35"));
        assert!(warning.contains("15+ above your level (20)"));
        assert!(warning.contains("Warning only; travel is unrestricted."));

        assert!(
            crate::map_difficulty_warning("orcsdun02", Some(35), Some(20), false).is_none(),
            "opt-out toggle false must suppress danger warning"
        );
        assert!(
            crate::map_difficulty_warning("unknown_map", None, Some(20), true).is_none(),
            "missing spawn records must not guess or trigger warning"
        );
    }
}
