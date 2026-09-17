//! Journal text for campaign hunts. Facts come from hunt_schema, counts from
//! inventory.

use super::hunt_schema::{HuntGuidance, HuntObjective, ObjectiveType};

pub fn you_carry_line(item_name: &str, carried: u32, needed: u32, source: &str) -> String {
    format!("Collect {item_name}       {carried} / {needed}  — {source}")
}

#[allow(dead_code)]
pub fn format_hunt_journal(objective: &HuntObjective, guidance: &HuntGuidance, carried: &[(u32, u32)]) -> String {
    let (icon, label, status) = objective_presentation(objective.objective_type, &objective.completion);
    let mut lines = vec![
        format!("{icon} {label}: {}", objective.name),
        format!(
            "{} objective · completion: {}{}",
            if objective.required { "Required" } else { "Optional" },
            objective.completion,
            if objective.dm_triggered { " · DM-triggered" } else { "" },
        ),
        status,
        format!(
            "Recommended area: {} ({})",
            guidance.area,
            objective.maps.first().cloned().unwrap_or_default()
        ),
        String::new(),
    ];
    for (idx, (item_id, needed)) in objective
        .item_counts
        .iter()
        .enumerate()
        .filter(|_| objective.objective_type == ObjectiveType::Collect)
    {
        let have = carried.iter().find(|(id, _)| id == item_id).map(|(_, n)| *n).unwrap_or(0);
        let source = objective
            .sources
            .get(idx)
            .map(|s| {
                let mname = if !s.name.is_empty() {
                    s.name.as_str()
                } else {
                    display_monster(s.monster_id, &s.rank)
                };
                if s.rank == "vocal" || s.rank == "boss" {
                    format!("{mname} (boss-type, rare spawn)")
                } else {
                    mname.to_owned()
                }
            })
            .unwrap_or_else(|| "unknown".to_owned());
        let name = item_display_name(*item_id);
        lines.push(you_carry_line(&name, have, *needed, &source));
    }
    lines.push(String::new());
    lines.push(format!("Turn in: {} — {}", guidance.npc, guidance.area));
    lines.push("You carry: counts shown above".to_owned());
    lines.push(format!("Party quest state: {}", objective.party_share));
    lines.join("\n")
}

fn objective_presentation(objective_type: ObjectiveType, completion: &str) -> (&'static str, &'static str, String) {
    let (icon, label, authority) = match objective_type {
        ObjectiveType::Talk => ("[TALK]", "Talk", "awaiting authoritative server confirmation"),
        ObjectiveType::Kill => ("[KILL]", "Defeat", "authoritative quest packets; defeat progress below"),
        ObjectiveType::Collect => ("[GET]", "Collect", "inventory-backed; carried counts below"),
        ObjectiveType::Explore => ("[EXPLORE]", "Explore", "awaiting authoritative server confirmation"),
        ObjectiveType::Interact => ("[USE]", "Interact", "awaiting authoritative server confirmation"),
        ObjectiveType::DmEncounter => ("[DM]", "Encounter", "awaiting server/DM encounter confirmation"),
    };
    (icon, label, format!("Status: {authority} · completion: {completion}"))
}

#[allow(dead_code)]
pub fn item_display_name(id: u32) -> String {
    match id {
        940 => "Grasshopper's Leg".to_owned(),
        919 => "Animal Skin".to_owned(),
        752 => "Rocker Doll".to_owned(),
        _ => format!("Item {id}"),
    }
}

pub fn display_monster(id: u32, rank: &str) -> &'static str {
    match (id, rank) {
        (1052, _) => "Rocker",
        (1167, _) => "Savage Babe",
        (1088, _) => "Vocal",
        _ => "Monster",
    }
}

#[cfg(test)]
mod tests {
    use super::super::hunt_schema::{BUNDLED_GUIDANCE, BUNDLED_OBJECTIVES, parse_guidance, parse_objectives};
    use super::*;

    #[test]
    fn rockers_acceptance_example() {
        let objectives = parse_objectives(BUNDLED_OBJECTIVES).unwrap();
        let guidance = parse_guidance(BUNDLED_GUIDANCE).unwrap();
        let hunt = objectives.get(&20003).unwrap();
        let guide = guidance.get(&20003).unwrap();
        let text = format_hunt_journal(hunt, guide, &[(940, 7), (919, 4), (752, 1)]);
        assert!(text.contains("Field Contract: Rockers and Rumors"));
        assert!(text.contains("prt_fild07"));
        assert!(text.contains("You carry"));
        assert!(text.contains("7 / 10"));
        assert!(text.contains("Rocker"));
        assert!(text.contains("Savage Babe"));
        assert!(text.contains("Vocal"));
        assert!(text.contains("Wynne"));
        assert!(text.contains("Party quest state"));
        assert!(!text.contains("shared counter"));
    }

    #[test]
    fn pickup_and_drop_change_you_carry() {
        let objectives = parse_objectives(BUNDLED_OBJECTIVES).unwrap();
        let guidance = parse_guidance(BUNDLED_GUIDANCE).unwrap();
        let hunt = objectives.get(&20003).unwrap();
        let guide = guidance.get(&20003).unwrap();
        let after_pickup = format_hunt_journal(hunt, guide, &[(940, 8), (919, 4), (752, 1)]);
        let after_drop = format_hunt_journal(hunt, guide, &[(940, 7), (919, 4), (752, 1)]);
        assert!(after_pickup.contains("8 / 10"));
        assert!(after_drop.contains("7 / 10"));
        assert_ne!(after_pickup, after_drop);
    }

    #[test]
    fn every_typed_objective_renders_a_badge_and_authority() {
        let objective_pack = "# schema=1
1\tTalk to Wynne\tTalk\t1052:normal\t940:1\tprontera\tpersonal\tWynne
2\tKill Porings\tKill\t1002:normal\t909:1\tprt_fild08\tparty\tWynne
3\tCollect Legs\tCollect\t1052:normal\t940:10\tprt_fild07\tinventory\tWynne
4\tExplore Field\tExplore\t1052:normal\t940:1\tprt_fild07\tpersonal\tWynne
5\tInteract Chest\tInteract\t1052:normal\t940:1\tprt_fild07\tpersonal\tWynne
6\tDM encounter\tDM\t1052:vocal\t940:1\tprt_fild07\tparty\tWynne";
        let objectives = parse_objectives(objective_pack).unwrap();
        let guidance = HuntGuidance {
            quest_id: 1,
            npc: "Wynne".into(),
            area: "Prontera".into(),
            steps: vec!["Follow the objective".into()],
        };
        let expected = ["[TALK]", "[KILL]", "[GET]", "[EXPLORE]", "[USE]", "[DM]"];

        for (id, badge) in expected.into_iter().enumerate() {
            let objective = objectives.get(&(id as u32 + 1)).unwrap();
            let text = format_hunt_journal(objective, &guidance, &[]);
            assert!(text.contains(badge), "{text}");
            assert!(text.contains("Status:"), "{text}");
        }
    }
}
