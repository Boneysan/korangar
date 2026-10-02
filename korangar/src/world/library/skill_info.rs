//! Skill hover tooltips, generated from the server's own skill database.
//!
//! Sourced from `docs/skills.json` (`tools/export_skill_info.py` reading
//! Hercules `db/re/skill_db.conf`), the same pattern as [`item_stats`] and the
//! status name table. Factual rather than flavourful — the official
//! `skilldescript.lub` in the GRF is prose, but ours is a `kr_ro1_live` build
//! whose body text is Korean.
//!
//! The line that earns its place is **Requires**: a skill that silently fails
//! for want of a Blue Gemstone looks exactly like a client bug.
//!
//! [`item_stats`]: super::item_stats

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

const SKILLS_JSON: &str = include_str!("../../../../docs/skills.json");

/// A field that `skill_db.conf` writes either as a scalar or as a per-level
/// table — `Range: 9` but `CastTime: { Lv1: 500, … }`. The exporter normalises
/// both into this one shape.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Levelled {
    Flat { flat: i64 },
    Levels { levels: Vec<i64> },
}

impl Levelled {
    /// Value at `level` (1-based). Levels past the end clamp to the last
    /// entry: `skill_db` sometimes defines fewer rows than `MaxLevel`.
    fn at(&self, level: u16) -> Option<i64> {
        match self {
            Self::Flat { flat } => Some(*flat),
            Self::Levels { levels } => {
                let index = (level.max(1) as usize - 1).min(levels.len().checked_sub(1)?);
                levels.get(index).copied()
            }
        }
    }
}

/// Same shape as [`Levelled`] but for text — six skills vary their element by
/// level (`TK_SEVENWIND` cycles through all of them).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum LevelledText {
    Flat { flat: String },
    Levels { levels: Vec<String> },
}

impl LevelledText {
    fn at(&self, level: u16) -> Option<&str> {
        match self {
            Self::Flat { flat } => Some(flat.as_str()),
            Self::Levels { levels } => {
                let index = (level.max(1) as usize - 1).min(levels.len().checked_sub(1)?);
                levels.get(index).map(String::as_str).filter(|text| !text.is_empty())
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillPrerequisite {
    #[serde(default, rename = "SkillId")]
    pub skill_id: Option<u16>,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Level")]
    pub level: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillSource {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub record: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SkillRow {
    #[serde(rename = "Id")]
    id: u16,
    #[serde(default, rename = "Name")]
    name: String,
    #[serde(default, rename = "Description")]
    description: String,
    #[serde(default, rename = "MaxLevel")]
    maximum_level: u16,
    #[serde(default, rename = "AttackType")]
    attack_type: Option<String>,
    #[serde(default, rename = "Element")]
    element: Option<LevelledText>,
    #[serde(default, rename = "Target")]
    target: Option<String>,
    #[serde(default, rename = "Range")]
    range: Option<Levelled>,
    #[serde(default, rename = "CastTime")]
    cast_time: Option<Levelled>,
    #[serde(default, rename = "FixedCastTime")]
    fixed_cast_time: Option<Levelled>,
    #[serde(default, rename = "AfterCastActDelay")]
    after_cast_action_delay: Option<Levelled>,
    #[serde(default, rename = "AfterCastWalkDelay")]
    after_cast_walk_delay: Option<Levelled>,
    #[serde(default, rename = "CoolDown")]
    cooldown: Option<Levelled>,
    #[serde(default, rename = "NumberOfHits")]
    hits: Option<Levelled>,
    #[serde(default, rename = "SkillData1")]
    duration_ms: Option<Levelled>,
    #[serde(default, rename = "SPCost")]
    sp_cost: Option<Levelled>,
    #[serde(default, rename = "Layout")]
    layout: Option<Levelled>,
    #[serde(default, rename = "StatusChange")]
    status_change: Option<String>,
    #[serde(default, rename = "Prerequisites")]
    prerequisites: Vec<SkillPrerequisite>,
    #[serde(default, rename = "Source")]
    source: Option<SkillSource>,
    #[serde(default, rename = "Items")]
    items: Vec<String>,
}

fn table() -> &'static HashMap<u16, SkillRow> {
    static TABLE: OnceLock<HashMap<u16, SkillRow>> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str::<Vec<SkillRow>>(SKILLS_JSON)
            .expect("embedded skills.json is valid")
            .into_iter()
            .map(|row| (row.id, row))
            .collect()
    })
}

/// The skill's display name from `skill_db` (its description), falling back to
/// the constant name.
pub fn skill_display_name(skill_id: u16) -> Option<&'static str> {
    let row = table().get(&skill_id)?;
    match row.description.is_empty() {
        false => Some(row.description.as_str()),
        true if !row.name.is_empty() => Some(row.name.as_str()),
        true => None,
    }
}

/// Look up the configured status change associated with a skill.
pub fn skill_status_change(skill_id: u16) -> Option<&'static str> {
    table().get(&skill_id)?.status_change.as_deref()
}

/// Look up direct skill prerequisites from the server tree.
pub fn skill_prerequisites(skill_id: u16) -> &'static [SkillPrerequisite] {
    table().get(&skill_id).map(|row| row.prerequisites.as_slice()).unwrap_or(&[])
}

/// Look up configured aftercast action delay in milliseconds at `level`.
pub fn skill_after_cast_act_delay(skill_id: u16, level: u16) -> Option<i64> {
    table().get(&skill_id)?.after_cast_action_delay.as_ref()?.at(level)
}

/// Look up configured aftercast walk delay in milliseconds at `level`.
pub fn skill_after_cast_walk_delay(skill_id: u16, level: u16) -> Option<i64> {
    table().get(&skill_id)?.after_cast_walk_delay.as_ref()?.at(level)
}

/// Look up configured cooldown in milliseconds at `level`.
pub fn skill_cooldown(skill_id: u16, level: u16) -> Option<i64> {
    table().get(&skill_id)?.cooldown.as_ref()?.at(level)
}

/// Look up source provenance for a skill record.
pub fn skill_source(skill_id: u16) -> Option<&'static SkillSource> {
    table().get(&skill_id)?.source.as_ref()
}

/// `Ele_Fire` → `Fire`. Neutral is the default for most skills and adds no
/// information, so it is dropped rather than shown.
fn element_label(element: &str) -> Option<&str> {
    let name = element.strip_prefix("Ele_").unwrap_or(element);
    (!name.eq_ignore_ascii_case("Neutral")).then_some(name)
}

/// Ground-unit footprint in cells. Hercules square layouts are `(2N+1)²`
/// (`skill.c`, `skill_init_unit_layout`); **`-1` is a custom shape**, so there
/// is no honest square to print for it.
fn layout_cells(layout: i64) -> Option<u32> {
    (layout >= 0).then(|| (layout as u32) * 2 + 1)
}

/// Raw `Layout` value for a skill at `level`, or `None` when the skill places
/// no ground unit. `-1` is passed through unchanged so callers can tell "custom
/// shape" apart from "no ground unit" — the aiming cursor needs that
/// distinction, the tooltip does not.
/// `skill_db.conf` element at `level`, such as `Ele_Fire` or `Ele_Weapon`.
/// Levelled elements (Seven Wind) need the cast level; an unknown level returns
/// nothing rather than guessing the first row.
pub fn skill_element_name(skill_id: u16, level: u16) -> Option<&'static str> {
    let element = table().get(&skill_id)?.element.as_ref()?;
    match element {
        LevelledText::Flat { flat } => Some(flat.as_str()),
        LevelledText::Levels { .. } if level == 0 => None,
        LevelledText::Levels { .. } => element.at(level),
    }
}

pub fn skill_layout_value(skill_id: u16, level: u16) -> Option<i64> {
    table().get(&skill_id)?.layout.as_ref()?.at(level)
}

/// Build the hover tooltip for a skill at the level the player has.
///
/// `display_name` is the live client name and wins over the export's when set;
/// an unknown skill id degrades to just that name rather than showing nothing.
pub fn skill_tooltip_text(skill_id: u16, display_name: &str, level: u16, maximum_level: u16) -> String {
    let title = match display_name.is_empty() {
        false => display_name.to_owned(),
        true => format!("Skill #{skill_id}"),
    };

    let Some(row) = table().get(&skill_id) else {
        return title;
    };

    let title = match display_name.is_empty() {
        true if !row.description.is_empty() => row.description.clone(),
        true if !row.name.is_empty() => row.name.clone(),
        _ => title,
    };

    let maximum = match maximum_level {
        0 => row.maximum_level,
        value => value,
    };
    let mut lines = vec![match maximum {
        0 => title,
        maximum => format!("{title}  Lv {level}/{maximum}"),
    }];

    // Kind: what it is, what element, what it aims at.
    let kind: Vec<&str> = [
        row.attack_type.as_deref(),
        row.element.as_ref().and_then(|value| value.at(level)).and_then(element_label),
        row.target.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !kind.is_empty() {
        lines.push(kind.join(" · "));
    }

    // Cost: what casting it takes. Cast time is the base total — renewal
    // splits it into variable + fixed, and only the variable part is reduced
    // by stats, so the sum is what an unbuffed cast actually takes.
    let mut cost = Vec::new();
    if let Some(range) = row.range.as_ref().and_then(|value| value.at(level)) {
        cost.push(format!("Range {range}"));
    }
    if let Some(sp) = row.sp_cost.as_ref().and_then(|value| value.at(level)) {
        cost.push(format!("SP {sp}"));
    }
    let cast = row.cast_time.as_ref().and_then(|value| value.at(level)).unwrap_or(0)
        + row.fixed_cast_time.as_ref().and_then(|value| value.at(level)).unwrap_or(0);
    if cast > 0 {
        cost.push(format!("Cast {:.1}s", cast as f32 / 1000.0));
    }
    if !cost.is_empty() {
        lines.push(cost.join(" · "));
    }

    let mut timing = Vec::new();
    if let Some(delay) = row
        .after_cast_action_delay
        .as_ref()
        .and_then(|value| value.at(level))
        .filter(|delay| *delay > 0)
    {
        timing.push(format!("Configured aftercast delay {:.1}s", delay as f32 / 1000.0));
    }
    if let Some(walk_delay) = row
        .after_cast_walk_delay
        .as_ref()
        .and_then(|value| value.at(level))
        .filter(|delay| *delay > 0)
    {
        timing.push(format!("Walk delay {:.1}s", walk_delay as f32 / 1000.0));
    }
    if let Some(cooldown) = row
        .cooldown
        .as_ref()
        .and_then(|value| value.at(level))
        .filter(|cooldown| *cooldown > 0)
    {
        timing.push(format!("Configured cooldown {:.1}s", cooldown as f32 / 1000.0));
    }
    if !timing.is_empty() {
        lines.push(timing.join(" · "));
    }

    // Effect: multi-hit, how long a ground field lasts, how much it covers.
    let mut effect = Vec::new();
    if let Some(hits) = row.hits.as_ref().and_then(|value| value.at(level)).filter(|hits| *hits > 1) {
        effect.push(format!("{hits} hits"));
    }
    if let Some(duration) = row.duration_ms.as_ref().and_then(|value| value.at(level)).filter(|ms| *ms > 0) {
        effect.push(format!("Lasts {}s", duration / 1000));
    }
    if let Some(cells) = row.layout.as_ref().and_then(|value| value.at(level)).and_then(layout_cells) {
        effect.push(format!("{cells}x{cells} cells"));
    }
    if !effect.is_empty() {
        lines.push(effect.join(" · "));
    }

    if !row.items.is_empty() {
        lines.push(format!("Requires: {}", row.items.join(", ")));
    }

    if level == 0 && !row.prerequisites.is_empty() {
        let reqs = row
            .prerequisites
            .iter()
            .map(|req| format!("{} Lv {}", req.name, req.level))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("Requires skill: {reqs}"));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Skill ids from Hercules `db/re/skill_db.conf`.
    const FIRE_BOLT: u16 = 19;
    const LAND_PROTECTOR: u16 = 288;
    const VENOM_DUST: u16 = 140;

    #[test]
    fn the_embedded_table_loads() {
        assert!(table().len() > 1000, "expected the full skill db, got {}", table().len());
    }

    #[test]
    fn per_level_fields_track_the_learned_level() {
        // Fire Bolt's SP and hit count both rise with level; showing level 1's
        // numbers on a level 7 skill would be actively misleading.
        let low = skill_tooltip_text(FIRE_BOLT, "Fire Bolt", 1, 10);
        let high = skill_tooltip_text(FIRE_BOLT, "Fire Bolt", 7, 10);

        assert!(low.contains("SP 12"), "{low}");
        assert!(high.contains("SP 24"), "{high}");
        assert!(high.contains("7 hits"), "{high}");
        assert!(!low.contains("hits"), "a single-hit level must not say 'hits': {low}");
    }

    #[test]
    fn levels_past_the_table_clamp_instead_of_panicking() {
        // skill_db sometimes defines fewer rows than MaxLevel.
        let text = skill_tooltip_text(FIRE_BOLT, "Fire Bolt", 99, 10);
        assert!(text.contains("SP 30"), "expected the last defined level: {text}");
    }

    /// The line this whole feature exists for.
    #[test]
    fn reagents_are_listed() {
        let text = skill_tooltip_text(LAND_PROTECTOR, "Magnetic Earth", 5, 5);
        assert!(text.contains("Requires: Blue Gemstone, Yellow Gemstone"), "{text}");
    }

    #[test]
    fn ground_fields_show_duration_and_area() {
        let text = skill_tooltip_text(LAND_PROTECTOR, "Magnetic Earth", 5, 5);
        assert!(text.contains("Ground target"), "{text}");
        assert!(text.contains("Lasts 345s"), "{text}");
        // Layout 5 at Lv5 -> 5*2+1 = 11 cells across.
        assert!(text.contains("11x11 cells"), "{text}");
    }

    #[test]
    fn a_custom_layout_prints_no_area() {
        // Venom Dust is Layout -1 — a custom shape. Printing a square size for
        // it would be confidently wrong.
        let text = skill_tooltip_text(VENOM_DUST, "Venom Dust", 5, 10);
        assert!(!text.contains("cells"), "custom layouts have no square size: {text}");
        assert!(text.contains("Requires: Red Gemstone"), "{text}");
    }

    #[test]
    fn neutral_element_is_not_worth_a_line() {
        assert_eq!(element_label("Ele_Fire"), Some("Fire"));
        assert_eq!(element_label("Ele_Neutral"), None);
    }

    /// Pins the whole rendered block, not just substrings — line count and
    /// width are what decide whether the tooltip crowds the screen.
    #[test]
    fn the_rendered_block_stays_compact() {
        let text = skill_tooltip_text(LAND_PROTECTOR, "Magnetic Earth", 5, 5);

        assert_eq!(
            text,
            "Magnetic Earth  Lv 5/5\nMagic · Ground target\nRange 2 · SP 50 · Cast 5.0s\nLasts 345s · 11x11 cells\nRequires: Blue \
             Gemstone, Yellow Gemstone"
        );
        assert!(text.lines().count() <= 5, "tooltip should stay short: {text}");
        assert!(
            text.lines().all(|line| line.chars().count() <= 45),
            "no line should be wide enough to crowd the screen: {text}"
        );
    }

    #[test]
    fn an_unknown_skill_degrades_to_its_name() {
        assert_eq!(skill_tooltip_text(u16::MAX, "Mystery Skill", 1, 1), "Mystery Skill");
    }

    #[test]
    fn the_first_line_always_carries_the_name_and_level() {
        let text = skill_tooltip_text(FIRE_BOLT, "Fire Bolt", 3, 10);
        let headline = text.lines().next().unwrap();
        assert!(headline.starts_with("Fire Bolt"), "{headline}");
        assert!(headline.contains("Lv 3/10"), "{headline}");
    }

    #[test]
    fn timing_helpers_and_status_change_links_load() {
        const BLESSING: u16 = 34;
        const MAGNUM_BREAK: u16 = 7;

        assert_eq!(skill_status_change(BLESSING), Some("SC_BLESSING"));
        assert_eq!(skill_status_change(MAGNUM_BREAK), Some("SC_SUB_WEAPONPROPERTY"));
        assert_eq!(skill_after_cast_act_delay(MAGNUM_BREAK, 1), Some(500));
        assert_eq!(skill_cooldown(MAGNUM_BREAK, 1), Some(2000));

        const GRAND_CROSS: u16 = 254;
        assert_eq!(skill_after_cast_walk_delay(GRAND_CROSS, 1), Some(900));
        let gx_text = skill_tooltip_text(GRAND_CROSS, "Grand Cross", 1, 10);
        assert!(gx_text.contains("Walk delay 0.9s"), "{gx_text}");

        let source = skill_source(FIRE_BOLT).expect("Fire Bolt source");
        assert_eq!(source.path, "db/re/skill_db.conf");
        assert_eq!(source.record, "Id=19");
    }

    #[test]
    fn skill_prerequisites_are_linked() {
        const FIRE_WALL: u16 = 18;
        const MAGNUM_BREAK: u16 = 7;

        let fw_prereqs = skill_prerequisites(FIRE_WALL);
        assert_eq!(fw_prereqs.len(), 2);
        assert!(
            fw_prereqs
                .iter()
                .any(|p| p.name == "MG_FIREBALL" && p.level == 5 && p.skill_id == Some(17))
        );
        assert!(
            fw_prereqs
                .iter()
                .any(|p| p.name == "MG_SIGHT" && p.level == 1 && p.skill_id == Some(10))
        );

        let mb_prereqs = skill_prerequisites(MAGNUM_BREAK);
        assert_eq!(mb_prereqs.len(), 1);
        assert_eq!(mb_prereqs[0].name, "SM_BASH");
        assert_eq!(mb_prereqs[0].level, 5);
        assert_eq!(mb_prereqs[0].skill_id, Some(5));
    }

    #[test]
    fn unlearned_skills_show_prerequisites_in_tooltip() {
        const FIRE_WALL: u16 = 18;
        let unlearned = skill_tooltip_text(FIRE_WALL, "Fire Wall", 0, 10);
        assert!(
            unlearned.contains("Requires skill: MG_FIREBALL Lv 5, MG_SIGHT Lv 1"),
            "{unlearned}"
        );

        let learned = skill_tooltip_text(FIRE_WALL, "Fire Wall", 1, 10);
        assert!(!learned.contains("Requires skill:"), "{learned}");
    }
}
