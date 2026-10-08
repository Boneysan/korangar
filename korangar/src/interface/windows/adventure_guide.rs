//! Player-facing, open-search reference guide. This intentionally does not
//! reuse the DM Bestiary: campaign reveal/spawn controls and unlocks are not
//! part of the player's mechanical reference.

use std::collections::HashSet;
use std::sync::Arc;

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox, StateElement};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, RustState, State, VecIndexExt};

use crate::dm::reference_data::{
    ReferenceCraftingEntry, ReferenceItem, ReferenceJobBonuses, ReferenceMonster, ReferenceNpc, ReferenceNpcServiceReview,
    ReferenceRefinement, ReferenceRumor, ReferenceSkill, reference_data,
};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::{Library, TownPoi, stat_formulas};

const MAX_QUERY: usize = 48;
const MAX_RESULTS: usize = 60;
const JOB_NAMES: &str = include_str!("../../world/library/hercules_job_names.tsv");

#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct GuideResult {
    pub label: String,
    pub kind: String,
    pub id: u32,
}

#[derive(RustState, StateElement)]
pub struct AdventureGuideWindowState {
    query: String,
    category: String,
    results: Vec<GuideResult>,
    detail: Vec<String>,
}

impl Default for AdventureGuideWindowState {
    fn default() -> Self {
        Self {
            query: String::new(),
            category: "All".to_owned(),
            results: Vec::new(),
            detail: vec!["Choose a category and search, then select an entry.".to_owned()],
        }
    }
}

pub struct AdventureGuideWindow<A> {
    state_path: A,
    library: Arc<Library>,
}

impl<A> AdventureGuideWindow<A> {
    pub fn new(state_path: A, library: Arc<Library>) -> Self {
        Self { state_path, library }
    }
}

fn listed<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| "not listed".to_owned(), |value| value.to_string())
}

/// EXP, combat numbers and behavior straight from the monster's `mob_db.conf`
/// record. Values are shown as configured; `Attack` keeps the database's own
/// two-number form because the two values feed different damage paths.
fn monster_combat_lines(monster: &ReferenceMonster) -> Vec<String> {
    // The database stores two attack figures in either order. Show the lower
    // one first so the line reads as a range.
    let attack = monster.attack.map_or_else(
        || "not listed".to_owned(),
        |pair| {
            let (low, high) = if pair[0] <= pair[1] {
                (pair[0], pair[1])
            } else {
                (pair[1], pair[0])
            };
            format!("{low}–{high}")
        },
    );
    let speed = match monster.move_speed {
        Some(speed) => format!("{speed} ms per cell (an unhasted player is 150; lower is faster)"),
        None => "not listed".to_owned(),
    };
    vec![
        format!("EXP: base {}   job {}", listed(monster.base_exp), listed(monster.job_exp)),
        format!(
            "Attack: {attack}   DEF {}   MDEF {}",
            listed(monster.defense),
            listed(monster.magic_defense)
        ),
        format!(
            "Attack range {} cells   Sight range {} cells",
            listed(monster.attack_range),
            listed(monster.view_range)
        ),
        format!("Move speed: {speed}"),
        format!("Behavior: {}", monster_behavior(&monster.modes)),
    ]
}

/// Player-facing reading of Hercules `Mode` flags. Each phrase follows the
/// server code path, not the flag's name: only `Aggressive` makes a monster
/// look for targets on its own (`mob.c` target search), while `Angry` changes
/// its skill state and is left out. Flags without a verified description are
/// listed by name.
fn monster_behavior(modes: &[String]) -> String {
    let has = |flag: &str| modes.iter().any(|mode| mode == flag);
    let mut parts = Vec::new();

    if !has("CanAttack") {
        parts.push("never attacks".to_owned());
    } else if has("Aggressive") {
        parts.push("aggressive — attacks players on sight".to_owned());
    } else {
        parts.push("passive — fights back only when attacked".to_owned());
    }
    if !has("CanMove") {
        parts.push("does not move".to_owned());
    }
    if has("Assist") {
        parts.push("nearby monsters of the same kind join its fights".to_owned());
    }
    if has("Looter") {
        parts.push("picks up items from the ground".to_owned());
    }
    if has("Detector") {
        parts.push("sees hidden and cloaked players".to_owned());
    }
    if has("CastSensorIdle") || has("CastSensorChase") {
        parts.push("turns on a player who starts casting a skill at it".to_owned());
    }
    if has("Boss") {
        parts.push("boss".to_owned());
    }

    const DESCRIBED: [&str; 9] = [
        "CanAttack",
        "Aggressive",
        "CanMove",
        "Assist",
        "Looter",
        "Detector",
        "CastSensorIdle",
        "CastSensorChase",
        "Boss",
    ];
    let other: Vec<&str> = modes.iter().map(String::as_str).filter(|mode| !DESCRIBED.contains(mode)).collect();
    let mut text = parts.join("; ");
    if !other.is_empty() {
        text.push_str(&format!(" (other server flags: {})", other.join(", ")));
    }
    text
}

/// `MSS_BERSERK, MSC_MYHPLTMAXRATE 50, MST_SELF` — the raw mob_skill_db
/// fields; zero or null condition data and `val0` are left out.
fn raw_mob_skill_record(mob_skill: &crate::dm::reference_data::ReferenceMobSkill) -> String {
    let meaningful = |value: &serde_json::Value| match value {
        serde_json::Value::Null => None,
        serde_json::Value::Number(number) if number.as_i64() == Some(0) => None,
        serde_json::Value::String(text) if text.is_empty() || text == "0" => None,
        other => Some(other.to_string().trim_matches('"').to_owned()),
    };
    let condition = match meaningful(&mob_skill.condition_data) {
        Some(data) => format!("{} {data}", mob_skill.cast_condition),
        None => mob_skill.cast_condition.clone(),
    };
    let mut parts = vec![mob_skill.skill_state.clone(), condition, mob_skill.skill_target.clone()];
    if let Some(value0) = meaningful(&mob_skill.value0) {
        parts.push(format!("val0 {value0}"));
    }
    parts.join(", ")
}

/// One monster skill list with trigger and target notes, capped at eight rows.
fn push_monster_skills(
    lines: &mut Vec<String>,
    heading: &str,
    skills: &[crate::dm::reference_data::ReferenceMobSkill],
    source: &Option<crate::dm::reference_data::ReferenceSource>,
) {
    let data = reference_data();
    if skills.is_empty() {
        lines.push(format!("{heading} none."));
        return;
    }
    lines.push(heading.to_owned());
    for mob_skill in skills.iter().take(8) {
        let chance = if mob_skill.rate > 0 {
            format!("{:.2}%", mob_skill.rate as f32 / 100.0)
        } else {
            "not available".to_owned()
        };
        let cast_time = if mob_skill.cast_time_ms > 0 {
            format!("; cast time {:.1}s", mob_skill.cast_time_ms as f32 / 1000.0)
        } else {
            String::new()
        };
        let cancel = if mob_skill.cancelable {
            "; cast can be interrupted"
        } else {
            "; cast is not configured as interruptible"
        };
        let label = data
            .skills
            .iter()
            .find(|skill| skill.name.eq_ignore_ascii_case(&mob_skill.skill_name))
            .map(|skill| {
                format!(
                    "@guide:skill:{}|{} — Lv {} — {} — retry delay {:.1}s{}{}",
                    skill.id,
                    display_name(&skill.description, &skill.name),
                    mob_skill.level,
                    chance,
                    mob_skill.delay_ms as f32 / 1000.0,
                    cast_time,
                    cancel
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "{} — Lv {} — {} — retry delay {:.1}s{}{} (no matching skill reference)",
                    mob_skill.skill_name,
                    mob_skill.level,
                    chance,
                    mob_skill.delay_ms as f32 / 1000.0,
                    cast_time,
                    cancel
                )
            });
        lines.push(label);
        let translated = mob_skill.trigger_translation_status == "translated" && !mob_skill.trigger_summary.is_empty();
        match translated {
            true => lines.push(format!("  {}", mob_skill.trigger_summary)),
            false => lines.push(format!(
                "  Trigger details have not been translated from the server record ({}).",
                display_name(&mob_skill.trigger_translation_status, "no status")
            )),
        }
        if !mob_skill.target_summary.is_empty() {
            lines.push(format!("  {}", mob_skill.target_summary));
        }
        // The translation above is derived from these raw fields. The record
        // stays on a source line so a reader can check mob_skill_db.conf.
        lines.push(format!("Source: {}.", raw_mob_skill_record(mob_skill)));
    }
    if skills.len() > 8 {
        lines.push(format!("Source: {} additional skill records omitted.", skills.len() - 8));
    }
    if let Some(source) = source {
        lines.push(format!("Source: {} ({}).", source.path, source.record));
    }
}

fn monster_details(monster: &ReferenceMonster) -> Vec<String> {
    let data = reference_data();
    let mut lines = vec![
        display_name(&monster.name, &monster.sprite_name),
        format!("Level {}   HP {}", monster.level, monster.hp),
        format!("@hunting-goal:{}|Add to my hunting goals", monster.id),
    ];
    if let Some(element) = &monster.element {
        lines.push(format!("Element: {} {}", element.r#type, element.level));
    }
    if let Some(race) = &monster.race {
        lines.push(format!("Race: {race}"));
    }
    if let Some(size) = &monster.size {
        lines.push(format!("Size: {size}"));
    }
    lines.extend(monster_combat_lines(monster));
    let pilot = data.pilot_skill_layer.as_ref();
    match (&monster.pilot_skills, pilot) {
        (Some(pilot_skills), Some(layer)) => {
            lines.push(match layer.active {
                true => "Uses the newer skill list on this server.".to_owned(),
                false => "Uses the original skill list on this server.".to_owned(),
            });
            lines.push(format!(
                "Source: {} = {} in {}.",
                layer.setting, layer.configured_value, layer.config_source
            ));
            let (first, second) = match layer.active {
                true => ("Pilot skills (in use):", "Stock skills (used only when the pilot is off):"),
                false => ("Stock skills (in use):", "Pilot skills (used only when the pilot is on):"),
            };
            let (first_skills, first_source, second_skills, second_source) = match layer.active {
                true => (
                    pilot_skills.as_slice(),
                    &monster.pilot_skills_source,
                    monster.skills.as_slice(),
                    &monster.skills_source,
                ),
                false => (
                    monster.skills.as_slice(),
                    &monster.skills_source,
                    pilot_skills.as_slice(),
                    &monster.pilot_skills_source,
                ),
            };
            push_monster_skills(&mut lines, first, first_skills, first_source);
            push_monster_skills(&mut lines, second, second_skills, second_source);
        }
        _ => push_monster_skills(&mut lines, "Skills:", &monster.skills, &monster.skills_source),
    }
    if monster.drops.is_empty() {
        lines.push("No drops listed.".to_owned());
    } else {
        lines.push("Drops:".to_owned());
        lines.push("Source: drop rates are the database figures. The server's drop-rate setting can change them.".to_owned());
    }
    let mut sorted_drops = monster.drops.iter().collect::<Vec<_>>();
    sorted_drops.sort_by_key(|drop| {
        let is_card = data.card_by_id(drop.item_id).is_some()
            || crate::world::item_stats(drop.item_id)
                .as_ref()
                .is_some_and(|s| s.item_type.eq_ignore_ascii_case("Card"));
        let is_mvp = drop.kind == "mvp";
        if is_card {
            0u8
        } else if is_mvp {
            1u8
        } else {
            2u8
        }
    });
    for drop in sorted_drops.iter().take(8) {
        lines.push(format!(
            "@guide:item:{}|{} — {} drop {:.2}%",
            drop.item_id,
            display_name(&drop.name, &drop.aegis_name),
            if drop.kind == "mvp" { "MVP" } else { "normal" },
            drop.rate_per_10000 as f32 / 100.0
        ));
    }
    if let Some(source) = &monster.source {
        lines.push(format!("Source: {} ({})", source.path, source.record));
    }
    let routeable_regions = monster
        .spawn_regions
        .iter()
        .filter(|region| is_graph_map(&region.map))
        .collect::<Vec<_>>();
    if routeable_regions.is_empty() {
        lines.push("No map with a route lists this monster.".to_owned());
    } else {
        lines.push("Found on:".to_owned());
        for region in routeable_regions.iter().take(12) {
            lines.push(region.map.clone());
            lines.push(format!(
                "Source: {} loaded records, {} monsters listed.",
                region.spawn_records, region.listed_monsters
            ));
            for placement in region.placements.iter().take(3) {
                let place = if placement.random_map_cell {
                    "a random cell on the map".to_owned()
                } else if placement.x_spread != 0 || placement.y_spread != 0 {
                    format!(
                        "around ({}, {}), spread {}×{}",
                        placement.x, placement.y, placement.x_spread, placement.y_spread
                    )
                } else {
                    format!("at ({}, {})", placement.x, placement.y)
                };
                lines.push(format!("  {place} ×{}", placement.amount));
                lines.push(format!("Source: {}.", placement.source));
            }
            if region.placements.len() > 3 {
                lines.push(format!("Source: {} more placement records.", region.placements.len() - 3));
            }
            lines.push(format!("@route:{}", region.map));
        }
        if routeable_regions.len() > 12 {
            lines.push(format!(
                "Source: {} additional spawn maps omitted.",
                routeable_regions.len() - 12
            ));
        }
    }
    if !monster.scripted_spawn_references.is_empty() {
        lines.push("Also placed by events or quests at (conditions not shown):".to_owned());
        for spawn in monster.scripted_spawn_references.iter().take(8) {
            let location = match (spawn.map.as_deref(), spawn.coordinates.as_slice()) {
                (Some(map), [Some(x), Some(y)]) => format!("{map} at ({x}, {y})"),
                (Some(map), [Some(x1), Some(y1), Some(x2), Some(y2)]) => {
                    format!("{map} in area ({x1}, {y1})–({x2}, {y2})")
                }
                (None, _) if spawn.map_template.is_some() => {
                    format!(
                        "{}an instance of {}",
                        if spawn.map_template_approximate {
                            "approximately on "
                        } else {
                            "on "
                        },
                        spawn.map_template.as_deref().unwrap_or_default()
                    )
                }
                (None, [Some(x), Some(y)]) => format!("a script-chosen map at ({x}, {y})"),
                (None, [Some(x1), Some(y1), Some(x2), Some(y2)]) => {
                    format!("a script-chosen map in area ({x1}, {y1})–({x2}, {y2})")
                }
                (..) => "a script-chosen map".to_owned(),
            };
            let amount = spawn.amount.map(|count| format!(" ×{count}")).unwrap_or_default();
            lines.push(format!("{location}{amount}."));
            if !spawn.map_expression.is_empty() {
                lines.push(format!("Source: map expression `{}`.", spawn.map_expression));
            }
            lines.push(format!("Source: {} — {}.", spawn.spawn_kind, spawn.source));
            if let (Some(map), [Some(x), Some(y)]) = (spawn.map.as_deref(), spawn.coordinates.as_slice()) {
                lines.push(format!("@route-cell:{map}:{x}:{y}|Route near scripted spawn — {map}"));
            } else if let (Some(map), [Some(x1), Some(y1), Some(x2), Some(y2)]) = (spawn.map.as_deref(), spawn.coordinates.as_slice()) {
                let x = (x1 + x2) / 2;
                let y = (y1 + y2) / 2;
                lines.push(format!("@route-cell:{map}:{x}:{y}|Route near scripted spawn area — {map}"));
            }
        }
        if monster.scripted_spawn_references.len() > 8 {
            lines.push(format!(
                "Source: {} additional scripted spawns omitted.",
                monster.scripted_spawn_references.len() - 8
            ));
        }
        lines.push("A script placement does not mean the monster is there for every player right now.".to_owned());
    }
    let related_rumors = reference_data().rumors_for_monster(monster.id);
    if !related_rumors.is_empty() {
        lines.push("Related local rumors:".to_owned());
        for rumor in related_rumors {
            if !rumor.is_story_spoiler {
                lines.push(format!("@guide:rumor:{}|{} ({})", rumor.id, rumor.title, rumor.category));
            }
        }
    }
    lines
}

/// One clickable "dropped by" row: the monster's player-facing name and level
/// when the bestiary has it, falling back to the database sprite name.
fn drop_source_line(monster_id: u32, sprite_name: &str, kind: &str, rate_per_10000: u32) -> String {
    let (name, level) = match reference_data().monster_by_id(monster_id) {
        Some(monster) => (display_name(&monster.name, &monster.sprite_name), Some(monster.level)),
        None => (sprite_name.to_owned(), None),
    };
    let level = level.map(|level| format!(" (Lv {level})")).unwrap_or_default();
    format!(
        "@guide:monster:{monster_id}|{name}{level} — {} drop {:.2}%",
        if kind == "mvp" { "MVP" } else { "normal" },
        rate_per_10000 as f32 / 100.0
    )
}

/// The "dropped by" rows of an item page. They lead the page because "where
/// do I get this" is the question most players open it with.
fn push_item_drop_lines(lines: &mut Vec<String>, item: &ReferenceItem) {
    if !item.drops_from.is_empty() {
        lines.push("Dropped by:".to_owned());
        for source in item.drops_from.iter().take(8) {
            lines.push(drop_source_line(
                source.monster_id,
                &source.sprite_name,
                &source.kind,
                source.rate_per_10000,
            ));
            if let Some(monster) = reference_data().monster_by_id(source.monster_id) {
                for region in monster.spawn_regions.iter().take(3) {
                    if !is_graph_map(&region.map) {
                        continue;
                    }
                    lines.push(format!("@route:{}", region.map));
                }
            }
        }
    } else {
        let dropping_monsters = reference_data().monsters_dropping_item(item.id);
        if !dropping_monsters.is_empty() {
            lines.push("Dropped by:".to_owned());
            for monster in dropping_monsters.iter().take(8) {
                if let Some(drop) = monster.drops.iter().find(|d| d.item_id == item.id) {
                    lines.push(drop_source_line(
                        monster.id,
                        &monster.sprite_name,
                        &drop.kind,
                        drop.rate_per_10000,
                    ));
                    for region in monster.spawn_regions.iter().take(3) {
                        if !is_graph_map(&region.map) {
                            continue;
                        }
                        lines.push(format!("@route:{}", region.map));
                    }
                }
            }
        }
    }
    if lines.iter().any(|line| line.starts_with("Dropped by:")) {
        lines.push("Source: drop rates are the database figures. The server's drop-rate setting can change them.".to_owned());
    }
}

/// Combat numbers, the effect, the price, and who sells it. These come before
/// the script clues so a player sees what the item is and where to get it.
fn push_item_player_facts(lines: &mut Vec<String>, item: &ReferenceItem) {
    if !item.job.is_empty() {
        let mut jobs = item
            .job
            .iter()
            .filter(|(_, allowed)| **allowed)
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();
        jobs.sort_unstable();
        lines.push(format!("Allowed jobs: {}", jobs.join(", ")));
    }
    if let Some(gender) = &item.gender {
        let label = match gender.as_str() {
            "SEX_MALE" => "Male characters",
            "SEX_FEMALE" => "Female characters",
            _ => gender,
        };
        lines.push(format!("Gender restriction: {label}"));
    }
    if let Some(location) = &item.loc {
        let locations = match location {
            serde_json::Value::String(value) => vec![value.as_str()],
            serde_json::Value::Array(values) => values.iter().filter_map(serde_json::Value::as_str).collect(),
            _ => Vec::new(),
        };
        if !locations.is_empty() {
            let readable = locations
                .iter()
                .map(|location| location.strip_prefix("EQP_").unwrap_or(location).replace('_', " "))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("Worn on: {readable}"));
        } else if let Some(flags) = location.as_i64() {
            lines.push(format!("Source: equipment location flags {flags}."));
        }
    }
    if let Some(level) = &item.equip_level {
        let requirement = match level {
            serde_json::Value::Array(range) if range.len() == 2 => {
                format!("{}–{}", range[0], range[1])
            }
            other => other.to_string(),
        };
        lines.push(format!("Required base level: {requirement}"));
    }
    if let Some(weapon_level) = item.weapon_level {
        lines.push(format!("Weapon level: {weapon_level}"));
    }
    if let Some(refineable) = item.refine {
        lines.push(if refineable { "Can be refined." } else { "Cannot be refined." }.to_owned());
    }
    if let Some(atk) = item.atk {
        lines.push(format!("ATK: {atk}"));
    }
    if let Some(matk) = item.matk {
        lines.push(format!("MATK: {matk}"));
    }
    if let Some(defense) = item.defense {
        lines.push(format!("DEF: {defense}"));
    }
    if let Some(slots) = item.slots {
        lines.push(format!("Slots: {slots}"));
    }
    if let Some(summary) = &item.effect_summary {
        lines.push(format!("Effect: {summary}"));
        lines.push("Source: the effect text covers recognized script patterns. Stacking with other effects may be unstated.".to_owned());
    } else if item.effect_status == "scripted_not_translated" {
        lines.push("Effect: not translated yet.".to_owned());
    } else if item.effect_status == "no_script_field" {
        lines.push("No special effect listed.".to_owned());
    } else if !item.effect_status.is_empty() {
        lines.push(format!("Source: effect coverage {}.", item.effect_status.replace('_', " ")));
    } else {
        lines.push("Effect details: not documented yet.".to_owned());
    }
    if item.buy > 0 {
        lines.push(format!("Buy price: {}z", item.buy));
    }
    if let Some(sell) = item.sell {
        lines.push(format!("Sell price: {sell}z"));
    }
    if item.shops.is_empty() {
        return;
    }
    lines.push("Sold by:".to_owned());
    for shop in item.shops.iter().take(8) {
        let price = if shop.uses_item_db_price {
            if item.buy > 0 {
                format!("{} {}", item.buy, shop.currency)
            } else {
                "the item database price".to_owned()
            }
        } else {
            format!("{} {}", shop.price.unwrap_or_default(), shop.currency)
        };
        let place = match shop.shop_type.as_str() {
            "cashshop" => "cash shop",
            _ => "shop",
        };
        lines.push(format!(
            "{place}: {} on {} at ({}, {}) for {price}.",
            shop.npc_name, shop.map, shop.x, shop.y,
        ));
        if let (Ok(x), Ok(y)) = (u16::try_from(shop.x), u16::try_from(shop.y))
            && is_graph_map(&shop.map)
        {
            lines.push(format!(
                "@route-cell:{map}:{x}:{y}|Route to {} — {map}",
                shop.npc_name,
                map = shop.map
            ));
        }
        if !shop.source.is_empty() {
            lines.push(format!("Source: {}.", shop.source));
        }
    }
    if item.shops.len() > 8 {
        lines.push(format!("{} additional shops omitted.", item.shops.len() - 8));
    }
    lines.push("Source: only shops whose stock is written out in a script are listed.".to_owned());
}

fn item_details(item: &ReferenceItem, card: bool) -> Vec<String> {
    let kind = if card { "Card" } else { item.item_type.as_str() };
    let mut lines = vec![
        display_name(&item.name, &item.aegis_name),
        format!("Type: {}   Weight: {}", item_type_label(kind), item.weight),
    ];
    push_item_drop_lines(&mut lines, item);
    push_item_player_facts(&mut lines, item);
    let relevant = reference_data()
        .crafting_entries
        .iter()
        .filter_map(|entry| match entry {
            ReferenceCraftingEntry::Production(recipe) | ReferenceCraftingEntry::ArrowConversion(recipe) => Some(recipe),
            ReferenceCraftingEntry::Combo(_) => None,
        })
        .filter(|recipe| recipe.output_id == item.id || recipe.materials.iter().any(|material| material.item_id == item.id))
        .collect::<Vec<_>>();
    if !relevant.is_empty() {
        lines.push("Crafting and conversion recipes:".to_owned());
        let mut recipe_sources = Vec::new();
        for recipe in relevant.iter().take(12) {
            if recipe.output_id == item.id {
                let materials = recipe
                    .materials
                    .iter()
                    .map(|material| {
                        let count = if material.required {
                            format!("×{}", material.amount)
                        } else {
                            "guide required".to_owned()
                        };
                        format!("@guide:item:{}|{} {count}", material.item_id, material.item_name)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let requirement = recipe
                    .skill_name
                    .as_deref()
                    .map(|name| format!("; {name} Lv {} required", recipe.skill_level.unwrap_or_default()))
                    .unwrap_or_default();
                let item_level = recipe.item_level.map(|level| format!("; item level {level}")).unwrap_or_default();
                lines.push(format!(
                    "Makes {} ×{} from {materials}{requirement}{item_level}",
                    recipe.output_name, recipe.output_amount
                ));
            } else {
                lines.push(format!(
                    "Used to make @guide:item:{}|{} ×{}",
                    recipe.output_id, recipe.output_name, recipe.output_amount
                ));
            }
            let citation = format!("{}:{}", recipe.source.path, recipe.source.record);
            if !recipe.source.path.is_empty() && !recipe_sources.contains(&citation) {
                recipe_sources.push(citation);
            }
        }
        if relevant.len() > 12 {
            lines.push(format!("{} additional recipe links omitted.", relevant.len() - 12));
        }
        for citation in recipe_sources {
            lines.push(format!("Source: recipe {citation}."));
        }
        lines.push(
            "Source: recipe lists are configured server recipes. They do not establish access to the required skill or guarantee success."
                .to_owned(),
        );
    }
    let reviewed_quest_rewards = reference_data()
        .quests
        .iter()
        .filter_map(|quest| {
            let flow = quest.flow_review.as_ref()?;
            let reward = flow.verified_item_rewards.iter().find(|reward| reward.item_id == item.id)?;
            Some((quest, flow, reward))
        })
        .collect::<Vec<_>>();
    if !reviewed_quest_rewards.is_empty() {
        lines.push("Quest rewards:".to_owned());
        for (quest, flow, reward) in reviewed_quest_rewards {
            lines.push(format!(
                "@guide:quest:{}|From the quest {}: ×{}",
                quest.id, quest.name, reward.amount
            ));
            if !reward.explanation.is_empty() {
                lines.push(reward.explanation.clone());
            }
            for condition in &flow.conditions {
                lines.push(format!("Reward condition: {condition}"));
            }
            lines.push(format!(
                "Source: {} ×{} at {}:{} ({}).",
                reward.item_name,
                reward.amount,
                reward.source_path,
                reward.source_line,
                reward.evidence_state.label()
            ));
        }
    }
    let reviewed_reward_source_keys = reference_data()
        .quests
        .iter()
        .flat_map(|quest| quest.flow_review.iter().flat_map(|flow| flow.verified_item_rewards.iter()))
        .map(|reward| (reward.item_id, reward.amount, reward.source_path.clone(), reward.source_line))
        .collect::<HashSet<_>>();
    let grants = reference_data()
        .item_grants
        .iter()
        .filter(|grant| {
            grant.item_id == item.id
                && !reviewed_reward_source_keys.contains(&(grant.item_id, grant.amount, grant.source.path.clone(), grant.source.line))
        })
        .collect::<Vec<_>>();
    if !grants.is_empty() {
        lines.push("May also be given:".to_owned());
        for grant in grants.iter().take(10) {
            let status = match grant.condition_status.as_str() {
                "script_context_unreviewed" => "conditions not reviewed",
                other => other,
            };
            if let Some(npc) = &grant.npc_clue {
                lines.push(format!(
                    "May be given near {} on {} at ({}, {}): ×{}.",
                    npc.name, npc.map, npc.x, npc.y, grant.amount
                ));
                if let (Ok(x), Ok(y)) = (u16::try_from(npc.x), u16::try_from(npc.y))
                    && is_graph_map(&npc.map)
                {
                    lines.push(format!(
                        "@route-cell:{map}:{x}:{y}|Route to {} — {map}",
                        npc.name,
                        map = npc.map
                    ));
                }
            } else {
                lines.push(format!("May be given: ×{}.", grant.amount));
            }
            lines.push(format!(
                "Source: {} via {}, {status}, {}:{}.",
                grant.item_name, grant.grant_kind, grant.source.path, grant.source.line
            ));
        }
        if grants.len() > 10 {
            lines.push(format!("{} additional script clues omitted.", grants.len() - 10));
        }
        lines.push(
            "Source: these calls show possible grants in the script. The surrounding conditions and whether a player can trigger them are \
             not reviewed."
                .to_owned(),
        );
    }
    let uses = reference_data()
        .item_consumptions
        .iter()
        .filter(|use_ref| use_ref.item_id == item.id)
        .collect::<Vec<_>>();
    if !uses.is_empty() {
        lines.push("Used as a turn-in or ingredient:".to_owned());
        for use_ref in uses.iter().take(10) {
            if let Some(npc) = &use_ref.npc_clue {
                lines.push(format!(
                    "Used near {} on {} at ({}, {}): ×{}.",
                    npc.name, npc.map, npc.x, npc.y, use_ref.amount
                ));
                if let (Ok(x), Ok(y)) = (u16::try_from(npc.x), u16::try_from(npc.y))
                    && is_graph_map(&npc.map)
                {
                    lines.push(format!(
                        "@route-cell:{map}:{x}:{y}|Route to {} — {map}",
                        npc.name,
                        map = npc.map
                    ));
                }
            } else {
                lines.push(format!("Used: ×{}.", use_ref.amount));
            }
            lines.push(format!(
                "Source: {} at {}:{}.",
                use_ref.item_name, use_ref.source.path, use_ref.source.line
            ));
        }
        if uses.len() > 10 {
            lines.push(format!("{} additional consumption clues omitted.", uses.len() - 10));
        }
        lines.push(
            "Source: these literal removals do not prove an exchange. Rewards and conditions may be elsewhere or dynamic.".to_owned(),
        );
    }
    let exchanges = reference_data()
        .item_exchanges
        .iter()
        .filter(|exchange| {
            exchange.inputs.iter().any(|input| input.item_id == item.id)
                || exchange.outcomes.iter().any(|outcome| outcome.item_ids.contains(&item.id))
        })
        .collect::<Vec<_>>();
    if !exchanges.is_empty() {
        lines.push("Exchanges:".to_owned());
        for exchange in exchanges {
            lines.push(format!(
                "{} — @guide:npc:{}|{} at {} ({}, {})",
                exchange.title, exchange.npc.npc_id, exchange.npc.name, exchange.npc.map, exchange.npc.x, exchange.npc.y
            ));
            // `first_owned_in_list`: the script takes only the first listed item
            // the player carries, so the inputs are alternatives, not a bundle.
            let first_owned = exchange.input_selection.as_deref() == Some("first_owned_in_list");
            let cost = exchange
                .inputs
                .iter()
                .map(|input| format!("@guide:item:{}|{} ×{}", input.item_id, input.item_name, input.amount))
                .collect::<Vec<_>>()
                .join(if first_owned { " or " } else { ", " });
            if !cost.is_empty() {
                match first_owned {
                    true => lines.push(format!("Costs one of (the first listed item you carry is used): {cost}")),
                    false => lines.push(format!("Costs: {cost}")),
                }
            }
            for outcome in &exchange.outcomes {
                let rewards = outcome
                    .items
                    .iter()
                    .map(|output| format!("@guide:item:{}|{} ×{}", output.item_id, output.item_name, outcome.amount))
                    .collect::<Vec<_>>()
                    .join(" or ");
                lines.push(format!(
                    "{}: {rewards} ({})",
                    outcome.label,
                    outcome.selection.replace('_', " ")
                ));
            }
            for condition in &exchange.conditions {
                lines.push(format!("Condition: {condition}"));
            }
            lines.push(format!(
                "Source: {} lines {:?}.",
                exchange.source.path, exchange.source.reviewed_lines
            ));
            lines.push(format!(
                "Source: reviewed by {} on {}.",
                exchange.reviewed_by, exchange.reviewed_on
            ));
            if is_graph_map(&exchange.npc.map) {
                lines.push(format!(
                    "@route-cell:{}:{}:{}|Route to exchange — {}",
                    exchange.npc.map, exchange.npc.x, exchange.npc.y, exchange.npc.map
                ));
            }
        }
    }
    if !item.combos.is_empty() {
        lines.push("Item set combinations:".to_owned());
        for combo in item.combos.iter().take(8) {
            let members = combo
                .members
                .iter()
                .filter(|member| member.id != item.id)
                .map(|member| format!("@guide:item:{}|{}", member.id, display_name(&member.name, &member.aegis_name)))
                .collect::<Vec<_>>()
                .join(" + ");
            let effect_note = combo.effect_summary.as_deref().unwrap_or_else(|| {
                if combo.effect_status == "scripted_not_translated" {
                    "combo effect not described yet"
                } else {
                    "no combo effect listed"
                }
            });
            lines.push(format!("{members} — {effect_note}"));
            lines.push(format!("Source: combo {} ({}).", combo.source.path, combo.source.record));
        }
        if item.combos.len() > 8 {
            lines.push(format!("{} additional item combinations omitted.", item.combos.len() - 8));
        }
    }
    if !item.group_contents.is_empty() {
        lines.push("One of these, at random:".to_owned());
        for entry in item.group_contents.iter().take(16) {
            lines.push(format!(
                "@guide:item:{}|{} — {:.2}% (weight {}/{})",
                entry.item.id,
                display_name(&entry.item.name, &entry.item.aegis_name),
                entry.selection_chance_percent,
                entry.selection_weight,
                entry.total_weight
            ));
        }
        if item.group_contents.len() > 16 {
            lines.push(format!(
                "{} additional possible contents omitted.",
                item.group_contents.len() - 16
            ));
        }
        if let Some(entry) = item.group_contents.first() {
            lines.push(format!("Source: contents {} ({}).", entry.source.path, entry.source.record));
        }
        lines.push(
            "Source: these are configured group-selection chances. Custom item scripts may apply additional rules or grant multiple \
             results."
                .to_owned(),
        );
    }
    if !item.contained_in_groups.is_empty() {
        lines.push("Can be selected from these containers:".to_owned());
        for group in item.contained_in_groups.iter().take(8) {
            lines.push(format!(
                "@guide:item:{}|{} — {:.2}% (weight {}/{})",
                group.container.id,
                display_name(&group.container.name, &group.container.aegis_name),
                group.selection_chance_percent,
                group.selection_weight,
                group.total_weight
            ));
        }
        if item.contained_in_groups.len() > 8 {
            lines.push(format!("{} additional containers omitted.", item.contained_in_groups.len() - 8));
        }
    }
    if let Some(source) = &item.source {
        lines.push(format!("Source: {} ({})", source.path, source.record));
    }
    let related_rumors = reference_data().rumors_for_item(item.id);
    if !related_rumors.is_empty() {
        lines.push("Related local rumors:".to_owned());
        for rumor in related_rumors {
            if !rumor.is_story_spoiler {
                lines.push(format!("@guide:rumor:{}|{} ({})", rumor.id, rumor.title, rumor.category));
            }
        }
    }
    lines
}

fn refinement_details(refinement: &ReferenceRefinement, id: u32) -> Vec<String> {
    if id == 0 {
        // Armor Refinement Details
        let mut lines = vec![
            "Armor Refinement (NPC Blacksmith)".to_owned(),
            "Armor can be refined by Blacksmith NPCs (such as Hollgrehenn, Antonio, Aragham).".to_owned(),
        ];
        if let Some(armor) = &refinement.armor {
            lines.push(format!(
                "Material: 1 {} per attempt · Fee: {} Zeny.",
                armor.material, armor.cost_zeny
            ));
            lines.push(format!(
                "Safe refine limit: +{} (100% success chance up to +{}).",
                armor.safe_level, armor.safe_level
            ));
            lines.push(format!("On failure: {}.", armor.on_failure));
            lines.push("Stat bonuses: +1 DEF per level (Lv 1–4), +2 DEF per level (Lv 5–8), +3 DEF per level (Lv 9–10).".to_owned());
            let odds = (1..=refinement.max_useful_refine_level)
                .filter_map(|target| {
                    armor
                        .base_chance_percent_by_target_level
                        .get(&target.to_string())
                        .map(|chance| format!("+{target}: {chance}%"))
                })
                .collect::<Vec<_>>()
                .join(" · ");
            lines.push(format!("Success chances: {odds}"));
            let def_line = (1..=refinement.max_useful_refine_level)
                .filter_map(|target| {
                    armor
                        .def_bonus_by_target_level
                        .get(&target.to_string())
                        .map(|def| format!("+{target}: +{def} DEF"))
                })
                .collect::<Vec<_>>()
                .join(" · ");
            lines.push(format!("DEF increment: {def_line}"));
        } else {
            lines.push("Armor refine reference data is unavailable.".to_owned());
        }
        lines.push("Source: bundled Hercules refine_db.conf (Armors) and npc/merchants/refine.txt.".to_owned());
        return lines;
    }

    if (1..=4).contains(&id) {
        // Specific Weapon Level Refinement Details
        let weapon_level = id as u8;
        let mut lines = vec![format!("Weapon Level {weapon_level} Refinement")];
        if let Some(weapon) = refinement.weapon_levels.iter().find(|w| w.weapon_level == weapon_level) {
            lines.push(format!(
                "NPC Blacksmith: consumes 1 {} and {} Zeny. Safe limit: +{} (100% chance).",
                weapon.material, weapon.cost_zeny, weapon.safe_level
            ));
            lines.push(format!(
                "Stat bonus: +{} ATK & MATK per refine level. Past +{}, adds a random ATK bonus of 0 to (Lv - {}) x {}.",
                weapon.stat_per_level,
                weapon.random_bonus_start_level.saturating_sub(1),
                weapon.random_bonus_start_level.saturating_sub(1),
                weapon.random_bonus_max_per_level
            ));
            lines.push(format!("On failure: {}.", refinement.on_failure));
            let odds = (1..=refinement.max_useful_refine_level)
                .filter_map(|target| {
                    weapon
                        .base_chance_percent_by_target_level
                        .get(&target.to_string())
                        .map(|chance| format!("+{target}: {chance}%"))
                })
                .collect::<Vec<_>>()
                .join(" · ");
            lines.push(format!("Base success chances: {odds}"));
            lines.push("Whitesmith Weapon Refine (WS_WEAPONREFINE):".to_owned());
            lines.push(format!(
                "Consumes 1 {} with 0 Zeny cost. Adds +{}% chance per job level relative to 50 (+{}% for Mechanic Transcendent).",
                weapon.material,
                refinement.job_level_bonus_per_job_level_from_50_per_mille as f32 / 10.0,
                refinement.mechanic_transcendent_flat_bonus_percent
            ));
            lines.push("Whitesmith self-refine has no safe limit: even +1 can fail if the caster's job level is low.".to_owned());
        }
        lines.push("Source: bundled Hercules refine_db.conf, npc/merchants/refine.txt, and skill_weaponrefine.".to_owned());
        return lines;
    }

    // General Weapon Refine / Full Overview
    let mut lines = vec![
        "Weapon Refinement (WS_WEAPONREFINE & NPC Blacksmith)".to_owned(),
        "Weapons can be refined at NPC Blacksmiths or via the Whitesmith WS_WEAPONREFINE skill.".to_owned(),
        "NPC Blacksmith safe limits: Weapon Lv 1 +7; Lv 2 +6; Lv 3 +5; Lv 4 +4. Refining past the safe limit risks item destruction."
            .to_owned(),
        format!("On failure: {}.", refinement.on_failure),
        format!(
            "Whitesmith self-refine bonus: {} percentage points per job level relative to 50 (or +{} points for Mechanic Transcendent).",
            refinement.job_level_bonus_per_job_level_from_50_per_mille as f32 / 10.0,
            refinement.mechanic_transcendent_flat_bonus_percent
        ),
        format!(
            "A weapon can be refined with WS_WEAPONREFINE only below the skill level and below +{}.",
            refinement.max_useful_refine_level
        ),
        "No Zeny cost is charged by WS_WEAPONREFINE; one ore material is consumed per attempt.".to_owned(),
    ];
    for weapon in &refinement.weapon_levels {
        lines.push(format!(
            "Weapon Level {} — material: {}, fee: {} Zeny, safe limit: +{}, stat: +{} ATK/MATK per level",
            weapon.weapon_level, weapon.material, weapon.cost_zeny, weapon.safe_level, weapon.stat_per_level
        ));
        let odds = (1..=refinement.max_useful_refine_level)
            .filter_map(|target| {
                weapon
                    .base_chance_percent_by_target_level
                    .get(&target.to_string())
                    .map(|chance| format!("+{target}: {chance}%"))
            })
            .collect::<Vec<_>>()
            .join(" · ");
        lines.push(odds);
    }
    if let Some(armor) = &refinement.armor {
        lines.push(format!(
            "Armor — material: {}, fee: {} Zeny, safe limit: +{}, stat: +1 to +3 DEF per level",
            armor.material, armor.cost_zeny, armor.safe_level
        ));
        let odds = (1..=refinement.max_useful_refine_level)
            .filter_map(|target| {
                armor
                    .base_chance_percent_by_target_level
                    .get(&target.to_string())
                    .map(|chance| format!("+{target}: {chance}%"))
            })
            .collect::<Vec<_>>()
            .join(" · ");
        lines.push(odds);
    }
    lines.push("Source: bundled Hercules refine_db.conf, npc/merchants/refine.txt, and skill_weaponrefine implementation.".to_owned());
    lines
}

fn refinement_query_matches(query: &str) -> bool {
    let query = query.to_lowercase();
    query.is_empty()
        || [
            "refine",
            "weapon",
            "armor",
            "phracon",
            "emveretarcon",
            "oridecon",
            "elunium",
            "whitesmith",
            "mechanic",
            "hollgrehenn",
            "blacksmith",
            "safe limit",
        ]
        .iter()
        .any(|term| term.contains(&query) || query.contains(term))
}

fn server_rule_matches(rule: &crate::dm::reference_data::ReferenceServerRule, query: &str) -> bool {
    let query = query.to_lowercase();
    query.is_empty()
        || rule.title.to_lowercase().contains(&query)
        || rule.summary.to_lowercase().contains(&query)
        || rule.category.to_lowercase().contains(&query)
        || rule.details.iter().any(|detail| detail.to_lowercase().contains(&query))
}

fn server_rule_details(rule: &crate::dm::reference_data::ReferenceServerRule) -> Vec<String> {
    let mut lines = vec![rule.title.clone(), rule.summary.clone()];
    lines.extend(rule.details.iter().cloned());
    lines.push("Source configuration:".to_owned());
    lines.extend(rule.sources.iter().map(|source| format!("{} ({})", source.path, source.record)));
    lines.push(data_provenance_line());
    lines.push("Values are exported from this Hercules source revision and can change with server configuration.".to_owned());
    lines
}

/// Which server revision the Guide's numbers were read from, so a player (or
/// the DM) can tell when they might be out of date. The Guide never reads the
/// live server.
fn data_provenance_line() -> String {
    let data = reference_data();
    let revision: String = data.source_revision.chars().take(10).collect();
    let tree = if data.source_worktree_dirty {
        "the server had uncommitted changes at export"
    } else {
        "the server tree was committed at export"
    };
    format!("Exported from Hercules {revision} ({} mode); {tree}.", data.mode)
}

fn coverage_details() -> Vec<String> {
    let report = &reference_data().coverage_report;
    let mut lines = vec![
        "Encyclopedia coverage counts — these measure exported data, not completion of player explanations.".to_owned(),
        format!("Source revision: {} ({})", report.source_revision, report.mode),
    ];
    let mut categories = report.categories.iter().collect::<Vec<_>>();
    categories.sort_by_key(|(name, _)| *name);
    for (category, fields) in categories {
        lines.push(format!("{}:", category.replace('_', " ")));
        let Some(fields) = fields.as_object() else { continue };
        let mut fields = fields.iter().collect::<Vec<_>>();
        fields.sort_by_key(|(name, _)| *name);
        for (name, value) in fields {
            if let (Some(observed), Some(total)) = (
                value.get("observed").and_then(serde_json::Value::as_u64),
                value.get("total").and_then(serde_json::Value::as_u64),
            ) {
                lines.push(format!("  {}: {observed}/{total}", name.replace('_', " ")));
            } else if let Some(count) = value.as_u64() {
                lines.push(format!("  {}: {count}", name.replace('_', " ")));
            } else if let Some(status) = value.get("evidence_state").and_then(serde_json::Value::as_str) {
                lines.push(format!("  {}: {status}", name.replace('_', " ")));
            }
        }
    }
    lines.push(format!("Counting rule: {}", report.counting_policy));
    let state_label = |state| match state {
        crate::dm::reference_data::EvidenceState::Verified => "verified",
        crate::dm::reference_data::EvidenceState::Conditional => "conditional",
        crate::dm::reference_data::EvidenceState::ConfiguredEstimate => "configured estimate",
        crate::dm::reference_data::EvidenceState::SourceClue => "source clue",
        crate::dm::reference_data::EvidenceState::NotReviewed => "not reviewed",
        crate::dm::reference_data::EvidenceState::Unknown => "unknown",
    };
    let mut evidence_states = report.evidence_states.iter().collect::<Vec<_>>();
    evidence_states.sort_by_key(|(state, _)| state_label(**state));
    lines.push("Evidence labels:".to_owned());
    lines.extend(
        evidence_states
            .into_iter()
            .map(|(state, description)| format!("  {} — {description}", state_label(*state))),
    );
    lines
}

/// A source reference the open player Guide may show. A campaign script's path
/// names its arc and act, which is plot, so it is withheld; the fact it
/// established (a map flag, say) is still shown.
fn player_source(path: &str, line: u32) -> String {
    match crate::dm::reference_data::is_campaign_source_path(path) {
        true => "campaign event".to_owned(),
        false => format!("{path}:{line}"),
    }
}

fn is_graph_map(map_name: &str) -> bool {
    crate::world::navigation_graph()
        .maps
        .iter()
        .any(|known| known.eq_ignore_ascii_case(map_name))
}

/// The server's item type constant in words ("IT_HEALING" -> "Healing").
fn item_type_label(item_type: &str) -> String {
    match item_type {
        "IT_HEALING" => "Healing".to_owned(),
        "IT_USABLE" => "Usable".to_owned(),
        // Hercules defaults an item_db record with no `Type` to IT_ETC.
        "IT_ETC" | "" => "Miscellaneous".to_owned(),
        "IT_WEAPON" => "Weapon".to_owned(),
        "IT_ARMOR" => "Armor".to_owned(),
        "IT_CARD" | "Card" => "Card".to_owned(),
        "IT_PETEGG" => "Pet egg".to_owned(),
        "IT_PETARMOR" => "Pet accessory".to_owned(),
        "IT_AMMO" => "Ammunition".to_owned(),
        "IT_DELAYCONSUME" => "Usable (delayed)".to_owned(),
        "IT_CASH" => "Cash shop item".to_owned(),
        other => other.strip_prefix("IT_").unwrap_or(other).replace('_', " ").to_lowercase(),
    }
}

fn display_name(name: &str, fallback: &str) -> String {
    if name.is_empty() { fallback.to_owned() } else { name.to_owned() }
}

/// Player label for an NPC script declaration. The raw token stays in the data.
fn npc_kind_label(declared_type: &str) -> &'static str {
    match declared_type {
        "shop" => "Shop",
        "cashshop" => "Cash shop",
        "trader" => "Trader",
        "warp" => "Warp",
        _ => "NPC",
    }
}

/// `kafra_suite` is an export token. The page shows the words.
fn service_kind_words(kind: &str) -> String {
    kind.replace('_', " ")
}

/// The name a player sees. The client draws NPC names without the `#` suffix.
fn talk_name(internal_name: &str) -> &str {
    internal_name
        .split('#')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(internal_name)
}

fn quest_details(quest: &crate::state::quests::QuestEntry) -> Vec<String> {
    let data = reference_data();
    let mut lines = vec![quest.name().to_owned()];
    if quest.hunt_objectives().is_empty() && quest.requirements().is_empty() {
        lines.push("No objective details are available from the server or bundled campaign data.".to_owned());
    }
    for objective in quest.hunt_objectives() {
        lines.push(format!(
            "@guide:monster:{}|Hunt {} — {}/{}",
            objective.monster_id,
            display_name(&objective.monster_name, &format!("Monster {}", objective.monster_id)),
            objective.current_count,
            objective.total_count,
        ));
        if let Some(monster) = data.monster_by_id(objective.monster_id) {
            let mut route_count = 0;
            for region in monster.spawn_regions.iter().filter(|region| is_graph_map(&region.map)).take(8) {
                lines.push(format!("@route:{}", region.map));
                route_count += 1;
            }
            if route_count == 0 {
                lines.push("No loaded static spawn-map route is known for this objective.".to_owned());
            }
        }
    }
    for requirement in quest.requirements() {
        lines.push(format!(
            "@guide:item:{}|Collect {} — {}/?",
            requirement.item_id.0, requirement.item_name, requirement.needed
        ));
    }
    if let Some(reference) = data.quest_by_id(quest.quest_id) {
        lines.extend(quest_reference_details(reference));
    }
    lines
}

fn quest_reference_details(quest: &crate::dm::reference_data::ReferenceQuest) -> Vec<String> {
    let data = reference_data();
    let story = crate::world::newbie_quest_guide(quest.id);
    let mut lines = vec![quest.name.clone()];
    if let Some(story) = story {
        let role = if story.is_turn_in { "Turn in to" } else { "Go to" };
        lines.push(format!("{role}: {} on {} at ({}, {}).", story.npc, story.map, story.x, story.y));
        lines.extend(story.steps.iter().map(|step| (*step).to_owned()));
        lines.push(format!(
            "@route-cell:{}:{}:{}|Route to {} — {}",
            story.map, story.x, story.y, story.npc, story.map
        ));
        lines.push("Source: npc/re/jobs/novice/academy.txt.".to_owned());
    }
    if let Some(flow) = &quest.flow_review {
        if !flow.title.is_empty() {
            lines.push(flow.title.clone());
        }
        for condition in &flow.conditions {
            lines.push(format!("Requirement: {condition}"));
        }
        for reward in &flow.verified_item_rewards {
            lines.push(format!(
                "@guide:item:{}|Reward: {} ×{}",
                reward.item_id, reward.item_name, reward.amount
            ));
            if !reward.explanation.is_empty() {
                lines.push(reward.explanation.clone());
            }
            lines.push(format!(
                "Source: {} ×{} at {}:{} ({}).",
                reward.item_name,
                reward.amount,
                reward.source_path,
                reward.source_line,
                reward.evidence_state.label()
            ));
        }
        for source in &flow.sources {
            lines.push(format!("Source: {} lines {:?}.", source.path, source.lines));
        }
        lines.push(format!(
            "Source: reviewed by {} on {}. Method: {} ({}).",
            flow.reviewed_by,
            flow.reviewed_on,
            flow.review_method,
            flow.evidence_state.label()
        ));
    }
    if story.is_none() && quest.targets.is_empty() {
        lines.push("No hunt target is listed.".to_owned());
    }
    if story.is_none() && quest.npc_references.is_empty() {
        lines.push("No quest giver or turn-in is listed.".to_owned());
    }
    for target in &quest.targets {
        let title = format!("{} × {}", target.monster_name, target.count);
        lines.push(format!("Objective: {title}"));
        if let Some(level) = target.level_range {
            lines.push(format!("Level {}–{}.", level[0], level[1]));
        }
        let map_has_route = target.map_name.as_ref().is_some_and(|map| is_graph_map(map));
        if let Some(map) = &target.map_name {
            if map_has_route {
                lines.push(format!("@route:{map}"));
            } else {
                lines.push(format!("Target map {map} has no loaded navigation route."));
            }
        }
        if !map_has_route && let Some(mob_id) = target.mob_id {
            if target.monster_data_known == Some(true) {
                lines.push(format!(
                    "@guide:monster:{mob_id}|View {} and known spawn routes",
                    target.monster_name
                ));
            } else {
                lines.push("This monster has no Guide entry yet.".to_owned());
            }
        }
    }
    let unreviewed_reward_candidates = quest
        .item_reward_candidates
        .iter()
        .filter(|candidate| {
            !quest.flow_review.as_ref().is_some_and(|flow| {
                flow.verified_item_rewards.iter().any(|reward| {
                    reward.item_id == candidate.item_id
                        && reward.amount == candidate.amount
                        && reward.source_path == candidate.source_path
                        && reward.source_line == candidate.source_line
                })
            })
        })
        .collect::<Vec<_>>();
    if !unreviewed_reward_candidates.is_empty() {
        lines.push("Possible reward, not confirmed:".to_owned());
        for candidate in unreviewed_reward_candidates.iter().take(16) {
            lines.push(format!(
                "@guide:item:{}|{} ×{}",
                candidate.item_id, candidate.item_name, candidate.amount
            ));
            lines.push(format!(
                "Source: {}:{}; a completequest call is {} lines away, at line {}.",
                candidate.source_path, candidate.source_line, candidate.distance_lines, candidate.nearby_completequest_line
            ));
        }
        if unreviewed_reward_candidates.len() > 16 {
            lines.push(format!(
                "{} additional candidate grants omitted.",
                unreviewed_reward_candidates.len() - 16
            ));
        }
    }
    for npc in quest.npc_references.iter().take(8) {
        let role = match npc.reviewed_role.as_deref() {
            Some("offer") => "Offered by",
            Some("turn_in") => "Turn in to",
            _ => "Mentioned near",
        };
        lines.push(format!("{role} {} on {} at ({}, {}).", npc.name, npc.map_name, npc.x, npc.y));
        if npc.reviewed_role.is_some() {
            lines.push(format!(
                "Source: {} source lines {:?}.",
                npc.source_path, npc.reviewed_source_lines
            ));
            if let Some(evidence) = &npc.review_evidence {
                lines.push("Requirements or availability may still apply.".to_owned());
                lines.push(format!("Source: {evidence}"));
            }
            if let Some(reward) = &npc.verified_reward {
                let item_name = data
                    .item_by_id(reward.item_id)
                    .map(|item| display_name(&item.name, &item.aegis_name))
                    .unwrap_or_else(|| format!("Item {}", reward.item_id));
                lines.push(format!(
                    "Reward: {} base EXP and @guide:item:{}|{} ×{}",
                    reward.base_exp, reward.item_id, item_name, reward.item_amount
                ));
            }
        } else {
            lines.push(format!("Source: {}:{}.", npc.source_path, npc.source_line));
        }
        if !npc.uses.is_empty() {
            let clues = npc
                .uses
                .iter()
                .map(|usage| match usage.as_str() {
                    "setquest" => "sets quest state",
                    "questprogress" => "checks or advances quest progress",
                    "completequest" => "completes quest state",
                    "erasequest" => "removes quest state",
                    "checkquest" => "checks a quest-state prerequisite or progress condition",
                    "que_dic" => "calls shared hunting-request completion logic",
                    _ => "uses quest state",
                })
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "Script call evidence: {clues}; this does not prove NPC role or current availability."
            ));
        }
        if is_graph_map(&npc.map_name) {
            let route_label = match npc.reviewed_role.as_deref() {
                Some("offer") => "Route to quest offer",
                Some("turn_in") => "Route to quest turn-in",
                _ => "Route to related NPC",
            };
            lines.push(format!(
                "@route-cell:{}:{}:{}|{route_label}: {} — {}",
                npc.map_name, npc.x, npc.y, npc.name, npc.map_name
            ));
        } else {
            lines.push(format!("{} is not currently in the loaded navigation graph.", npc.map_name));
        }
    }
    if quest.npc_references.len() > 8 {
        lines.push(format!(
            "Source: {} additional related NPCs omitted.",
            quest.npc_references.len() - 8
        ));
    }
    if story.is_none() {
        lines.push("Source: steps, requirements, and rewards are only what the loaded scripts record.".to_owned());
    }
    lines
}

fn map_details(map_name: &str, town_pois: &[TownPoi]) -> Vec<String> {
    let graph = crate::world::navigation_graph();
    let reference = reference_data();
    let is_template = scripted_map_templates().iter().any(|name| name.eq_ignore_ascii_case(map_name));
    let mut lines = vec![if is_template {
        format!("Instance map template: {map_name}")
    } else {
        format!("Map: {map_name}")
    }];
    let mut configured_flags = reference
        .map_flags
        .iter()
        .filter(|entry| entry.map.eq_ignore_ascii_case(map_name) && entry.effective_static)
        .collect::<Vec<_>>();
    configured_flags.sort_by_key(|entry| {
        (
            entry.flag.to_lowercase(),
            entry.value.to_lowercase(),
            entry.source.path.clone(),
            entry.source.line,
        )
    });
    configured_flags.dedup_by(|left, right| left.flag.eq_ignore_ascii_case(&right.flag) && left.value.eq_ignore_ascii_case(&right.value));
    if !configured_flags.is_empty() {
        lines.push("Map rules:".to_owned());
        for entry in configured_flags.iter().take(24) {
            let shown = if entry.description.is_empty() {
                if entry.value.is_empty() {
                    entry.flag.clone()
                } else {
                    format!("{} ({})", entry.flag, entry.value)
                }
            } else if entry.value.is_empty() {
                entry.description.clone()
            } else {
                format!("{} ({})", entry.description, entry.value)
            };
            lines.push(shown);
            lines.push(format!(
                "Source: {} at {}.",
                entry.flag,
                player_source(&entry.source.path, entry.source.line)
            ));
            if let Some(overridden) = entry.overridden_directive_count {
                lines.push(format!(
                    "{} earlier loaded-script directive(s) for this flag are superseded.",
                    overridden
                ));
            }
        }
        if configured_flags.len() > 24 {
            lines.push(format!("{} additional map-flag entries omitted.", configured_flags.len() - 24));
        }
        lines.push("Scripts can still change these rules during play.".to_owned());
    }
    for review in reference
        .runtime_map_flag_reviews
        .iter()
        .filter(|review| review.map.eq_ignore_ascii_case(map_name))
    {
        lines.push(review.title.clone());
        lines.push(review.summary.clone());
        for condition in &review.conditions {
            lines.push(format!("Condition: {condition}"));
        }
        for source in &review.sources {
            lines.push(format!("Source: {} lines {:?}.", source.path, source.lines));
        }
        lines.push(format!(
            "Source: reviewed by {} on {} ({}).",
            review.reviewed_by,
            review.reviewed_on,
            review.evidence_state.label()
        ));
    }
    let mut runtime_flags = reference
        .runtime_map_flag_clues
        .iter()
        .filter(|clue| clue.map.as_deref().is_some_and(|map| map.eq_ignore_ascii_case(map_name)))
        .collect::<Vec<_>>();
    runtime_flags.sort_by_key(|clue| (clue.flag.to_lowercase(), clue.source.path.clone(), clue.source.line));
    if !runtime_flags.is_empty() {
        lines.push("Scripts can change these rules:".to_owned());
        for clue in runtime_flags.iter().take(16) {
            let shown = if clue.value.is_empty() {
                clue.flag.clone()
            } else {
                format!("{} ({})", clue.flag, clue.value)
            };
            lines.push(shown);
            lines.push(format!(
                "Source: {}({}, {}) at {}:{}.",
                clue.operation, clue.map_expression, clue.flag, clue.source.path, clue.source.line
            ));
        }
        if runtime_flags.len() > 16 {
            lines.push(format!(
                "{} additional runtime map-flag clues omitted.",
                runtime_flags.len() - 16
            ));
        }
        lines.push("Source: a script call does not prove that the event runs.".to_owned());
    }
    match reference.map_spawn_details(map_name) {
        Some(details) => {
            let range_str = if details.min_level != details.max_level {
                format!(" (range: {}–{})", details.min_level, details.max_level)
            } else {
                String::new()
            };
            lines.push(format!("Suggested level: ~{}{range_str}", details.mean_level));
            lines.push(format!(
                "Static population: {} kinds of monster in {} spawn groups",
                details.species, details.records
            ));
        }
        None => {
            lines.push(if is_template {
                "No base-map static population is attached to this instance template.".to_owned()
            } else {
                "Suggested level: not listed.".to_owned()
            });
            if !is_template {
                lines.push("Static population: none listed.".to_owned());
            }
        }
    }

    let mut static_mobs = reference
        .monsters
        .iter()
        .filter(|monster| monster.spawn_regions.iter().any(|region| region.map.eq_ignore_ascii_case(map_name)))
        .collect::<Vec<_>>();
    static_mobs.sort_by_key(|monster| (monster.name.to_lowercase(), monster.id));
    if !static_mobs.is_empty() {
        lines.push(format!("Monsters that spawn here ({} kinds):", static_mobs.len()));
        for monster in &static_mobs {
            lines.push(format!(
                "@guide:monster:{}|{}",
                monster.id,
                display_name(&monster.name, &monster.sprite_name)
            ));
        }
    }

    let mut scripted_mobs = reference
        .monsters
        .iter()
        .filter_map(|monster| {
            let call_sites = monster
                .scripted_spawn_references
                .iter()
                .filter(|spawn| {
                    spawn.map.as_deref().is_some_and(|map| map.eq_ignore_ascii_case(map_name))
                        || spawn.map_template.as_deref().is_some_and(|map| map.eq_ignore_ascii_case(map_name))
                })
                .count();
            (call_sites > 0).then_some((monster, call_sites))
        })
        .collect::<Vec<_>>();
    scripted_mobs.sort_by_key(|(monster, _)| (monster.name.to_lowercase(), monster.id));
    if !scripted_mobs.is_empty() {
        lines.push(format!(
            "Also placed by scripts ({} kinds; the trigger is not listed):",
            scripted_mobs.len()
        ));
        for (monster, call_sites) in &scripted_mobs {
            lines.push(format!(
                "@guide:monster:{}|{}",
                monster.id,
                display_name(&monster.name, &monster.sprite_name)
            ));
            lines.push(format!("Source: {call_sites} script placements."));
        }
    } else {
        lines.push("No script places a monster on this map.".to_owned());
    }
    lines.push("These are configured spawns, not how many are alive right now.".to_owned());
    lines.push("Source: a script that picks the map while it runs may not be listed on this map.".to_owned());

    let mut exits: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "walk_warp" && edge.from.map.eq_ignore_ascii_case(map_name))
        .collect();
    exits.sort_by_key(|edge| (edge.to.map.to_ascii_lowercase(), edge.from.x, edge.from.y, edge.to.x, edge.to.y));
    lines.push(format!("Exits: {}", exits.len()));
    for edge in exits.iter().take(8) {
        lines.push(format!("Exit at ({}, {}) to {}", edge.from.x, edge.from.y, edge.to.map));
        lines.push(format!("@route:{}", edge.to.map));
    }
    if exits.len() > 8 {
        lines.push(format!("{} additional exits omitted.", exits.len() - 8));
    }
    let mut services: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "npc_service" && edge.from.map.eq_ignore_ascii_case(map_name))
        .collect();
    services.sort_by_key(|edge| (edge.to.map.to_ascii_lowercase(), edge.from.x, edge.from.y));
    if !services.is_empty() {
        lines.push(format!("Travel services: {}", services.len()));
        for edge in services {
            let action = edge.action.as_deref().unwrap_or("Talk to the listed NPC.");
            let availability = if edge.availability == "conditional" { " (conditional)" } else { "" };
            lines.push(format!(
                "Service at ({}, {}): {action}{availability} › {}",
                edge.from.x, edge.from.y, edge.to.map
            ));
            if let Some(requirements) = &edge.requirements {
                lines.push(format!("Requirement: {requirements}"));
            }
            if let Some(source) = &edge.source {
                lines.push(format!("Source: {source}."));
            }
            lines.push(format!("@route:{}", edge.to.map));
        }
    }
    let mut routeable_poi_count = 0;
    for poi in town_pois {
        let (Ok(x), Ok(y)) = (u16::try_from(poi.x), u16::try_from(poi.y)) else {
            continue;
        };
        if routeable_poi_count == 8 {
            break;
        }
        lines.push(format!("{} at ({x}, {y})", poi.name));
        lines.push(format!("@route-cell:{map_name}:{x}:{y}|Route to {} — {map_name}", poi.name));
        routeable_poi_count += 1;
    }
    let valid_poi_count = town_pois
        .iter()
        .filter(|poi| u16::try_from(poi.x).is_ok() && u16::try_from(poi.y).is_ok())
        .count();
    if valid_poi_count > routeable_poi_count {
        lines.push(format!(
            "Source: {} additional facility routes omitted.",
            valid_poi_count - routeable_poi_count
        ));
    }
    if town_pois.is_empty() {
        lines.push("No facilities are marked on this map.".to_owned());
    }
    let map_services = reference.services_for_map(map_name);
    if !map_services.is_empty() {
        lines.push(format!("Services here ({}):", map_services.len()));
        for service in map_services {
            if let Some(index) = reference.npc_service_index_by_id(&service.id) {
                lines.push(format!(
                    "@guide:service:{index}|{} ({})",
                    service.title,
                    service_kind_words(&service.service_kind)
                ));
            }
        }
    }
    let map_rumors = reference.rumors_for_map(map_name);
    if !map_rumors.is_empty() {
        lines.push(format!("Local rumors ({}):", map_rumors.len()));
        for rumor in map_rumors {
            if !rumor.is_story_spoiler {
                lines.push(format!("@guide:rumor:{}|{} ({})", rumor.id, rumor.title, rumor.category));
            }
        }
    }
    let chests = crate::world::hidden_chests_for_map(map_name);
    if !chests.is_empty() {
        lines.push(format!(
            "Hidden treasure chests ({}): each appears when you walk close and opens once per character. This list cannot show which you \
             have opened.",
            chests.len()
        ));
        for chest in chests {
            lines.push(format!(
                "@route-cell:{map_name}:{}:{}|Route to hidden chest — {map_name} ({}, {})",
                chest.x, chest.y, chest.x, chest.y
            ));
        }
    }
    lines.push("Spawn rosters are configured data and script clues; they do not represent live monster counts.".to_owned());
    if is_graph_map(map_name) {
        lines.push(format!("@route:{map_name}"));
    } else if is_template {
        lines.push("Instance template: the runtime map name and route depend on the created instance.".to_owned());
    }
    lines
}

fn scripted_map_templates() -> Vec<String> {
    let mut templates = std::collections::BTreeSet::new();
    for monster in &reference_data().monsters {
        templates.extend(
            monster
                .scripted_spawn_references
                .iter()
                .filter_map(|spawn| spawn.map_template.clone()),
        );
    }
    templates.into_iter().collect()
}

fn parse_route_cell_link(line: &str) -> Option<(String, u16, u16, String)> {
    let route = line.strip_prefix("@route-cell:")?;
    let (destination, label) = route.split_once('|')?;
    let mut parts = destination.rsplitn(3, ':');
    let y = parts.next()?.parse().ok()?;
    let x = parts.next()?.parse().ok()?;
    let map_name = parts.next()?;
    Some((map_name.to_owned(), x, y, label.to_owned()))
}

fn npc_details(npc: &ReferenceNpc) -> Vec<String> {
    let display = display_name(&npc.display_name, &npc.name);
    let mut lines = vec![
        display.clone(),
        format!("Type: {}.", npc_kind_label(&npc.declared_type)),
        format!("On {} at ({}, {}).", npc.map, npc.x, npc.y),
    ];
    if !npc.internal_name.is_empty() {
        lines.push(format!("Internal script name: {}", npc.internal_name));
    }
    if !npc.sprite.is_empty() && npc.declared_type != "warp" {
        lines.push(format!("Source: sprite {}.", npc.sprite));
    }
    lines.push(format!("Source: {}:{}.", npc.source.path, npc.source.line));
    for exchange in reference_data()
        .item_exchanges
        .iter()
        .filter(|exchange| exchange.npc.npc_id == npc.id)
    {
        let role = exchange.npc.service_role.as_str();
        if role.contains(' ') {
            lines.push(format!("Service: {role}."));
        } else {
            lines.push(format!("Source: service role {role}."));
        }
        lines.push(format!("Exchange: {}.", exchange.title));
        let costs = exchange
            .inputs
            .iter()
            .map(|input| format!("@guide:item:{}|{} ×{}", input.item_id, input.item_name, input.amount))
            .collect::<Vec<_>>()
            .join(", ");
        if !costs.is_empty() {
            lines.push(format!("Costs: {costs}"));
        }
        for outcome in &exchange.outcomes {
            let rewards = outcome
                .items
                .iter()
                .map(|item| format!("@guide:item:{}|{} ×{}", item.item_id, item.item_name, outcome.amount))
                .collect::<Vec<_>>()
                .join(" or ");
            lines.push(format!("{}: {rewards}", outcome.label));
        }
        for condition in &exchange.conditions {
            lines.push(format!("Condition: {condition}"));
        }
        lines.push(format!(
            "Source: {} lines {:?}.",
            exchange.source.path, exchange.source.reviewed_lines
        ));
        lines.push(format!(
            "Source: reviewed by {} on {} ({}).",
            exchange.reviewed_by,
            exchange.reviewed_on,
            exchange.evidence_state.label()
        ));
    }
    if is_graph_map(&npc.map) {
        if let (Ok(x), Ok(y)) = (u16::try_from(npc.x), u16::try_from(npc.y)) {
            lines.push(format!(
                "@route-cell:{}:{}:{}|Route to {} — {}",
                npc.map, x, y, display, npc.map
            ));
        } else {
            lines.push(format!("@route:{}", npc.map));
        }
    } else {
        lines.push("This map is not in the current navigation graph, so no route action is available.".to_owned());
    }
    if matches!(npc.declared_type.as_str(), "shop" | "cashshop" | "trader") {
        if npc.offers.is_empty() {
            lines.push("No listed stock. The shop may still sell other things.".to_owned());
        } else {
            lines.push("Sells:".to_owned());
            for offer in npc.offers.iter().take(24) {
                let price = if offer.uses_item_db_price {
                    format!("the item database price ({})", offer.currency)
                } else {
                    format!("{} {}", offer.price.unwrap_or_default(), offer.currency)
                };
                lines.push(format!("@guide:item:{}|{} — {price}", offer.item_id, offer.item_name));
            }
            let mut seen_sources = Vec::new();
            for offer in npc.offers.iter().take(24) {
                if !offer.source.is_empty() && !seen_sources.contains(&offer.source.as_str()) {
                    seen_sources.push(offer.source.as_str());
                    lines.push(format!("Source: {}.", offer.source));
                }
            }
            if npc.offers.len() > 24 {
                lines.push(format!("Source: {} additional offers omitted.", npc.offers.len() - 24));
            }
        }
        lines.push("Source: only shops whose stock is written out in a script are listed.".to_owned());
    }
    for service in reference_data()
        .npc_services
        .iter()
        .filter(|s| s.npc.as_ref().is_some_and(|n| n.npc_id == npc.id))
    {
        if let Some(index) = reference_data().npc_service_index_by_id(&service.id) {
            lines.push(format!(
                "@guide:service:{index}|{} ({})",
                service.title,
                service_kind_words(&service.service_kind)
            ));
        }
    }
    lines
}

fn service_details(service: &ReferenceNpcServiceReview) -> Vec<String> {
    let mut lines = vec![
        service.title.clone(),
        format!("Service: {}.", service_kind_words(&service.service_kind)),
        format!(
            "Source: reviewed by {} on {}. Method: {} ({}).",
            service.reviewed_by,
            service.reviewed_on,
            service.review_method,
            service.evidence_state.label()
        ),
    ];
    if let Some(npc) = &service.npc {
        let name = talk_name(&npc.internal_name);
        lines.push(format!("Talk to {name} on {} at ({}, {}).", npc.map, npc.x, npc.y));
        lines.push(format!("Internal script name: {}.", npc.internal_name));
        lines.push(format!(
            "@route-cell:{}:{}:{}|Route to {} ({}, {})",
            npc.map, npc.x, npc.y, name, npc.x, npc.y
        ));
        lines.push(format!("@guide:npc:{}|{name}", npc.npc_id));
        if is_graph_map(&npc.map) {
            lines.push(format!("@route:{}", npc.map));
        }
    }
    if !service.conditions.is_empty() {
        lines.push("Requirements:".to_owned());
        for condition in &service.conditions {
            lines.push(condition.clone());
        }
    }
    if let Some(route_table) = &service.route_table {
        lines.push("Kafra routes:".to_owned());
        for route in route_table {
            lines.push(format!("From {}:", route.origin_map));
            for dest in &route.destinations {
                lines.push(format!("  {} — Fee: {} Zeny", dest.name, dest.fee));
            }
            if is_graph_map(&route.origin_map) {
                lines.push(format!("@route:{}|Route to origin {}", route.origin_map, route.origin_map));
            }
        }
    }
    if let Some(note) = &service.route_table_note {
        lines.push(format!("Route note: {note}"));
    }
    if let Some(count) = service.location_count {
        lines.push(format!("Found in {count} places."));
    }
    for source in &service.sources {
        let cited = source.lines.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
        lines.push(format!("Source: {}:{cited}.", source.path));
    }
    lines
}

fn rumor_details(rumor: &ReferenceRumor) -> Vec<String> {
    let mut lines = vec![
        format!("{}  (Rumor: {})", rumor.title, rumor.category),
        format!("Evidence state: {}", rumor.evidence_state.label()),
        format!("\"{}\"", rumor.text),
        format!("Source: {}", rumor.source_location),
    ];
    if let (Some(map), Some([x, y])) = (&rumor.map_name, rumor.coordinates) {
        lines.push(format!("Map location: {map} at ({x}, {y})"));
        lines.push(format!("@route-cell:{map}:{x}:{y}|Route to {map} ({x}, {y})"));
        if is_graph_map(map) {
            lines.push(format!("@route:{map}"));
        }
    } else if let Some(map) = &rumor.map_name {
        lines.push(format!("Map: {map}"));
        if is_graph_map(map) {
            lines.push(format!("@route:{map}"));
        }
    }
    let data = reference_data();
    if let Some(monster_id) = rumor.related_monster_id {
        let monster_name = data
            .monster_by_id(monster_id)
            .map(|m| display_name(&m.name, &m.sprite_name))
            .unwrap_or_else(|| "Unknown monster".to_owned());
        lines.push(format!(
            "@guide:monster:{monster_id}|Related monster: {monster_name} (ID {monster_id})"
        ));
    }
    if let Some(item_id) = rumor.related_item_id {
        let item_name = data
            .item_by_id(item_id)
            .map(|i| display_name(&i.name, &i.aegis_name))
            .unwrap_or_else(|| "Unknown item".to_owned());
        lines.push(format!("@guide:item:{item_id}|Related item: {item_name} (ID {item_id})"));
    }
    if rumor.is_story_spoiler {
        lines.push("Classification: Story spoiler (contains main narrative elements).".to_owned());
    } else {
        lines.push("Classification: Non-story world lore / local rumor.".to_owned());
    }
    lines
}

fn resolve_details(result: &GuideResult) -> Vec<String> {
    let data = reference_data();
    match result.kind.as_str() {
        "monster" => data
            .monster_by_id(result.id)
            .map(monster_details)
            .unwrap_or_else(|| vec!["Reference entry unavailable.".to_owned()]),
        "item" => data
            .item_by_id(result.id)
            .or_else(|| data.card_by_id(result.id))
            .map(|item| item_details(item, data.card_by_id(result.id).is_some()))
            .unwrap_or_else(|| vec!["Reference entry unavailable.".to_owned()]),
        "card" => data
            .card_by_id(result.id)
            .map(|item| item_details(item, true))
            .unwrap_or_else(|| vec!["Reference entry unavailable.".to_owned()]),
        "skill" => data
            .skill_by_id(result.id)
            .map(skill_details)
            .unwrap_or_else(|| vec!["Reference entry unavailable.".to_owned()]),
        "status" => data
            .statuses
            .iter()
            .find(|status| status.id == result.id)
            .map(status_details)
            .unwrap_or_else(|| vec!["Status reference entry unavailable.".to_owned()]),
        "quest" => data
            .quest_by_id(result.id)
            .map(quest_reference_details)
            .unwrap_or_else(|| vec!["Quest reference entry unavailable.".to_owned()]),
        "map" => crate::world::navigation_graph()
            .maps
            .get(result.id as usize)
            .map(|map_name| map_details(map_name, &[]))
            .unwrap_or_else(|| vec!["Map entry unavailable.".to_owned()]),
        "map-template" => scripted_map_templates()
            .get(result.id as usize)
            .map(|map_name| map_details(map_name, &[]))
            .unwrap_or_else(|| vec!["Instance map template unavailable.".to_owned()]),
        "npc" => data
            .npcs
            .iter()
            .find(|npc| npc.id == result.id)
            .map(npc_details)
            .unwrap_or_else(|| vec!["NPC reference entry unavailable.".to_owned()]),
        "job" => job_names()
            .find(|(id, _)| *id as u32 == result.id)
            .map(|(_, name)| job_details(result.id as u16, name))
            .unwrap_or_else(|| vec!["Job entry unavailable.".to_owned()]),
        "mechanic" => refinement_details(&data.refinement, result.id),
        "coverage" => coverage_details(),
        "server-rule" => data
            .server_rules
            .get(result.id as usize)
            .map(server_rule_details)
            .unwrap_or_else(|| vec!["Server-rule reference unavailable.".to_owned()]),
        "service" => data
            .npc_service_by_index(result.id as usize)
            .map(service_details)
            .unwrap_or_else(|| vec!["NPC service reference entry unavailable.".to_owned()]),
        "rumor" => data
            .rumor_by_id(result.id)
            .map(rumor_details)
            .unwrap_or_else(|| vec!["Rumor reference entry unavailable.".to_owned()]),
        _ => vec!["Unsupported category.".to_owned()],
    }
}

fn resolve_details_with_library(result: &GuideResult, library: &Library) -> Vec<String> {
    if result.kind == "map" {
        return crate::world::navigation_graph()
            .maps
            .get(result.id as usize)
            .map(|map_name| {
                let town_pois = library.town_pois(map_name);
                map_details(map_name, &town_pois)
            })
            .unwrap_or_else(|| vec!["Map entry unavailable.".to_owned()]);
    }
    resolve_details(result)
}

/// One person on the first-job path, at the cell their script declares.
struct FirstJobStop {
    /// Name a player should look for. The Acolyte guild NPC speaks as Father
    /// Mareusis; the script declaration is Cleric.
    label: &'static str,
    declared_name: &'static str,
    map: &'static str,
    x: u16,
    y: u16,
    source: &'static str,
}

/// Academy instructor plus the town guild that actually changes the job.
struct FirstJobPath {
    job_id: u16,
    town: &'static str,
    directions: &'static str,
    academy: FirstJobStop,
    guild: FirstJobStop,
    /// The Thief guildsman changes the job only after the guide's interview.
    ceremony: Option<FirstJobStop>,
}

/// Renewal first jobs. Cells are the script declarations, not the speech
/// markers (those disagree: Adric's sign says 62,51 and the NPC stands at
/// 60,51).
const FIRST_JOBS: &[FirstJobPath] = &[
    FirstJobPath {
        job_id: 1,
        town: "Izlude",
        directions: "Swordman Trainer teaches on the Academy's second floor and does not change your job. The guild indoors in Izlude, \
                     west of the plaza, does.",
        academy: FirstJobStop {
            label: "Swordman Trainer",
            declared_name: "Swordman Trainer",
            map: "iz_ac02",
            x: 60,
            y: 51,
            source: "npc/re/jobs/novice/academy.txt",
        },
        guild: FirstJobStop {
            label: "Swordman",
            declared_name: "Swordman",
            map: "izlude_in",
            x: 74,
            y: 172,
            source: "npc/re/jobs/1-1/swordman.txt",
        },
        ceremony: None,
    },
    FirstJobPath {
        job_id: 2,
        town: "Geffen",
        directions: "Mage Chuck teaches on the Academy's second floor and sends you to Geffen. The Mage Guildsman there changes your job.",
        academy: FirstJobStop {
            label: "Mage Chuck",
            declared_name: "Mage Chuck",
            map: "iz_ac02",
            x: 148,
            y: 110,
            source: "npc/re/jobs/novice/academy.txt",
        },
        guild: FirstJobStop {
            label: "Mage Guildsman",
            declared_name: "Mage Guildsman",
            map: "geffen_in",
            x: 164,
            y: 124,
            source: "npc/re/jobs/1-1/mage.txt",
        },
        ceremony: None,
    },
    FirstJobPath {
        job_id: 3,
        town: "Payon",
        directions: "Archer Teacher teaches on the Academy's second floor and sends you to Payon. The guild is in Archer Village, north \
                     of town.",
        academy: FirstJobStop {
            label: "Archer Teacher",
            declared_name: "Archer Teacher",
            map: "iz_ac02",
            x: 65,
            y: 109,
            source: "npc/re/jobs/novice/academy.txt",
        },
        guild: FirstJobStop {
            label: "Archer Guildsman",
            declared_name: "Archer Guildsman",
            map: "payon_in02",
            x: 64,
            y: 71,
            source: "npc/re/jobs/1-1/archer.txt",
        },
        ceremony: None,
    },
    FirstJobPath {
        job_id: 4,
        town: "Prontera",
        directions: "Acolyte Leader Alice teaches on the Academy's second floor and sends you to the Prontera church. Father Mareusis \
                     there changes your job. His script name is Cleric.",
        academy: FirstJobStop {
            label: "Acolyte Leader Alice",
            declared_name: "Acolyte Leader Alice",
            map: "iz_ac02",
            x: 156,
            y: 169,
            source: "npc/re/jobs/novice/academy.txt",
        },
        guild: FirstJobStop {
            label: "Father Mareusis",
            declared_name: "Cleric",
            map: "prt_church",
            x: 184,
            y: 41,
            source: "npc/re/jobs/1-1/acolyte.txt",
        },
        ceremony: None,
    },
    FirstJobPath {
        job_id: 5,
        town: "Alberta",
        directions: "Salim Hamid teaches on the Academy's second floor and sends you to Alberta. The Merchant inside changes your job.",
        academy: FirstJobStop {
            label: "Salim Hamid",
            declared_name: "Salim Hamid",
            map: "iz_ac02",
            x: 50,
            y: 169,
            source: "npc/re/jobs/novice/academy.txt",
        },
        guild: FirstJobStop {
            label: "Merchant",
            declared_name: "Merchant",
            map: "alberta_in",
            x: 53,
            y: 43,
            source: "npc/re/jobs/1-1/merchant.txt",
        },
        ceremony: None,
    },
    FirstJobPath {
        job_id: 6,
        town: "Morocc",
        directions: "Guest Lecturer Mayssel teaches on the Academy's second floor and sends you to the pyramid northwest of Morocc. Talk \
                     to the Thief Guide first. The guildsman beside them changes your job after that interview.",
        academy: FirstJobStop {
            label: "Guest Lecturer Mayssel",
            declared_name: "Guest Lecturer Mayssel",
            map: "iz_ac02",
            x: 52,
            y: 136,
            source: "npc/re/jobs/novice/academy.txt",
        },
        guild: FirstJobStop {
            label: "Thief Guide",
            declared_name: "Thief Guide",
            map: "moc_prydb1",
            x: 39,
            y: 129,
            source: "npc/re/jobs/1-1/thief.txt",
        },
        ceremony: Some(FirstJobStop {
            label: "Thief Guildsman",
            declared_name: "Thief Guildsman",
            map: "moc_prydb1",
            x: 42,
            y: 133,
            source: "npc/re/jobs/1-1/thief.txt",
        }),
    },
];

fn first_job_path(job_id: u16) -> Option<&'static FirstJobPath> {
    FIRST_JOBS.iter().find(|path| path.job_id == job_id)
}

fn push_first_job_stop(lines: &mut Vec<String>, role: &str, stop: &FirstJobStop) {
    lines.push(format!("{role}: {} on {} at ({}, {}).", stop.label, stop.map, stop.x, stop.y));
    if stop.label != stop.declared_name {
        lines.push(format!("Script name: {}.", stop.declared_name));
    }
    lines.push(format!(
        "@route-cell:{}:{}:{}|Route to {} — {}",
        stop.map, stop.x, stop.y, stop.label, stop.map
    ));
}

fn first_job_source_note(paths: &[&FirstJobPath]) -> String {
    let mut sources = Vec::new();
    for path in paths {
        for stop in [&path.academy, &path.guild].into_iter().chain(path.ceremony.as_ref()) {
            if !sources.contains(&stop.source) {
                sources.push(stop.source);
            }
        }
    }
    format!("Source: {}.", sources.join("; "))
}

fn push_one_first_job(lines: &mut Vec<String>, path: &FirstJobPath) {
    lines.push(format!("Town for this class: {}.", path.town));
    lines.push(path.directions.to_owned());
    push_first_job_stop(lines, "Academy trainer", &path.academy);
    push_first_job_stop(lines, "Town guild", &path.guild);
    if let Some(ceremony) = &path.ceremony {
        push_first_job_stop(lines, "Job ceremony", ceremony);
    }
}

/// Novice sees every first job. Each first job sees only its own trainer and
/// guild.
fn append_first_job_path(lines: &mut Vec<String>, job_id: u16) {
    if job_id == 0 {
        lines.push(
            "Criatura Academy, second floor (iz_ac02), teaches the six first jobs. The instructor does not change your job. At job level \
             10 they send you to the town guild, and that guild's quest does."
                .to_owned(),
        );
        for path in FIRST_JOBS {
            let name = job_names()
                .find(|(id, _)| *id == path.job_id)
                .map(|(_, name)| name)
                .unwrap_or("First job");
            lines.push(format!("{name} — {}", path.town));
            push_one_first_job(lines, path);
        }
        lines.push(first_job_source_note(&FIRST_JOBS.iter().collect::<Vec<_>>()));
        return;
    }
    let Some(path) = first_job_path(job_id) else {
        return;
    };
    push_one_first_job(lines, path);
    lines.push(first_job_source_note(&[path]));
}

/// Searches that should open the novice page even though its title is not those
/// words.
fn newbie_path_query(query: &str) -> bool {
    let query = query.trim().to_lowercase();
    [
        "trainer",
        "job change",
        "which town",
        "newbie",
        "new player",
        "first job",
        "criatura",
    ]
    .iter()
    .any(|phrase| query.contains(phrase))
        || query == "academy"
        || query.starts_with("academ")
}

fn job_details(job_id: u16, name: &str) -> Vec<String> {
    let mut lines = vec![name.to_owned()];
    append_first_job_path(&mut lines, job_id);
    // The numbers a player checks first: the cap, the body, then the EXP grind
    // and the skill list.
    append_stat_rule_details(&mut lines, job_id);
    append_class_table_details(&mut lines, job_id);
    let data = reference_data();
    if let Some(bonuses) = data.job_bonuses_by_id(job_id) {
        append_job_bonus_details(&mut lines, bonuses);
    } else {
        lines.push("No job-level stat bonus schedule is present for this job ID in Hercules job_db2.txt.".to_owned());
    }
    append_exp_details(&mut lines, job_id);
    let Some(tree) = data.job_skill_tree_by_id(job_id) else {
        lines.push("No matching skill tree is present in the bundled Hercules job-skill export.".to_owned());
        lines.push("Job bonus source: bundled Hercules job_db2.txt export; conditional-script effects are not inferred.".to_owned());
        return lines;
    };

    lines.push(format!(
        "Skill tree: {} — {} skills including inherited skills",
        tree.tree_name,
        tree.skills.len()
    ));
    for skill in &tree.skills {
        let mut label = format!("{} — max level {}", skill.name, skill.max_level);
        if skill.minimum_job_level > 0 {
            label.push_str(&format!(", job level {}", skill.minimum_job_level));
        }
        if !skill.prerequisites.is_empty() {
            let prerequisites = skill
                .prerequisites
                .iter()
                .map(|prerequisite| format!("{} Lv {}", prerequisite.name, prerequisite.level))
                .collect::<Vec<_>>()
                .join(", ");
            label.push_str(&format!(" — requires {prerequisites}"));
        }
        lines.push(format!("@guide:skill:{}|{label}", skill.skill_id));
    }
    lines.push(
        "Source: bundled Hercules renewal skill_tree.conf and skill_db.conf for skills; job_db2.txt for job-level stat bonuses. \
         Conditional-script effects are not inferred."
            .to_owned(),
    );
    lines
}

/// `1234567` -> `1,234,567`.
fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

fn append_exp_table(lines: &mut Vec<String>, label: &str, group_name: &str, group: &crate::dm::reference_data::ReferenceExpGroup) {
    lines.push(format!(
        "{label} EXP (server group {group_name}, maximum level {}): EXP to advance from level to the next",
        group.max_level
    ));
    let mut milestones: Vec<u16> = (1..=group.exp.len() as u16)
        .filter(|level| *level == 1 || level % 10 == 0)
        .collect();
    if let Some(last) = u16::try_from(group.exp.len()).ok().filter(|last| !milestones.contains(last)) {
        milestones.push(last);
    }
    for level in milestones {
        let Some(exp) = group.exp_to_next(level) else { continue };
        lines.push(format!("  {level}–{}: {}", level + 1, group_digits(exp)));
    }
    if let Some(total) = group.total_to_reach(group.max_level) {
        lines.push(format!(
            "  Total from level 1 to level {}: {}",
            group.max_level,
            group_digits(total)
        ));
    }
}

/// `GatlingGun` -> `Gatling Gun`.
fn spaced(name: &str) -> String {
    let mut spaced = String::new();
    for (index, character) in name.chars().enumerate() {
        if index > 0 && character.is_uppercase() {
            spaced.push(' ');
        }
        spaced.push(character);
    }
    spaced
}

fn is_ranged_weapon(weapon: &str) -> bool {
    matches!(
        weapon,
        "Bow" | "Instrument" | "Whip" | "Revolver" | "Rifle" | "GatlingGun" | "Shotgun" | "GrenadeLauncher"
    )
}

/// Base HP/SP at a few levels and the class's base ASPD per weapon, from the
/// server's class tables (see the Max HP/SP and ASPD rules for the formulas).
fn append_class_table_details(lines: &mut Vec<String>, job_id: u16) {
    let data = reference_data();
    let Some(job) = data.job_tables.job(job_id) else {
        lines.push("No class table is defined for this job in Hercules job_db.conf.".to_owned());
        return;
    };
    // `status.c` `status_calc_pc`: max_weight = class base + 300 × STR, in
    // tenths of a weight unit; skills such as Enlarge Weight Limit add more.
    lines.push(format!(
        "Weight capacity: {} plus 30 per base STR point (before skill bonuses).",
        group_digits(u64::from(job.weight_base / 10))
    ));
    let cap = data.stat_job(job_id).map_or(99, |stat| stat.max_stats as i32);
    let mut levels: Vec<usize> = [1, 50, job.max_level as usize]
        .into_iter()
        .filter(|level| *level <= job.max_level as usize)
        .collect();
    levels.dedup();

    let mut push_levels = |header: String, at_one: &dyn Fn(usize) -> Option<u64>, at_cap: &dyn Fn(usize) -> Option<u64>| {
        lines.push(header);
        for level in &levels {
            let (Some(low), Some(high)) = (at_one(*level), at_cap(*level)) else {
                continue;
            };
            lines.push(format!("  level {level}: {} / {}", group_digits(low), group_digits(high)));
        }
    };
    push_levels(
        format!("Base max HP at VIT 1 / VIT {cap} (class table before gear and statuses):"),
        &|level| data.job_tables.base_max_hp(job_id, level, 1),
        &|level| data.job_tables.base_max_hp(job_id, level, cap),
    );
    push_levels(
        format!("Base max SP at INT 1 / INT {cap}:"),
        &|level| data.job_tables.base_max_sp(job_id, level, 1),
        &|level| data.job_tables.base_max_sp(job_id, level, cap),
    );

    // A few generated tables collapse to a tiny value. Say so rather than show
    // a plausible-looking table with a hole in it.
    for (label, table) in [
        ("HP", data.job_tables.hp_tables.get(&job.hp_table)),
        ("SP", data.job_tables.sp_tables.get(&job.sp_table)),
    ] {
        let Some(table) = table else { continue };
        if let Some(first) = table
            .iter()
            .enumerate()
            .find(|(index, value)| *index >= 10 && **value <= 5)
            .map(|(index, _)| index + 1)
        {
            lines.push(format!(
                "Warning: this class's {label} table drops to {} at level {first} and stays there to level {}. The server's loader \
                 generated that tail, and a level-161 Baby Kagerou was observed with max HP 1 on this server.",
                table[first - 1],
                table.len()
            ));
        }
    }

    let mut weapons: Vec<(&String, &u16)> = job.base_aspd.iter().filter(|(_, value)| **value > 0).collect();
    weapons.sort_by_key(|(name, _)| name.as_str());
    if weapons.is_empty() {
        lines.push("No base ASPD values are defined for this job.".to_owned());
        return;
    }
    lines.push(format!(
        "Base ASPD values by weapon (the B in the ASPD rule). Maximum ASPD {}.",
        job.max_aspd
    ));
    for (name, value) in &weapons {
        lines.push(format!("  {} {value}", spaced(name)));
    }
    // Passive ASPD skills this job learns, at their maximum level. They add to
    // the base formula and only count with the right weapon.
    if let Some(tree) = data.job_skill_tree_by_id(job_id) {
        let passives = [
            (274, "Advanced Book (with a book)", 1),
            (509, "Single Action", 2),
            (225, "Plagiarism", 3),
            (315, "Musical Lesson (with an instrument)", 4),
        ];
        let listed: Vec<String> = passives
            .iter()
            .filter_map(|(skill_id, label, slot)| {
                let max_level = tree.skills.iter().find(|skill| skill.skill_id == *skill_id)?.max_level as i32;
                let mut levels = [0; 4];
                levels[slot - 1] = max_level;
                let bonus = stat_formulas::passive_aspd_bonus(true, true, levels[0], levels[1], levels[2], levels[3]);
                Some(format!("{label} +{bonus}"))
            })
            .collect();
        if !listed.is_empty() {
            lines.push("Passive ASPD skills at maximum level (added inside the base ASPD formula):".to_owned());
            for skill in listed {
                lines.push(format!("  {skill}"));
            }
        }
    }
    lines.push(format!(
        "Base ASPD at DEX {cap} and AGI {cap}, no passive ASPD skills, no shield, before gear and statuses:"
    ));
    for (name, value) in weapons.iter().filter(|(name, _)| name.as_str() != "Shield") {
        // The server's own pipeline, including the class cap, then back to
        // the ASPD number the game displays: 200 - motion / 10.
        let motion = stat_formulas::attack_motion(&stat_formulas::AspdInputs {
            agi: cap,
            dex: cap,
            class_base: **value,
            ranged: is_ranged_weapon(name),
            max_aspd: job.max_aspd,
            ..Default::default()
        });
        lines.push(format!("  {} {}", spaced(name), 200 - motion / 10));
    }
    // A shield adds its value to B, which slows you down.
    let shield = job.base_aspd.get("Shield").copied().unwrap_or(0);
    if shield > 0 {
        lines.push(format!("The same with a shield worn (its value, {shield}, slows you down):"));
        for weapon in ["Fist", "Dagger", "Sword", "Spear", "Axe", "Mace", "Rod", "Knuckle", "Book"] {
            let Some(value) = job.base_aspd.get(weapon).copied() else {
                continue;
            };
            let motion = stat_formulas::attack_motion(&stat_formulas::AspdInputs {
                agi: cap,
                dex: cap,
                class_base: stat_formulas::class_aspd_base(value, None, shield),
                ranged: false,
                max_aspd: job.max_aspd,
                ..Default::default()
            });
            lines.push(format!("  {} {}", spaced(weapon), 200 - motion / 10));
        }
    }
}

fn append_stat_rule_details(lines: &mut Vec<String>, job_id: u16) {
    let data = reference_data();
    let Some(job) = data.stat_job(job_id) else {
        lines.push("No stat rules are defined for this job in Hercules job_db.conf.".to_owned());
        return;
    };
    lines.push(format!(
        "Stat cap: {} (parameter group {}). {}",
        job.max_stats,
        job.parameters_group,
        match job.upper {
            true => format!(
                "Upper class: a stat reset grants {} extra points.",
                data.stat_rules.upper_class_extra_points
            ),
            false => "Not an upper class: no extra points on a stat reset.".to_owned(),
        }
    ));
}

fn append_exp_details(lines: &mut Vec<String>, job_id: u16) {
    let Some((job, base, job_group)) = reference_data().exp_groups_for_job(job_id) else {
        lines.push("No EXP group is defined for this job in Hercules job_db.conf.".to_owned());
        return;
    };
    append_exp_table(lines, "Base", &job.base_group, base);
    append_exp_table(lines, "Job", &job.job_group, job_group);
    lines.push(
        "Source: Hercules db/re/exp_group_db.conf and job_db.conf. These are the table values; the server's base, job and quest EXP rates \
         (see the Experience and drop modifiers rule) and the level-difference modifiers scale what a kill awards."
            .to_owned(),
    );
}

fn append_job_bonus_details(lines: &mut Vec<String>, bonuses: &ReferenceJobBonuses) {
    lines.push(format!("Job-level stat bonuses through job level {}:", bonuses.max_job_level));
    for stat in ["STR", "AGI", "VIT", "INT", "DEX", "LUK"] {
        let levels = bonuses
            .bonus_levels
            .iter()
            .filter(|bonus| bonus.stat == stat)
            .map(|bonus| bonus.job_level.to_string())
            .collect::<Vec<_>>();
        if levels.is_empty() {
            continue;
        }
        let total = bonuses.total_bonuses.get(stat).copied().unwrap_or(levels.len() as u16);
        lines.push(format!("{stat} +{total}: job levels {}", levels.join(", ")));
    }
}

fn skill_details(skill: &ReferenceSkill) -> Vec<String> {
    let mut lines = crate::world::skill_tooltip_text(skill.id, &skill.description, 1, skill.maximum_level)
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let data = reference_data();

    if !skill.prerequisites.is_empty() {
        lines.push("Requires:".to_owned());
        for req in &skill.prerequisites {
            let name = match req.skill_id {
                Some(req_id) => skill_player_name(u32::from(req_id), &req.name),
                None => req.name.clone(),
            };
            match req.skill_id {
                Some(req_id) => lines.push(format!("@guide:skill:{req_id}|{name} level {}", req.level)),
                None => lines.push(format!("{name} level {}", req.level)),
            }
        }
    }

    let job_sources: Vec<_> = data
        .job_skill_trees
        .iter()
        .filter_map(|tree| {
            tree.skills
                .iter()
                .find(|job_skill| job_skill.skill_id == skill.id)
                .map(|job_skill| (tree, job_skill))
        })
        .collect();
    if !job_sources.is_empty() {
        lines.push("Classes that learn this:".to_owned());
        for (tree, job_skill) in job_sources.iter().take(12) {
            let mut details = format!("{}: up to level {}", tree.tree_name, job_skill.max_level);
            if job_skill.minimum_job_level > 0 {
                details.push_str(&format!(", from job level {}", job_skill.minimum_job_level));
            }
            if !job_skill.prerequisites.is_empty() {
                let prerequisites = job_skill
                    .prerequisites
                    .iter()
                    .map(|prerequisite| {
                        format!(
                            "{} level {}",
                            skill_player_name(u32::from(prerequisite.skill_id), &prerequisite.name),
                            prerequisite.level
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                details.push_str(&format!(", after {prerequisites}"));
            }
            lines.push(format!("@guide:job:{}|{details}", tree.job_id));
        }
        if job_sources.len() > 12 {
            lines.push(format!("Source: {} additional job trees omitted.", job_sources.len() - 12));
        }
    } else {
        lines.push("No class lists this skill.".to_owned());
    }

    let linked_statuses: Vec<_> = data
        .statuses
        .iter()
        .filter(|status| {
            status.statuses.iter().any(|mechanic| {
                mechanic.associated_skill.as_ref().is_some_and(|source| source.id == skill.id)
                    || mechanic.status_change_skills.iter().any(|source| source.id == skill.id)
            })
        })
        .collect();
    if !linked_statuses.is_empty() {
        lines.push("Applies:".to_owned());
        for status in &linked_statuses {
            lines.push(format!("@guide:status:{}|{}", status.id, status.name));
        }
    }
    if let Some(status_change) = &skill.status_change {
        let has_linked_icon = linked_statuses
            .iter()
            .any(|status| status.statuses.iter().any(|mechanic| mechanic.constant == *status_change));
        if !has_linked_icon {
            lines.push("Applies a status that has no icon.".to_owned());
            lines.push(format!("Source: status change {status_change}."));
        }
    }

    if let Some(review) = data.skill_formula_review_for_skill(skill.id) {
        lines.push(review.title.clone());
        lines.push(review.formula.clone());
        if !review.worked_example.is_empty() {
            lines.push(format!("Worked example: {}", review.worked_example));
        }
        for cond in &review.conditions {
            lines.push(format!("Note: {cond}"));
        }
        lines.push(format!("Source: reviewed as {}.", review.evidence_state.label()));
        for source in &review.sources {
            let cited = source.lines.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
            lines.push(format!("Source: {}:{cited}.", source.path));
        }
    } else {
        lines.push("No damage formula is written up for this skill.".to_owned());
        lines.push("Source: no reviewed formula in the Hercules renewal engine.".to_owned());
    }

    lines.push("Source: bundled Hercules skill database export.".to_owned());
    if let Some(source) = &skill.source {
        lines.push(format!("Source: {} ({}).", source.path, source.record));
    }
    lines
}

/// The name a player reads for a skill. The export token is the fallback.
fn skill_player_name(id: u32, fallback: &str) -> String {
    reference_data()
        .skill_by_id(id)
        .map(|skill| display_name(&skill.description, &skill.name))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| fallback.to_owned())
}

fn status_details(status: &crate::dm::reference_data::ReferenceStatus) -> Vec<String> {
    let mut lines = vec![status.name.clone()];
    if status.iconless {
        lines.push("This status has no icon. The name is derived from the server constant.".to_owned());
    }
    if status.statuses.is_empty() {
        lines.push("No server rules are listed for this status.".to_owned());
        lines.push("Source: server status-icon name only; no matching sc_config record.".to_owned());
    } else {
        lines.push("How the server treats this status:".to_owned());
        for mechanic in &status.statuses {
            lines.push(format!("Source: server status {}.", mechanic.constant));
            let flag_meanings = mechanic
                .flags
                .iter()
                .filter_map(|flag| match flag.as_str() {
                    "NoDeathReset" => Some("persists through death"),
                    "NoSave" => Some("not saved as persistent character state"),
                    "NoDispelReset" => Some("not removed by Dispel"),
                    "NoClearanceReset" => Some("not removed by Clearance"),
                    "Buff" => Some("classified by the server as a buff"),
                    "Debuff" => Some("classified by the server as a debuff"),
                    "NoMadoReset" => Some("not cleared when MADO Gear is removed"),
                    "NoAllReset" => Some("not cleared by the server's general status reset"),
                    "NoBoss" => Some("cannot be applied to boss monsters"),
                    "NoBBReset" => Some("not removed by Banishing Buster"),
                    "NoMagicBlocked" => Some("cannot be applied while the target is in a no-magic state"),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if flag_meanings.is_empty() {
                lines.push("  No lifecycle or classification flags listed.".to_owned());
            } else {
                lines.push(format!("  Server lifecycle rules: {}.", flag_meanings.join("; ")));
            }
            if !mechanic.calculation_flags.is_empty() {
                let labels = mechanic
                    .calculation_flags
                    .iter()
                    .map(|flag| match flag.as_str() {
                        "Str" => "STR",
                        "Agi" => "AGI",
                        "Vit" => "VIT",
                        "Int" => "INT",
                        "Dex" => "DEX",
                        "Luk" => "LUK",
                        "Hit" => "HIT",
                        "Flee" => "FLEE",
                        "Def" => "DEF",
                        "Mdef" => "MDEF",
                        "Maxhp" => "maximum HP",
                        "Maxsp" => "maximum SP",
                        "Batk" => "base attack",
                        "Watk" => "weapon attack",
                        "Matk" => "magic attack",
                        "Atk_Ele" => "attack element",
                        "Def_Ele" => "defense element",
                        "Aspd" => "attack speed",
                        "Dspd" => "movement speed",
                        "Def2" => "defense",
                        "Mdef2" => "magic defense",
                        "Flee2" => "perfect dodge",
                        "AtkPerc" => "attack percentage",
                        "DefPerc" => "defense percentage",
                        "MatkPerc" => "magic attack percentage",
                        "MdefPerc" => "magic defense percentage",
                        "Cri" => "critical rate",
                        other => other,
                    })
                    .collect::<Vec<_>>();
                lines.push(format!(
                    "  Server recalculates these stat groups when this status changes: {}. This does not state whether the status raises \
                     or lowers them.",
                    labels.join(", ")
                ));
            }
            if let Some(skill) = &mechanic.associated_skill {
                let label = display_name(&skill.description, &skill.name);
                lines.push(format!("@guide:skill:{}|Applied by {label}", skill.id));
                if !skill.name.is_empty() {
                    lines.push(format!("Source: skill {}.", skill.name));
                }
            }
            for skill in &mechanic.status_change_skills {
                if mechanic
                    .associated_skill
                    .as_ref()
                    .is_some_and(|associated| associated.id == skill.id)
                {
                    continue;
                }
                let label = display_name(&skill.description, &skill.name);
                lines.push(format!("@guide:skill:{}|Applied by {label}", skill.id));
                if !skill.name.is_empty() {
                    lines.push(format!("Source: skill {}.", skill.name));
                }
            }
            if !mechanic.code_call_sites.is_empty() {
                lines.push(format!(
                    "Source: sc_start call sites are not exhaustive ({} found).",
                    mechanic.code_call_sites.len()
                ));
                for source in mechanic.code_call_sites.iter().take(8) {
                    lines.push(format!("Source: {}:{}.", source.path, source.line));
                }
                if mechanic.code_call_sites.len() > 8 {
                    lines.push(format!(
                        "Source: {} additional call sites omitted.",
                        mechanic.code_call_sites.len() - 8
                    ));
                }
            }
        }
    }
    lines.push(
        "These server lifecycle rules do not explain the status's gameplay effect. Its exact effect, duration, per-level odds, complete \
         sources, interactions, and cures are not documented yet."
            .to_owned(),
    );
    lines
}

fn parse_guide_link(line: &str) -> Option<GuideResult> {
    let link = line.strip_prefix("@guide:")?;
    let (target, label) = link.split_once('|')?;
    let (kind, id) = target.split_once(':')?;
    match kind {
        "item" | "monster" | "skill" | "status" | "quest" | "npc" | "job" | "rumor" => Some(GuideResult {
            label: label.to_owned(),
            kind: kind.to_owned(),
            id: id.parse().ok()?,
        }),
        "service" => {
            let service_index = if let Ok(idx) = id.parse::<u32>() {
                idx
            } else {
                reference_data().npc_service_index_by_id(id)? as u32
            };
            Some(GuideResult {
                label: label.to_owned(),
                kind: "service".to_owned(),
                id: service_index,
            })
        }
        "map" => {
            let map_index = if let Ok(idx) = id.parse::<u32>() {
                idx
            } else {
                crate::world::navigation_graph()
                    .maps
                    .iter()
                    .position(|m| m.eq_ignore_ascii_case(id))? as u32
            };
            Some(GuideResult {
                label: label.to_owned(),
                kind: "map".to_owned(),
                id: map_index,
            })
        }
        "refinement" | "mechanic" => Some(GuideResult {
            label: label.to_owned(),
            kind: "mechanic".to_owned(),
            id: id.parse().unwrap_or(1),
        }),
        "coverage" => Some(GuideResult {
            label: label.to_owned(),
            kind: "coverage".to_owned(),
            id: id.parse().unwrap_or(1),
        }),
        "server-rule" => Some(GuideResult {
            label: label.to_owned(),
            kind: "server-rule".to_owned(),
            id: id.parse().ok()?,
        }),
        _ => None,
    }
}

struct GuideResultList<A> {
    state_path: A,
    library: Arc<Library>,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> Element<ClientState> for GuideResultList<A>
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy + 'static,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use korangar_interface::prelude::*;
            let count = state.get(&self.state_path.results()).len();
            self.elements.truncate(count);
            for index in self.elements.len()..count {
                let row_path = self.state_path.results().index(index).manually_asserted();
                let detail_path = self.state_path.detail();
                let library = self.library.clone();
                self.elements.push(ErasedElement::new(button! {
                    text: row_path.label(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                    event: move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
                        let result = state.get(&row_path).clone();
                        let detail = if result.kind == "quest" {
                            state
                                .get(&client_state().quest_log())
                                .quests()
                                .iter()
                                .find(|quest| quest.quest_id == result.id)
                                .map(quest_details)
                                .or_else(|| reference_data().quest_by_id(result.id).map(quest_reference_details))
                                .unwrap_or_else(|| vec!["This quest is no longer active. Refresh the search to update the list.".to_owned()])
                        } else {
                            resolve_details_with_library(&result, &library)
                        };
                        state.update_value(detail_path, detail);
                    },
                }));
            }
            for (index, element) in self.elements.iter_mut().enumerate() {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            }
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        for (index, element) in self.elements.iter().enumerate() {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        }
    }
}

/// How a Guide route link is presented from the player's current map.
#[derive(Debug, PartialEq)]
enum RouteOffer {
    /// A clickable route labelled with how many map transitions it takes,
    /// followed by the lock and unlock steps of any locked hop on it.
    Button { text: String, notes: Vec<String> },
    /// No verified route from here: the reason, as text, instead of a button
    /// that would only fail with a toast after the click.
    Unavailable(Vec<String>),
}

/// "3 maps · 1,200 z · ~180 cells · 1 locked step" for a route button.
fn route_summary_text(summary: crate::world::RouteSummary) -> String {
    let mut parts = vec![match summary.hops {
        1 => "1 map".to_owned(),
        hops => format!("{hops} maps"),
    }];
    if summary.zeny > 0 {
        parts.push(format!("{} z", group_digits(u64::from(summary.zeny))));
    }
    parts.push(format!("~{} cells", group_digits(u64::from(summary.walk_cells))));
    match summary.locked_hops {
        0 => {}
        1 => parts.push("1 locked step".to_owned()),
        locked => parts.push(format!("{locked} locked steps")),
    }
    parts.join(" · ")
}

fn route_offer(current_map: &str, target_map: &str, label: &str) -> RouteOffer {
    // Outside a map (or before the minimap knows it) there is nothing to
    // measure from; keep the plain action and let the click handler decide.
    if current_map.is_empty() {
        return RouteOffer::Button {
            text: label.to_owned(),
            notes: Vec::new(),
        };
    }
    let Some(summary) = crate::world::route_summary(current_map, target_map) else {
        return RouteOffer::Unavailable(crate::world::unreachable_explanation(current_map, target_map));
    };
    let text = match summary.hops {
        0 => format!("{label} (this map)"),
        _ => format!(
            "{label} ({}; {})",
            route_summary_text(summary),
            crate::world::route_preference().label()
        ),
    };
    RouteOffer::Button {
        text,
        notes: crate::world::route_lock_notes(current_map, target_map),
    }
}

/// Plain wrapped text rows under a Guide line (route lock steps, reasons).
fn push_text_lines(elements: &mut Vec<ElementBox<ClientState>>, lines: Vec<String>) {
    use korangar_interface::prelude::*;

    for line in lines {
        elements.push(ErasedElement::new(
            text! { text: line, overflow_behavior: OverflowBehavior::LineBreak },
        ));
    }
}

/// Whether a Guide page line is a note about where the data came from (file
/// references, export revisions, internal names) rather than something a
/// player reads for. Those collect into a folded section at the end of the
/// page instead of interrupting it. `in_call_site_list` is true while the
/// indented lines under a call-site header are being read.
fn is_source_note(line: &str, in_call_site_list: bool) -> bool {
    const PREFIXES: [&str; 14] = [
        "Source:",
        "Values are exported",
        "Exported from Hercules",
        "Job bonus source:",
        "Internal script name:",
        "Literal Hercules C",
        "Related NPC script reference:",
        "Script call evidence:",
        "Formula: Unreviewed",
        "Verified reference:",
        "Spawn rosters are configured data",
        "No job-level stat bonus schedule",
        "No matching skill tree is present",
        "No class table is defined",
    ];
    (in_call_site_list && line.starts_with("  "))
        || PREFIXES.iter().any(|prefix| line.starts_with(prefix))
        || line.contains("in Hercules job_db")
        || line.ends_with("script clues omitted.")
        || line.ends_with("NPC script references omitted.")
        || line.contains("loaded-script directive(s) for this flag are superseded")
}

struct GuideLines<A> {
    path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> GuideLines<A> {
    fn new(path: A) -> Self {
        Self {
            path,
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for GuideLines<A>
where
    A: Path<ClientState, Vec<String>> + Copy,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use korangar_interface::prelude::*;
            // Detail strings contain dynamic cross-links/actions; rebuild them
            // when the selected entry changes, even if the line count does not.
            self.elements.clear();
            let current_map = state.get(&client_state().minimap()).map_name().to_owned();
            let count = state.get(&self.path).len();
            let mut source_notes: Vec<ElementBox<ClientState>> = Vec::new();
            let mut in_call_site_list = false;
            for index in 0..count {
                let line = self.path.index(index).manually_asserted();
                let value = state.get(&line).clone();
                let starts_call_sites = value.starts_with("Literal Hercules C");
                if is_source_note(&value, in_call_site_list) {
                    in_call_site_list = starts_call_sites || in_call_site_list;
                    source_notes.push(ErasedElement::new(text! {
                        text: value.trim_start().to_owned(),
                        color: crate::graphics::Color::rgb_u8(150, 150, 150),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    }));
                    continue;
                }
                in_call_site_list = false;
                if let Some(monster_id) = value
                    .strip_prefix("@hunting-goal:")
                    .and_then(|link| link.split_once('|'))
                    .and_then(|(monster_id, _)| monster_id.parse::<u32>().ok())
                {
                    self.elements.push(ErasedElement::new(button! {
                        text: "Add to my hunting goals",
                        tooltip: "Saved for this character. This is not a server quest and has no kill counter.",
                        event: InputEvent::AddClientHuntingGoal { monster_id },
                    }));
                } else if let Some((map_name, x, y, label)) = parse_route_cell_link(&value) {
                    match route_offer(&current_map, &map_name, &label) {
                        RouteOffer::Button { text, notes } => {
                            self.elements.push(ErasedElement::new(button! {
                                text: text,
                                event: InputEvent::SetNavigationDestination { map_name, x, y },
                            }));
                            push_text_lines(&mut self.elements, notes);
                        }
                        RouteOffer::Unavailable(lines) => push_text_lines(&mut self.elements, lines),
                    }
                } else if let Some(map_name) = value.strip_prefix("@route:") {
                    let map_name = map_name.to_owned();
                    match route_offer(&current_map, &map_name, &format!("Route to {map_name}")) {
                        RouteOffer::Button { text, notes } => {
                            self.elements.push(ErasedElement::new(button! {
                                text: text,
                                event: InputEvent::SetNavigationMapDestination { map_name },
                            }));
                            push_text_lines(&mut self.elements, notes);
                        }
                        RouteOffer::Unavailable(lines) => push_text_lines(&mut self.elements, lines),
                    }
                } else if let Some(result) = parse_guide_link(&value) {
                    let detail_path = self.path;
                    self.elements.push(ErasedElement::new(button! {
                        text: result.label.clone(),
                        overflow_behavior: OverflowBehavior::LineBreak,
                        event: move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
                            state.update_value(detail_path, resolve_details(&result));
                        },
                    }));
                } else {
                    self.elements.push(ErasedElement::new(
                        text! { text: line, overflow_behavior: OverflowBehavior::LineBreak },
                    ));
                }
            }
            if !source_notes.is_empty() {
                self.elements.push(ErasedElement::new(collapsible! {
                    text: "Where this comes from",
                    initially_expanded: false,
                    children: (super::rows::Rows { elements: source_notes },),
                }));
            }
            for (index, element) in self.elements.iter_mut().enumerate() {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            }
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        for (index, element) in self.elements.iter().enumerate() {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        }
    }
}

/// The search text after one keystroke, or `None` when the key does nothing.
/// Enter and Tab keep the text (and search again); Backspace removes the
/// last character; other control keys are ignored; the length is capped.
fn next_query(current: &str, character: char) -> Option<String> {
    match character {
        '\x09' | '\x0d' => Some(current.to_owned()),
        '\x08' => {
            let mut text = current.to_owned();
            text.pop();
            Some(text)
        }
        character if !character.is_control() && current.len() < MAX_QUERY => Some(format!("{current}{character}")),
        _ => None,
    }
}

/// The Guide's search box: edits the text like the default text box, and
/// searches again after every change, so results narrow as you type. Enter
/// still searches too.
struct LiveSearchHandler<A> {
    path: A,
}

impl<A> korangar_interface::event::InputHandler<ClientState> for LiveSearchHandler<A>
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy,
{
    fn handle_character(
        &self,
        state: &State<ClientState>,
        queue: &mut korangar_interface::event::EventQueue<ClientState>,
        character: char,
    ) {
        if character == '\x1b' {
            queue.queue(korangar_interface::event::Event::Unfocus);
            return;
        }
        let current = state.get(&self.path.query()).clone();
        let Some(next) = next_query(&current, character) else {
            return;
        };
        if next != current {
            state.update_value(self.path.query(), next.clone());
        }
        let category = state.get(&self.path.category()).clone();
        run_search_for(state, self.path, &next, &category);
    }
}

/// Search with the query and category given, not read back from state.
/// `rust_state` queues updates until the frame applies them, so a caller
/// that has just changed the category or the text must pass the new value:
/// reading it back gave the *previous* one (a category button searched the
/// category you had left, and live search would lag a keystroke).
fn run_search_for<A>(state: &State<ClientState>, path: A, query: &str, category: &str)
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy,
{
    let query = query.to_lowercase();
    let category = category.to_owned();
    let data = reference_data();
    let discovery_path = client_state().discovery();
    let discovery = state.get(&discovery_path);
    let mut rows = Vec::new();

    if category == "All" {
        let quest_log_path = client_state().quest_log();
        rows.extend(search_all_categories(&query, discovery, state.get(&quest_log_path).quests()));
    } else if category == "Monsters" {
        rows.extend(
            data.search_monsters(&query, MAX_RESULTS).into_iter().map(|monster| GuideResult {
                label: format!(
                    "{}{}  Lv {}",
                    match (discovery.snapshot_complete(), discovery.milestone(monster.id as u16).is_some()) {
                        (_, true) => "[Discovered] ",
                        (true, false) => "[Not encountered] ",
                        (false, false) => "[Sync pending] ",
                    },
                    display_name(&monster.name, &monster.sprite_name),
                    monster.level,
                ),
                kind: "monster".to_owned(),
                id: monster.id,
            }),
        );
    } else if category == "Skills" {
        rows.extend(data.search_skills(&query, MAX_RESULTS).into_iter().map(|skill| GuideResult {
            label: display_name(&skill.description, &skill.name),
            kind: "skill".to_owned(),
            id: skill.id as u32,
        }));
    } else if category == "Status Effects" {
        rows.extend(data.search_statuses(&query, MAX_RESULTS).into_iter().map(|status| GuideResult {
            label: status.name.clone(),
            kind: "status".to_owned(),
            id: status.id,
        }));
    } else if category == "NPCs" {
        rows.extend(data.search_npcs(&query, MAX_RESULTS).into_iter().map(|npc| GuideResult {
            label: format!(
                "{}  ({} on {} at {}, {})",
                npc.display_name,
                npc_kind_label(&npc.declared_type),
                npc.map,
                npc.x,
                npc.y
            ),
            kind: "npc".to_owned(),
            id: npc.id,
        }));
    } else if category == "Maps" {
        rows.extend(
            crate::world::navigation_graph()
                .maps
                .iter()
                .enumerate()
                .filter(|(_, map)| map_matches(&query, map))
                .take(MAX_RESULTS)
                .map(|(index, map)| GuideResult {
                    label: format!(
                        "{}{}  (map)",
                        match (discovery.map_snapshot_complete(), discovery.visited_map(map)) {
                            (_, true) => "[Visited] ",
                            (true, false) => "[Not visited] ",
                            (false, false) => "[Sync pending] ",
                        },
                        map,
                    ),
                    kind: "map".to_owned(),
                    id: index as u32,
                }),
        );
        let templates = scripted_map_templates();
        rows.extend(
            templates
                .iter()
                .enumerate()
                .filter(|(_, map)| map_matches(&query, map))
                .take(MAX_RESULTS.saturating_sub(rows.len()))
                .map(|(index, map)| GuideResult {
                    label: format!("{map}  (instance map template)"),
                    kind: "map-template".to_owned(),
                    id: index as u32,
                }),
        );
    } else if category == "Jobs" {
        rows.extend(
            job_names()
                .filter(|(_, name)| job_matches(&query, name))
                .take(MAX_RESULTS)
                .map(|(id, name)| GuideResult {
                    label: format!("{name}  (job)"),
                    kind: "job".to_owned(),
                    id: id as u32,
                }),
        );
    } else if category == "Services" {
        rows.extend(data.search_services(&query, MAX_RESULTS).into_iter().filter_map(|service| {
            let index = data.npc_service_index_by_id(&service.id)?;
            Some(GuideResult {
                label: format!(
                    "{}{}  (Service: {})",
                    match (discovery.service_snapshot_complete(), discovery.visited_service(&service.id)) {
                        (_, true) => "[Visited] ",
                        (true, false) => "[Not visited] ",
                        (false, false) => "[Sync pending] ",
                    },
                    service.title,
                    service_kind_words(&service.service_kind),
                ),
                kind: "service".to_owned(),
                id: index as u32,
            })
        }));
    } else if category == "Rumors" {
        rows.extend(
            data.search_rumors(&query, MAX_RESULTS)
                .into_iter()
                .filter(|rumor| !rumor.is_story_spoiler)
                .map(|rumor| GuideResult {
                    label: format!(
                        "{}{}  (Rumor: {})",
                        match (discovery.rumor_snapshot_complete(), discovery.unlocked_rumor(rumor.id)) {
                            (_, true) => "[Discovered] ",
                            (true, false) => "[Undiscovered] ",
                            (false, false) => "[Sync pending] ",
                        },
                        rumor.title,
                        rumor.category,
                    ),
                    kind: "rumor".to_owned(),
                    id: rumor.id,
                }),
        );
    } else if category == "Refinement" {
        if refinement_query_matches(&query) {
            rows.push(GuideResult {
                label: "Weapon refinement odds and rules".to_owned(),
                kind: "mechanic".to_owned(),
                id: 100,
            });
            if data.refinement.armor.is_some() {
                rows.push(GuideResult {
                    label: "Armor refinement odds and rules".to_owned(),
                    kind: "mechanic".to_owned(),
                    id: 0,
                });
            }
        }
        for weapon in &data.refinement.weapon_levels {
            let label = format!("Weapon Level {} refinement ({})", weapon.weapon_level, weapon.material);
            if query.is_empty() || label.to_lowercase().contains(&query) || weapon.material.to_lowercase().contains(&query) {
                rows.push(GuideResult {
                    label,
                    kind: "mechanic".to_owned(),
                    id: weapon.weapon_level as u32,
                });
            }
        }
    } else if category == "Effective Rules" {
        rows.extend(
            data.server_rules
                .iter()
                .enumerate()
                .filter(|(_, rule)| server_rule_matches(rule, &query))
                .take(MAX_RESULTS)
                .map(|(index, rule)| GuideResult {
                    label: format!("{} ({})", rule.title, rule.category),
                    kind: "server-rule".to_owned(),
                    id: index as u32,
                }),
        );
    } else if category == "Mechanics" {
        if query.is_empty() || "encyclopedia coverage evidence".contains(&query) {
            rows.push(GuideResult {
                label: "Encyclopedia coverage and evidence labels".to_owned(),
                kind: "coverage".to_owned(),
                id: 1,
            });
        }
        if refinement_query_matches(&query) {
            rows.push(GuideResult {
                label: "Weapon refinement odds and rules".to_owned(),
                kind: "mechanic".to_owned(),
                id: 100,
            });
            if data.refinement.armor.is_some() {
                rows.push(GuideResult {
                    label: "Armor refinement odds and rules".to_owned(),
                    kind: "mechanic".to_owned(),
                    id: 0,
                });
            }
        }
        rows.extend(
            data.server_rules
                .iter()
                .enumerate()
                .filter(|(_, rule)| server_rule_matches(rule, &query))
                .map(|(index, rule)| GuideResult {
                    label: format!("{} ({})", rule.title, rule.category),
                    kind: "server-rule".to_owned(),
                    id: index as u32,
                }),
        );
    } else if category == "Quests" {
        let quest_log_path = client_state().quest_log();
        let active_quests = state.get(&quest_log_path).quests();
        let active_ids = active_quests
            .iter()
            .map(|quest| quest.quest_id)
            .collect::<std::collections::HashSet<_>>();
        rows.extend(data.search_quests(&query, MAX_RESULTS).into_iter().map(|quest| GuideResult {
            label: format!(
                "{}{}",
                quest.name,
                if active_ids.contains(&quest.id) { " — Active" } else { "" }
            ),
            kind: "quest".to_owned(),
            id: quest.id,
        }));
        let listed_ids = rows.iter().map(|row| row.id).collect::<std::collections::HashSet<_>>();
        let active = active_quests
            .iter()
            .filter(|quest| query.is_empty() || quest.name().to_lowercase().contains(&query) || quest.quest_id.to_string().contains(&query))
            .filter(|quest| !listed_ids.contains(&quest.quest_id))
            .take(MAX_RESULTS.saturating_sub(rows.len()))
            .map(|quest| GuideResult {
                label: quest.name().to_owned(),
                kind: "quest".to_owned(),
                id: quest.quest_id,
            });
        rows.extend(active);
    } else {
        let cards_only = category == "Cards";
        let matches = if cards_only {
            data.search_cards(&query, MAX_RESULTS)
        } else {
            data.search_items(&query, MAX_RESULTS)
        };
        rows.extend(
            matches
                .into_iter()
                .filter(|item| cards_only || item.item_type != "IT_CARD")
                .map(|item| GuideResult {
                    label: display_name(&item.name, &item.aegis_name),
                    kind: if cards_only { "card" } else { "item" }.to_owned(),
                    id: item.id,
                }),
        );
    }
    state.update_value(path.results(), rows);
    state.update_value(path.detail(), vec![
        "Select an entry to see its verified fields and source.".to_owned(),
    ]);
}

fn search_all_categories(
    query: &str,
    discovery: &crate::state::discovery::DiscoveryState,
    active_quests: &[crate::state::quests::QuestEntry],
) -> Vec<GuideResult> {
    let data = reference_data();
    let mut rows = Vec::new();

    if "encyclopedia coverage evidence".contains(query) {
        rows.push(GuideResult {
            label: "Encyclopedia coverage and evidence labels (Mechanics)".to_owned(),
            kind: "coverage".to_owned(),
            id: 1,
        });
    }

    if refinement_query_matches(query) {
        rows.push(GuideResult {
            label: "Weapon refinement odds and rules (Mechanics)".to_owned(),
            kind: "mechanic".to_owned(),
            id: 100,
        });
        if data.refinement.armor.is_some() {
            rows.push(GuideResult {
                label: "Armor refinement odds and rules (Mechanics)".to_owned(),
                kind: "mechanic".to_owned(),
                id: 0,
            });
        }
    }
    rows.extend(
        data.server_rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| server_rule_matches(rule, query))
            .take(all_category_result_slots(&rows))
            .map(|(index, rule)| GuideResult {
                label: format!("{} (Mechanics: {})", rule.title, rule.category),
                kind: "server-rule".to_owned(),
                id: index as u32,
            }),
    );

    rows.extend(
        data.search_monsters(query, all_category_result_slots(&rows))
            .into_iter()
            .map(|monster| GuideResult {
                label: format!(
                    "{}  (Monster, Lv {})",
                    display_name(&monster.name, &monster.sprite_name),
                    monster.level
                ),
                kind: "monster".to_owned(),
                id: monster.id,
            }),
    );
    rows.extend(
        data.search_items(query, data.items.len())
            .into_iter()
            .filter(|item| item.item_type != "IT_CARD")
            .take(all_category_result_slots(&rows))
            .map(|item| GuideResult {
                label: format!("{}  (Item)", display_name(&item.name, &item.aegis_name)),
                kind: "item".to_owned(),
                id: item.id,
            }),
    );
    rows.extend(
        data.search_cards(query, all_category_result_slots(&rows))
            .into_iter()
            .take(all_category_result_slots(&rows))
            .map(|card| GuideResult {
                label: format!("{}  (Card)", display_name(&card.name, &card.aegis_name)),
                kind: "card".to_owned(),
                id: card.id,
            }),
    );
    rows.extend(
        data.search_skills(query, all_category_result_slots(&rows))
            .into_iter()
            .map(|skill| GuideResult {
                label: format!("{}  (Skill)", display_name(&skill.description, &skill.name)),
                kind: "skill".to_owned(),
                id: skill.id as u32,
            }),
    );
    rows.extend(
        data.search_statuses(query, all_category_result_slots(&rows))
            .into_iter()
            .map(|status| GuideResult {
                label: format!("{}  (Status)", status.name),
                kind: "status".to_owned(),
                id: status.id,
            }),
    );
    rows.extend(
        data.search_npcs(query, all_category_result_slots(&rows))
            .into_iter()
            .map(|npc| GuideResult {
                label: format!("{}  ({} on {})", npc.display_name, npc_kind_label(&npc.declared_type), npc.map),
                kind: "npc".to_owned(),
                id: npc.id,
            }),
    );

    rows.extend(
        crate::world::navigation_graph()
            .maps
            .iter()
            .enumerate()
            .filter(|(_, map)| map_matches(query, map))
            .take(all_category_result_slots(&rows))
            .map(|(index, map)| GuideResult {
                label: format!("{map}  (Map{})", if discovery.visited_map(map) { ", Visited" } else { "" }),
                kind: "map".to_owned(),
                id: index as u32,
            }),
    );
    let templates = scripted_map_templates();
    rows.extend(
        templates
            .iter()
            .enumerate()
            .filter(|(_, map)| map_matches(query, map))
            .take(all_category_result_slots(&rows))
            .map(|(index, map)| GuideResult {
                label: format!("{map}  (Instance map template)"),
                kind: "map-template".to_owned(),
                id: index as u32,
            }),
    );
    if newbie_path_query(query) {
        rows.push(GuideResult {
            label: "First jobs: Academy trainers and which town (Job)".to_owned(),
            kind: "job".to_owned(),
            id: 0,
        });
    }
    rows.extend(
        job_names()
            .filter(|(_, name)| job_matches(query, name))
            .take(all_category_result_slots(&rows))
            .map(|(id, name)| GuideResult {
                label: format!("{name}  (Job)"),
                kind: "job".to_owned(),
                id: id as u32,
            }),
    );
    rows.extend(
        data.search_quests(query, all_category_result_slots(&rows))
            .into_iter()
            .map(|quest| GuideResult {
                label: format!("{}  (Quest)", quest.name),
                kind: "quest".to_owned(),
                id: quest.id,
            }),
    );

    rows.extend(
        data.search_services(query, all_category_result_slots(&rows))
            .into_iter()
            .filter_map(|service| {
                let index = data.npc_service_index_by_id(&service.id)?;
                Some(GuideResult {
                    label: format!(
                        "{}{}  (Service: {})",
                        if discovery.visited_service(&service.id) { "[Visited] " } else { "" },
                        service.title,
                        service_kind_words(&service.service_kind)
                    ),
                    kind: "service".to_owned(),
                    id: index as u32,
                })
            }),
    );
    rows.extend(
        data.search_rumors(query, all_category_result_slots(&rows))
            .into_iter()
            .filter(|rumor| !rumor.is_story_spoiler)
            .map(|rumor| GuideResult {
                label: format!(
                    "{}{}  (Rumor: {})",
                    if discovery.unlocked_rumor(rumor.id) { "[Discovered] " } else { "" },
                    rumor.title,
                    rumor.category
                ),
                kind: "rumor".to_owned(),
                id: rumor.id,
            }),
    );

    let listed_ids = rows
        .iter()
        .filter(|row| row.kind == "quest")
        .map(|row| row.id)
        .collect::<std::collections::HashSet<_>>();
    rows.extend(
        active_quests
            .iter()
            .filter(|quest| query.is_empty() || quest.name().to_lowercase().contains(query) || quest.quest_id.to_string().contains(query))
            .filter(|quest| !listed_ids.contains(&quest.quest_id))
            .take(all_category_result_slots(&rows))
            .map(|quest| GuideResult {
                label: format!("{}  (Active Quest {})", quest.name(), quest.quest_id),
                kind: "quest".to_owned(),
                id: quest.quest_id,
            }),
    );
    rows
}

fn all_category_result_slots(rows: &[GuideResult]) -> usize {
    7.min(MAX_RESULTS.saturating_sub(rows.len()))
}

/// Open the Guide on a monster, item or map from a chat link (F23). The key
/// was validated when the link was parsed; an entry that has since vanished
/// shows the same "unavailable" line as an item.
pub fn open_guide_entry<A>(state: &State<ClientState>, path: A, kind: super::GuideLinkKind, key: &str, town_pois: &[TownPoi])
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy,
{
    let data = reference_data();
    let (category, detail) = match kind {
        super::GuideLinkKind::Item => {
            if let Ok(item_id) = key.parse() {
                open_item_entry(state, path, item_id);
            }
            return;
        }
        super::GuideLinkKind::Monster => (
            "Monsters",
            key.parse()
                .ok()
                .and_then(|id| data.monster_by_id(id))
                .map(monster_details)
                .unwrap_or_else(|| vec!["Reference entry unavailable.".to_owned()]),
        ),
        super::GuideLinkKind::Map => ("Maps", map_details(key, town_pois)),
    };
    state.update_value(path.category(), category.to_owned());
    state.update_value(path.query(), key.to_owned());
    run_search_for(state, path, key, category);
    state.update_value(path.detail(), detail);
}

/// Select an item from another in-game surface, populate the Guide search and
/// detail panes, and leave the guide ready to continue browsing.
pub fn open_item_entry<A>(state: &State<ClientState>, path: A, item_id: u32)
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy,
{
    state.update_value(path.category(), "Items".to_owned());
    // Search by name, so the box shows what the player would type, not "501".
    let data = reference_data();
    let query = data
        .item_by_id(item_id)
        .or_else(|| data.card_by_id(item_id))
        .map(|item| display_name(&item.name, &item.aegis_name))
        .unwrap_or_else(|| item_id.to_string());
    run_search_for(state, path, &query, "Items");
    state.update_value(path.query(), query);
    let data = reference_data();
    let detail = data
        .item_by_id(item_id)
        .map(|item| item_details(item, false))
        .or_else(|| data.card_by_id(item_id).map(|item| item_details(item, true)))
        .unwrap_or_else(|| vec!["Reference entry unavailable.".to_owned()]);
    state.update_value(path.detail(), detail);
}

impl<A> CustomWindow<ClientState> for AdventureGuideWindow<A>
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::AdventureGuide)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;
        struct GuideSearchBox;
        let path = self.state_path;
        let library = self.library;
        let set_category = |category: &'static str| {
            move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
                state.update_value(path.category(), category.to_owned());
                let query = state.get(&path.query()).clone();
                run_search_for(state, path, &query, category);
            }
        };
        // A category button looks pressed while it is the open one, like tabs.
        let category_button = |label: &'static str, category: &'static str| {
            button! {
                text: label,
                event: set_category(category),
                disabled: ComputedSelector::new_default(move |state: &ClientState| rust_state::PathExt::follow_safe(&path.category(), state) == category),
            }
        };
        window! {
            title: "Adventure Guide",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 640.0,
            maximum_width: 1400.0,
            elements: (
                text_box! { ghost_text: "Search by name", state: path.query(), input_handler: LiveSearchHandler { path }, focus_id: GuideSearchBox, overflow_behavior: OverflowBehavior::Shrink },
                split! { gaps: theme().window().gaps(), children: (
                    category_button("All", "All"),
                    category_button("Monsters", "Monsters"),
                    category_button("Items", "Items"),
                    category_button("Cards", "Cards"),
                    category_button("Maps", "Maps"),
                ) },
                split! { gaps: theme().window().gaps(), children: (
                    category_button("NPCs", "NPCs"),
                    category_button("Quests", "Quests"),
                    category_button("Services", "Services"),
                    category_button("Rumors", "Rumors"),
                    category_button("Jobs", "Jobs"),
                ) },
                split! { gaps: theme().window().gaps(), children: (
                    category_button("Skills", "Skills"),
                    category_button("Statuses", "Status Effects"),
                    category_button("Refining", "Refinement"),
                    category_button("Server rules", "Effective Rules"),
                    category_button("How things work", "Mechanics"),
                ) },
                // Side by side, not stacked: a scroll view takes all the height left
                // in the window, so a details view placed below the results list
                // got none and every entry looked empty.
                split! { gaps: theme().window().gaps(), children: (
                    scroll_view! { children: GuideResultList { state_path: path, library: library.clone(), elements: Vec::new() } },
                    scroll_view! { children: GuideLines::new(path.detail()) },
                ) },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GuideResult, ReferenceItem, display_name, item_details, job_matches, job_names, map_details, monster_details, parse_guide_link,
        parse_route_cell_link, quest_details, quest_reference_details, reference_data, refinement_details, resolve_details, rumor_details,
        search_all_categories, server_rule_details, skill_details, status_details,
    };
    use crate::dm::reference_data::{ReferenceQuest, ReferenceQuestTarget};
    use crate::state::discovery::DiscoveryState;
    use crate::state::quests::{QuestEntry, QuestHuntObjectiveEntry, QuestRequirementEntry};
    use crate::world::{TownPoi, TownPoiKind};

    #[test]
    fn each_keystroke_yields_the_text_to_search_for() {
        use super::{MAX_QUERY, next_query};
        assert_eq!(next_query("pori", 'n').as_deref(), Some("porin"));
        assert_eq!(next_query("poring", '\x08').as_deref(), Some("porin"));
        assert_eq!(next_query("", '\x08').as_deref(), Some(""), "backspace on empty stays empty");
        assert_eq!(
            next_query("poring", '\x0d').as_deref(),
            Some("poring"),
            "Enter searches the same text"
        );
        assert_eq!(next_query("poring", '\x01'), None, "other control keys do nothing");
        let full = "x".repeat(MAX_QUERY);
        assert_eq!(next_query(&full, 'y'), None, "the length is capped");
    }

    #[test]
    fn source_notes_are_told_apart_from_player_information() {
        use super::is_source_note;
        assert!(is_source_note(
            "Source: bundled Hercules refine_db.conf (Armors) and npc/merchants/refine.txt.",
            false
        ));
        assert!(is_source_note("Internal script name: Kafra#prt", false));
        assert!(
            is_source_note("  src/map/skill.c:1234", true),
            "indented call sites under their header"
        );
        assert!(!is_source_note("  src/map/skill.c:1234", false));
        // Player information stays on the page.
        assert!(!is_source_note("persists through death", false));
        assert!(!is_source_note(
            "Also placed by events or quests at (conditions not shown):",
            false
        ));
        assert!(!is_source_note("Level 1   HP 60", false));
    }

    #[test]
    fn item_pages_show_a_name_and_a_type_in_words_not_ids_or_constants() {
        let item = reference_data().item_by_id(501).expect("Red Potion");
        let lines = item_details(item, false);
        assert_eq!(lines[0], "Red Potion");
        assert!(lines[1].starts_with("Type: Healing"), "{lines:?}");
        assert!(
            lines.iter().all(|line| !line.contains("(ID ") && !line.contains("IT_")),
            "{lines:?}"
        );
    }

    #[test]
    fn guide_labels_untranslated_script_effects_and_missing_fields() {
        let item = ReferenceItem {
            id: 501,
            aegis_name: "RED_POTION".to_owned(),
            name: "Red Potion".to_owned(),
            item_type: "IT_HEALING".to_owned(),
            buy: 0,
            sell: None,
            weight: 0,
            atk: None,
            matk: None,
            defense: None,
            slots: None,
            job: Default::default(),
            gender: None,
            loc: None,
            equip_level: None,
            refine: None,
            weapon_level: None,
            effect_status: "scripted_not_translated".to_owned(),
            effect_summary: None,
            combos: Vec::new(),
            group_contents: Vec::new(),
            contained_in_groups: Vec::new(),
            shops: Vec::new(),
            source: None,
            drops_from: Vec::new(),
        };
        let lines = item_details(&item, false).join("\n");
        assert!(lines.contains("not translated"));
        assert!(!lines.contains("not documented yet"));
        let mut undocumented = item;
        undocumented.effect_status.clear();
        assert!(item_details(&undocumented, false).join("\n").contains("not documented yet"));
        assert_eq!(display_name("", "PORING"), "PORING");
    }

    #[test]
    fn item_search_accepts_exact_numeric_ids_without_matching_other_rows() {
        let item = ReferenceItem {
            id: 501,
            aegis_name: "RED_POTION".to_owned(),
            name: "Red Potion".to_owned(),
            item_type: "IT_HEALING".to_owned(),
            buy: 0,
            sell: None,
            weight: 0,
            atk: None,
            matk: None,
            defense: None,
            slots: None,
            job: Default::default(),
            gender: None,
            loc: None,
            equip_level: None,
            refine: None,
            weapon_level: None,
            effect_status: String::new(),
            effect_summary: None,
            combos: Vec::new(),
            group_contents: Vec::new(),
            contained_in_groups: Vec::new(),
            shops: Vec::new(),
            source: None,
            drops_from: Vec::new(),
        };
        assert!(item.matches_query("501"));
        assert!(!item.matches_query("502"));
        assert!(item.matches_query("red pot"));
    }

    #[test]
    fn guide_drop_lists_are_clickable_cross_references() {
        let data = reference_data();
        let monster = data
            .monsters
            .iter()
            .find(|monster| !monster.drops.is_empty())
            .expect("monster drop data");
        let item_link = monster_details(monster)
            .into_iter()
            .find_map(|line| line.strip_prefix("@guide:item:").map(str::to_owned))
            .expect("monster drop cross-link");
        let (target, _) = item_link.split_once('|').expect("well-formed item cross-link");
        let item_id = target.parse::<u32>().expect("numeric item id");
        assert!(data.item_by_id(item_id).is_some() || data.card_by_id(item_id).is_some());

        let item = data
            .items
            .iter()
            .find(|item| {
                item.drops_from.iter().any(|source| {
                    data.monster_by_id(source.monster_id)
                        .is_some_and(|monster| !monster.spawn_regions.is_empty())
                })
            })
            .expect("item drop source data");
        let item_detail = item_details(item, false);
        let monster_link = item_detail
            .iter()
            .cloned()
            .find_map(|line| line.strip_prefix("@guide:monster:").map(str::to_owned))
            .expect("item source cross-link");
        let (target, _) = monster_link.split_once('|').expect("well-formed monster cross-link");
        let monster_id = target.parse::<u32>().expect("numeric monster id");
        assert!(data.monster_by_id(monster_id).is_some());
        let source_map = data
            .monster_by_id(monster_id)
            .and_then(|monster| monster.spawn_regions.first())
            .map(|region| region.map.as_str())
            .expect("drop-source monster map");
        assert!(
            crate::world::navigation_graph()
                .maps
                .iter()
                .any(|known| known.eq_ignore_ascii_case(source_map))
        );
        assert!(item_detail.iter().any(|line| line == &format!("@route:{source_map}")));

        let detail = resolve_details(&GuideResult {
            label: String::new(),
            kind: "monster".to_owned(),
            id: monster_id,
        });
        assert!(!detail.is_empty());
    }

    #[test]
    fn monster_skill_details_link_verified_skill_ids_and_explain_triggers() {
        let monster = reference_data().monster_by_id(1002).expect("Poring bestiary record");
        let details = monster_details(monster);
        assert!(
            details
                .iter()
                .any(|line| line == &format!("@hunting-goal:{}|Add to my hunting goals", monster.id))
        );
        let skill_link = details
            .iter()
            .find(|line| line.starts_with("@guide:skill:184|"))
            .and_then(|line| parse_guide_link(line))
            .expect("Poring's Water Attack should link to its verified skill record");
        assert_eq!(skill_link.kind, "skill");
        assert_eq!(skill_link.id, 184);
        assert!(details.iter().any(|line| line.contains("20.00%")));
        assert!(details.iter().any(|line| line.contains("retry delay 5.0s")));
        assert!(details.iter().any(|line| line.contains("Configured chance: 20.00% chance")));
        assert!(
            resolve_details(&skill_link)
                .iter()
                .any(|line| line.contains("Water Attribute Attack"))
        );
    }

    #[test]
    fn guide_searches_existing_skill_export_and_shows_verified_tooltip_fields() {
        let matches = reference_data().search_skills("Fire Bolt", 10);
        let skill = matches.iter().find(|skill| skill.id == 19).expect("Fire Bolt skill row");
        let detail = skill_details(skill).join("\n");
        assert!(detail.contains("Fire Bolt"));
        assert!(detail.contains("SP "));
        assert!(detail.contains("Source: bundled Hercules skill database export."));
        assert!(detail.contains("Source: db/re/skill_db.conf (Id=19)."));
    }

    #[test]
    fn guide_skill_details_display_reviewed_formulas_and_unreviewed_flags() {
        let data = reference_data();

        // Heal (28) has a reviewed formula
        let heal = data.search_skills("AL_HEAL", 1).into_iter().next().expect("Heal skill");
        let heal_details = skill_details(heal).join("\n");
        assert!(heal_details.contains("Heal's Renewal healing formula"), "{heal_details}");
        assert!(
            heal_details.contains("AL_HEAL (id 28) computes its healed HP"),
            "{heal_details}"
        );
        assert!(heal_details.contains("Worked example:"), "{heal_details}");
        assert!(heal_details.contains("Source: reviewed as conditional."), "{heal_details}");
        assert!(heal_details.contains("Source: src/map/skill.c:"), "{heal_details}");

        // Fire Bolt (19) has a reviewed formula
        let fire_bolt = data.search_skills("MG_FIREBOLT", 1).into_iter().next().expect("Fire Bolt skill");
        let bolt_details = skill_details(fire_bolt).join("\n");
        assert!(bolt_details.contains("Source: reviewed as "), "{bolt_details}");
        assert!(bolt_details.contains("Fire/Cold/Lightning Bolt"), "{bolt_details}");

        // Basic Skill (1) is unreviewed and should be explicitly flagged
        let basic = data.search_skills("NV_BASIC", 1).into_iter().next().expect("Basic Skill");
        let basic_details = skill_details(basic).join("\n");
        assert!(
            basic_details.contains("No damage formula is written up for this skill."),
            "{basic_details}"
        );
        assert!(
            basic_details.contains("Source: no reviewed formula in the Hercules renewal engine."),
            "{basic_details}"
        );
    }

    #[test]
    fn guide_skill_details_display_direct_prerequisites_and_source_records() {
        let data = reference_data();

        // Fire Wall (18) requires Fire Ball (17) Lv 5 and Sight (10) Lv 1
        let firewall = data.skill_by_id(18).expect("Fire Wall skill");
        let fw_details = skill_details(firewall).join("\n");
        assert!(fw_details.contains("Requires:"), "{fw_details}");
        let fire_ball = data.skill_by_id(17).expect("Fire Ball");
        let sight = data.skill_by_id(10).expect("Sight");
        assert!(
            fw_details.contains(&format!(
                "@guide:skill:17|{} level 5",
                display_name(&fire_ball.description, &fire_ball.name)
            )),
            "{fw_details}"
        );
        assert!(
            fw_details.contains(&format!(
                "@guide:skill:10|{} level 1",
                display_name(&sight.description, &sight.name)
            )),
            "{fw_details}"
        );
        assert!(fw_details.contains("Source: db/re/skill_db.conf (Id=18)."), "{fw_details}");

        // Test clickable link navigation to prerequisite
        let prereq_link = parse_guide_link(&format!(
            "@guide:skill:17|{} level 5",
            display_name(&fire_ball.description, &fire_ball.name)
        ))
        .expect("skill link");
        assert_eq!(prereq_link.kind, "skill");
        assert_eq!(prereq_link.id, 17);
        let resolved = resolve_details(&prereq_link).join("\n");
        assert!(resolved.contains("Fire Ball"), "{resolved}");
    }

    #[test]
    fn status_and_skill_reference_links_are_navigable_in_both_directions() {
        let blessing = reference_data()
            .search_skills("AL_BLESSING", 1)
            .into_iter()
            .next()
            .expect("Blessing skill row");
        let skill_lines = skill_details(blessing);
        let status_link = skill_lines
            .iter()
            .find(|line| line.starts_with("@guide:status:"))
            .expect("skill links to its status icon record");
        let target = parse_guide_link(status_link).expect("status link is an actionable Guide link");
        assert_eq!(target.kind, "status");
        let status_lines = resolve_details(&target).join("\n");
        assert!(
            status_lines.lines().any(|line| line == "Source: server status SC_BLESSING."),
            "{status_lines}"
        );
        assert!(!status_lines.contains("status ID"), "no raw ids for players: {status_lines}");
        assert!(status_lines.contains("@guide:skill:34|Applied by Blessing"));
    }

    #[test]
    fn status_guide_links_explicit_skill_db_status_change_fields() {
        let data = reference_data();
        for (status_name, skill_name, skill_id) in [
            ("Adaptation", "BD_ADAPTATION", 304),
            ("Assumptio", "CASH_ASSUMPTIO", 691),
            ("Basilica Buff", "HP_BASILICA", 362),
        ] {
            let status = data
                .search_statuses(status_name, 10)
                .into_iter()
                .find(|status| status.name == status_name)
                .expect("status icon row");
            let detail = status_details(status).join("\n");
            assert!(detail.contains(skill_name));
            assert!(detail.contains(&format!("@guide:skill:{skill_id}|Applied by")));

            let skill = data
                .search_skills(skill_name, 10)
                .into_iter()
                .find(|skill| skill.name == skill_name)
                .expect("skill row");
            assert!(
                skill_details(skill)
                    .iter()
                    .any(|line| line == &format!("@guide:status:{}|{}", status.id, status.name))
            );
            assert_eq!(data.search_statuses(skill_name, 10)[0].id, status.id);
        }
    }

    #[test]
    fn guide_map_details_offer_routing_for_graph_maps() {
        let maps = &crate::world::navigation_graph().maps;
        let index = maps.iter().position(|map| map == "prt_fild08").expect("known map");
        let result = GuideResult {
            label: "prt_fild08".to_owned(),
            kind: "map".to_owned(),
            id: index as u32,
        };
        let detail = resolve_details(&result);
        assert!(detail.iter().any(|line| line == "Map: prt_fild08"));
        assert!(detail.iter().any(|line| line.starts_with("Suggested level:")));
        assert!(detail.iter().any(|line| line.starts_with("Static population:")));
        assert!(detail.iter().any(|line| line.starts_with("Exits:")));
        assert!(detail.iter().any(|line| line.starts_with("Exit at (")));
        assert!(
            detail
                .iter()
                .any(|line| line.contains("Also placed by scripts") || line.contains("No script places a monster"))
        );
        assert!(detail.iter().any(|line| line == "@route:prt_fild08"));
        assert!(detail.iter().any(|line| line.starts_with("@route:")));
    }

    #[test]
    fn guide_map_details_list_hidden_chests_as_exact_cell_routes() {
        let maps = &crate::world::navigation_graph().maps;
        let index = maps.iter().position(|map| map == "prt_fild01").expect("known map");
        let result = GuideResult {
            label: "prt_fild01".to_owned(),
            kind: "map".to_owned(),
            id: index as u32,
        };
        let detail = resolve_details(&result);
        assert!(detail.iter().any(|line| line.starts_with("Hidden treasure chests (1):")));
        let route = detail
            .iter()
            .find(|line| line.contains("Route to hidden chest"))
            .and_then(|line| parse_route_cell_link(line))
            .expect("the chest has an exact-cell route");
        assert_eq!((route.0.as_str(), route.1, route.2), ("prt_fild01", 146, 126));

        // A map with no chest gets no chest section.
        let index = maps.iter().position(|map| map == "prontera").expect("known map");
        let none = resolve_details(&GuideResult {
            label: "prontera".to_owned(),
            kind: "map".to_owned(),
            id: index as u32,
        });
        assert!(!none.iter().any(|line| line.starts_with("Hidden treasure chests")));
    }

    #[test]
    fn guide_map_details_show_npc_service_action_and_requirement() {
        let index = crate::world::navigation_graph()
            .maps
            .iter()
            .position(|map| map == "izlude")
            .expect("Izlude is a known map");
        let result = GuideResult {
            label: "izlude".to_owned(),
            kind: "map".to_owned(),
            id: index as u32,
        };
        let detail = resolve_details(&result);

        // Izlude's verified outbound NPC services: the Byalan ferry, the
        // Malangdo cat fleet (2026-09-27), four Kafra teleport destinations
        // from `F_KafSet`'s Izlude branch, and the Jawaii honeymoon boat
        // (2026-10-03).
        assert!(detail.iter().any(|line| line == "Travel services: 7"));
        assert!(
            detail
                .iter()
                .any(|line| line.contains("choose the teleport service, then Al De Baran."))
        );
        assert!(
            detail
                .iter()
                .any(|line| line.contains("choose Byalan Island.") && line.contains("(conditional)"))
        );
        assert!(detail.iter().any(|line| line == "Requirement: Costs 150 zeny."));
        assert!(detail.iter().any(|line| line.contains("npc/re/cities/izlude.txt:37")));
        assert!(detail.iter().any(|line| line == "@route:izlu2dun"));

        assert!(
            detail
                .iter()
                .any(|line| line.contains("travel to Malangdo.") && line.contains("(conditional)"))
        );
        assert!(detail.iter().any(|line| line.starts_with("Requirement: Costs up to 1000 zeny")));
        assert!(detail.iter().any(|line| line.contains("npc/re/cities/malangdo.txt:176")));
        assert!(detail.iter().any(|line| line == "@route:malangdo"));
    }

    #[test]
    fn guide_map_details_offer_exact_routes_to_valid_towninfo_facilities() {
        let pois = [
            TownPoi {
                name: "Kafra Employee".to_owned(),
                x: 156,
                y: 191,
                kind: TownPoiKind::Kafra,
            },
            TownPoi {
                name: "Invalid negative point".to_owned(),
                x: -1,
                y: 10,
                kind: TownPoiKind::Other,
            },
        ];
        let details = map_details("prontera", &pois);
        let facility_route = details
            .iter()
            .find(|line| line.starts_with("@route-cell:"))
            .expect("valid Towninfo POI should have a route action");
        assert!(facility_route.contains("Route to Kafra Employee"));
        assert!(!details.iter().any(|line| line.contains("Invalid negative point")));
        assert_eq!(
            parse_route_cell_link(facility_route),
            Some(("prontera".to_owned(), 156, 191, "Route to Kafra Employee — prontera".to_owned()))
        );
    }

    #[test]
    fn status_effect_search_shows_verified_server_metadata_and_marks_gaps() {
        let data = reference_data();
        let blessing = data
            .search_statuses("blessing", 10)
            .into_iter()
            .next()
            .expect("Blessing status name");
        let details = super::status_details(blessing).join("\n");
        assert!(details.contains("How the server treats this status:"));
        assert!(
            details.lines().any(|line| line == "Source: server status SC_BLESSING."),
            "{details}"
        );
        assert!(details.contains(
            "Server lifecycle rules: classified by the server as a buff; cannot be applied to boss monsters; not cleared when MADO Gear \
             is removed; cannot be applied while the target is in a no-magic state."
        ));
        assert!(details.contains("Server recalculates these stat groups when this status changes: DEX, HIT, INT, STR."));
        assert!(details.contains("@guide:skill:34|Applied by Blessing"));
        assert!(details.contains("sc_start call sites are not exhaustive"));
        assert!(details.contains("src/map/skill.c:"));
        assert!(
            details
                .contains("Its exact effect, duration, per-level odds, complete sources, interactions, and cures are not documented yet.")
        );
        assert_eq!(data.statuses.iter().filter(|status| !status.iconless).count(), 700);
        assert!(
            data.search_statuses(&blessing.id.to_string(), 1)
                .iter()
                .any(|status| status.id == blessing.id)
        );
        assert_eq!(data.search_statuses("AL_BLESSING", 1)[0].id, blessing.id);

        let name_only = data.search_statuses("Stormkick Ready", 1).remove(0);
        assert!(
            super::status_details(name_only)
                .join("\n")
                .contains("server status-icon name only; no matching sc_config record")
        );
    }

    #[test]
    fn guide_job_entries_cross_link_to_verified_skill_trees() {
        let (id, name) = job_names().find(|(_, name)| *name == "Knight").expect("Knight job");
        let detail = resolve_details(&GuideResult {
            label: name.to_owned(),
            kind: "job".to_owned(),
            id: id as u32,
        });
        assert!(detail[0].contains("Knight"));
        assert!(detail.iter().any(|line| line.contains("inherited skills")));
        assert!(detail.iter().any(|line| line == "STR +8: job levels 4, 10, 15, 21, 27, 33, 46, 47"));
        assert!(
            detail
                .iter()
                .any(|line| line.starts_with("Job-level stat bonuses through job level 50:"))
        );
        assert!(detail.iter().any(|line| line.starts_with("@guide:skill:")));
        assert!(detail.iter().any(|line| line.contains("requires")));
        let bash_id = reference_data()
            .job_skill_tree_by_id(id)
            .and_then(|tree| tree.skills.iter().find(|skill| skill.name == "SM_BASH"))
            .map(|skill| skill.skill_id as u32)
            .expect("Knight tree includes inherited Swordsman-line skill");
        assert!(detail.iter().any(|line| line.starts_with(&format!("@guide:skill:{bash_id}|"))));
        let linked_skill = detail
            .iter()
            .find_map(|line| parse_guide_link(line))
            .expect("job skill must be clickable");
        assert_eq!(linked_skill.kind, "skill");
        // The link resolves to that skill's own page (its tooltip header),
        // not a fallback. Checked by content: the page no longer shows ids.
        let skill = reference_data().skill_by_id(linked_skill.id).expect("linked skill exists");
        let details = resolve_details(&linked_skill);
        assert!(
            details.first().is_some_and(|line| line.contains(&skill.description)),
            "{details:?}"
        );
    }

    #[test]
    fn guide_quest_details_show_server_hunt_progress_and_known_routes() {
        let quest = QuestEntry {
            quest_id: 42,
            name: "Poring hunt".to_owned(),
            requirements: vec![QuestRequirementEntry {
                item_id: ragnarok_packets::ItemId(501),
                item_name: "Red Potion".to_owned(),
                needed: 2,
            }],
            hunt_objectives: vec![QuestHuntObjectiveEntry {
                monster_id: 1002,
                monster_name: "Poring".to_owned(),
                total_count: 10,
                current_count: 3,
            }],
            location: None,
            guidance: Vec::new(),
        };
        let detail = quest_details(&quest);
        assert!(detail.iter().any(|line| line.contains("Poring — 3/10")));
        assert!(detail.iter().any(|line| line.starts_with("@route:")));
        assert!(detail.iter().any(|line| line.starts_with("@guide:monster:")));
        assert!(detail.iter().any(|line| line.starts_with("@guide:item:501|")));
    }

    #[test]
    fn guide_static_quest_details_include_searchable_targets_and_route_links() {
        let data = reference_data();
        let quest = data.quest_by_id(1100).expect("tracked reference quest");
        let detail = quest_reference_details(quest);
        assert_eq!(detail[0], quest.name);
        assert!(detail.iter().any(|line| line.starts_with("@guide:monster:")) || detail.iter().any(|line| line.starts_with("@route:")));
        assert!(detail.iter().any(|line| line == "No quest giver or turn-in is listed."));

        let quest_link = parse_guide_link("@guide:quest:1100|Open quest").expect("quest links are supported");
        assert_eq!(quest_link.kind, "quest");
        assert!(!resolve_details(&quest_link).is_empty());
    }

    #[test]
    fn first_step_towards_a_new_world_routes_to_hun() {
        let quest = reference_data().quest_by_id(7472).expect("First step towards a new world");
        let detail = quest_reference_details(quest).join("\n");
        assert!(detail.contains("Turn in to: Hun on izlude at (122, 207)."), "{detail}");
        assert!(detail.contains("@route-cell:izlude:122:207|Route to Hun — izlude"), "{detail}");
        assert!(detail.contains("200 base EXP"), "{detail}");
        assert!(!detail.contains("No NPC giver"), "{detail}");
        assert!(crate::world::newbie_quest_guide(1).is_none());
    }

    #[test]
    fn guide_quest_with_unsupported_map_falls_back_to_monster_routes() {
        let quest = ReferenceQuest {
            id: 1,
            name: "Test quest".to_owned(),
            npc_references: Vec::new(),
            targets: vec![ReferenceQuestTarget {
                mob_id: Some(1002),
                monster_name: "Poring".to_owned(),
                monster_data_known: Some(true),
                count: 3,
                level_range: None,
                map_name: Some("not_in_navigation_graph".to_owned()),
            }],
            item_reward_candidates: Vec::new(),
            flow_review: None,
        };
        let detail = quest_reference_details(&quest);
        assert!(detail.iter().any(|line| line.contains("no loaded navigation route")));
        assert!(detail.iter().any(|line| line.starts_with("@guide:monster:1002|")));
    }

    #[test]
    fn quest_npc_script_reference_offers_an_exact_cell_route() {
        let quest = reference_data().quest_by_id(9030).expect("tracked Lost Puppies offer quest");
        let detail = quest_reference_details(quest);
        let route = detail
            .iter()
            .find_map(|line| parse_route_cell_link(line))
            .expect("NPC script cell has a route action");
        assert_eq!(
            route,
            (
                "brasilis".to_owned(),
                297,
                307,
                "Route to quest offer: Angelo#br — brasilis".to_owned()
            )
        );
        assert!(detail.iter().any(|line| line.contains("Offered by Angelo#br")));
        assert!(detail.iter().any(|line| line.contains("source lines [90]")));
        assert!(
            detail
                .iter()
                .any(|line| line.contains("Requirements or availability may still apply"))
        );
    }

    #[test]
    fn reviewed_quest_turn_in_routes_to_the_source_verified_npc() {
        let quest = reference_data().quest_by_id(9031).expect("tracked Lost Puppies turn-in quest");
        let detail = quest_reference_details(quest);
        assert!(detail.iter().any(|line| line.contains("Turn in to Angelo#br")));
        assert!(
            detail
                .iter()
                .any(|line| line.starts_with("@route-cell:brasilis:297:307|Route to quest turn-in:"))
        );
    }

    #[test]
    fn quest_npc_script_evidence_explains_calls_without_claiming_giver_role() {
        let quest = ReferenceQuest {
            id: 7,
            name: "Test quest".to_owned(),
            targets: Vec::new(),
            npc_references: vec![crate::dm::reference_data::ReferenceQuestNpc {
                name: "Test NPC".to_owned(),
                map_name: "prontera".to_owned(),
                x: 100,
                y: 100,
                source_path: "npc/re/quests/test.txt".to_owned(),
                source_line: 12,
                uses: vec!["setquest".to_owned(), "completequest".to_owned()],
                reviewed_role: None,
                reviewed_source_lines: Vec::new(),
                review_evidence: None,
                verified_reward: None,
            }],
            item_reward_candidates: Vec::new(),
            flow_review: None,
        };
        let detail = quest_reference_details(&quest);
        assert!(detail.iter().any(|line| line.contains("sets quest state, completes quest state")));
        assert!(
            detail
                .iter()
                .any(|line| line.contains("does not prove NPC role or current availability"))
        );
        assert!(detail.iter().any(|line| line.starts_with("@route-cell:prontera:100:100|")));
    }

    #[test]
    fn classic_ailments_without_a_client_icon_are_searchable_statuses() {
        let data = reference_data();
        for (constant, label) in [
            ("SC_STONE", "Stone"),
            ("SC_FREEZE", "Freeze"),
            ("SC_STUN", "Stun"),
            ("SC_SLEEP", "Sleep"),
            ("SC_CURSE", "Curse"),
            ("SC_CONFUSION", "Confusion"),
            ("SC_BLIND", "Blind"),
        ] {
            let status = data
                .search_statuses(constant, 10)
                .into_iter()
                .find(|status| status.statuses.iter().any(|mechanic| mechanic.constant == constant))
                .unwrap_or_else(|| panic!("{constant} is not a guide status"));
            assert!(status.iconless, "{constant}");
            assert_eq!(status.name, label);
            let detail = status_details(status).join("\n");
            assert!(detail.contains(&format!("server status {constant}")), "{detail}");
            assert!(detail.contains("derived from the server constant"), "{detail}");
        }
    }

    #[test]
    fn a_petrify_search_reaches_stone_through_the_skill_that_applies_it() {
        let rows = search_all_categories("petrify", &DiscoveryState::default(), &[]);
        let stone = reference_data()
            .search_statuses("SC_STONE", 10)
            .into_iter()
            .find(|status| status.iconless)
            .expect("stone status");
        assert!(
            rows.iter().any(|row| row.kind == "status" && row.id == stone.id),
            "petrify results: {:?}",
            rows.iter().map(|row| row.label.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_skill_links_the_iconless_status_it_inflicts_and_back() {
        let data = reference_data();
        let stone = data
            .search_statuses("SC_STONE", 10)
            .into_iter()
            .find(|status| status.iconless)
            .expect("stone status");
        let skill_link = stone
            .statuses
            .iter()
            .flat_map(|mechanic| mechanic.status_change_skills.iter().chain(mechanic.associated_skill.iter()))
            .next()
            .expect("a skill that applies stone");
        let skill = data
            .search_skills(&skill_link.name, 10)
            .into_iter()
            .find(|skill| skill.name == skill_link.name)
            .expect("skill row");
        assert!(
            skill_details(skill)
                .iter()
                .any(|line| line == &format!("@guide:status:{}|{}", stone.id, stone.name)),
            "skill page must link the status"
        );
        assert!(
            status_details(stone)
                .iter()
                .any(|line| line.starts_with(&format!("@guide:skill:{}|", skill_link.id)))
        );
    }

    #[test]
    fn iconless_statuses_rank_after_client_visible_ones() {
        let rows = reference_data().search_statuses("basilica", 10);
        let first_iconless = rows.iter().position(|status| status.iconless).expect("an iconless basilica status");
        let last_icon = rows.iter().rposition(|status| !status.iconless).expect("an icon basilica status");
        assert!(last_icon < first_iconless);
    }

    #[test]
    fn alias_rule_is_exact_for_short_terms_and_prefix_from_four_letters() {
        use crate::dm::reference_data::alias_matches;
        assert!(alias_matches("lk", "LK"));
        assert!(!alias_matches("pally", "pal"), "three letters never match by prefix");
        assert!(alias_matches("petrification", "petrif"));
        assert!(!alias_matches("petrification", "tion"));
        assert!(!alias_matches("sg", "s"));
    }

    #[test]
    fn player_words_for_ailments_reach_the_status_entry() {
        let data = reference_data();
        for (word, constant) in [
            ("petrification", "SC_STONE"),
            ("petrified", "SC_STONE"),
            ("frozen", "SC_FREEZE"),
            ("stunned", "SC_STUN"),
            ("asleep", "SC_SLEEP"),
            ("cursed", "SC_CURSE"),
            ("confused", "SC_CONFUSION"),
            ("blindness", "SC_BLIND"),
            ("silenced", "SC_SILENCE"),
            ("poisoned", "SC_POISON"),
            ("bleeding", "SC_BLOODING"),
        ] {
            let rows = search_all_categories(word, &DiscoveryState::default(), &[]);
            let reached = rows.iter().any(|row| {
                row.kind == "status"
                    && data
                        .statuses
                        .iter()
                        .find(|status| status.id == row.id)
                        .is_some_and(|status| status.statuses.iter().any(|mechanic| mechanic.constant == constant))
            });
            assert!(
                reached,
                "{word:?} should reach {constant}; got {:?}",
                rows.iter().map(|row| row.label.as_str()).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn job_abbreviations_reach_their_job_and_every_alias_names_a_real_job() {
        let names: Vec<&str> = job_names().map(|(_, name)| name).collect();
        for alias in reference_data().aliases.iter().filter(|alias| alias.kind == "job") {
            assert!(names.contains(&alias.target.as_str()), "{} -> {}", alias.alias, alias.target);
        }
        let rows = search_all_categories("lk", &DiscoveryState::default(), &[]);
        assert!(rows.iter().any(|row| row.kind == "job" && row.label.starts_with("Lord Knight")));
        assert!(job_matches("pally", "Paladin"));
        assert!(!job_matches("pally", "Priest"));
    }

    #[test]
    fn aliases_do_not_displace_the_established_status_results() {
        let plain = reference_data().search_statuses("stun", 10);
        assert!(plain.iter().any(|status| status.name == "Stun"));
        // "stunned" reaches the same Stun entry through the alias.
        let aliased = reference_data().search_statuses("stunned", 10);
        assert!(aliased.iter().any(|status| status.name == "Stun"));
    }

    /// GDD 9.5: "No result is a dead end." Every `@guide:` link any detail page
    /// emits must open a real entry. This walks every entry of every category.
    #[test]
    fn no_guide_page_links_to_an_entry_that_does_not_exist() {
        let data = reference_data();
        let mk = |kind: &str, id: u32| GuideResult {
            label: String::new(),
            kind: kind.to_owned(),
            id,
        };
        let mut entries: Vec<GuideResult> = Vec::new();
        entries.extend(data.monsters.iter().map(|entry| mk("monster", entry.id)));
        entries.extend(data.items.iter().map(|entry| mk("item", entry.id)));
        entries.extend(data.cards.iter().map(|entry| mk("card", entry.id)));
        entries.extend(data.skills.iter().map(|entry| mk("skill", entry.id as u32)));
        entries.extend(data.statuses.iter().map(|entry| mk("status", entry.id)));
        entries.extend(data.quests.iter().map(|entry| mk("quest", entry.id)));
        entries.extend(data.npcs.iter().map(|entry| mk("npc", entry.id)));
        entries.extend((0..data.npc_services.len()).map(|index| mk("service", index as u32)));
        entries.extend(data.rumors.iter().map(|entry| mk("rumor", entry.id)));
        entries.extend(job_names().map(|(id, _)| mk("job", id as u32)));
        entries.extend((0..crate::world::navigation_graph().maps.len()).map(|index| mk("map", index as u32)));

        let mut dead = Vec::new();
        for entry in &entries {
            for line in resolve_details(entry) {
                let Some(link) = parse_guide_link(&line) else { continue };
                let opens = resolve_details(&link);
                if opens
                    .first()
                    .is_some_and(|first| first.contains("unavailable") || first.contains("Unsupported"))
                {
                    dead.push(format!("{}:{} -> {}:{}", entry.kind, entry.id, link.kind, link.id));
                }
            }
        }
        assert!(
            dead.is_empty(),
            "{} dead guide links, e.g. {:?}",
            dead.len(),
            &dead[..dead.len().min(5)]
        );
    }

    #[test]
    fn rumors_are_not_presented_as_verified_and_link_only_real_entries() {
        let data = reference_data();
        assert!(!data.rumors.is_empty());
        for rumor in &data.rumors {
            // Authored flavour: nothing in the server delivers it.
            assert_ne!(
                rumor.evidence_state,
                crate::dm::reference_data::EvidenceState::Verified,
                "rumor {}",
                rumor.id
            );
            if let Some(id) = rumor.related_monster_id {
                assert!(data.monster_by_id(id).is_some(), "rumor {} monster {id}", rumor.id);
            }
            if let Some(id) = rumor.related_item_id {
                assert!(data.item_by_id(id).is_some(), "rumor {} item {id}", rumor.id);
            }
        }
        let lines = rumor_details(&data.rumors[0]);
        assert!(lines.iter().any(|line| line == "Evidence state: not reviewed"), "{lines:?}");
    }

    /// GDD 9.5/9.6: the Guide is open to a fresh account, and "campaign plot
    /// spoilers ... remain separate". No page, link or search of the player
    /// Guide may surface Seal Cascade story content: its quests, its NPCs
    /// (including hidden set-piece NPCs), or its script paths.
    #[test]
    fn the_open_guide_never_surfaces_campaign_story_content() {
        use std::collections::{BTreeMap, BTreeSet};
        let data = reference_data();
        let story_quests: BTreeSet<u32> = data.quests.iter().filter(|quest| quest.is_story()).map(|quest| quest.id).collect();
        let story_npcs: BTreeSet<u32> = data.npcs.iter().filter(|npc| npc.is_story()).map(|npc| npc.id).collect();
        assert!(!story_quests.is_empty() && !story_npcs.is_empty());

        // Names that only campaign NPCs use; a name shared with a general NPC
        // (e.g. a Kafra) proves nothing about a leak.
        let general_names: BTreeSet<String> = data
            .npcs
            .iter()
            .filter(|npc| !npc.is_story())
            .flat_map(|npc| [npc.name.to_lowercase(), npc.display_name.to_lowercase()])
            .collect();
        // A campaign NPC may share a name with part of a real card, item or
        // monster ("Memory of Thanatos" / "Memory of Thanatos Card"); that
        // page is not a leak.
        let game_names: BTreeSet<String> = data
            .items
            .iter()
            .map(|entry| entry.name.to_lowercase())
            .chain(data.cards.iter().map(|entry| entry.name.to_lowercase()))
            .chain(data.monsters.iter().map(|entry| entry.name.to_lowercase()))
            .collect();
        let story_names: BTreeSet<String> = data
            .npcs
            .iter()
            .filter(|npc| npc.is_story())
            .flat_map(|npc| [npc.display_name.clone(), npc.name.split('#').next().unwrap_or("").to_owned()])
            .map(|name| name.trim().to_lowercase())
            .filter(|name| name.len() >= 6 && !general_names.contains(name) && !game_names.iter().any(|game| game.contains(name.as_str())))
            .collect();

        // Guard the guard: enough distinctive names must survive the filters,
        // or the name check below would pass by testing almost nothing.
        assert!(story_names.len() >= 20, "only {} distinctive campaign names", story_names.len());

        let mk = |kind: &str, id: u32| GuideResult {
            label: String::new(),
            kind: kind.to_owned(),
            id,
        };
        let mut entries: Vec<GuideResult> = Vec::new();
        entries.extend(data.monsters.iter().map(|entry| mk("monster", entry.id)));
        entries.extend(data.items.iter().map(|entry| mk("item", entry.id)));
        entries.extend(data.cards.iter().map(|entry| mk("card", entry.id)));
        entries.extend(data.skills.iter().map(|entry| mk("skill", entry.id as u32)));
        entries.extend(data.statuses.iter().map(|entry| mk("status", entry.id)));
        entries.extend(
            data.quests
                .iter()
                .filter(|quest| !quest.is_story())
                .map(|quest| mk("quest", quest.id)),
        );
        entries.extend(data.npcs.iter().filter(|npc| !npc.is_story()).map(|npc| mk("npc", npc.id)));
        entries.extend((0..data.npc_services.len()).map(|index| mk("service", index as u32)));
        entries.extend(data.rumors.iter().map(|entry| mk("rumor", entry.id)));
        entries.extend(job_names().map(|(id, _)| mk("job", id as u32)));
        entries.extend((0..crate::world::navigation_graph().maps.len()).map(|index| mk("map", index as u32)));

        // (page kind, what leaked) -> (count, first example)
        let mut leaks: BTreeMap<(String, &'static str), (usize, String)> = BTreeMap::new();
        for entry in &entries {
            for line in resolve_details(entry) {
                let lower = line.to_lowercase();
                let mut note = |what: &'static str| {
                    let slot = leaks.entry((entry.kind.clone(), what)).or_insert((
                        0,
                        format!("{}:{} | {}", entry.kind, entry.id, line.chars().take(110).collect::<String>()),
                    ));
                    slot.0 += 1;
                };
                if lower.contains("dm_campaign") {
                    note("campaign script path");
                }
                if let Some(link) = parse_guide_link(&line) {
                    if (link.kind == "quest" && story_quests.contains(&link.id)) || (link.kind == "npc" && story_npcs.contains(&link.id)) {
                        note("link to a story entry");
                    }
                }
                if story_names.iter().any(|name| lower.contains(name.as_str())) {
                    note("campaign NPC name");
                }
            }
        }

        // Searching must not find them either.
        let mut found = BTreeSet::new();
        for quest in data.quests.iter().filter(|quest| quest.is_story()) {
            for row in search_all_categories(&quest.name.to_lowercase(), &DiscoveryState::default(), &[]) {
                if row.kind == "quest" && story_quests.contains(&row.id) {
                    found.insert(format!("quest {}", row.id));
                }
            }
        }
        for name in story_names.iter().take(40) {
            for row in search_all_categories(name, &DiscoveryState::default(), &[]) {
                if row.kind == "npc" && story_npcs.contains(&row.id) {
                    found.insert(format!("npc {}", row.id));
                }
            }
        }

        let report: Vec<String> = leaks
            .iter()
            .map(|((kind, what), (count, example))| format!("{kind} pages: {what} x{count}, e.g. {example}"))
            .collect();
        assert!(
            report.is_empty() && found.is_empty(),
            "story content leaks into the open Guide:\n{}\nsearchable story entries: {} e.g. {:?}",
            report.join("\n"),
            found.len(),
            found.iter().take(3).collect::<Vec<_>>()
        );
    }

    #[test]
    fn level_difference_rule_is_searchable_and_states_the_exact_difference_behaviour() {
        let data = reference_data();
        let rows = search_all_categories("level difference", &DiscoveryState::default(), &[]);
        let row = rows
            .iter()
            .find(|row| row.kind == "server-rule" && data.server_rules[row.id as usize].title.contains("Level difference"))
            .expect("the level difference rule is searchable");
        let detail = resolve_details(row).join("\n");
        // Spot values from db/re/level_penalty.conf: +10 is 140% EXP, -6 is 95%, +16
        // drops 50%.
        assert!(detail.contains("+10: 140%"), "{detail}");
        assert!(detail.contains("-6: 95%"), "{detail}");
        assert!(detail.contains("+16: 50%"), "{detail}");
        // The caveat that matters: gaps and anything beyond the last row are
        // unmodified.
        assert!(detail.contains("Any other difference takes 100%"), "{detail}");
        assert!(detail.contains("not observed in play"), "{detail}");
    }

    #[test]
    fn job_pages_show_the_servers_exp_tables_for_that_job() {
        let data = reference_data();
        let (knight, base, job) = data.exp_groups_for_job(7).expect("Knight has EXP groups");
        assert_eq!(
            (knight.base_group.as_str(), knight.job_group.as_str()),
            ("FirstClasses", "SecondClasses")
        );
        let (lord_knight, ..) = data.exp_groups_for_job(4008).expect("Lord Knight has EXP groups");
        assert_eq!(lord_knight.base_group, "TranscendedClasses");
        // Values read straight from db/re/exp_group_db.conf: FirstClasses starts at
        // 350, its 50th entry is 47000, and its last (level 98 -> 99) is 3300000.
        assert_eq!(base.exp_to_next(1), Some(350));
        assert_eq!(base.exp_to_next(50), Some(47_000));
        assert_eq!(base.exp_to_next(98), Some(3_300_000));
        assert_eq!(base.exp_to_next(99), None, "no table entry past the last level");
        assert_eq!(base.exp_to_next(0), None);
        assert_eq!(base.total_to_reach(1), Some(0));
        assert_eq!(base.total_to_reach(2), Some(350));
        assert!(job.max_level >= 2);

        let (id, name) = job_names().find(|(_, name)| *name == "Knight").expect("Knight job");
        let text = super::job_details(id, name).join("\n");
        assert!(text.contains("server group FirstClasses, maximum level 99"), "{text}");
        assert!(text.contains("1–2: 350"), "{text}");
        assert!(!text.contains("1–2: 350; "), "milestones are one per line: {text}");
        assert!(text.contains("50–51: 47,000"), "{text}");
        assert!(text.contains("98–99: 3,300,000"), "{text}");
        assert!(text.contains("Total from level 1 to level 99:"), "{text}");
        assert!(text.contains("server group SecondClasses"), "{text}");
        assert!(text.contains("Source: Hercules db/re/exp_group_db.conf"), "{text}");
        let cap = text.find("Stat cap: 99").expect("stat cap");
        let weight = text.find("Weight capacity:").expect("weight");
        let exp = text.find("server group FirstClasses").expect("base exp");
        let skills = text.find("Skill tree:").expect("skills");
        assert!(cap < weight && weight < exp && exp < skills, "{text}");
    }

    #[test]
    fn first_job_pages_name_the_academy_trainer_and_the_town_guild() {
        let data = reference_data();
        let page = |name: &str| {
            let (id, listed) = job_names().find(|(_, listed)| *listed == name).unwrap_or_else(|| panic!("{name}"));
            (id, super::job_details(id, listed))
        };
        let (swordman_id, swordman) = page("Swordman");
        assert_eq!(swordman_id, 1);
        let swordman_text = swordman.join("\n");
        assert!(swordman_text.contains("Town for this class: Izlude."), "{swordman_text}");
        assert!(swordman_text.contains("@route-cell:iz_ac02:60:51|Route to Swordman Trainer — iz_ac02"));
        assert!(swordman_text.contains("@route-cell:izlude_in:74:172|Route to Swordman — izlude_in"));
        assert!(!swordman_text.contains("62, 51"), "the sign cell is not the NPC");

        let novice = page("Novice").1.join("\n");
        for town in ["Izlude", "Geffen", "Payon", "Prontera", "Alberta", "Morocc"] {
            assert!(
                novice.contains(&format!("Town for this class: {town}.")),
                "{town} missing: {novice}"
            );
        }
        assert!(novice.contains("Thief Guide"));
        assert!(novice.contains("@route-cell:moc_prydb1:42:133|Route to Thief Guildsman — moc_prydb1"));
        assert!(novice.contains("Father Mareusis"));
        assert!(novice.contains("@route-cell:prt_church:184:41|"));

        let knight = page("Knight").1.join("\n");
        assert!(!knight.contains("Town for this class"), "{knight}");
        assert!(!knight.contains("Criatura Academy"), "{knight}");

        for path in super::FIRST_JOBS {
            for stop in [&path.academy, &path.guild].into_iter().chain(path.ceremony.as_ref()) {
                let declared = data.npcs.iter().any(|npc| {
                    npc.map == stop.map
                        && npc.x == i32::from(stop.x)
                        && npc.y == i32::from(stop.y)
                        && npc.source.path == stop.source
                        && npc.declared_type != "warp"
                        && !npc.is_story()
                        && (npc.display_name == stop.declared_name || npc.name.starts_with(stop.declared_name))
                });
                assert!(
                    declared,
                    "{} at {},{} is not the declared NPC",
                    stop.declared_name, stop.map, stop.x
                );
            }
        }

        for query in ["trainer", "which town", "newbie", "first job", "academy"] {
            let rows = search_all_categories(query, &DiscoveryState::default(), &[]);
            assert!(
                rows.iter().any(|row| row.kind == "job" && row.id == 0),
                "{query} did not open the novice path: {rows:?}"
            );
        }
    }

    #[test]
    fn a_job_the_server_has_no_exp_group_for_says_so_instead_of_guessing() {
        let data = reference_data();
        let missing = job_names().find(|(id, _)| data.exp_groups_for_job(*id).is_none());
        let (id, name) = missing.expect("some listed job has no job_db.conf block");
        let text = super::job_details(id, name).join("\n");
        assert!(text.contains("No EXP group is defined for this job"), "{name}: {text}");
        assert!(!text.contains("server group"), "{name}: {text}");
    }

    #[test]
    fn exp_numbers_are_grouped_in_threes() {
        assert_eq!(super::group_digits(0), "0");
        assert_eq!(super::group_digits(350), "350");
        assert_eq!(super::group_digits(47_000), "47,000");
        assert_eq!(super::group_digits(3_300_000), "3,300,000");
        assert_eq!(super::group_digits(1_000), "1,000");
    }

    #[test]
    fn derived_stat_formulas_rule_states_the_server_arithmetic() {
        let data = reference_data();
        let rows = search_all_categories("derived stat formulas", &DiscoveryState::default(), &[]);
        let row = rows
            .iter()
            .find(|row| row.kind == "server-rule" && data.server_rules[row.id as usize].title.contains("Derived stat formulas"))
            .expect("the derived stat formulas rule is searchable");
        let detail = resolve_details(row).join("\n");
        assert!(detail.contains("HIT = Base Level + DEX + floor(LUK / 3) + 175"), "{detail}");
        // The two corrections that matter most: one truncation for DEF/MDEF, and a
        // square root for cast time.
        assert!(detail.contains("rounded down once"), "{detail}");
        assert!(detail.contains("sqrt((DEX x 2 + INT) / 530)"), "{detail}");
        assert!(detail.contains("a square root, not a straight line"), "{detail}");
        assert!(detail.contains("swap STR and DEX"), "{detail}");
        assert!(detail.contains("not observed on a live server"), "{detail}");
        // The 530 comes from the configured scale, so the entry cites it.
        assert!(detail.contains("skill.conf"), "{detail}");
    }

    #[test]
    fn job_pages_state_the_stat_cap_and_upper_class_bonus_from_the_server_tables() {
        let page = |name: &str| {
            let (id, name) = job_names()
                .find(|(_, listed)| *listed == name)
                .unwrap_or_else(|| panic!("{name} job"));
            super::job_details(id, name).join("\n")
        };
        let knight = page("Knight");
        assert!(knight.contains("Stat cap: 99 (parameter group SecondClasses)"), "{knight}");
        assert!(knight.contains("Not an upper class"), "{knight}");
        let baby = page("Baby");
        assert!(baby.contains("Stat cap: 80"), "{baby}");
        // Rune Knight Trans: a third class (cap 130) that is also an upper class (+52).
        let trans = page("Rune Knight T");
        assert!(trans.contains("Stat cap: 130"), "{trans}");
        assert!(trans.contains("Upper class: a stat reset grants 52 extra points"), "{trans}");
    }

    #[test]
    fn stat_points_rule_is_searchable_with_the_servers_totals_and_costs() {
        let data = reference_data();
        let rows = search_all_categories("stat points", &DiscoveryState::default(), &[]);
        let row = rows
            .iter()
            .find(|row| row.kind == "server-rule" && data.server_rules[row.id as usize].title.contains("Stat points"))
            .expect("the stat points rule is searchable");
        let detail = resolve_details(row).join("\n");
        assert!(detail.contains("level 99: 1,273"), "{detail}");
        assert!(detail.contains("level 175: 3,278"), "{detail}");
        assert!(detail.contains("99 to 100 costs 11, 100 to 101 costs 16"), "{detail}");
        assert!(detail.contains("1 to 99 costs 628"), "{detail}");
        assert!(detail.contains("plus 52 extra points for upper classes"), "{detail}");
        assert!(detail.contains("130: ThirdClasses"), "{detail}");
        assert!(detail.contains("max_parameter is not the player cap"), "{detail}");
    }

    #[test]
    fn job_pages_show_class_hp_sp_and_aspd_from_the_servers_tables() {
        let (id, name) = job_names().find(|(_, name)| *name == "Knight").expect("Knight job");
        let text = super::job_details(id, name).join("\n");
        // Computed from the exported tables: HP[1] = 40, HP[50] = 2208, HP[99] = 7978,
        // then + 1% per VIT in integer steps (VIT 1 and VIT 99).
        assert!(text.contains("Base max HP at VIT 1 / VIT 99"), "{text}");
        assert!(text.contains("level 1: 40 / 79"), "{text}");
        assert!(text.contains("level 50: 2,230 / 4,393"), "{text}");
        assert!(text.contains("level 99: 8,057 / 15,876"), "{text}");
        assert!(!text.contains("level 1: 40 / 79; "), "HP levels are one per line: {text}");
        assert!(text.contains("Base max SP at INT 1 / INT 99"), "{text}");
        assert!(text.contains("level 1: 13 / 25"), "{text}");
        assert!(text.contains("level 50: 161 / 318"), "{text}");
        assert!(text.contains("level 99: 310 / 610"), "{text}");
        // job_db.conf BaseASPD for Knight, and the cap from MaxASPD.
        assert!(
            text.contains("Sword 45") && text.contains("Two Hand Sword 52") && text.contains("Shield 5"),
            "{text}"
        );
        assert!(text.contains("Maximum ASPD 190"), "{text}");
        // 196 + sqrt(99^2/5 + 99^2/2)/4 = 216.7 -> 216, minus 45 = 171.
        assert!(text.contains("Sword 171"), "{text}");
        // Sword 45 + Shield 5 = 50: 216 - 50 = 166.
        assert!(
            text.contains("The same with a shield worn (its value, 5, slows you down)") && text.contains("Sword 166"),
            "{text}"
        );
        assert!(!text.contains("Shield 171"), "the shield value is not a weapon: {text}");
    }

    #[test]
    fn a_class_table_that_collapses_says_so_on_the_job_page() {
        let page = |name: &str| {
            let (id, name) = job_names()
                .find(|(_, listed)| *listed == name)
                .unwrap_or_else(|| panic!("{name} job"));
            super::job_details(id, name).join("\n")
        };
        // Baby Kagerou's HP and SP tables are 1 from level 161 (observed live at 161).
        let kagerou = page("Baby Kagerou");
        assert!(kagerou.contains("HP table drops to 1 at level 161"), "{kagerou}");
        assert!(kagerou.contains("SP table drops to 1 at level 161"), "{kagerou}");
        // An ordinary class has no warning.
        assert!(
            !page("Knight").contains("Warning: this class's"),
            "Knight has no collapsed table"
        );
    }

    #[test]
    fn job_pages_list_the_passive_aspd_skills_with_their_bonus() {
        let page = |name: &str| {
            let (id, name) = job_names()
                .find(|(_, listed)| *listed == name)
                .unwrap_or_else(|| panic!("{name} job"));
            super::job_details(id, name).join("\n")
        };
        // Skill level 10 each: (10 - 1) / 2 + 1 = 5, (10 + 1) / 2 = 5, 10, 10.
        assert!(page("Sage").contains("Advanced Book (with a book) +5"), "Sage");
        assert!(page("Gunslinger").contains("Single Action +5"), "Gunslinger");
        assert!(page("Rogue").contains("Plagiarism +10"), "Rogue");
        assert!(page("Bard").contains("Musical Lesson (with an instrument) +10"), "Bard");
        assert!(!page("Knight").contains("Passive ASPD skills"), "Knight has none");
    }

    #[test]
    fn a_job_with_no_class_table_says_so_instead_of_inventing_hp() {
        let data = reference_data();
        let (id, name) = job_names()
            .find(|(id, _)| data.job_tables.job(*id).is_none())
            .expect("a listed job with no job_db.conf block");
        let text = super::job_details(id, name).join("\n");
        assert!(text.contains("No class table is defined for this job"), "{name}: {text}");
        assert!(!text.contains("Base max HP"), "{name}: {text}");
    }

    #[test]
    fn hp_sp_and_aspd_rules_are_searchable_and_state_the_formulas() {
        let data = reference_data();
        let find = |query: &str, title: &str| {
            let rows = search_all_categories(query, &DiscoveryState::default(), &[]);
            let row = rows
                .iter()
                .find(|row| row.kind == "server-rule" && data.server_rules[row.id as usize].title.contains(title))
                .unwrap_or_else(|| panic!("{title} is searchable"));
            resolve_details(row).join("\n")
        };
        let hp = find("max hp and sp", "Max HP and SP");
        assert!(hp.contains("plus 25% for upper classes or times 70% for baby classes"), "{hp}");
        assert!(hp.contains("plus 1% per VIT"), "{hp}");
        // Worked example from the exported tables: Lord Knight, level 99, VIT 99.
        assert!(hp.contains("Lord Knight (upper) 19,844"), "{hp}");
        assert!(hp.contains("Every value matched"), "{hp}");
        assert!(hp.contains("Expanded Super Novice at level 150"), "{hp}");
        assert!(
            hp.contains("Baby Kagerou and Baby Oboro tables fall to 1 from level 161 to 175"),
            "{hp}"
        );
        let aspd = find("attack speed", "Attack speed (ASPD)");
        assert!(aspd.contains("sqrt(DEX x DEX / 5 + AGI x AGI / 2) / 4"), "{aspd}");
        assert!(aspd.contains("DEX x DEX / 7"), "{aspd}");
        assert!(aspd.contains("193: BabyThirdClasses, SuperNovice, ThirdClasses"), "{aspd}");
        assert!(
            aspd.contains("115 measurements") && aspd.contains("Every value matched"),
            "{aspd}"
        );
        assert!(aspd.contains("only the strongest potion counts"), "{aspd}");
        assert!(aspd.contains("(500 + 100 x Cavalier Mastery level) / 1000"), "{aspd}");
        assert!(aspd.contains("100 ms at 190, 70 ms at 193"), "{aspd}");
        assert!(aspd.contains("not compared: the Dragon mount"), "{aspd}");
    }

    #[test]
    fn all_search_finds_matching_monster_card_and_quest_together() {
        let rows = search_all_categories("poring", &DiscoveryState::default(), &[]);
        assert!(rows.iter().any(|row| row.kind == "monster" && row.id == 1002));
        assert!(rows.iter().any(|row| row.kind == "card" && row.id == 4001));
        assert!(rows.iter().any(|row| row.kind == "quest"));
    }

    #[test]
    fn armor_and_weapon_refinement_are_searchable_and_display_rates() {
        let data = reference_data();
        let armor_lines = refinement_details(&data.refinement, 0).join("\n");
        assert!(armor_lines.contains("Armor Refinement"), "{armor_lines}");
        assert!(armor_lines.contains("Elunium"), "{armor_lines}");
        assert!(armor_lines.contains("2000 Zeny"), "{armor_lines}");
        assert!(armor_lines.contains("Safe refine limit: +4"), "{armor_lines}");
        assert!(armor_lines.contains("+1 DEF per level (Lv 1–4)"), "{armor_lines}");
        assert!(armor_lines.contains("+2 DEF per level (Lv 5–8)"), "{armor_lines}");
        assert!(armor_lines.contains("+3 DEF per level (Lv 9–10)"), "{armor_lines}");

        let w1_lines = refinement_details(&data.refinement, 1).join("\n");
        assert!(w1_lines.contains("Weapon Level 1 Refinement"), "{w1_lines}");
        assert!(w1_lines.contains("Phracon"), "{w1_lines}");
        assert!(w1_lines.contains("50 Zeny"), "{w1_lines}");
        assert!(w1_lines.contains("Safe limit: +7"), "{w1_lines}");

        let w4_lines = refinement_details(&data.refinement, 4).join("\n");
        assert!(w4_lines.contains("Weapon Level 4 Refinement"), "{w4_lines}");
        assert!(w4_lines.contains("Oridecon"), "{w4_lines}");
        assert!(w4_lines.contains("20000 Zeny"), "{w4_lines}");
        assert!(w4_lines.contains("Safe limit: +4"), "{w4_lines}");
        assert!(w4_lines.contains("WS_WEAPONREFINE"), "{w4_lines}");

        let armor_search = search_all_categories("armor", &DiscoveryState::default(), &[]);
        assert!(
            armor_search
                .iter()
                .any(|r| r.kind == "mechanic" && r.id == 0 && r.label.contains("Armor refinement odds"))
        );

        let refine_search = search_all_categories("refine", &DiscoveryState::default(), &[]);
        assert!(
            refine_search
                .iter()
                .any(|r| r.kind == "mechanic" && r.id == 100 && r.label.contains("Weapon refinement odds"))
        );
        assert!(
            refine_search
                .iter()
                .any(|r| r.kind == "mechanic" && r.id == 0 && r.label.contains("Armor refinement odds"))
        );
    }

    #[test]
    fn provenance_rules_cover_precedence_discovery_guidance_and_dm_mode() {
        let data = reference_data();
        let rule = |id: &str| {
            data.server_rules
                .iter()
                .find(|rule| rule.id == id)
                .unwrap_or_else(|| panic!("provenance rule '{id}' should exist"))
        };

        // Import precedence names the real load order, ending in the server's
        // overrides.
        let precedence = rule("config-precedence").details.join(" ");
        assert!(precedence.contains("conf/map/battle.conf"), "{precedence}");
        assert!(precedence.contains("conf/import/battle.conf"), "{precedence}");
        assert!(precedence.contains("later wins"), "{precedence}");

        // Discovery never gates mechanics and is never shared with the party.
        let discovery = rule("discovery-scope").details.join(" ");
        assert!(discovery.contains("never shared with party members"), "{discovery}");
        assert!(
            discovery.contains("does not hide or reveal mechanical information"),
            "{discovery}"
        );

        // Guidance is the client's, not the server's.
        let guidance = rule("quest-guidance").details.join(" ");
        assert!(guidance.contains("does not send item turn-in lists"), "{guidance}");

        // DM mode: party-bound content, but MVP suppression is server-wide.
        let dm_mode = rule("dm-mode").details.join(" ");
        assert!(dm_mode.contains("needs a party"), "{dm_mode}");
        assert!(dm_mode.contains("server-wide"), "{dm_mode}");

        // Every rule cites at least one source file.
        assert!(data.server_rules.iter().all(|rule| !rule.sources.is_empty()));
    }

    #[test]
    fn a_rule_page_names_the_server_revision_it_was_exported_from() {
        let data = reference_data();
        let rule = data.server_rules.first().expect("at least one rule");
        let lines = server_rule_details(rule);
        let revision: String = data.source_revision.chars().take(10).collect();
        let provenance = lines
            .iter()
            .find(|line| line.starts_with("Exported from Hercules "))
            .unwrap_or_else(|| panic!("no revision line in {lines:?}"));
        assert!(provenance.contains(&revision), "{provenance}");
        assert!(provenance.contains("mode"), "{provenance}");
        assert!(provenance.contains("at export"), "{provenance}");
    }

    #[test]
    fn server_rules_cover_campaign_distance_recovery_and_limits() {
        let data = reference_data();
        let find_rule = |title: &str| {
            data.server_rules
                .iter()
                .find(|rule| rule.title.contains(title))
                .unwrap_or_else(|| panic!("server rule '{title}' should exist"))
        };

        let campaign = find_rule("DM Campaign progression and recovery");
        assert_eq!(campaign.id, "dm-campaign-rules");
        let campaign_text = campaign.details.join(" ");
        assert!(campaign_text.contains("8000 ms"), "{campaign_text}");
        assert!(campaign_text.contains("10000 ms"), "{campaign_text}");
        assert!(campaign_text.contains("5x"), "{campaign_text}");
        assert!(campaign_text.contains("Party story synchronization"), "{campaign_text}");

        let distance = find_rule("View distance and area radius");
        assert_eq!(distance.id, "view-distance-and-aoe");
        let distance_text = distance.details.join(" ");
        assert!(distance_text.contains("30 cells"), "{distance_text}");
        assert!(distance_text.contains("48 cells"), "{distance_text}");

        let recovery = find_rule("Natural recovery and weight thresholds");
        assert_eq!(recovery.id, "natural-recovery-and-weight");
        let recovery_text = recovery.details.join(" ");
        assert!(recovery_text.contains("6000 ms"), "{recovery_text}");
        assert!(recovery_text.contains("8000 ms"), "{recovery_text}");
        assert!(recovery_text.contains("50%"), "{recovery_text}");
        assert!(recovery_text.contains("90%"), "{recovery_text}");

        let limits = find_rule("Multi-level-up and progression limits");
        assert_eq!(limits.id, "progression-limits");
        let limits_text = limits.details.join(" ");
        assert!(limits_text.contains("multi_level_up"), "{limits_text}");
        assert!(limits_text.contains("MaxStats"), "{limits_text}");
        assert!(limits_text.contains("WoE"), "{limits_text}");

        let equip_refine = find_rule("Equipment refinement odds and mechanics");
        assert_eq!(equip_refine.id, "equipment-refinement");
        let equip_text = equip_refine.details.join(" ");
        assert!(equip_text.contains("Elunium"), "{equip_text}");
        assert!(equip_text.contains("Phracon"), "{equip_text}");
    }

    #[test]
    fn search_aliases_resolve_for_items_monsters_skills_and_maps() {
        let data = reference_data();

        // Item aliases
        let white_pots = data.search_items("hp pot", 5);
        assert!(white_pots.iter().any(|item| item.aegis_name == "White_Potion"), "hp pot alias");
        let obb = data.search_items("obb", 5);
        assert!(obb.iter().any(|item| item.aegis_name == "Old_Blue_Box"), "obb alias");
        let gr_card = data.search_cards("gr card", 5);
        assert!(gr_card.iter().any(|card| card.aegis_name == "Ghostring_Card"), "gr card alias");

        // Monster aliases
        let bapho = data.search_monsters("bapho", 10);
        assert!(
            bapho.iter().any(|m| m.sprite_name.eq_ignore_ascii_case("BAPHOMET")),
            "bapho alias"
        );
        let gtb = data.search_monsters("gtb", 5);
        assert!(
            gtb.iter().any(|m| m.sprite_name.eq_ignore_ascii_case("GOLDEN_BUG")),
            "gtb alias"
        );

        // Skill aliases
        let bb = data.search_skills("bb", 5);
        assert!(
            bb.iter().any(|s| s.description.eq_ignore_ascii_case("Bowling Bash")),
            "bb alias"
        );
        let edp = data.search_skills("edp", 5);
        assert!(
            edp.iter().any(|s| s.description.eq_ignore_ascii_case("Enchant Deadly Poison")),
            "edp alias"
        );

        // Map aliases
        let gh_search = search_all_categories("gh", &DiscoveryState::default(), &[]);
        assert!(
            gh_search.iter().any(|r| r.kind == "map" && r.label.contains("glast_01")),
            "gh alias"
        );
        let prt_search = search_all_categories("prt", &DiscoveryState::default(), &[]);
        assert!(
            prt_search.iter().any(|r| r.kind == "map" && r.label.contains("prontera")),
            "prt alias"
        );
    }
}

/// A job matches by name, or by an authored alias such as "lk" or "pally".
fn job_matches(query: &str, name: &str) -> bool {
    query.is_empty() || name.to_lowercase().contains(query) || reference_data().alias_targets("job", query).contains(&name)
}

/// A map matches by filename, or by an authored alias such as "gh" or "prt".
fn map_matches(query: &str, map: &str) -> bool {
    let query_lower = query.to_lowercase();
    query.is_empty()
        || map.to_lowercase().contains(&query_lower)
        || reference_data()
            .alias_targets("map", query)
            .iter()
            .any(|target| target.eq_ignore_ascii_case(map) || map.to_lowercase().contains(&target.to_lowercase()))
}

fn job_names() -> impl Iterator<Item = (u16, &'static str)> {
    JOB_NAMES.lines().filter_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (id, name) = line.split_once('\t')?;
        Some((id.parse().ok()?, name.trim()))
    })
}

#[cfg(test)]
mod monster_page_and_route_offer_tests {
    use super::{RouteOffer, drop_source_line, monster_behavior, monster_details, route_offer};
    use crate::dm::reference_data::reference_data;

    #[test]
    fn monster_page_shows_exp_combat_numbers_and_behavior_from_the_database() {
        let poring = reference_data().monster_by_id(1002).expect("Poring is exported");
        let lines = monster_details(poring);

        assert!(lines.contains(&"EXP: base 36   job 20".to_owned()), "{lines:#?}");
        assert!(lines.contains(&"Attack: 1–8   DEF 2   MDEF 5".to_owned()));
        assert!(lines.contains(&"Attack range 1 cells   Sight range 10 cells".to_owned()));
        assert!(lines.iter().any(|line| line.starts_with("Move speed: 400 ms per cell")));
        assert!(lines.contains(&"Behavior: passive — fights back only when attacked; picks up items from the ground".to_owned()));
    }

    #[test]
    fn pilot_skills_are_shown_as_in_use_while_the_server_switch_is_on() {
        let layer = reference_data()
            .pilot_skill_layer
            .as_ref()
            .expect("bestiary records the pilot switch");
        assert_eq!(layer.setting, "mob_pilot_version");
        assert!(layer.active, "this server's import config turns the pilot on");

        let elite = monster_details(reference_data().monster_by_id(20901).expect("the elite is exported"));
        assert!(elite.iter().any(|line| line == "Uses the newer skill list on this server."));
        assert!(
            elite.iter().any(|line| line.starts_with("Source: mob_pilot_version = 1")),
            "{elite:#?}"
        );
        let in_use = elite.iter().position(|line| line == "Pilot skills (in use):").expect("pilot list");
        let stock = elite
            .iter()
            .position(|line| line == "Stock skills (used only when the pilot is off): none.")
            .expect("the elite has no stock skills");
        assert!(elite[in_use..stock].iter().any(|line| line.contains("Bash")), "{elite:#?}");

        let eddga = monster_details(reference_data().monster_by_id(1115).expect("Eddga is exported"));
        assert!(eddga.contains(&"Pilot skills (in use):".to_owned()));
        assert!(eddga.contains(&"Stock skills (used only when the pilot is off):".to_owned()));

        // Monsters the pilot file does not touch keep the single list.
        let poring = monster_details(reference_data().monster_by_id(1002).expect("Poring is exported"));
        assert!(poring.contains(&"Skills:".to_owned()));
        assert!(!poring.iter().any(|line| line.starts_with("AI pilot layer")));
    }

    #[test]
    fn a_first_owned_exchange_lists_its_inputs_as_alternatives() {
        let ring = reference_data().item_by_id(2864).expect("Light Of Cure is exported");
        let lines = super::item_details(ring, false);
        let costs = lines
            .iter()
            .find(|line| line.starts_with("Costs one of (the first listed item you carry is used): "))
            .unwrap_or_else(|| panic!("{lines:#?}"));
        assert!(costs.contains(" or "), "{costs}");
    }

    #[test]
    fn job_pages_state_weight_capacity_and_skills_show_their_server_record() {
        let novice = super::job_details(0, "Novice");
        assert!(
            novice.contains(&"Weight capacity: 2,000 plus 30 per base STR point (before skill bonuses).".to_owned()),
            "{novice:#?}"
        );

        let eddga = monster_details(reference_data().monster_by_id(1115).expect("Eddga is exported"));
        assert!(
            eddga.iter().any(|line| line.starts_with("Source: MSS_") && line.contains("MSC_")),
            "{eddga:#?}"
        );
    }

    #[test]
    fn only_the_aggressive_flag_makes_a_monster_aggressive() {
        let orc_skeleton = reference_data().monster_by_id(1152).expect("Orc Skeleton is exported");
        let behavior = monster_behavior(&orc_skeleton.modes);
        assert!(behavior.starts_with("aggressive — attacks players on sight"), "{behavior}");

        // `Angry` alone selects a skill state; it does not start fights.
        let angry_only = monster_behavior(&["CanAttack".to_owned(), "CanMove".to_owned(), "Angry".to_owned()]);
        assert!(angry_only.starts_with("passive"), "{angry_only}");
        assert!(angry_only.ends_with("(other server flags: Angry)"), "{angry_only}");

        let plant = monster_behavior(&["Plant".to_owned()]);
        assert!(plant.starts_with("never attacks; does not move"), "{plant}");
    }

    #[test]
    fn drop_rows_name_the_monster_and_its_level() {
        let line = drop_source_line(1002, "PORING", "normal", 150);
        assert_eq!(line, "@guide:monster:1002|Poring (Lv 1) — normal drop 1.50%");

        // A monster missing from the bestiary keeps the database name.
        let unknown = drop_source_line(999_999, "NOT_EXPORTED", "mvp", 1);
        assert_eq!(unknown, "@guide:monster:999999|NOT_EXPORTED — MVP drop 0.01%");
    }

    #[test]
    fn routes_say_how_far_they_are_or_why_they_are_not_offered() {
        let plain = |text: &str| RouteOffer::Button {
            text: text.to_owned(),
            notes: Vec::new(),
        };
        assert_eq!(route_offer("", "izlude", "Route to izlude"), plain("Route to izlude"));
        assert_eq!(
            route_offer("prontera", "prontera", "Route to prontera"),
            plain("Route to prontera (this map)")
        );

        let RouteOffer::Button { text, notes } = route_offer("prontera", "izlude", "Route to izlude") else {
            panic!("prontera reaches izlude");
        };
        assert!(text.starts_with("Route to izlude (1 map · 600 z · ~"), "{text}");
        assert!(text.ends_with(" cells; Fewest maps)"), "{text}");
        assert!(notes.is_empty());

        let RouteOffer::Button { notes, .. } = route_offer("prontera", "eclage", "Route to eclage") else {
            panic!("eclage is routable through its locked entrance");
        };
        assert!(
            notes.iter().any(|line| line.starts_with("Locked step ecl_fild01 › eclage")),
            "{notes:#?}"
        );

        let RouteOffer::Unavailable(lines) = route_offer("prontera", "gld_dun01", "Route to gld_dun01") else {
            panic!("guild dungeons have no known entrance");
        };
        assert!(lines[0].starts_with("gld_dun01: no loaded warp"), "{lines:#?}");
    }

    #[test]
    fn item_page_leads_with_drops_and_names_untyped_items() {
        // Jellopy has recipes, script clues and containers; the drop list used
        // to come after ~60 of those lines.
        let lines = super::item_details(super::reference_data().item_by_id(909).expect("Jellopy"), false);
        assert_eq!(lines[1], "Type: Miscellaneous   Weight: 10");
        let drops = lines.iter().position(|line| line.starts_with("Dropped by")).expect("drop header");
        let recipes = lines.iter().position(|line| line.starts_with("Crafting")).expect("recipe header");
        assert_eq!(drops, 2, "{lines:#?}");
        assert!(drops < recipes);
        assert!(lines[drops + 1].starts_with("@guide:monster:1002|Poring"), "{lines:#?}");
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("Source: drop rates are the database figures"))
        );
        assert!(
            lines
                .iter()
                .filter(|line| line.starts_with("Makes ") || line.starts_with("Used to make "))
                .all(|line| !line.contains(".txt")),
            "{lines:#?}"
        );
    }

    #[test]
    fn item_page_leads_with_what_it_does_and_who_sells_it() {
        let item = super::reference_data()
            .items
            .iter()
            .find(|item| item.atk.unwrap_or(0) > 0 && item.shops.iter().any(|shop| shop.source.contains(".txt")))
            .expect("a weapon sold from a written-out shop");
        let lines = super::item_details(item, false);
        let atk = lines.iter().position(|line| line.starts_with("ATK:")).expect("atk");
        let sold = lines.iter().position(|line| line == "Sold by:").expect("sold by");
        assert!(atk < sold, "{lines:#?}");
        for header in ["Crafting", "May also be given", "Quest rewards", "Used as a turn-in", "Exchanges:"] {
            if let Some(at) = lines.iter().position(|line| line.starts_with(header)) {
                assert!(sold < at, "{header} should follow the shop\n{lines:#?}");
            }
        }
        assert_eq!(lines.iter().filter(|line| line.starts_with("ATK:")).count(), 1);
        assert_eq!(
            lines.iter().filter(|line| line.starts_with("Buy price:")).count(),
            usize::from(item.buy > 0)
        );
        let offer = lines
            .iter()
            .find(|line| line.starts_with("shop:") || line.starts_with("cash shop:"))
            .expect("shop offer");
        assert!(offer.contains(" for "), "{offer}");
        assert!(!offer.contains(".txt"), "{offer}");
        let shop = item.shops.iter().find(|shop| shop.source.contains(".txt")).expect("shop script");
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("Source:") && line.contains(shop.source.as_str())),
            "{lines:#?}"
        );
        assert!(
            !lines
                .iter()
                .any(|line| line.contains("Literal shop listings") || line.starts_with("Equipment location"))
        );
    }
}
