//! Item display names and icon resource names.
//!
//! All UI that shows item names (inventory, equipment, NPC shops buy/sell,
//! pickup chat, etc.) goes through [`ItemName`] → this table. We prefer
//! English names generated from the Hercules item DB are overlaid on the GRF
//! table so every shop/NPC path gets readable labels while icon paths continue
//! to come from the installed game resources.

use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use ragnarok_packets::ItemId;

use super::{HashMapExt, ItemName, ItemResource, Library, Table, fix_encoding};
use crate::loaders::GameFileLoader;

const HERCULES_ITEM_NAMES: &str = include_str!("hercules_item_names.tsv");

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
        // 2) Overlay bundled English display names generated from Hercules data.
        // Every shop/inventory/sell path uses this single table via ItemName.
        let mut map = HashMap::new();
        let mut sources_used = Vec::new();

        for (label, data) in iteminfo_candidates(game_file_loader) {
            match parse_iteminfo_table(&data) {
                Ok(parsed) if !parsed.is_empty() => {
                    if map.is_empty() {
                        map = parsed;
                        sources_used.push(format!("{label} (base, {} items)", map.len()));
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
                        print_debug!("[{}] failed to parse itemInfo from {}: {:?}", "warning".yellow(), label, _error);
                    }
                }
            }
        }

        let bundled_count = overlay_bundled_english_names(&mut map);
        sources_used.push(format!("bundled Hercules names (EN overlay, {bundled_count} items)"));

        // If we only got English (no Korean base), that's fine — EN file is large.
        // If we only got Korean, names may be Hangul (font tofu without KR glyphs).
        #[cfg(feature = "debug")]
        {
            use korangar_debug::logging::print_debug;
            if sources_used.is_empty() {
                print_debug!("[warning] no usable itemInfo source found");
            } else {
                for source in &sources_used {
                    print_debug!("itemInfo: {}", source);
                }
            }
        }

        // Always print a short one-liner so non-debug builds still confirm source.
        if !sources_used.is_empty() {
            client_log!("[itemInfo] {}", sources_used.join(" + "));
        } else {
            client_log!("[itemInfo] WARNING: no itemInfo loaded — names will be NOTFOUND");
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

/// Copy English display names onto the base table; insert missing rows fully.
#[cfg(test)]
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

fn overlay_bundled_english_names(base: &mut HashMap<ItemId, ItemInfo>) -> usize {
    let mut count = 0;

    for line in HERCULES_ITEM_NAMES.lines() {
        let Some((id, name)) = line.split_once('\t') else {
            continue;
        };
        let Ok(id) = id.parse::<u32>() else {
            continue;
        };
        let item_id = ItemId(id);

        if let Some(item) = base.get_mut(&item_id) {
            item.identified_name = ItemName::from_option(Some(name.to_owned()));
            item.unidentified_name = ItemName::from_option(Some(name.to_owned()));
        } else {
            base.insert(item_id, ItemInfo {
                identified_name: ItemName::from_option(Some(name.to_owned())),
                unidentified_name: ItemName::from_option(Some(name.to_owned())),
                identified_resource: ItemResource::not_found_value(),
                unidentified_resource: ItemResource::not_found_value(),
            });
        }
        count += 1;
    }

    count
}

#[cfg(test)]
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
    let state = super::new_sandboxed_lua()?;
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

fn iteminfo_candidates(game_file_loader: &GameFileLoader) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();

    // --- Base sources (full tables; Korean GRF is the usual complete set) ---
    const BASE_GAME: &[&str] = &[
        "data\\luafiles514\\lua files\\datainfo\\iteminfo.lub",
        "System\\itemInfo_true.lub",
        "System\\itemInfo.lub",
    ];
    for path in BASE_GAME {
        if let Ok(data) = game_file_loader.get(path) {
            out.push(((*path).to_owned(), data));
            break;
        }
    }

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
        // Force a non-English-looking name by using only Hangul-range via UTF-8 string
        // (not byte string).
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
    fn bundled_hercules_names_are_complete_and_human_readable() {
        let mut map = HashMap::new();
        let count = overlay_bundled_english_names(&mut map);
        assert_eq!(count, 13_182);
        assert_eq!(map.get(&ItemId(501)).expect("501").identified_name.to_string(), "Red Potion");
    }
}
