//! Journal text for campaign hunts. Facts come from hunt_schema, counts from
//! inventory.

use super::hunt_schema::{HuntGuidance, HuntObjective};

pub fn you_carry_line(item_name: &str, carried: u32, needed: u32, source: &str) -> String {
    format!("Collect {item_name}       {carried} / {needed}  — {source}")
}

pub fn format_hunt_journal(objective: &HuntObjective, guidance: &HuntGuidance, carried: &[(u32, u32)]) -> String {
    let mut lines = vec![
        objective.name.clone(),
        format!(
            "Recommended area: {} ({})",
            guidance.area,
            objective.maps.first().cloned().unwrap_or_default()
        ),
        String::new(),
    ];
    for (idx, (item_id, needed)) in objective.item_counts.iter().enumerate() {
        let have = carried.iter().find(|(id, _)| id == item_id).map(|(_, n)| *n).unwrap_or(0);
        let source = objective
            .sources
            .get(idx)
            .map(|s| {
                if s.rank == "vocal" || s.rank == "boss" {
                    format!("{} (boss-type, rare spawn)", display_monster(s.monster_id, &s.rank))
                } else {
                    display_monster(s.monster_id, &s.rank)
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

fn item_display_name(id: u32) -> String {
    match id {
        940 => "Grasshopper's Leg".to_owned(),
        919 => "Animal Skin".to_owned(),
        752 => "Rocker Doll".to_owned(),
        _ => format!("Item {id}"),
    }
}

fn display_monster(id: u32, rank: &str) -> String {
    match (id, rank) {
        (1052, _) => "Rocker".to_owned(),
        (1167, _) => "Savage Babe".to_owned(),
        (1088, _) => "Vocal".to_owned(),
        _ => format!("Monster {id}"),
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
}
