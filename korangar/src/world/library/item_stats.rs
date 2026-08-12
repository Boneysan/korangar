//! Shared item combat/utility stats for tooltips (M1-009).
//!
//! Sourced from Hercules-backed `docs/items.json` — same export the DM module
//! uses — but kept under `world/library` so inventory UI can use it without
//! coupling to `src/dm/` (rebase isolation).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

const ITEMS_JSON: &str = include_str!("../../../../docs/items.json");

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ItemStatsRow {
    id: u32,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "Type")]
    item_type: String,
    #[serde(default)]
    weight: u32,
    #[serde(default)]
    atk: Option<i32>,
    #[serde(default)]
    matk: Option<i32>,
    #[serde(default)]
    def: Option<i32>,
    #[serde(default)]
    slots: Option<u8>,
    #[serde(default)]
    equip_lv: Option<serde_json::Value>,
    #[serde(default)]
    loc: Option<serde_json::Value>,
}

/// Combat / equip stats for tooltip display.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemStats {
    pub item_id: u32,
    pub name: String,
    pub item_type: String,
    pub weight: u32,
    pub atk: Option<i32>,
    pub matk: Option<i32>,
    pub def: Option<i32>,
    pub slots: Option<u8>,
    pub equip_lv: Option<u16>,
    /// Raw equip location string from the export (e.g. `EQP_SHIELD`).
    pub loc: Option<String>,
}

impl ItemStats {
    /// True if this row has any combat/equip fields worth showing.
    pub fn has_combat_stats(&self) -> bool {
        self.atk.is_some() || self.matk.is_some() || self.def.is_some() || self.slots.is_some() || self.equip_lv.is_some()
    }

    fn format_lines(&self, refinement: Option<u8>) -> Vec<String> {
        let mut lines = Vec::new();
        let title = if let Some(refine) = refinement.filter(|r| *r > 0) {
            format!("+{refine} {}", prettify(&self.name))
        } else {
            prettify(&self.name)
        };
        lines.push(title);

        if let Some(atk) = self.atk {
            lines.push(format!("ATK {atk}"));
        }
        if let Some(matk) = self.matk {
            lines.push(format!("MATK {matk}"));
        }
        if let Some(def) = self.def {
            lines.push(format!("DEF {def}"));
        }
        if let Some(slots) = self.slots {
            lines.push(format!("Slots {slots}"));
        }
        if let Some(lv) = self.equip_lv {
            lines.push(format!("Req. Lv {lv}"));
        }
        if self.weight > 0 {
            // Hercules weights are ×10 of the displayed value.
            let display_weight = self.weight as f32 / 10.0;
            lines.push(format!("Weight {display_weight}"));
        }
        if !self.item_type.is_empty() && self.item_type != "IT_ETC" {
            lines.push(type_label(&self.item_type).to_owned());
        }
        lines
    }
}

fn prettify(name: &str) -> String {
    name.split(['_', ' '])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn type_label(item_type: &str) -> &str {
    match item_type {
        "IT_WEAPON" => "Weapon",
        "IT_ARMOR" => "Armor",
        "IT_HEALING" => "Healing",
        "IT_USABLE" => "Usable",
        "IT_CARD" => "Card",
        "IT_PETEGG" | "IT_PETARMOR" => "Pet",
        "IT_AMMO" => "Ammo",
        "IT_DELAYCONSUME" => "Consumable",
        "IT_CASH" => "Cash",
        other => other.strip_prefix("IT_").unwrap_or(other),
    }
}

fn parse_equip_lv(value: &Option<serde_json::Value>) -> Option<u16> {
    match value {
        Some(serde_json::Value::Number(n)) => n.as_u64().and_then(|v| u16::try_from(v).ok()),
        Some(serde_json::Value::String(s)) => {
            // Export sometimes emits broken strings like `"[1"` — take first digits.
            let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
            digits.parse().ok()
        }
        _ => None,
    }
}

fn parse_loc(value: &Option<serde_json::Value>) -> Option<String> {
    match value {
        Some(serde_json::Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(serde_json::Value::Array(arr)) => {
            let parts: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect();
            if parts.is_empty() { None } else { Some(parts.join("|")) }
        }
        _ => None,
    }
}

fn table() -> &'static HashMap<u32, ItemStats> {
    static TABLE: OnceLock<HashMap<u32, ItemStats>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let rows: Vec<ItemStatsRow> = serde_json::from_str(ITEMS_JSON).expect("embedded items.json is valid");
        rows.into_iter()
            .map(|row| {
                let stats = ItemStats {
                    item_id: row.id,
                    name: row.name,
                    item_type: row.item_type,
                    weight: row.weight,
                    atk: row.atk,
                    matk: row.matk,
                    def: row.def,
                    slots: row.slots,
                    equip_lv: parse_equip_lv(&row.equip_lv),
                    loc: parse_loc(&row.loc),
                };
                (row.id, stats)
            })
            .collect()
    })
}

/// Lookup stats for an item id, if the export knows it.
pub fn item_stats(item_id: u32) -> Option<&'static ItemStats> {
    table().get(&item_id)
}

/// Build a multi-line hover tooltip: name, stats, optional compare vs equipped.
///
/// `display_name` is the already-localized inventory name (preferred over the
/// export's Aegis-style name when non-empty).
pub fn item_tooltip_text(
    item_id: u32,
    display_name: &str,
    refinement: Option<u8>,
    equipped: Option<&ItemStats>,
    equipped_refinement: Option<u8>,
) -> String {
    let Some(stats) = item_stats(item_id) else {
        return if display_name.is_empty() {
            format!("Item #{item_id}")
        } else {
            display_name.to_owned()
        };
    };

    let mut lines = stats.format_lines(refinement);
    if !display_name.is_empty() && display_name != lines[0] && !lines[0].contains(display_name) {
        // Prefer the live client name as the title when it differs.
        let rest = lines.split_off(1);
        lines = vec![if let Some(r) = refinement.filter(|r| *r > 0) {
            format!("+{r} {display_name}")
        } else {
            display_name.to_owned()
        }];
        lines.extend(rest);
    }

    if let Some(eq) = equipped
        && (stats.has_combat_stats() || eq.has_combat_stats())
    {
        lines.push(String::new());
        lines.push("— vs equipped —".to_owned());
        push_delta(&mut lines, "ATK", stats.atk, eq.atk);
        push_delta(&mut lines, "MATK", stats.matk, eq.matk);
        push_delta(&mut lines, "DEF", stats.def, eq.def);
        if let (Some(a), Some(b)) = (stats.slots, eq.slots)
            && a != b
        {
            lines.push(format!("Slots {a} (eq {b})"));
        }
        if equipped_refinement.filter(|r| *r > 0).is_some() || refinement.filter(|r| *r > 0).is_some() {
            let er = equipped_refinement.unwrap_or(0);
            let tr = refinement.unwrap_or(0);
            if er != tr {
                lines.push(format!("Refine +{tr} (eq +{er})"));
            }
        }
    }

    lines.join("\n")
}

fn push_delta(lines: &mut Vec<String>, label: &str, mine: Option<i32>, theirs: Option<i32>) {
    match (mine, theirs) {
        (Some(a), Some(b)) if a != b => {
            let delta = a - b;
            let sign = if delta > 0 { "+" } else { "" };
            lines.push(format!("{label} {a} ({sign}{delta})"));
        }
        (Some(a), Some(_)) => lines.push(format!("{label} {a} (=)")),
        (Some(a), None) => lines.push(format!("{label} {a} (new)")),
        (None, Some(b)) => lines.push(format!("{label} — (eq {b})")),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sword_has_atk() {
        let sword = item_stats(1101).expect("Sword in items.json");
        assert_eq!(sword.atk, Some(25));
        assert!(sword.item_type.contains("WEAPON"));
    }

    #[test]
    fn guard_has_def() {
        let guard = item_stats(2101).expect("Guard in items.json");
        assert_eq!(guard.def, Some(20));
    }

    #[test]
    fn tooltip_includes_compare_deltas() {
        let sword = item_stats(1101).unwrap();
        let better = ItemStats {
            atk: Some(40),
            ..sword.clone()
        };
        let text = item_tooltip_text(1101, "Sword", Some(0), Some(&better), Some(0));
        // Hovered is 25, equipped is 40 → delta -15
        assert!(text.contains("ATK 25"), "{text}");
        assert!(text.contains("-15") || text.contains("(-15)"), "{text}");
        assert!(text.contains("vs equipped"), "{text}");
    }

    #[test]
    fn unknown_item_falls_back_to_name() {
        assert_eq!(item_tooltip_text(u32::MAX, "Mystery", None, None, None), "Mystery");
    }
}
