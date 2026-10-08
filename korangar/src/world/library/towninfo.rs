//! Official RO town facility markers (`System/Towninfo*.lub`).
//!
//! The raw table still decides whether a map is a town (`is_town`). Marks the
//! player sees go through [`verified_facility_pois`]: a Towninfo row is kept
//! only when a non-warp, non-story server NPC stands on that exact cell and
//! matches the role, and an exact-name dealer the table missed is added at the
//! server cell. The table shape is `mapNPCInfoTable[map] = { { name, X, Y, TYPE
//! }, ... }`.

use hashbrown::HashMap;
use korangar_loaders::FileLoader;

use crate::loaders::GameFileLoader;

/// Facility kind used by `Towninfo.lub` (`TYPE` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TownPoiKind {
    ToolDealer = 0,
    WeaponDealer = 1,
    ArmorDealer = 2,
    Smith = 3,
    Guide = 4,
    Inn = 5,
    Kafra = 6,
    /// Styling shop (and a few other TYPE=7 entries such as "Star").
    Style = 7,
    Other = 255,
}

impl TownPoiKind {
    pub fn from_type_id(type_id: u8) -> Self {
        match type_id {
            0 => Self::ToolDealer,
            1 => Self::WeaponDealer,
            2 => Self::ArmorDealer,
            3 => Self::Smith,
            4 => Self::Guide,
            5 => Self::Inn,
            6 => Self::Kafra,
            7 => Self::Style,
            _ => Self::Other,
        }
    }

    /// Path relative to `data\texture\` for the minimap / information icon.
    pub fn icon_texture_path(self) -> &'static str {
        match self {
            Self::ToolDealer => "유저인터페이스\\information\\store.bmp",
            Self::WeaponDealer => "유저인터페이스\\information\\weaponshop.bmp",
            Self::ArmorDealer => "유저인터페이스\\information\\armorshops.bmp",
            Self::Smith => "유저인터페이스\\information\\smithy.bmp",
            Self::Guide => "유저인터페이스\\information\\guide.bmp",
            Self::Inn => "유저인터페이스\\information\\inn.bmp",
            Self::Kafra => "유저인터페이스\\information\\kafra.bmp",
            Self::Style => "유저인터페이스\\information\\style.bmp",
            Self::Other => "유저인터페이스\\information\\store.bmp",
        }
    }

    /// Fallback solid color when the icon texture is missing.
    pub fn fallback_color_rgb(self) -> (u8, u8, u8) {
        match self {
            Self::ToolDealer => (80, 200, 80),
            Self::WeaponDealer => (220, 80, 80),
            Self::ArmorDealer => (80, 120, 220),
            Self::Smith => (180, 140, 60),
            Self::Guide => (240, 220, 60),
            Self::Inn => (180, 100, 220),
            Self::Kafra => (60, 180, 220),
            Self::Style => (240, 140, 200),
            Self::Other => (200, 200, 200),
        }
    }
}

/// A single facility marker from Towninfo.
#[derive(Debug, Clone)]
pub struct TownPoi {
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub kind: TownPoiKind,
}

/// All map → facility entries loaded from Towninfo.
#[derive(Debug, Default, Clone)]
pub struct TownInfoTable {
    by_map: HashMap<String, Vec<TownPoi>>,
}

impl TownInfoTable {
    pub fn load(game_file_loader: &GameFileLoader) -> Self {
        let Some(data) = load_towninfo_bytes(game_file_loader) else {
            #[cfg(feature = "debug")]
            {
                use korangar_debug::logging::{Colorize, print_debug};
                print_debug!(
                    "[{}] Towninfo not found (expected System/Towninfo_EN.lub); minimap facility POIs disabled",
                    "warning".yellow()
                );
            }
            return Self::default();
        };

        match parse_towninfo(&data) {
            Ok(table) => {
                let maps = table.by_map.len();
                let pois: usize = table.by_map.values().map(Vec::len).sum();
                client_log!("[towninfo] parsed {maps} maps, {pois} POIs");
                #[cfg(feature = "debug")]
                {
                    use korangar_debug::logging::print_debug;
                    print_debug!("loaded Towninfo: {maps} maps, {pois} POIs");
                }
                table
            }
            Err(error) => {
                client_log!("[towninfo] parse failed: {error:?}");
                #[cfg(feature = "debug")]
                {
                    use korangar_debug::logging::{Colorize, print_debug};
                    print_debug!("[{}] failed to parse Towninfo: {:?}", "warning".yellow(), error);
                }
                Self::default()
            }
        }
    }

    /// Every facility on every map, for whole-table checks.
    #[cfg(test)]
    pub(super) fn all_pois(&self) -> impl Iterator<Item = &TownPoi> {
        self.by_map.values().flatten()
    }

    pub fn pois_for_map(&self, map_name: &str) -> &[TownPoi] {
        self.by_map.get(&Self::key(map_name)).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Whether the map appears in the Towninfo facility data — a good proxy for
    /// "town / safe map" (towns and their shop/inn interiors), where the battle
    /// stance should relax to peaceful idle.
    pub fn is_town(&self, map_name: &str) -> bool {
        self.by_map.contains_key(&Self::key(map_name))
    }

    fn key(map_name: &str) -> String {
        map_name
            .trim_end_matches(".gat")
            .trim_end_matches(".GAT")
            .trim_end_matches(".rsw")
            .trim_end_matches(".RSW")
            .to_lowercase()
    }
}

/// One static server NPC, reduced to the fields facility verification uses.
#[derive(Debug, Clone, Copy)]
pub(super) struct DeclaredNpc<'a> {
    pub map: &'a str,
    /// Script name or the export's display name. [`shown_npc_name`] splits on
    /// `#`.
    pub visible_name: &'a str,
    pub source_path: &'a str,
    pub x: i32,
    pub y: i32,
    pub is_warp: bool,
    pub is_story: bool,
}

/// The part of an NPC name Hercules shows: `display_name` when it is set,
/// otherwise the script name, then the text before `#`.
pub(super) fn shown_npc_name<'a>(display_name: &'a str, script_name: &'a str) -> &'a str {
    let raw = if display_name.is_empty() { script_name } else { display_name };
    raw.split('#').next().unwrap_or(raw).trim()
}

/// Same result as [`verified_facility_pois`] for the loaded NPC export. The
/// world map asks for this on every layout, and both inputs are fixed for the
/// session, so each map is computed once.
pub(super) fn cached_verified_pois(map: &str, towninfo: &[TownPoi]) -> Vec<TownPoi> {
    use std::sync::{Mutex, OnceLock};

    static CACHE: OnceLock<Mutex<HashMap<String, Vec<TownPoi>>>> = OnceLock::new();
    let key = TownInfoTable::key(map);
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(pois) = guard.get(&key) {
        return pois.clone();
    }
    let pois = verified_facility_pois(map, towninfo, reference_facility_npcs());
    guard.insert(key, pois.clone());
    pois
}

/// Server NPC rows used to check facility cells. The export does not change
/// during a session, so the slice is built once.
pub(super) fn reference_facility_npcs() -> &'static [DeclaredNpc<'static>] {
    use std::sync::OnceLock;

    static NPCS: OnceLock<Vec<DeclaredNpc<'static>>> = OnceLock::new();
    NPCS.get_or_init(|| {
        crate::dm::reference_data::reference_data()
            .npcs
            .iter()
            .map(|npc| DeclaredNpc {
                map: npc.map.as_str(),
                visible_name: shown_npc_name(&npc.display_name, &npc.name),
                source_path: npc.source.path.as_str(),
                x: npc.x,
                y: npc.y,
                is_warp: npc.declared_type == "warp",
                is_story: npc.is_story(),
            })
            .collect()
    })
    .as_slice()
}

/// Towninfo rows that match a server NPC, followed by exact-name facilities
/// the table does not already mark. Surviving Towninfo order is preserved so
/// the world map's first three route buttons stay stable.
pub(super) fn verified_facility_pois(map: &str, towninfo: &[TownPoi], npcs: &[DeclaredNpc<'_>]) -> Vec<TownPoi> {
    let wanted = TownInfoTable::key(map);
    let on_map: Vec<&DeclaredNpc<'_>> = npcs
        .iter()
        .filter(|npc| !npc.is_warp && !npc.is_story && TownInfoTable::key(npc.map) == wanted)
        .collect();

    let mut kept = Vec::new();
    for poi in towninfo {
        if poi.x < 0 || poi.y < 0 {
            continue;
        }
        let matched = on_map
            .iter()
            .any(|npc| npc.x == i32::from(poi.x) && npc.y == i32::from(poi.y) && role_matches(poi.kind, npc));
        if matched {
            kept.push(poi.clone());
        }
    }

    let mut marked: Vec<(i16, i16)> = kept.iter().map(|poi| (poi.x, poi.y)).collect();
    let mut adds = Vec::new();
    for npc in &on_map {
        let Some((x, y)) = facility_cell(npc.x, npc.y) else {
            continue;
        };
        if marked.contains(&(x, y)) {
            continue;
        }
        let visible = shown_npc_name(npc.visible_name, npc.visible_name);
        let Some(kind) = exact_facility_kind(&visible.to_ascii_lowercase()) else {
            continue;
        };
        adds.push(TownPoi {
            name: visible.to_owned(),
            x,
            y,
            kind,
        });
    }
    adds.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then(left.x.cmp(&right.x))
            .then(left.y.cmp(&right.y))
    });
    for poi in adds {
        let cell = (poi.x, poi.y);
        if marked.contains(&cell) {
            continue;
        }
        marked.push(cell);
        kept.push(poi);
    }
    kept
}

fn facility_cell(x: i32, y: i32) -> Option<(i16, i16)> {
    if x < 0 || y < 0 {
        return None;
    }
    Some((i16::try_from(x).ok()?, i16::try_from(y).ok()?))
}

fn role_matches(kind: TownPoiKind, npc: &DeclaredNpc<'_>) -> bool {
    let name = shown_npc_name(npc.visible_name, npc.visible_name).to_ascii_lowercase();
    let path = npc.source_path.replace('\\', "/").to_ascii_lowercase();
    match kind {
        TownPoiKind::ToolDealer => name.contains("tool"),
        TownPoiKind::WeaponDealer => name.contains("weapon"),
        TownPoiKind::ArmorDealer => name.contains("armor") || name.contains("armour"),
        TownPoiKind::Smith => name.contains("smith") || path.contains("/refine"),
        TownPoiKind::Guide => name.contains("guide"),
        TownPoiKind::Inn => name.contains("inn"),
        TownPoiKind::Kafra => name.contains("kafra") || path.contains("/kafras/"),
        TownPoiKind::Style => name.contains("styl"),
        TownPoiKind::Other => false,
    }
}

/// Visible names that are themselves a facility role. Anything else stays off
/// the map, including quest NPCs and warps.
fn exact_facility_kind(visible_name: &str) -> Option<TownPoiKind> {
    Some(match visible_name {
        "kafra employee" => TownPoiKind::Kafra,
        "guide" => TownPoiKind::Guide,
        "tool dealer" => TownPoiKind::ToolDealer,
        "weapon dealer" => TownPoiKind::WeaponDealer,
        "armor dealer" => TownPoiKind::ArmorDealer,
        "smith" | "blacksmith" => TownPoiKind::Smith,
        "innkeeper" => TownPoiKind::Inn,
        "styling shop" => TownPoiKind::Style,
        _ => return None,
    })
}

fn load_towninfo_bytes(game_file_loader: &GameFileLoader) -> Option<Vec<u8>> {
    // Prefer English labels when available; fall back to default/KR table.
    const GAME_PATHS: &[&str] = &[
        "System\\Towninfo_EN.lub",
        "System\\Towninfo.lub",
        "system\\Towninfo_EN.lub",
        "system\\Towninfo.lub",
        "system\\towninfo_en.lub",
        "system\\towninfo.lub",
    ];
    for path in GAME_PATHS {
        if let Ok(data) = game_file_loader.get(path) {
            client_log!("[towninfo] loaded from game archive: {path}");
            return Some(data);
        }
    }

    // Plain files are usually *outside* GRFs (official client keeps them next to
    // the executable under System/). Search several roots — cwd is often the
    // nested `korangar/korangar/` crate dir when launching with cargo.
    const RELATIVE_NAMES: &[&str] = &[
        "System/Towninfo_EN.lub",
        "System/Towninfo.lub",
        "System/Towninfo_EN.lua",
        "System/Towninfo.lua",
        "client/System/Towninfo_EN.lub",
        "client/System/Towninfo.lub",
    ];

    let mut roots: Vec<std::path::PathBuf> = Vec::new();

    if let Ok(root) = std::env::var("KORANGAR_CLIENT_ROOT") {
        roots.push(std::path::PathBuf::from(root));
    }
    if let Ok(system_dir) = std::env::var("KORANGAR_SYSTEM_DIR") {
        roots.push(std::path::PathBuf::from(system_dir));
    }

    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.clone());
        // Walk a few parents (crate → repo → sibling RO client trees).
        let mut walk = cwd;
        for _ in 0..6 {
            if let Some(parent) = walk.parent() {
                walk = parent.to_path_buf();
                roots.push(walk.clone());
                roots.push(walk.join("RO/client"));
                roots.push(walk.join("client"));
            } else {
                break;
            }
        }
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        roots.push(dir.to_path_buf());
        if let Some(parent) = dir.parent() {
            roots.push(parent.to_path_buf());
        }
    }

    // Dedup while preserving order.
    let mut seen = std::collections::HashSet::new();
    roots.retain(|r| seen.insert(r.clone()));

    for root in &roots {
        for name in RELATIVE_NAMES {
            let path = root.join(name);
            if let Ok(data) = std::fs::read(&path) {
                client_log!("[towninfo] loaded from {}", path.display());
                return Some(data);
            }
        }
        // Also allow root *being* the System directory itself.
        for file in ["Towninfo_EN.lub", "Towninfo.lub", "Towninfo_EN.lua", "Towninfo.lua"] {
            let path = root.join(file);
            if let Ok(data) = std::fs::read(&path) {
                client_log!("[towninfo] loaded from {}", path.display());
                return Some(data);
            }
        }
    }

    client_log!(
        "[towninfo] not found — minimap facility POIs disabled (looked in GRF System\\ and on disk under cwd/parents/KORANGAR_CLIENT_ROOT)"
    );
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_towninfo_table() {
        let src = r#"
mapNPCInfoTable = {
  prontera = {
    { name = [=[Kafra Employee]=], X = 146, Y = 89, TYPE = 6 },
    { name = [=[Tool Dealer]=], X = 134, Y = 221, TYPE = 0 },
  },
  izlude = {
    { name = [=[Guide]=], X = 129, Y = 175, TYPE = 4 },
  },
}
"#;
        let table = parse_towninfo(src.as_bytes()).expect("parse");
        assert_eq!(table.pois_for_map("prontera").len(), 2);
        assert_eq!(table.pois_for_map("PRONTERA").len(), 2);
        assert_eq!(table.pois_for_map("izlude.gat").len(), 1);
        assert_eq!(table.pois_for_map("geffen").len(), 0);
        let kafra = &table.pois_for_map("prontera")[0];
        assert_eq!(kafra.kind, TownPoiKind::Kafra);
        assert_eq!(kafra.x, 146);
        assert_eq!(kafra.y, 89);
    }

    fn poi(name: &str, x: i16, y: i16, kind: TownPoiKind) -> TownPoi {
        TownPoi {
            name: name.to_owned(),
            x,
            y,
            kind,
        }
    }

    fn declared<'a>(
        map: &'a str,
        visible_name: &'a str,
        source_path: &'a str,
        x: i32,
        y: i32,
        is_warp: bool,
        is_story: bool,
    ) -> DeclaredNpc<'a> {
        DeclaredNpc {
            map,
            visible_name,
            source_path,
            x,
            y,
            is_warp,
            is_story,
        }
    }

    #[test]
    fn an_empty_towninfo_cell_is_dropped() {
        let pois = [poi("Kafra Employee", 128, 148, TownPoiKind::Kafra)];
        assert!(verified_facility_pois("izlude", &pois, &[]).is_empty());
    }

    #[test]
    fn a_kafra_on_the_same_cell_is_kept() {
        let pois = [poi("Kafra Employee", 146, 89, TownPoiKind::Kafra)];
        let npcs = [declared(
            "prontera",
            "Kafra Employee",
            "npc/kafras/kafras.txt",
            146,
            89,
            false,
            false,
        )];
        let out = verified_facility_pois("prontera", &pois, &npcs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Kafra Employee");
        assert_eq!((out[0].x, out[0].y), (146, 89));
        assert_eq!(out[0].kind, TownPoiKind::Kafra);
    }

    #[test]
    fn a_guide_mark_on_a_different_npc_is_dropped() {
        let pois = [poi("Guide", 150, 326, TownPoiKind::Guide)];
        let npcs = [declared(
            "prontera",
            "Wanted Notice",
            "npc/quests/quests_nameless.txt",
            150,
            326,
            false,
            false,
        )];
        assert!(verified_facility_pois("prontera", &pois, &npcs).is_empty());
    }

    #[test]
    fn a_kafra_script_keeps_the_mark_when_the_name_does_not_say_kafra() {
        let pois = [poi("Kafra Employee", 88, 168, TownPoiKind::Kafra)];
        let npcs = [declared(
            "hugel",
            "Cool Event Corp. Staff",
            "npc\\kafras\\cool_event_corp.txt",
            88,
            168,
            false,
            false,
        )];
        let out = verified_facility_pois("hugel", &pois, &npcs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Kafra Employee");
        assert_eq!(out[0].kind, TownPoiKind::Kafra);
    }

    #[test]
    fn a_refine_script_keeps_a_smith_mark() {
        let pois = [poi("Smith", 144, 173, TownPoiKind::Smith)];
        let npcs = [declared("payon", "Antonio", "npc/merchants/refine.txt", 144, 173, false, false)];
        let out = verified_facility_pois("payon", &pois, &npcs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Smith");
        assert_eq!(out[0].kind, TownPoiKind::Smith);
    }

    #[test]
    fn a_warp_on_the_cell_does_not_count() {
        let pois = [poi("Kafra Employee", 10, 10, TownPoiKind::Kafra)];
        let npcs = [declared("prontera", "Kafra Employee", "npc/kafras/kafras.txt", 10, 10, true, false)];
        assert!(verified_facility_pois("prontera", &pois, &npcs).is_empty());
    }

    #[test]
    fn a_story_npc_is_not_a_facility() {
        let pois = [poi("Guide", 1, 2, TownPoiKind::Guide)];
        let npcs = [declared("prontera", "Guide", "npc/custom/dm_campaign/hub.txt", 1, 2, false, true)];
        assert!(verified_facility_pois("prontera", &pois, &npcs).is_empty());
        assert!(verified_facility_pois("prontera", &[], &npcs).is_empty());
    }

    #[test]
    fn a_server_tool_dealer_missing_from_towninfo_is_added() {
        let npcs = [declared(
            "izlude_in",
            "Tool Dealer#iz",
            "npc/re/merchants/shops.txt",
            57,
            110,
            false,
            false,
        )];
        let out = verified_facility_pois("izlude_in", &[], &npcs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Tool Dealer");
        assert_eq!((out[0].x, out[0].y), (57, 110));
        assert_eq!(out[0].kind, TownPoiKind::ToolDealer);
    }

    #[test]
    fn an_off_by_one_towninfo_cell_uses_the_server_cell() {
        let pois = [poi("Kafra Employee", 82, 362, TownPoiKind::Kafra)];
        let npcs = [declared(
            "mjolnir_02",
            "Kafra Employee",
            "npc/kafras/kafras.txt",
            83,
            362,
            false,
            false,
        )];
        let out = verified_facility_pois("mjolnir_02", &pois, &npcs);
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].x, out[0].y), (83, 362));
        assert_eq!(out[0].kind, TownPoiKind::Kafra);
    }

    #[test]
    fn a_mismatched_role_is_replaced_by_the_servers_exact_role() {
        let pois = [poi("Weapon Dealer", 57, 110, TownPoiKind::WeaponDealer)];
        let npcs = [declared(
            "izlude_in",
            "Tool Dealer",
            "npc/re/merchants/shops.txt",
            57,
            110,
            false,
            false,
        )];
        let out = verified_facility_pois("izlude_in", &pois, &npcs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Tool Dealer");
        assert_eq!(out[0].kind, TownPoiKind::ToolDealer);
    }

    #[test]
    fn kept_towninfo_order_comes_before_added_dealers() {
        let pois = [
            poi("Kafra Employee", 1, 1, TownPoiKind::Kafra),
            poi("Guide", 9, 9, TownPoiKind::Guide),
        ];
        let npcs = [
            declared("prontera", "Tool Dealer", "npc/re/merchants/shops.txt", 5, 5, false, false),
            declared("prontera", "Kafra Employee", "npc/kafras/kafras.txt", 1, 1, false, false),
        ];
        let out = verified_facility_pois("prontera", &pois, &npcs);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].kind, TownPoiKind::Kafra);
        assert_eq!(out[1].name, "Tool Dealer");
    }

    #[test]
    fn added_facilities_sort_by_name_then_cell() {
        let npcs = [
            declared(
                "izlude_in",
                "Weapon Dealer",
                "npc/re/merchants/shops.txt",
                60,
                127,
                false,
                false,
            ),
            declared("izlude_in", "Tool Dealer", "npc/re/merchants/shops.txt", 57, 110, false, false),
            declared("izlude_in", "Armor Dealer", "npc/re/merchants/shops.txt", 70, 127, false, false),
        ];
        let out = verified_facility_pois("izlude_in", &[], &npcs);
        let names: Vec<_> = out.iter().map(|poi| poi.name.as_str()).collect();
        assert_eq!(names, ["Armor Dealer", "Tool Dealer", "Weapon Dealer"]);
    }

    #[test]
    fn a_dealer_already_marked_is_not_added_again() {
        let pois = [poi("Tool Dealer", 57, 110, TownPoiKind::ToolDealer)];
        let npcs = [declared(
            "izlude_in",
            "Tool Dealer",
            "npc/re/merchants/shops.txt",
            57,
            110,
            false,
            false,
        )];
        assert_eq!(verified_facility_pois("izlude_in", &pois, &npcs).len(), 1);
    }

    #[test]
    fn negative_coordinates_are_dropped() {
        let pois = [poi("Kafra Employee", -1, 10, TownPoiKind::Kafra)];
        let npcs = [declared(
            "prontera",
            "Kafra Employee",
            "npc/kafras/kafras.txt",
            -1,
            10,
            false,
            false,
        )];
        assert!(verified_facility_pois("prontera", &pois, &npcs).is_empty());
    }

    #[test]
    fn an_unknown_towninfo_type_is_dropped() {
        let pois = [poi("Star", 3, 4, TownPoiKind::Other)];
        let npcs = [declared("prontera", "Star", "npc/cities/prontera.txt", 3, 4, false, false)];
        assert!(verified_facility_pois("prontera", &pois, &npcs).is_empty());
    }

    #[test]
    fn map_names_ignore_case_and_the_gat_suffix() {
        let pois = [poi("Guide", 4, 5, TownPoiKind::Guide)];
        let npcs = [declared("Prontera.gat", "Guide", "npc/cities/guides.txt", 4, 5, false, false)];
        let out = verified_facility_pois("PRONTERA", &pois, &npcs);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, TownPoiKind::Guide);
    }

    #[test]
    fn server_cells_replace_empty_towninfo_marks() {
        let source = include_str!("../../../archive/System/Towninfo_EN.lub");
        let table = parse_towninfo(source.as_bytes()).expect("towninfo");
        let npcs = reference_facility_npcs();
        let mut maps = std::collections::BTreeSet::new();
        maps.extend(table.by_map.keys().cloned());
        for npc in npcs {
            maps.insert(TownInfoTable::key(npc.map));
        }

        let prontera = verified_facility_pois("prontera", table.pois_for_map("prontera"), npcs);
        assert!(
            prontera
                .iter()
                .any(|poi| poi.kind == TownPoiKind::Kafra && poi.x == 146 && poi.y == 89)
        );
        assert!(prontera.iter().all(|poi| !(poi.x == 150 && poi.y == 326)));
        assert_eq!(prontera.len(), 6);

        let izlude = verified_facility_pois("izlude", table.pois_for_map("izlude"), npcs);
        assert!(izlude.is_empty());

        let izlude_in = verified_facility_pois("izlude_in", table.pois_for_map("izlude_in"), npcs);
        assert!(
            izlude_in
                .iter()
                .any(|poi| { poi.kind == TownPoiKind::ToolDealer && poi.x == 57 && poi.y == 110 && poi.name == "Tool Dealer" })
        );
        assert_eq!(izlude_in.iter().map(|poi| poi.name.as_str()).collect::<Vec<_>>(), [
            "Armor Dealer",
            "Tool Dealer",
            "Weapon Dealer"
        ]);

        let mjolnir = verified_facility_pois("mjolnir_02", table.pois_for_map("mjolnir_02"), npcs);
        assert_eq!(mjolnir.len(), 1);
        assert_eq!((mjolnir[0].x, mjolnir[0].y), (83, 362));

        let mora = verified_facility_pois("mora", table.pois_for_map("mora"), npcs);
        assert!(
            mora.iter()
                .all(|poi| !(poi.kind == TownPoiKind::Inn && poi.x == 44 && poi.y == 127))
        );
        assert!(mora.iter().any(|poi| poi.name == "Innkeeper" && poi.x == 43 && poi.y == 127));

        let payon = verified_facility_pois("payon", table.pois_for_map("payon"), npcs);
        assert!(
            payon
                .iter()
                .any(|poi| poi.kind == TownPoiKind::Smith && poi.name == "Smith" && poi.x == 144 && poi.y == 173)
        );

        let hugel = verified_facility_pois("hugel", table.pois_for_map("hugel"), npcs);
        assert!(
            hugel
                .iter()
                .any(|poi| poi.kind == TownPoiKind::Kafra && poi.x == 88 && poi.y == 168)
        );

        let mut total = 0;
        for map in &maps {
            let pois = verified_facility_pois(map, table.pois_for_map(map), npcs);
            for poi in &pois {
                assert!(poi.x >= 0 && poi.y >= 0, "{map} {poi:?}");
                let occupied = npcs.iter().any(|npc| {
                    !npc.is_warp
                        && !npc.is_story
                        && TownInfoTable::key(npc.map) == *map
                        && npc.x == i32::from(poi.x)
                        && npc.y == i32::from(poi.y)
                        && (role_matches(poi.kind, npc)
                            || exact_facility_kind(&shown_npc_name(npc.visible_name, npc.visible_name).to_ascii_lowercase())
                                == Some(poi.kind))
                });
                assert!(
                    occupied,
                    "{map} {} at ({}, {}) has no matching server NPC",
                    poi.name, poi.x, poi.y
                );
            }
            total += pois.len();
        }
        // Mora's Towninfo lists Guide 167,76 twice. Both rows match a server
        // guide, so both stay. 93 kept rows plus 108 added dealers.
        assert_eq!(total, 201);
    }
}

fn parse_towninfo(data: &[u8]) -> mlua::Result<TownInfoTable> {
    let state = super::new_sandboxed_lua()?;
    // Provide a no-op `AddTownInfo` so calling `main()` (if present) does not fail.
    state.globals().set(
        "AddTownInfo",
        state.create_function(|_, _: (String, String, i32, i32, i32)| Ok((true, "good")))?,
    )?;
    state.load(data).exec()?;

    let globals = state.globals();
    let map_table: mlua::Table = globals.get("mapNPCInfoTable")?;
    let mut by_map: HashMap<String, Vec<TownPoi>> = HashMap::new();

    for pair in map_table.pairs::<String, mlua::Table>() {
        let (map_name, entries) = match pair {
            Ok(v) => v,
            Err(_) => continue,
        };
        let mut pois = Vec::new();
        for entry in entries.sequence_values::<mlua::Table>() {
            let Ok(entry) = entry else { continue };
            let name: String = entry.get("name").unwrap_or_default();
            let x: i32 = entry.get("X").or_else(|_| entry.get("x")).unwrap_or(0);
            let y: i32 = entry.get("Y").or_else(|_| entry.get("y")).unwrap_or(0);
            let type_id: u8 = entry
                .get::<u8>("TYPE")
                .or_else(|_| entry.get::<u8>("Type"))
                .or_else(|_| entry.get::<u8>("type"))
                .unwrap_or(255);
            pois.push(TownPoi {
                name,
                x: x.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                y: y.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                kind: TownPoiKind::from_type_id(type_id),
            });
        }
        if !pois.is_empty() {
            by_map.insert(map_name.to_lowercase(), pois);
        }
    }

    Ok(TownInfoTable { by_map })
}
