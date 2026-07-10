//! Item display names and icon resource names.
//!
//! All UI that shows item names (inventory, equipment, NPC shops buy/sell,
//! pickup chat, etc.) goes through [`ItemName`] → this table. We prefer
//! English `System/itemInfo_EN.lua` and overlay it on the GRF Korean table so
//! every shop/NPC path gets English labels while keeping icon paths working.

use std::path::PathBuf;

use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::Lua;
use ragnarok_packets::ItemId;

use super::{HashMapExt, ItemName, ItemResource, Library, Table, fix_encoding};
use crate::loaders::GameFileLoader;

#[derive(Debug, Clone)]
pub struct ItemInfo {
    pub(super) identified_name: ItemName,
    pub(super) unidentified_name: ItemName,
    pub(super) identified_resource: ItemResource,
    pub(super) unidentified_resource: ItemResource,
}

impl Table for ItemInfo {
    type Key<'a> = ItemId;
    type Storage = HashMap<ItemId, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        // 1) Full base table (often Korean GRF) for complete id coverage + icon paths.
        // 2) Overlay English display names from System/itemInfo_EN.lua (and similar).
        // Every shop/inventory/sell path uses this single table via ItemName.
        let mut map = HashMap::new();
        let mut sources_used = Vec::new();

        for (label, data, role) in iteminfo_candidates(game_file_loader) {
            match parse_iteminfo_table(&data) {
                Ok(parsed) if !parsed.is_empty() => {
                    match role {
                        SourceRole::Base => {
                            if map.is_empty() {
                                map = parsed;
                                sources_used.push(format!("{label} (base, {} items)", map.len()));
                            }
                        }
                        SourceRole::EnglishOverlay => {
                            let before = map.len();
                            overlay_english_names(&mut map, parsed);
                            sources_used.push(format!(
                                "{label} (EN overlay, table was {before}, now {} items)",
                                map.len()
                            ));
                        }
                    }
                }
                Ok(_) => {
                    #[cfg(feature = "debug")]
                    {
                        use korangar_debug::logging::{Colorize, print_debug};
                        print_debug!("[{}] itemInfo from {} produced 0 items", "warning".yellow(), label);
                    }
                }
                Err(_error) => {
                    #[cfg(feature = "debug")]
                    {
                        use korangar_debug::logging::{Colorize, print_debug};
                        print_debug!(
                            "[{}] failed to parse itemInfo from {}: {:?}",
                            "warning".yellow(),
                            label,
                            _error
                        );
                    }
                }
            }
        }

        // If we only got English (no Korean base), that's fine — EN file is large.
        // If we only got Korean, names may be Hangul (font tofu without KR glyphs).
        #[cfg(feature = "debug")]
        {
            use korangar_debug::logging::print_debug;
            if sources_used.is_empty() {
                print_debug!("[warning] no usable itemInfo source found");
            } else {
                for s in &sources_used {
                    print_debug!("itemInfo: {s}");
                }
            }
        }

        // Always print a short one-liner so non-debug builds still confirm source.
        if !sources_used.is_empty() {
            eprintln!("[itemInfo] {}", sources_used.join(" + "));
        } else {
            eprintln!("[itemInfo] WARNING: no itemInfo loaded — names will be NOTFOUND");
        }

        Ok(map.compact())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        library.item_info_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: ItemInfo = ItemInfo {
            identified_name: ItemName::not_found_value(),
            unidentified_name: ItemName::not_found_value(),
            identified_resource: ItemResource::not_found_value(),
            unidentified_resource: ItemResource::not_found_value(),
        };
        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}

#[derive(Clone, Copy)]
enum SourceRole {
    /// Full table (names + resources).
    Base,
    /// Prefer these display names; keep existing resources when present.
    EnglishOverlay,
}

/// Copy English display names onto the base table; insert missing rows fully.
fn overlay_english_names(base: &mut HashMap<ItemId, ItemInfo>, english: HashMap<ItemId, ItemInfo>) {
    for (id, en) in english {
        match base.get_mut(&id) {
            Some(row) => {
                if looks_like_english_name(&en.identified_name.to_string()) {
                    row.identified_name = en.identified_name;
                }
                if looks_like_english_name(&en.unidentified_name.to_string()) {
                    row.unidentified_name = en.unidentified_name;
                }
                // Prefer EN resource only if base is missing / NOTFOUND.
                if row.identified_resource.to_string() == "NOTFOUND" {
                    row.identified_resource = en.identified_resource;
                }
                if row.unidentified_resource.to_string() == "NOTFOUND" {
                    row.unidentified_resource = en.unidentified_resource;
                }
            }
            None => {
                base.insert(id, en);
            }
        }
    }
}

fn looks_like_english_name(name: &str) -> bool {
    if name.is_empty() || name == "NOTFOUND" {
        return false;
    }
    // Prefer pure ASCII display names (ROenglishRE style).
    if name.is_ascii() {
        return name.chars().any(|c| c.is_ascii_alphabetic());
    }
    // Also accept Latin-1 style English with accents, reject pure Hangul blocks.
    let hangul = name.chars().filter(|c| ('\u{AC00}'..='\u{D7A3}').contains(c)).count();
    let total = name.chars().filter(|c| c.is_alphabetic()).count().max(1);
    hangul * 2 < total
}

fn parse_iteminfo_table(data: &[u8]) -> mlua::Result<HashMap<ItemId, ItemInfo>> {
    let state = Lua::new();
    let data = data.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(data);
    state.load(data).exec()?;

    let globals = state.globals();
    let mut result = HashMap::new();

    let table = match globals.get::<mlua::Table>("tbl") {
        Ok(t) => t,
        Err(_) => globals.get::<mlua::Table>("ItemInfo")?,
    };

    for (item_id, item_table) in table.pairs::<u32, mlua::Table>().flatten() {
        let info = ItemInfo {
            identified_name: ItemName::from_option(item_table.get("identifiedDisplayName").ok().map(decode_item_string)),
            unidentified_name: ItemName::from_option(item_table.get("unidentifiedDisplayName").ok().map(decode_item_string)),
            identified_resource: ItemResource::from_option(item_table.get("identifiedResourceName").ok().map(decode_item_string)),
            unidentified_resource: ItemResource::from_option(item_table.get("unidentifiedResourceName").ok().map(decode_item_string)),
        };

        result.insert(ItemId(item_id), info);
    }

    Ok(result.compact())
}

fn decode_item_string(value: String) -> String {
    if value.is_ascii() {
        return value;
    }
    if value.chars().any(|c| c as u32 > 0xFF) {
        return value;
    }
    fix_encoding(value)
}

/// Search several roots so we find System/ whether cwd is repo root or `korangar/`.
fn read_from_search_paths(relative: &str) -> Option<(String, Vec<u8>)> {
    let mut roots: Vec<PathBuf> = vec![
        PathBuf::from("."),
        PathBuf::from("korangar"),
        PathBuf::from(".."),
    ];
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.clone());
        roots.push(cwd.join("korangar"));
        if let Some(parent) = cwd.parent() {
            roots.push(parent.to_path_buf());
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            roots.push(dir.to_path_buf());
            roots.push(dir.join("korangar"));
            if let Some(parent) = dir.parent() {
                roots.push(parent.to_path_buf());
                roots.push(parent.join("korangar"));
            }
        }
    }

    for root in roots {
        let path = root.join(relative);
        if let Ok(data) = std::fs::read(&path) {
            return Some((path.display().to_string(), data));
        }
        // Also try with backslashes normalized
        let path = root.join(relative.replace('\\', "/"));
        if let Ok(data) = std::fs::read(&path) {
            return Some((path.display().to_string(), data));
        }
    }
    None
}

fn iteminfo_candidates(game_file_loader: &GameFileLoader) -> Vec<(String, Vec<u8>, SourceRole)> {
    let mut out = Vec::new();

    // --- English sources (overlay role) ---
    const EN_FS: &[&str] = &[
        "System/itemInfo_EN.lua",
        "System/itemInfo_EN.lub",
        "System/itemInfo.lua",
        "archive/System/itemInfo_EN.lua",
        "korangar/System/itemInfo_EN.lua",
        "korangar/archive/System/itemInfo_EN.lua",
    ];
    for rel in EN_FS {
        if let Some((label, data)) = read_from_search_paths(rel) {
            out.push((label, data, SourceRole::EnglishOverlay));
            break; // one EN file is enough
        }
    }

    const EN_GAME: &[&str] = &[
        "System\\itemInfo_EN.lua",
        "system\\itemInfo_EN.lua",
        "system\\iteminfo_en.lua",
        "System\\itemInfo.lua",
        "data\\luafiles514\\lua files\\datainfo\\iteminfo_en.lub",
    ];
    for path in EN_GAME {
        if let Ok(data) = game_file_loader.get(path) {
            out.push(((*path).to_owned(), data, SourceRole::EnglishOverlay));
            break;
        }
    }

    // --- Base sources (full tables; Korean GRF is the usual complete set) ---
    const BASE_GAME: &[&str] = &[
        "data\\luafiles514\\lua files\\datainfo\\iteminfo.lub",
        "System\\itemInfo_true.lub",
        "System\\itemInfo.lub",
    ];
    for path in BASE_GAME {
        if let Ok(data) = game_file_loader.get(path) {
            out.push(((*path).to_owned(), data, SourceRole::Base));
            break;
        }
    }

    // Ensure base is processed before overlay when both present: reorder.
    out.sort_by_key(|(_, _, role)| match role {
        SourceRole::Base => 0u8,
        SourceRole::EnglishOverlay => 1u8,
    });

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_english_tbl() {
        let src = r#"
tbl = {
  [501] = {
    unidentifiedDisplayName = "Red Potion",
    identifiedDisplayName = "Red Potion",
    unidentifiedResourceName = "red",
    identifiedResourceName = "red",
  },
}
"#;
        let map = parse_iteminfo_table(src.as_bytes()).expect("parse");
        assert_eq!(map.get(&ItemId(501)).unwrap().identified_name.to_string(), "Red Potion");
    }

    #[test]
    fn overlay_replaces_non_english_with_english() {
        let mut base = parse_iteminfo_table(
            br#"
tbl = { [501] = {
  identifiedDisplayName = "HangulName",
  unidentifiedDisplayName = "HangulName",
  identifiedResourceName = "red",
  unidentifiedResourceName = "red",
}}
"#,
        )
        .unwrap();
        // Force a non-English-looking name by using only Hangul-range via UTF-8 string (not byte string).
        if let Some(row) = base.get_mut(&ItemId(501)) {
            row.identified_name = ItemName::from_option(Some("\u{be68}\u{ac04}\u{d3ec}\u{c158}".to_owned()));
            row.unidentified_name = ItemName::from_option(Some("\u{be68}\u{ac04}\u{d3ec}\u{c158}".to_owned()));
        }
        let en = parse_iteminfo_table(
            br#"
tbl = { [501] = {
  identifiedDisplayName = "Red Potion",
  unidentifiedDisplayName = "Red Potion",
  identifiedResourceName = "red_en",
  unidentifiedResourceName = "red_en",
}}
"#,
        )
        .unwrap();
        overlay_english_names(&mut base, en);
        assert_eq!(base.get(&ItemId(501)).unwrap().identified_name.to_string(), "Red Potion");
        // Resource kept from base when present
        assert_eq!(base.get(&ItemId(501)).unwrap().identified_resource.to_string(), "red");
    }

    #[test]
    fn parse_system_iteminfo_en_if_present() {
        let data = read_from_search_paths("System/itemInfo_EN.lua")
            .or_else(|| read_from_search_paths("korangar/System/itemInfo_EN.lua"));
        let Some((_, data)) = data else {
            eprintln!("skip: itemInfo_EN.lua not found");
            return;
        };
        let map = parse_iteminfo_table(&data).expect("EN itemInfo should parse");
        assert!(map.len() > 1000);
        assert_eq!(
            map.get(&ItemId(501)).expect("501").identified_name.to_string(),
            "Red Potion"
        );
    }
}
