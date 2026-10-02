//! Player-facing, open-search reference guide. This intentionally does not
//! reuse the DM Bestiary: campaign reveal/spawn controls and unlocks are not
//! part of the player's mechanical reference.

use std::collections::HashSet;
use std::sync::Arc;

use korangar_interface::components::text_box::DefaultHandler;
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
use crate::world::{Library, TownPoi};

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

fn monster_details(monster: &ReferenceMonster) -> Vec<String> {
    let data = reference_data();
    let mut lines = vec![
        format!("{}  (ID {})", display_name(&monster.name, &monster.sprite_name), monster.id),
        format!("Level {}   HP {}", monster.level, monster.hp),
        format!("@hunting-goal:{}|Add to personal hunting goals (client-only)", monster.id),
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
    lines.push(format!("Skills: {}   Drops: {}", monster.skills.len(), monster.drops.len()));
    if !monster.skills.is_empty() {
        lines.push("Configured monster skills and trigger conditions:".to_owned());
        for mob_skill in monster.skills.iter().take(8) {
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
            if !mob_skill.trigger_summary.is_empty() {
                lines.push(format!("  {}", mob_skill.trigger_summary));
            } else {
                lines.push("  Trigger details have not been translated from the server record.".to_owned());
            }
            if !mob_skill.target_summary.is_empty() {
                lines.push(format!("  {}", mob_skill.target_summary));
            }
        }
        if monster.skills.len() > 8 {
            lines.push(format!("{} additional skill records omitted.", monster.skills.len() - 8));
        }
        if let Some(source) = &monster.skills_source {
            lines.push(format!("Monster-skill source: {} ({})", source.path, source.record));
        }
    }
    if !monster.drops.is_empty() {
        lines.push("Configured database drop rates (server modifiers may change realized chances):".to_owned());
    }
    for drop in monster.drops.iter().take(8) {
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
        lines.push("No graph-known static spawn-map route is available; conditional/scripted spawns may also be omitted.".to_owned());
    } else {
        lines.push("Loaded static spawn placements (configured directive centers/spreads; exact runtime cells are randomized):".to_owned());
        for region in routeable_regions.iter().take(12) {
            lines.push(format!(
                "{} — {} loaded records, {} monsters listed",
                region.map, region.spawn_records, region.listed_monsters
            ));
            for placement in region.placements.iter().take(3) {
                let location = if placement.random_map_cell {
                    "random eligible cell on map".to_owned()
                } else if placement.x_spread != 0 || placement.y_spread != 0 {
                    format!(
                        "around ({}, {}) — spread {}×{}",
                        placement.x, placement.y, placement.x_spread, placement.y_spread
                    )
                } else {
                    format!("at ({}, {})", placement.x, placement.y)
                };
                lines.push(format!("  {location} — {} listed — {}", placement.amount, placement.source));
            }
            if region.placements.len() > 3 {
                lines.push(format!("  {} more placement records", region.placements.len() - 3));
            }
            lines.push(format!("@route:{}", region.map));
        }
        if routeable_regions.len() > 12 {
            lines.push(format!(
                "{} additional spawn maps omitted from this detail view.",
                routeable_regions.len() - 12
            ));
        }
    }
    if !monster.scripted_spawn_references.is_empty() {
        lines.push("Loaded-script spawn call sites (event/quest/instance conditions are not interpreted):".to_owned());
        for spawn in monster.scripted_spawn_references.iter().take(8) {
            let location = match (spawn.map.as_deref(), spawn.coordinates.as_slice()) {
                (Some(map), [Some(x), Some(y)]) => format!("{map} at ({x}, {y})"),
                (Some(map), [Some(x1), Some(y1), Some(x2), Some(y2)]) => {
                    format!("{map} in area ({x1}, {y1})–({x2}, {y2})")
                }
                (None, _) if spawn.map_template.is_some() => {
                    format!(
                        "{}instance of map template {}",
                        if spawn.map_template_approximate {
                            "approximately on an "
                        } else {
                            "on an "
                        },
                        spawn.map_template.as_deref().unwrap_or_default()
                    )
                }
                (None, [Some(x), Some(y)]) => format!("map expression `{}` at ({x}, {y})", spawn.map_expression),
                (None, [Some(x1), Some(y1), Some(x2), Some(y2)]) => {
                    format!("map expression `{}` in area ({x1}, {y1})–({x2}, {y2})", spawn.map_expression)
                }
                (..) => format!("map expression `{}`", spawn.map_expression),
            };
            let amount = spawn
                .amount
                .map(|count| format!("; configured call amount {count}"))
                .unwrap_or_default();
            lines.push(format!("{} — {location}{amount} — {}", spawn.spawn_kind, spawn.source));
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
                "{} additional scripted spawn references omitted.",
                monster.scripted_spawn_references.len() - 8
            ));
        }
        lines.push("A call site does not prove it runs for every player or when the monster is currently present.".to_owned());
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

fn item_details(item: &ReferenceItem, card: bool) -> Vec<String> {
    let kind = if card { "Card" } else { item.item_type.as_str() };
    let mut lines = vec![
        format!("{}  (ID {})", display_name(&item.name, &item.aegis_name), item.id),
        format!("Type: {kind}   Weight: {}", item.weight),
    ];
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
                lines.push(format!(
                    "Makes {} ×{} from {materials}{requirement} — {}:{}",
                    recipe.output_name, recipe.output_amount, recipe.source.path, recipe.source.record
                ));
            } else {
                lines.push(format!(
                    "Used to make @guide:item:{}|{} ×{} — {}:{}",
                    recipe.output_id, recipe.output_name, recipe.output_amount, recipe.source.path, recipe.source.record
                ));
            }
        }
        if relevant.len() > 12 {
            lines.push(format!("{} additional recipe links omitted.", relevant.len() - 12));
        }
        lines.push(
            "Recipe lists are configured server recipes; they do not establish access to the required skill or guarantee success."
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
        lines.push("Source-reviewed quest rewards:".to_owned());
        for (quest, flow, reward) in reviewed_quest_rewards {
            lines.push(format!(
                "@guide:quest:{}|{} ×{} — {} [{}; {}:{}]",
                quest.id,
                quest.name,
                reward.amount,
                reward.explanation,
                reward.evidence_state.label(),
                reward.source_path,
                reward.source_line
            ));
            for condition in &flow.conditions {
                lines.push(format!("Reward condition: {condition}"));
            }
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
        lines.push("Item grant clues in loaded NPC scripts:".to_owned());
        for grant in grants.iter().take(10) {
            let source = format!("{}:{}", grant.source.path, grant.source.line);
            if let Some(npc) = &grant.npc_clue {
                lines.push(format!(
                    "{} ×{} near {} at {} ({}, {}) — {source}",
                    grant.item_name, grant.amount, npc.name, npc.map, npc.x, npc.y
                ));
                if is_graph_map(&npc.map) {
                    lines.push(format!("@route:{}", npc.map));
                }
            } else {
                lines.push(format!("{} ×{} — {source}", grant.item_name, grant.amount));
            }
        }
        if grants.len() > 10 {
            lines.push(format!("{} additional script clues omitted.", grants.len() - 10));
        }
        lines.push(
            "These calls show possible grants in the script; the surrounding conditions and whether a player can trigger them are not \
             reviewed."
                .to_owned(),
        );
    }
    let uses = reference_data()
        .item_consumptions
        .iter()
        .filter(|use_ref| use_ref.item_id == item.id)
        .collect::<Vec<_>>();
    if !uses.is_empty() {
        lines.push("Consumed in loaded NPC scripts (possible requirements or exchanges):".to_owned());
        for use_ref in uses.iter().take(10) {
            let evidence = format!("{}:{}", use_ref.source.path, use_ref.source.line);
            if let Some(npc) = &use_ref.npc_clue {
                lines.push(format!(
                    "{} ×{} near {} at {} ({}, {}) — {evidence}",
                    use_ref.item_name, use_ref.amount, npc.name, npc.map, npc.x, npc.y
                ));
                if is_graph_map(&npc.map) {
                    lines.push(format!("@route:{}", npc.map));
                }
            } else {
                lines.push(format!("{} ×{} — {evidence}", use_ref.item_name, use_ref.amount));
            }
        }
        if uses.len() > 10 {
            lines.push(format!("{} additional consumption clues omitted.", uses.len() - 10));
        }
        lines.push("These literal removals do not prove an exchange; rewards and conditions may be elsewhere or dynamic.".to_owned());
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
        lines.push("Source-reviewed exchanges:".to_owned());
        for exchange in exchanges {
            lines.push(format!(
                "{} — @guide:npc:{}|{} at {} ({}, {})",
                exchange.title, exchange.npc.npc_id, exchange.npc.name, exchange.npc.map, exchange.npc.x, exchange.npc.y
            ));
            let cost = exchange
                .inputs
                .iter()
                .map(|input| format!("@guide:item:{}|{} ×{}", input.item_id, input.item_name, input.amount))
                .collect::<Vec<_>>()
                .join(", ");
            if !cost.is_empty() {
                lines.push(format!("Costs: {cost}"));
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
                "Reviewed source: {} lines {:?}",
                exchange.source.path, exchange.source.reviewed_lines
            ));
            lines.push(format!("Reviewed by {} on {}", exchange.reviewed_by, exchange.reviewed_on));
            if is_graph_map(&exchange.npc.map) {
                lines.push(format!(
                    "@route-cell:{}:{}:{}|Route to exchange — {}",
                    exchange.npc.map, exchange.npc.x, exchange.npc.y, exchange.npc.map
                ));
            }
        }
    }
    if item.buy > 0 {
        lines.push(format!("Buy price: {}z", item.buy));
    }
    if let Some(sell) = item.sell {
        lines.push(format!("Sell price: {sell}z"));
    }
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
            lines.push(format!("Equipment location: {readable}"));
        } else if let Some(flags) = location.as_i64() {
            lines.push(format!("Equipment location flags: {flags}"));
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
                    "combo effect script not translated"
                } else {
                    "no combo script field in source"
                }
            });
            lines.push(format!("{members} — {effect_note}"));
            lines.push(format!("Combo source: {} ({})", combo.source.path, combo.source.record));
        }
        if item.combos.len() > 8 {
            lines.push(format!("{} additional item combinations omitted.", item.combos.len() - 8));
        }
    }
    if !item.group_contents.is_empty() {
        lines.push("Randomized group contents (configured chance for one group selection):".to_owned());
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
            lines.push(format!("Contents source: {} ({})", entry.source.path, entry.source.record));
        }
        lines.push(
            "These are configured group-selection chances; custom item scripts may apply additional rules or grant multiple results."
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
    if !item.shops.is_empty() {
        lines.push("Literal shop listings in loaded NPC scripts (availability or conditions not reviewed):".to_owned());
        for shop in item.shops.iter().take(8) {
            let price = if shop.uses_item_db_price {
                if item.buy > 0 {
                    format!("{} {} (item DB value)", item.buy, shop.currency)
                } else {
                    "item DB value".to_owned()
                }
            } else {
                format!("{} {}", shop.price.unwrap_or_default(), shop.currency)
            };
            lines.push(format!(
                "{} — {} at {} ({}, {}) — {} — {}",
                shop.npc_name, shop.shop_type, shop.map, shop.x, shop.y, price, shop.source
            ));
            if is_graph_map(&shop.map) {
                lines.push(format!("@route:{}", shop.map));
            }
        }
        if item.shops.len() > 8 {
            lines.push(format!("{} additional loaded shop listings omitted.", item.shops.len() - 8));
        }
        lines.push(
            "Only literal stock declarations from loaded NPC scripts are indexed; conditional and runtime-added stock may be missing."
                .to_owned(),
        );
    }
    if let Some(summary) = &item.effect_summary {
        lines.push(format!("Effect: {summary}"));
        lines.push(
            "Effect summaries cover only recognized script patterns. Conditions and trigger odds are included when supported; stacking \
             and cross-effect interactions may remain undocumented."
                .to_owned(),
        );
    } else if item.effect_status == "scripted_not_translated" {
        lines.push("Script effect: not translated yet; conditions and interactions are not documented.".to_owned());
    } else if item.effect_status == "no_script_field" {
        lines.push("No item script effect field is present in the loaded item record.".to_owned());
    } else if !item.effect_status.is_empty() {
        lines.push(format!("Effect coverage: {}", item.effect_status.replace('_', " ")));
    } else {
        lines.push("Detailed script effect: not documented yet.".to_owned());
    }
    if !item.drops_from.is_empty() {
        lines.push("Monster database drop rates (server modifiers may change realized chances):".to_owned());
        for source in item.drops_from.iter().take(8) {
            lines.push(format!(
                "@guide:monster:{}|{} (ID {}) — {} drop {:.2}%",
                source.monster_id,
                source.sprite_name,
                source.monster_id,
                if source.kind == "mvp" { "MVP" } else { "normal" },
                source.rate_per_10000 as f32 / 100.0
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

fn refinement_details(refinement: &ReferenceRefinement) -> Vec<String> {
    let mut lines = vec![
        "Whitesmith Weapon Refine (WS_WEAPONREFINE)".to_owned(),
        "The chance shown is the configured base chance; the caster's job level changes the final chance.".to_owned(),
        format!(
            "Bonus: {} percentage points per job level relative to 50 (positive above 50, negative below), or +{} points for Mechanic \
             (Transcendent).",
            refinement.job_level_bonus_per_job_level_from_50_per_mille as f32 / 10.0,
            refinement.mechanic_transcendent_flat_bonus_percent
        ),
        format!(
            "A weapon can be refined only below the skill level and below +{}.",
            refinement.max_useful_refine_level
        ),
        format!("On failure: {}.", refinement.on_failure),
        "No Zeny cost is charged by this skill; one material is consumed per attempt.".to_owned(),
    ];
    for weapon in &refinement.weapon_levels {
        lines.push(format!("Weapon Level {} — material: {}", weapon.weapon_level, weapon.material));
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
    lines.push(
        "Source: bundled Hercules refine_db.conf and skill_weaponrefine implementation. Rates may differ if the server data changes."
            .to_owned(),
    );
    lines
}

fn refinement_query_matches(query: &str) -> bool {
    let query = query.to_lowercase();
    query.is_empty()
        || ["refine", "weapon", "phracon", "emveretarcon", "oridecon", "whitesmith", "mechanic"]
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
    lines.push("Values are exported from this Hercules source revision and can change with server configuration.".to_owned());
    lines
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
        true => "campaign script (source withheld)".to_owned(),
        false => format!("{path}:{line}"),
    }
}

fn is_graph_map(map_name: &str) -> bool {
    crate::world::navigation_graph()
        .maps
        .iter()
        .any(|known| known.eq_ignore_ascii_case(map_name))
}

fn display_name(name: &str, fallback: &str) -> String {
    if name.is_empty() { fallback.to_owned() } else { name.to_owned() }
}

fn quest_details(quest: &crate::state::quests::QuestEntry) -> Vec<String> {
    let data = reference_data();
    let mut lines = vec![format!("{}  (Quest ID {})", quest.name(), quest.quest_id)];
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
    let mut lines = vec![format!("{} — bundled hunt reference", quest.name)];
    if let Some(flow) = &quest.flow_review {
        lines.push(format!(
            "Source-reviewed flow: {} [{}; reviewed {}]",
            flow.title,
            flow.evidence_state.label(),
            flow.reviewed_on
        ));
        for condition in &flow.conditions {
            lines.push(format!("Verified condition: {condition}"));
        }
        for reward in &flow.verified_item_rewards {
            lines.push(format!(
                "@guide:item:{}|{} ×{} — {} [{}; {}:{}]",
                reward.item_id,
                reward.item_name,
                reward.amount,
                reward.explanation,
                reward.evidence_state.label(),
                reward.source_path,
                reward.source_line
            ));
        }
        for source in &flow.sources {
            lines.push(format!("Flow source: {} lines {:?}", source.path, source.lines));
        }
        lines.push(format!("Reviewed by {} on {}", flow.reviewed_by, flow.reviewed_on));
        lines.push(format!("Review method: {}", flow.review_method));
    }
    if quest.targets.is_empty() {
        lines.push("No explicit hunt targets are recorded for this quest.".to_owned());
    }
    if quest.npc_references.is_empty() {
        lines.push("No NPC giver or turn-in has been source-reviewed in this quest entry.".to_owned());
    }
    for target in &quest.targets {
        let title = format!("{} × {}", target.monster_name, target.count);
        lines.push(format!("Objective: {title}"));
        if let Some(level) = target.level_range {
            lines.push(format!("Target level bounds (raw server values): {} / {}", level[0], level[1]));
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
                lines.push(format!("Monster ID {mob_id} has no matching bundled bestiary entry."));
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
        lines.push("Nearby literal item-grant candidates (not verified quest rewards):".to_owned());
        for candidate in unreviewed_reward_candidates.iter().take(16) {
            lines.push(format!(
                "@guide:item:{}|{} × {} — candidate near quest completion",
                candidate.item_id, candidate.item_name, candidate.amount
            ));
            lines.push(format!(
                "Candidate evidence: {}:{}; nearby completequest call at line {} (offset {} lines)",
                candidate.source_path, candidate.source_line, candidate.nearby_completequest_line, candidate.distance_lines
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
        let reviewed_label = match npc.reviewed_role.as_deref() {
            Some("offer") => Some("Reviewed quest offer"),
            Some("turn_in") => Some("Reviewed quest turn-in"),
            _ => None,
        };
        if let Some(label) = reviewed_label {
            lines.push(format!(
                "{label}: {} — {} ({}, {}) [{}; source lines {:?}]",
                npc.name, npc.map_name, npc.x, npc.y, npc.source_path, npc.reviewed_source_lines
            ));
            if let Some(evidence) = &npc.review_evidence {
                lines.push(format!(
                    "Source-reviewed route: {evidence} Requirements or availability may still apply."
                ));
            }
            if let Some(reward) = &npc.verified_reward {
                let item_name = data
                    .item_by_id(reward.item_id)
                    .map(|item| display_name(&item.name, &item.aegis_name))
                    .unwrap_or_else(|| format!("Item {}", reward.item_id));
                lines.push(format!(
                    "Verified turn-in reward: {} base EXP + @guide:item:{}|{} ×{}",
                    reward.base_exp, reward.item_id, item_name, reward.item_amount
                ));
            }
        } else {
            lines.push(format!(
                "Related NPC script reference: {} — {} ({}, {}) [{}:{}]",
                npc.name, npc.map_name, npc.x, npc.y, npc.source_path, npc.source_line
            ));
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
                _ => "Route to related script NPC",
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
            "{} additional related NPC script references omitted.",
            quest.npc_references.len() - 8
        ));
    }
    lines.push(
        "Unreviewed quest steps, prerequisites, branch logic, and rewards remain incomplete unless explicitly source-reviewed above."
            .to_owned(),
    );
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
        lines.push("Configured map flags from loaded scripts:".to_owned());
        for entry in configured_flags.iter().take(24) {
            let value = if entry.value.is_empty() {
                String::new()
            } else {
                format!(" ({})", entry.value)
            };
            lines.push(format!(
                "{}{}: {} — {}",
                entry.flag,
                value,
                entry.description,
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
        lines.push(
            "These show the last literal directive in loaded-file order. Runtime setmapflag/removemapflag calls and map-zone \
             configuration can still change effective rules."
                .to_owned(),
        );
    }
    for review in reference
        .runtime_map_flag_reviews
        .iter()
        .filter(|review| review.map.eq_ignore_ascii_case(map_name))
    {
        lines.push(format!(
            "Source-reviewed runtime rule: {} [{}]",
            review.title,
            review.evidence_state.label()
        ));
        lines.push(review.summary.clone());
        for condition in &review.conditions {
            lines.push(format!("Condition: {condition}"));
        }
        for source in &review.sources {
            lines.push(format!("Reviewed source: {} lines {:?}", source.path, source.lines));
        }
        lines.push(format!("Reviewed by {} on {}", review.reviewed_by, review.reviewed_on));
    }
    let mut runtime_flags = reference
        .runtime_map_flag_clues
        .iter()
        .filter(|clue| clue.map.as_deref().is_some_and(|map| map.eq_ignore_ascii_case(map_name)))
        .collect::<Vec<_>>();
    runtime_flags.sort_by_key(|clue| (clue.flag.to_lowercase(), clue.source.path.clone(), clue.source.line));
    if !runtime_flags.is_empty() {
        lines.push("Runtime map-flag call sites (source clues; execution and conditions unreviewed):".to_owned());
        for clue in runtime_flags.iter().take(16) {
            lines.push(format!(
                "{}({}, {}){} — {}:{}",
                clue.operation,
                clue.map_expression,
                clue.flag,
                if clue.value.is_empty() {
                    String::new()
                } else {
                    format!(", {}", clue.value)
                },
                clue.source.path,
                clue.source.line
            ));
        }
        if runtime_flags.len() > 16 {
            lines.push(format!(
                "{} additional runtime map-flag clues omitted.",
                runtime_flags.len() - 16
            ));
        }
        lines.push("A call site does not prove that its event runs or establish the map's current rules.".to_owned());
    }
    match reference.map_spawn_details(map_name) {
        Some(details) => {
            let range_str = if details.min_level != details.max_level {
                format!(" (range: {}–{})", details.min_level, details.max_level)
            } else {
                String::new()
            };
            lines.push(format!(
                "Suggested level: ~{}{range_str} (static-spawn-record-weighted mean; reference only)",
                details.mean_level
            ));
            lines.push(format!(
                "Static population: {} spawn records across {} species",
                details.records, details.species
            ));
        }
        None => {
            lines.push(if is_template {
                "No base-map static population is attached to this instance template.".to_owned()
            } else {
                "Suggested level: unavailable (no verified static spawn records; low coverage)".to_owned()
            });
            if !is_template {
                lines.push("Static population: no verified spawn records (low coverage or non-combat map)".to_owned());
            }
        }
    }

    let mut static_mobs = reference
        .monsters
        .iter()
        .filter_map(|monster| {
            monster
                .spawn_regions
                .iter()
                .find(|region| region.map.eq_ignore_ascii_case(map_name))
                .map(|region| (monster, region))
        })
        .collect::<Vec<_>>();
    static_mobs.sort_by_key(|(monster, _)| (monster.name.to_lowercase(), monster.id));
    if !static_mobs.is_empty() {
        lines.push(format!("Static monster roster ({} species):", static_mobs.len()));
        for (monster, region) in &static_mobs {
            lines.push(format!(
                "@guide:monster:{}|{} — {} listed across {} configured spawn records",
                monster.id,
                display_name(&monster.name, &monster.sprite_name),
                region.listed_monsters,
                region.spawn_records
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
            "Scripted spawn clues ({} species; some map links are approximate; triggers and runtime populations unknown):",
            scripted_mobs.len()
        ));
        for (monster, call_sites) in &scripted_mobs {
            lines.push(format!(
                "@guide:monster:{}|{} — {} call sites",
                monster.id,
                display_name(&monster.name, &monster.sprite_name),
                call_sites
            ));
        }
    } else {
        lines.push("No loaded scripted-spawn call is currently linked to this map.".to_owned());
    }
    lines.push(
        "The roster lists configured static spawns and identifiable script call sites; it does not show live counts. Dynamic map \
         expressions may prevent some script spawns from being assigned to this map."
            .to_owned(),
    );

    let mut exits: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "walk_warp" && edge.from.map.eq_ignore_ascii_case(map_name))
        .collect();
    exits.sort_by_key(|edge| (edge.to.map.to_ascii_lowercase(), edge.from.x, edge.from.y, edge.to.x, edge.to.y));
    lines.push(format!("Verified outgoing portal connections: {}", exits.len()));
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
        lines.push(format!("Verified NPC travel services: {}", services.len()));
        for edge in services {
            let action = edge.action.as_deref().unwrap_or("Talk to the listed NPC.");
            let availability = if edge.availability == "conditional" { " (conditional)" } else { "" };
            lines.push(format!(
                "Service at ({}, {}): {action}{availability} → {}",
                edge.from.x, edge.from.y, edge.to.map
            ));
            if let Some(requirements) = &edge.requirements {
                lines.push(format!("Requirement: {requirements}"));
            }
            if let Some(source) = &edge.source {
                lines.push(format!("Reviewed source: {source}"));
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
        lines.push(format!("Towninfo facility: {} at ({x}, {y})", poi.name));
        lines.push(format!("@route-cell:{map_name}:{x}:{y}|Route to {} — {map_name}", poi.name));
        routeable_poi_count += 1;
    }
    let valid_poi_count = town_pois
        .iter()
        .filter(|poi| u16::try_from(poi.x).is_ok() && u16::try_from(poi.y).is_ok())
        .count();
    if valid_poi_count > routeable_poi_count {
        lines.push(format!(
            "{} additional Towninfo facility routes omitted.",
            valid_poi_count - routeable_poi_count
        ));
    }
    if town_pois.is_empty() {
        lines.push("Towninfo facilities: none listed for this map".to_owned());
    }
    let map_services = reference.services_for_map(map_name);
    if !map_services.is_empty() {
        lines.push(format!("Reviewed NPC services ({}):", map_services.len()));
        for service in map_services {
            if let Some(index) = reference.npc_service_index_by_id(&service.id) {
                lines.push(format!("@guide:service:{index}|{} ({})", service.title, service.service_kind));
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
    let kind = match npc.declared_type.as_str() {
        "shop" => "Static shop declaration",
        "cashshop" => "Cash shop declaration",
        "trader" => "Trader declaration (stock may be script-driven)",
        "warp" => "Warp declaration",
        _ => "NPC script declaration (specific behavior not inferred)",
    };
    let mut lines = vec![
        format!("{display}"),
        format!("Type: {kind}"),
        format!("Map: {} at ({}, {})", npc.map, npc.x, npc.y),
    ];
    if !npc.internal_name.is_empty() {
        lines.push(format!("Internal script name: {}", npc.internal_name));
    }
    if !npc.sprite.is_empty() && npc.declared_type != "warp" {
        lines.push(format!("Declared sprite: {}", npc.sprite));
    }
    lines.push(format!("Source: {}:{}", npc.source.path, npc.source.line));
    for exchange in reference_data()
        .item_exchanges
        .iter()
        .filter(|exchange| exchange.npc.npc_id == npc.id)
    {
        lines.push(format!("Reviewed service: {}", exchange.npc.service_role));
        lines.push(format!(
            "Source-reviewed exchange: {} [{}]",
            exchange.title,
            exchange.evidence_state.label()
        ));
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
            "Reviewed source: {} lines {:?}",
            exchange.source.path, exchange.source.reviewed_lines
        ));
        lines.push(format!("Reviewed by {} on {}", exchange.reviewed_by, exchange.reviewed_on));
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
            lines.push("No literal stock was linked to this declaration; conditional or runtime stock may still exist.".to_owned());
        } else {
            lines.push(format!("Indexed literal stock ({} offers):", npc.offers.len()));
            for offer in npc.offers.iter().take(24) {
                let price = if offer.uses_item_db_price {
                    format!("item DB price ({})", offer.currency)
                } else {
                    format!("{} {}", offer.price.unwrap_or_default(), offer.currency)
                };
                lines.push(format!("@guide:item:{}|{} — {price}", offer.item_id, offer.item_name));
            }
            if npc.offers.len() > 24 {
                lines.push(format!("{} additional offers omitted.", npc.offers.len() - 24));
            }
            if let Some(offer) = npc.offers.first() {
                lines.push(format!("Offer source: {}", offer.source));
            }
        }
        lines
            .push("Only literal offers from loaded scripts are indexed; conditional, barter, and runtime stock may be missing.".to_owned());
    }
    for service in reference_data()
        .npc_services
        .iter()
        .filter(|s| s.npc.as_ref().is_some_and(|n| n.npc_id == npc.id))
    {
        if let Some(index) = reference_data().npc_service_index_by_id(&service.id) {
            lines.push(format!(
                "@guide:service:{index}|Reviewed service: {} ({})",
                service.title, service.service_kind
            ));
        }
    }
    lines
}

fn service_details(service: &ReferenceNpcServiceReview) -> Vec<String> {
    let mut lines = vec![
        format!("{}  (Service: {})", service.title, service.service_kind),
        format!("Reviewed NPC service ({}):", service.evidence_state.label()),
        format!("  Review method: {}", service.review_method),
        format!("  Reviewed on: {} by {}", service.reviewed_on, service.reviewed_by),
    ];
    if let Some(npc) = &service.npc {
        lines.push(format!(
            "Primary NPC: {} on {} at ({}, {}) [ID {}]",
            npc.internal_name, npc.map, npc.x, npc.y, npc.npc_id
        ));
        lines.push(format!(
            "@route-cell:{}:{}:{}|Route to {} ({}, {})",
            npc.map, npc.x, npc.y, npc.internal_name, npc.x, npc.y
        ));
        lines.push(format!(
            "@guide:npc:{}|NPC Record: {} ({})",
            npc.npc_id, npc.internal_name, npc.map
        ));
        if is_graph_map(&npc.map) {
            lines.push(format!("@route:{}", npc.map));
        }
    }
    if !service.conditions.is_empty() {
        lines.push("Conditions & service rules:".to_owned());
        for condition in &service.conditions {
            lines.push(format!("  - {condition}"));
        }
    }
    if let Some(route_table) = &service.route_table {
        lines.push("Kafra transportation routes:".to_owned());
        for route in route_table {
            lines.push(format!("  Origin map {}:", route.origin_map));
            for dest in &route.destinations {
                lines.push(format!("    -> {} (Fee: {} Zeny)", dest.name, dest.fee));
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
        lines.push(format!(
            "Staff deployment: verified across {} locations in the game world.",
            count
        ));
    }
    if !service.sources.is_empty() {
        let citations = service
            .sources
            .iter()
            .map(|s| {
                format!(
                    "{}:{}",
                    s.path,
                    s.lines.iter().map(ToString::to_string).collect::<Vec<_>>().join(",")
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        lines.push(format!("Citations: {citations}"));
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
        "mechanic" => refinement_details(&data.refinement),
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
            .map(|map_name| map_details(map_name, library.town_pois(map_name)))
            .unwrap_or_else(|| vec!["Map entry unavailable.".to_owned()]);
    }
    resolve_details(result)
}

fn job_details(job_id: u16, name: &str) -> Vec<String> {
    let mut lines = vec![format!("{name}  (Job ID {job_id})")];
    let data = reference_data();
    if let Some(bonuses) = data.job_bonuses_by_id(job_id) {
        append_job_bonus_details(&mut lines, bonuses);
    } else {
        lines.push("No job-level stat bonus schedule is present for this job ID in Hercules job_db2.txt.".to_owned());
    }
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
        "Sources: bundled Hercules renewal skill_tree.conf and skill_db.conf for skills; job_db2.txt for job-level stat bonuses. \
         Conditional-script effects are not inferred."
            .to_owned(),
    );
    lines
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
    lines.push(format!("Skill identifier: {} (ID {})", skill.name, skill.id));
    let data = reference_data();

    if !skill.prerequisites.is_empty() {
        lines.push("Skill tree prerequisites:".to_owned());
        for req in &skill.prerequisites {
            if let Some(req_id) = req.skill_id {
                lines.push(format!("@guide:skill:{req_id}|Requires: {} Lv {}", req.name, req.level));
            } else {
                lines.push(format!("Requires: {} Lv {}", req.name, req.level));
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
        lines.push("Job-tree availability and prerequisites:".to_owned());
        for (tree, job_skill) in job_sources.iter().take(12) {
            let mut details = format!("{} — max Lv {}", tree.tree_name, job_skill.max_level);
            if job_skill.minimum_job_level > 0 {
                details.push_str(&format!(", job Lv {}", job_skill.minimum_job_level));
            }
            if !job_skill.prerequisites.is_empty() {
                let prerequisites = job_skill
                    .prerequisites
                    .iter()
                    .map(|prerequisite| format!("{} Lv {}", prerequisite.name, prerequisite.level))
                    .collect::<Vec<_>>()
                    .join(", ");
                details.push_str(&format!(", requires {prerequisites}"));
            }
            lines.push(format!("@guide:job:{}|{details}", tree.job_id));
        }
        if job_sources.len() > 12 {
            lines.push(format!("{} additional job trees omitted.", job_sources.len() - 12));
        }
    } else {
        lines.push("No exported job-tree requirement record is available for this skill.".to_owned());
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
        lines.push("Associated status references:".to_owned());
        for status in &linked_statuses {
            lines.push(format!("@guide:status:{}|{} ({})", status.id, status.name, status_tag(status)));
        }
    }
    if let Some(status_change) = &skill.status_change {
        let has_linked_icon = linked_statuses
            .iter()
            .any(|status| status.statuses.iter().any(|mechanic| mechanic.constant == *status_change));
        if !has_linked_icon {
            lines.push(format!(
                "Skill database StatusChange: {status_change} (no matching client status icon)"
            ));
        }
    }

    if let Some(review) = data.skill_formula_review_for_skill(skill.id) {
        lines.push(format!("Reviewed combat formula ({}):", review.evidence_state.label()));
        lines.push(format!("  Title: {}", review.title));
        lines.push(format!("  Formula: {}", review.formula));
        lines.push(format!("  Worked example: {}", review.worked_example));
        if !review.conditions.is_empty() {
            lines.push("  Conditions & caveats:".to_owned());
            for cond in &review.conditions {
                lines.push(format!("  - {cond}"));
            }
        }
        if !review.sources.is_empty() {
            let citations = review
                .sources
                .iter()
                .map(|s| {
                    format!(
                        "{}:{}",
                        s.path,
                        s.lines.iter().map(ToString::to_string).collect::<Vec<_>>().join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            lines.push(format!("  Citations: {citations}"));
        }
    } else {
        lines.push("Formula: Unreviewed in Hercules renewal engine source.".to_owned());
    }

    lines.push("Source: bundled Hercules skill database export.".to_owned());
    if let Some(source) = &skill.source {
        lines.push(format!("Source record: {} ({})", source.path, source.record));
    }
    lines
}

/// How a status is identified in a list: its client icon id, or, for the
/// classic ailments that have no icon, the server constant.
fn status_tag(status: &crate::dm::reference_data::ReferenceStatus) -> String {
    match (status.iconless, status.statuses.first()) {
        (true, Some(mechanic)) => format!("server status {}", mechanic.constant),
        _ => format!("icon {}", status.id),
    }
}

fn status_details(status: &crate::dm::reference_data::ReferenceStatus) -> Vec<String> {
    let mut lines = vec![match status.iconless {
        true => format!("{}  ({})", status.name, status_tag(status)),
        false => format!("{}  (Status icon ID {})", status.name, status.id),
    }];
    if status.iconless {
        lines.push("The client has no icon or name for this status; the name is derived from the server constant.".to_owned());
    }
    if status.statuses.is_empty() {
        lines.push("Verified reference: server status-icon name only; no matching sc_config record.".to_owned());
    } else {
        lines.push("Verified server metadata from renewal sc_config.conf:".to_owned());
        for mechanic in &status.statuses {
            lines.push(format!("{} (status ID {})", mechanic.constant, mechanic.id));
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
                let label = if skill.description.is_empty() {
                    skill.name.clone()
                } else {
                    format!("{} ({})", skill.description, skill.name)
                };
                lines.push(format!("@guide:skill:{}|Associated skill: {label}", skill.id));
            }
            for skill in &mechanic.status_change_skills {
                if mechanic
                    .associated_skill
                    .as_ref()
                    .is_some_and(|associated| associated.id == skill.id)
                {
                    continue;
                }
                let label = if skill.description.is_empty() {
                    skill.name.clone()
                } else {
                    format!("{} ({})", skill.description, skill.name)
                };
                lines.push(format!("@guide:skill:{}|Skill database StatusChange: {label}", skill.id));
            }
            if !mechanic.code_call_sites.is_empty() {
                lines.push(format!(
                    "Literal Hercules C sc_start call sites (not exhaustive; {} total):",
                    mechanic.code_call_sites.len()
                ));
                for source in mechanic.code_call_sites.iter().take(8) {
                    lines.push(format!("  {}:{}", source.path, source.line));
                }
                if mechanic.code_call_sites.len() > 8 {
                    lines.push(format!("  … and {} more", mechanic.code_call_sites.len() - 8));
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
            let count = state.get(&self.path).len();
            for index in 0..count {
                let line = self.path.index(index).manually_asserted();
                let value = state.get(&line).clone();
                if let Some(monster_id) = value
                    .strip_prefix("@hunting-goal:")
                    .and_then(|link| link.split_once('|'))
                    .and_then(|(monster_id, _)| monster_id.parse::<u32>().ok())
                {
                    self.elements.push(ErasedElement::new(button! {
                        text: "Add to personal hunting goals (client-only)",
                        tooltip: "Saved for this character. This is not a server quest and has no kill counter.",
                        event: InputEvent::AddClientHuntingGoal { monster_id },
                    }));
                } else if let Some((map_name, x, y, label)) = parse_route_cell_link(&value) {
                    self.elements.push(ErasedElement::new(button! {
                        text: label,
                        event: InputEvent::SetNavigationDestination { map_name, x, y },
                    }));
                } else if let Some(map_name) = value.strip_prefix("@route:") {
                    let map_name = map_name.to_owned();
                    self.elements.push(ErasedElement::new(button! {
                        text: format!("Route to {map_name}"),
                        event: InputEvent::SetNavigationMapDestination { map_name },
                    }));
                } else if let Some(result) = parse_guide_link(&value) {
                    let detail_path = self.path;
                    self.elements.push(ErasedElement::new(button! {
                        text: result.label.clone(),
                        event: move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
                            state.update_value(detail_path, resolve_details(&result));
                        },
                    }));
                } else {
                    self.elements.push(ErasedElement::new(
                        text! { text: line, overflow_behavior: OverflowBehavior::Shrink },
                    ));
                }
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

fn run_search<A>(state: &State<ClientState>, path: A)
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy,
{
    let query = state.get(&path.query()).to_lowercase();
    let category = state.get(&path.category()).clone();
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
            label: format!("{}  (ID {})", display_name(&skill.description, &skill.name), skill.id),
            kind: "skill".to_owned(),
            id: skill.id as u32,
        }));
    } else if category == "Status Effects" {
        rows.extend(data.search_statuses(&query, MAX_RESULTS).into_iter().map(|status| GuideResult {
            label: format!("{}  ({})", status.name, status_tag(status).replacen("icon", "status icon", 1)),
            kind: "status".to_owned(),
            id: status.id,
        }));
    } else if category == "NPCs" {
        rows.extend(data.search_npcs(&query, MAX_RESULTS).into_iter().map(|npc| GuideResult {
            label: format!(
                "{}  ({} • {} {}, {})",
                npc.display_name, npc.declared_type, npc.map, npc.x, npc.y
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
                .filter(|(_, map)| query.is_empty() || map.to_lowercase().contains(&query))
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
                .filter(|(_, map)| query.is_empty() || map.to_lowercase().contains(&query))
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
                    service.service_kind,
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
                id: 1,
            });
        }
        for weapon in &data.refinement.weapon_levels {
            let label = format!("Weapon Level {} refinement ({})", weapon.weapon_level, weapon.material);
            if query.is_empty() || label.to_lowercase().contains(&query) || weapon.material.to_lowercase().contains(&query) {
                rows.push(GuideResult {
                    label,
                    kind: "mechanic".to_owned(),
                    id: 1,
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
                id: 1,
            });
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
                "{}  (Quest {}){}",
                quest.name,
                quest.id,
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
                label: format!("{}  (Quest {})", quest.name(), quest.quest_id),
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
                    label: format!("{}  (ID {})", display_name(&item.name, &item.aegis_name), item.id),
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
            id: 1,
        });
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
                label: format!("{}  (Item, ID {})", display_name(&item.name, &item.aegis_name), item.id),
                kind: "item".to_owned(),
                id: item.id,
            }),
    );
    rows.extend(
        data.search_cards(query, all_category_result_slots(&rows))
            .into_iter()
            .take(all_category_result_slots(&rows))
            .map(|card| GuideResult {
                label: format!("{}  (Card, ID {})", display_name(&card.name, &card.aegis_name), card.id),
                kind: "card".to_owned(),
                id: card.id,
            }),
    );
    rows.extend(
        data.search_skills(query, all_category_result_slots(&rows))
            .into_iter()
            .map(|skill| GuideResult {
                label: format!("{}  (Skill, ID {})", display_name(&skill.description, &skill.name), skill.id),
                kind: "skill".to_owned(),
                id: skill.id as u32,
            }),
    );
    rows.extend(
        data.search_statuses(query, all_category_result_slots(&rows))
            .into_iter()
            .map(|status| GuideResult {
                label: format!("{}  ({})", status.name, status_tag(status).replacen("icon", "Status icon", 1)),
                kind: "status".to_owned(),
                id: status.id,
            }),
    );
    rows.extend(
        data.search_npcs(query, all_category_result_slots(&rows))
            .into_iter()
            .map(|npc| GuideResult {
                label: format!("{}  (NPC: {} — {})", npc.display_name, npc.declared_type, npc.map),
                kind: "npc".to_owned(),
                id: npc.id,
            }),
    );

    rows.extend(
        crate::world::navigation_graph()
            .maps
            .iter()
            .enumerate()
            .filter(|(_, map)| query.is_empty() || map.to_lowercase().contains(query))
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
            .filter(|(_, map)| query.is_empty() || map.to_lowercase().contains(query))
            .take(all_category_result_slots(&rows))
            .map(|(index, map)| GuideResult {
                label: format!("{map}  (Instance map template)"),
                kind: "map-template".to_owned(),
                id: index as u32,
            }),
    );
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
                label: format!("{}  (Quest {})", quest.name, quest.id),
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
                        service.service_kind
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

/// Select an item from another in-game surface, populate the Guide search and
/// detail panes, and leave the guide ready to continue browsing.
pub fn open_item_entry<A>(state: &State<ClientState>, path: A, item_id: u32)
where
    A: Path<ClientState, AdventureGuideWindowState> + Copy,
{
    state.update_value(path.category(), "Items".to_owned());
    state.update_value(path.query(), item_id.to_string());
    run_search(state, path);
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
        let search = move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| run_search(state, path);
        let set_category = |category: &'static str| {
            move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
                state.update_value(path.category(), category.to_owned());
                run_search(state, path);
            }
        };
        let revision = reference_data().source_revision.clone();
        window! {
            title: "Adventure Guide",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! { text: format!("Open reference • data revision {} • discovery badges sync per account; mechanics remain open • untranslated scripts and missing spawn data are labeled", revision), overflow_behavior: OverflowBehavior::Shrink },
                text_box! { ghost_text: "Search monsters, items, cards, skills, statuses, jobs, maps, NPCs, services, quests, rumors, refinement, rules…", state: path.query(), input_handler: DefaultHandler::<_, _, MAX_QUERY>::new(path.query(), search), focus_id: GuideSearchBox, overflow_behavior: OverflowBehavior::Shrink },
                split! { gaps: theme().window().gaps(), children: (
                    button! { text: "All", event: set_category("All") },
                    button! { text: "Monsters", event: set_category("Monsters") },
                    button! { text: "Items", event: set_category("Items") },
                    button! { text: "Cards", event: set_category("Cards") },
                    button! { text: "Skills", event: set_category("Skills") },
                    button! { text: "Status Effects", event: set_category("Status Effects") },
                    button! { text: "Jobs", event: set_category("Jobs") },
                    button! { text: "Maps", event: set_category("Maps") },
                ) },
                split! { gaps: theme().window().gaps(), children: (
                    button! { text: "NPCs", event: set_category("NPCs") },
                    button! { text: "Services", event: set_category("Services") },
                    button! { text: "Quests", event: set_category("Quests") },
                    button! { text: "Rumors", event: set_category("Rumors") },
                    button! { text: "Refinement", event: set_category("Refinement") },
                    button! { text: "Effective Rules", event: set_category("Effective Rules") },
                    button! { text: "Mechanics", event: set_category("Mechanics") },
                    button! { text: "Search", event: search },
                ) },
                scroll_view! { children: GuideResultList { state_path: path, library: library.clone(), elements: Vec::new() } },
                scroll_view! { children: GuideLines::new(path.detail()) },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GuideResult, ReferenceItem, display_name, item_details, job_matches, job_names, map_details, monster_details, parse_guide_link,
        parse_route_cell_link, quest_details, quest_reference_details, reference_data, refinement_details, resolve_details, rumor_details,
        search_all_categories, service_details, skill_details, status_details, status_tag,
    };
    use crate::dm::reference_data::{ReferenceQuest, ReferenceQuestTarget};
    use crate::state::discovery::DiscoveryState;
    use crate::state::quests::{QuestEntry, QuestHuntObjectiveEntry, QuestRequirementEntry};
    use crate::world::{TownPoi, TownPoiKind};

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
                .any(|line| line == &format!("@hunting-goal:{}|Add to personal hunting goals (client-only)", monster.id))
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
        assert!(detail.contains("Source record: db/re/skill_db.conf (Id=19)"));
    }

    #[test]
    fn guide_skill_details_display_reviewed_formulas_and_unreviewed_flags() {
        let data = reference_data();

        // Heal (28) has a reviewed formula
        let heal = data.search_skills("AL_HEAL", 1).into_iter().next().expect("Heal skill");
        let heal_details = skill_details(heal).join("\n");
        assert!(
            heal_details.contains("Reviewed combat formula (conditional):"),
            "{heal_details}"
        );
        assert!(heal_details.contains("Heal's Renewal healing formula"), "{heal_details}");
        assert!(
            heal_details.contains("Formula: AL_HEAL (id 28) computes its healed HP"),
            "{heal_details}"
        );
        assert!(heal_details.contains("Worked example:"), "{heal_details}");
        assert!(heal_details.contains("Citations: src/map/skill.c:"), "{heal_details}");

        // Fire Bolt (19) has a reviewed formula
        let fire_bolt = data.search_skills("MG_FIREBOLT", 1).into_iter().next().expect("Fire Bolt skill");
        let bolt_details = skill_details(fire_bolt).join("\n");
        assert!(bolt_details.contains("Reviewed combat formula"), "{bolt_details}");
        assert!(bolt_details.contains("Fire/Cold/Lightning Bolt"), "{bolt_details}");

        // Basic Skill (1) is unreviewed and should be explicitly flagged
        let basic = data.search_skills("NV_BASIC", 1).into_iter().next().expect("Basic Skill");
        let basic_details = skill_details(basic).join("\n");
        assert!(
            basic_details.contains("Formula: Unreviewed in Hercules renewal engine source."),
            "{basic_details}"
        );
    }

    #[test]
    fn guide_skill_details_display_direct_prerequisites_and_source_records() {
        let data = reference_data();

        // Fire Wall (18) requires Fire Ball (17) Lv 5 and Sight (10) Lv 1
        let firewall = data.skill_by_id(18).expect("Fire Wall skill");
        let fw_details = skill_details(firewall).join("\n");
        assert!(fw_details.contains("Skill tree prerequisites:"), "{fw_details}");
        assert!(
            fw_details.contains("@guide:skill:17|Requires: MG_FIREBALL Lv 5"),
            "{fw_details}"
        );
        assert!(fw_details.contains("@guide:skill:10|Requires: MG_SIGHT Lv 1"), "{fw_details}");
        assert!(
            fw_details.contains("Source record: db/re/skill_db.conf (Id=18)"),
            "{fw_details}"
        );

        // Test clickable link navigation to prerequisite
        let prereq_link = parse_guide_link("@guide:skill:17|Requires: MG_FIREBALL Lv 5").expect("skill link");
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
        assert!(status_lines.contains("SC_BLESSING (status ID 30)"));
        assert!(status_lines.contains("@guide:skill:34|Associated skill: Blessing (AL_BLESSING)"));
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
            assert!(detail.contains(&format!("@guide:skill:{skill_id}|Skill database StatusChange:")));

            let skill = data
                .search_skills(skill_name, 10)
                .into_iter()
                .find(|skill| skill.name == skill_name)
                .expect("skill row");
            assert!(
                skill_details(skill)
                    .iter()
                    .any(|line| line == &format!("@guide:status:{}|{} (icon {})", status.id, status.name, status.id))
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
        assert!(detail.iter().any(|line| line.starts_with("Verified outgoing portal connections:")));
        assert!(detail.iter().any(|line| line.starts_with("Exit at (")));
        assert!(
            detail
                .iter()
                .any(|line| line.contains("Scripted spawn clues") || line.contains("No loaded scripted-spawn call"))
        );
        assert!(detail.iter().any(|line| line == "@route:prt_fild08"));
        assert!(detail.iter().any(|line| line.starts_with("@route:")));
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

        // Izlude now has two verified outbound NPC services: the Byalan ferry
        // and the Malangdo cat fleet (added 2026-09-27).
        assert!(detail.iter().any(|line| line == "Verified NPC travel services: 2"));
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
        assert!(details.contains("Verified server metadata from renewal sc_config.conf"));
        assert!(details.contains("SC_BLESSING (status ID 30)"));
        assert!(details.contains(
            "Server lifecycle rules: classified by the server as a buff; cannot be applied to boss monsters; not cleared when MADO Gear \
             is removed; cannot be applied while the target is in a no-magic state."
        ));
        assert!(details.contains("Server recalculates these stat groups when this status changes: DEX, HIT, INT, STR."));
        assert!(details.contains("@guide:skill:34|Associated skill: Blessing (AL_BLESSING)"));
        assert!(details.contains("Literal Hercules C sc_start call sites (not exhaustive"));
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
        assert!(resolve_details(&linked_skill).iter().any(|line| line.contains("Skill identifier:")));
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
        assert!(detail.iter().any(|line| line.contains("bundled hunt reference")));
        assert!(detail.iter().any(|line| line.starts_with("@guide:monster:")) || detail.iter().any(|line| line.starts_with("@route:")));
        assert!(
            detail
                .iter()
                .any(|line| line.contains("No NPC giver or turn-in has been source-reviewed"))
        );

        let quest_link = parse_guide_link("@guide:quest:1100|Open quest").expect("quest links are supported");
        assert_eq!(quest_link.kind, "quest");
        assert!(!resolve_details(&quest_link).is_empty());
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
        assert!(detail.iter().any(|line| line.contains("Reviewed quest offer: Angelo#br")));
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
        assert!(detail.iter().any(|line| line.contains("Reviewed quest turn-in: Angelo#br")));
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
                .any(|line| line == &format!("@guide:status:{}|{} ({})", stone.id, stone.name, status_tag(stone))),
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
    fn all_search_finds_matching_monster_card_and_quest_together() {
        let rows = search_all_categories("poring", &DiscoveryState::default(), &[]);
        assert!(rows.iter().any(|row| row.kind == "monster" && row.id == 1002));
        assert!(rows.iter().any(|row| row.kind == "card" && row.id == 4001));
        assert!(rows.iter().any(|row| row.kind == "quest"));
    }
}

/// A job matches by name, or by an authored alias such as "lk" or "pally".
fn job_matches(query: &str, name: &str) -> bool {
    query.is_empty() || name.to_lowercase().contains(query) || reference_data().alias_targets("job", query).contains(&name)
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
