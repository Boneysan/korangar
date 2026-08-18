//! Official RO town facility markers (`System/Towninfo*.lub`).
//!
//! Populates minimap icons for tool shops, kafra, guides, etc. The table shape
//! is `mapNPCInfoTable[map] = { { name, X, Y, TYPE }, ... }`.

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
                eprintln!("[towninfo] parsed {maps} maps, {pois} POIs");
                #[cfg(feature = "debug")]
                {
                    use korangar_debug::logging::print_debug;
                    print_debug!("loaded Towninfo: {maps} maps, {pois} POIs");
                }
                table
            }
            Err(error) => {
                eprintln!("[towninfo] parse failed: {error:?}");
                #[cfg(feature = "debug")]
                {
                    use korangar_debug::logging::{Colorize, print_debug};
                    print_debug!("[{}] failed to parse Towninfo: {:?}", "warning".yellow(), error);
                }
                Self::default()
            }
        }
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
            eprintln!("[towninfo] loaded from game archive: {path}");
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
                eprintln!("[towninfo] loaded from {}", path.display());
                return Some(data);
            }
        }
        // Also allow root *being* the System directory itself.
        for file in ["Towninfo_EN.lub", "Towninfo.lub", "Towninfo_EN.lua", "Towninfo.lua"] {
            let path = root.join(file);
            if let Ok(data) = std::fs::read(&path) {
                eprintln!("[towninfo] loaded from {}", path.display());
                return Some(data);
            }
        }
    }

    eprintln!(
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
