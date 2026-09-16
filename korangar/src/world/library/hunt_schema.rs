//! Authoritative hunt objective and guidance schemas (QW-050).
//! Quest facts live here, not in UI code.

#![allow(dead_code)]

use std::collections::HashMap;

pub const HUNT_SCHEMA: u32 = 1;
pub const GUIDANCE_SCHEMA: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectiveType {
    Collect,
    Kill,
    Talk,
    Explore,
    Interact,
    DmEncounter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonsterSource {
    pub monster_id: u32,
    pub rank: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HuntObjective {
    pub quest_id: u32,
    pub name: String,
    pub objective_type: ObjectiveType,
    pub sources: Vec<MonsterSource>,
    pub item_counts: Vec<(u32, u32)>,
    pub maps: Vec<String>,
    pub party_share: String,
    pub turn_in: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HuntGuidance {
    pub quest_id: u32,
    pub npc: String,
    pub area: String,
    pub steps: Vec<String>,
}

pub fn parse_objectives(source: &str) -> Result<HashMap<u32, HuntObjective>, String> {
    let mut lines = source.lines();
    let header = lines.next().ok_or("empty hunt objectives")?;
    let schema = header
        .strip_prefix("# schema=")
        .and_then(|n| n.trim().parse::<u32>().ok())
        .ok_or("hunt objectives missing schema")?;
    if schema != HUNT_SCHEMA {
        return Err(format!("incompatible hunt schema {schema}"));
    }
    let mut out = HashMap::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let quest_id: u32 = fields.next().ok_or("missing quest id")?.parse().map_err(|_| "bad quest id")?;
        let name = fields.next().unwrap_or("").to_owned();
        if name.is_empty() {
            return Err(format!("quest {quest_id} missing name"));
        }
        let objective_type = match fields.next().unwrap_or("") {
            "Collect" => ObjectiveType::Collect,
            "Kill" => ObjectiveType::Kill,
            "Talk" => ObjectiveType::Talk,
            "Explore" => ObjectiveType::Explore,
            "Interact" => ObjectiveType::Interact,
            "DM" => ObjectiveType::DmEncounter,
            other => return Err(format!("quest {quest_id} unknown objective type {other}")),
        };
        let sources = fields
            .next()
            .unwrap_or("")
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|part| {
                let mut bits = part.split(':');
                let monster_id = bits.next().ok_or("missing monster")?.parse().map_err(|_| "bad monster")?;
                let rank = bits.next().unwrap_or("normal").to_owned();
                let name = bits.next().unwrap_or("").to_owned();
                Ok(MonsterSource { monster_id, rank, name })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let item_counts = fields
            .next()
            .unwrap_or("")
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|pair| {
                let (item, count) = pair.split_once(':').ok_or("item:count required")?;
                Ok((item.parse().map_err(|_| "bad item")?, count.parse().map_err(|_| "bad count")?))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let maps = fields
            .next()
            .unwrap_or("")
            .split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        let party_share = fields.next().unwrap_or("inventory").to_owned();
        let turn_in = fields.next().unwrap_or("").to_owned();
        if turn_in.is_empty() {
            return Err(format!("quest {quest_id} missing turn-in"));
        }
        out.insert(quest_id, HuntObjective {
            quest_id,
            name,
            objective_type,
            sources,
            item_counts,
            maps,
            party_share,
            turn_in,
        });
    }
    Ok(out)
}

pub fn parse_guidance(source: &str) -> Result<HashMap<u32, HuntGuidance>, String> {
    let mut lines = source.lines();
    let header = lines.next().ok_or("empty guidance")?;
    let schema = header
        .strip_prefix("# schema=")
        .and_then(|n| n.trim().parse::<u32>().ok())
        .ok_or("guidance missing schema")?;
    if schema != GUIDANCE_SCHEMA {
        return Err(format!("incompatible guidance schema {schema}"));
    }
    let mut out = HashMap::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let quest_id: u32 = fields.next().ok_or("missing quest id")?.parse().map_err(|_| "bad quest id")?;
        let npc = fields.next().unwrap_or("").to_owned();
        let area = fields.next().unwrap_or("").to_owned();
        let steps = fields
            .next()
            .unwrap_or("")
            .split('|')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if npc.is_empty() || area.is_empty() || steps.is_empty() {
            return Err(format!("quest {quest_id} guidance incomplete"));
        }
        out.insert(quest_id, HuntGuidance {
            quest_id,
            npc,
            area,
            steps,
        });
    }
    Ok(out)
}

pub const BUNDLED_OBJECTIVES: &str = include_str!("hunt_objectives.tsv");
pub const BUNDLED_GUIDANCE: &str = include_str!("hunt_guidance.tsv");

pub fn bundled_objectives() -> &'static HashMap<u32, HuntObjective> {
    static OBJECTIVES: std::sync::OnceLock<HashMap<u32, HuntObjective>> = std::sync::OnceLock::new();
    OBJECTIVES.get_or_init(|| parse_objectives(BUNDLED_OBJECTIVES).expect("bundled hunt objectives are valid"))
}

pub fn bundled_guidance() -> &'static HashMap<u32, HuntGuidance> {
    static GUIDANCE: std::sync::OnceLock<HashMap<u32, HuntGuidance>> = std::sync::OnceLock::new();
    GUIDANCE.get_or_init(|| parse_guidance(BUNDLED_GUIDANCE).expect("bundled hunt guidance is valid"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoryStep {
    pub id: String,
    pub speaker: String,
    pub clue: String,
    pub remaining: String,
    pub next: String,
    pub revealed: bool,
}

pub fn parse_story_steps(source: &str) -> Result<Vec<StoryStep>, String> {
    let mut out = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut f = line.split('\t');
        let id = f.next().unwrap_or("").to_owned();
        let speaker = f.next().unwrap_or("").to_owned();
        let clue = f.next().unwrap_or("").to_owned();
        let remaining = f.next().unwrap_or("").to_owned();
        let next = f.next().unwrap_or("").to_owned();
        let revealed = f.next().unwrap_or("hidden") == "revealed";
        if id.is_empty() || speaker.is_empty() {
            return Err("incomplete story step".into());
        }
        out.push(StoryStep {
            id,
            speaker,
            clue,
            remaining,
            next,
            revealed,
        });
    }
    Ok(out)
}

pub fn visible_story_steps(steps: &[StoryStep]) -> Vec<&StoryStep> {
    steps.iter().filter(|s| s.revealed).collect()
}

pub const OMENS_STEPS: &str = "\
fountain	Fountain keeper	The water is still	Ask who last drank	south gate	revealed
south	Guard	A traveler went west	Follow the west road	west field	hidden
west	Scout	Tracks lead to Prontera field	Return with the rumor	wynne	hidden
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rockers_and_rumors_is_data_not_ui() {
        let objectives = parse_objectives(BUNDLED_OBJECTIVES).unwrap();
        let guidance = parse_guidance(BUNDLED_GUIDANCE).unwrap();
        let hunt = objectives.get(&20003).expect("Rockers and Rumors");
        assert_eq!(hunt.objective_type, ObjectiveType::Collect);
        assert_eq!(hunt.maps, vec!["prt_fild07".to_owned()]);
        assert!(hunt.turn_in.contains("Wynne"), "{}", hunt.turn_in);
        assert!(hunt.sources.iter().any(|s| s.rank == "vocal"));
        assert_eq!(hunt.item_counts, vec![(940, 10), (919, 10), (752, 3)]);
        let guide = guidance.get(&20003).unwrap();
        assert!(guide.npc.contains("Wynne"), "{}", guide.npc);
        assert!(!guide.steps.is_empty());
    }

    #[test]
    fn malformed_references_fail() {
        assert!(parse_objectives("# schema=9\n20003	x	Collect	1:n	1:1	map	inventory	Npc").is_err());
        assert!(parse_objectives("# schema=1\n20003		Collect	1:n	1:1	map	inventory	Npc").is_err());
        assert!(parse_guidance("# schema=1\n20003			").is_err());
        assert!(parse_guidance("# schema=1\nnot-a-number	Wynne	field	step").is_err());
    }

    #[test]
    fn omens_hides_unrevealed_steps() {
        let steps = parse_story_steps(OMENS_STEPS).unwrap();
        let visible = visible_story_steps(&steps);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].speaker, "Fountain keeper");
        assert!(visible.iter().all(|s| s.revealed));
        assert!(!visible.iter().any(|s| s.id == "south" || s.id == "west"));
        assert!(visible[0].remaining.contains("Ask who last drank"));
        assert_eq!(visible[0].next, "south gate");
    }

    #[test]
    fn every_objective_type_fixture() {
        let pack = "# schema=1\n1	Talk to Wynne	Talk	1052:normal	940:1	prontera	personal	Wynne\n2	Kill Porings	Kill	1002:normal	909:1	\
                    prt_fild08	party	Wynne\n3	Collect Legs	Collect	1052:normal	940:10	prt_fild07	inventory	Wynne\n4	Explore Field	Explore	\
                    1052:normal	940:1	prt_fild07	personal	Wynne\n5	Interact Chest	Interact	1052:normal	940:1	prt_fild07	personal	Wynne\n6	\
                    DM encounter	DM	1052:vocal	940:1	prt_fild07	party	Wynne\n";
        let parsed = parse_objectives(pack).unwrap();
        assert_eq!(parsed[&1].objective_type, ObjectiveType::Talk);
        assert_eq!(parsed[&2].objective_type, ObjectiveType::Kill);
        assert_eq!(parsed[&3].objective_type, ObjectiveType::Collect);
        assert_eq!(parsed[&4].objective_type, ObjectiveType::Explore);
        assert_eq!(parsed[&5].objective_type, ObjectiveType::Interact);
        assert_eq!(parsed[&6].objective_type, ObjectiveType::DmEncounter);
        assert_eq!(parsed[&2].party_share, "party");
        assert_eq!(parsed[&3].party_share, "inventory");
    }
}
