//! Equipment eligibility from exported Hercules job/sex/level/location fields.
//!
//! Schema version is required. Stale packs without `schema=` fail to parse.

use std::collections::HashMap;
use std::sync::OnceLock;

pub const ELIGIBILITY_SCHEMA: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sex {
    Male,
    Female,
    Any,
}

impl From<ragnarok_packets::Sex> for Sex {
    fn from(sex: ragnarok_packets::Sex) -> Self {
        match sex {
            ragnarok_packets::Sex::Male => Sex::Male,
            ragnarok_packets::Sex::Female => Sex::Female,
            _ => Sex::Any,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EquipDenial {
    Level,
    Job,
    Sex,
    Location,
}

impl EquipDenial {
    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::Level => "Cannot equip: level",
            Self::Job => "Cannot equip: job",
            Self::Sex => "Cannot equip: sex",
            Self::Location => "Cannot equip: equipment location",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EligibilityRow {
    pub item_id: u32,
    pub jobs: Vec<u16>,
    pub sex: Sex,
    pub min_level: u16,
    pub loc: String,
    pub max_level: u16,
    pub upper_mask: u32,
    pub weapon_level: u8,
    pub slots: u8,
}

#[derive(Clone, Debug)]
pub struct EligibilityTable {
    rows: HashMap<u32, EligibilityRow>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wearer {
    pub job_id: u16,
    pub base_level: u16,
    pub sex: Sex,
    pub slot: &'static str,
}

impl Wearer {
    pub fn from_player(player: &crate::world::Player, slot: &'static str) -> Self {
        let common = player.get_common();
        Self {
            job_id: common.job_id.0,
            base_level: player.base_level as u16,
            sex: common.sex.into(),
            slot,
        }
    }
}

static GLOBAL_TABLE: OnceLock<EligibilityTable> = OnceLock::new();

impl EligibilityTable {
    pub fn get() -> &'static Self {
        GLOBAL_TABLE.get_or_init(bundled_table)
    }

    pub fn parse(source: &str) -> Result<Self, String> {
        let mut lines = source.lines();
        let header = lines.next().ok_or("empty eligibility pack")?;
        let schema: u32 = header
            .strip_prefix("# schema=")
            .and_then(|n| n.trim().parse().ok())
            .ok_or_else(|| "eligibility pack missing schema version".to_owned())?;
        if schema != ELIGIBILITY_SCHEMA {
            return Err(format!(
                "incompatible eligibility schema {schema}, expected {ELIGIBILITY_SCHEMA}"
            ));
        }
        let mut rows = HashMap::new();
        for line in lines {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split('\t');
            let item_id: u32 = parts.next().ok_or("missing id")?.parse().map_err(|_| "bad id")?;
            let jobs = parts
                .next()
                .unwrap_or("")
                .split('|')
                .filter(|s| !s.is_empty())
                .map(|s| s.parse::<u16>().map_err(|_| "bad job"))
                .collect::<Result<Vec<_>, _>>()?;
            let sex = match parts.next().unwrap_or("any") {
                "male" => Sex::Male,
                "female" => Sex::Female,
                _ => Sex::Any,
            };
            let min_level: u16 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let loc = parts.next().unwrap_or("").to_owned();
            let max_level: u16 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let upper_mask: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let weapon_level: u8 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            let slots: u8 = parts.next().unwrap_or("0").parse().unwrap_or(0);
            rows.insert(item_id, EligibilityRow {
                item_id,
                jobs,
                sex,
                min_level,
                loc,
                max_level,
                upper_mask,
                weapon_level,
                slots,
            });
        }
        Ok(Self { rows })
    }

    pub fn denial(&self, item_id: u32, wearer: Wearer) -> Option<EquipDenial> {
        let row = self.rows.get(&item_id)?;
        if !row.loc.is_empty() && wearer.slot != "*" {
            let matches = row.loc.split('|').any(|slot| {
                slot == wearer.slot
                    || (slot == "EQP_WEAPON" && wearer.slot == "EQP_HAND_R")
                    || (slot == "EQP_SHIELD" && wearer.slot == "EQP_HAND_L")
                    || (slot == "EQP_ARMS" && (wearer.slot == "EQP_HAND_R" || wearer.slot == "EQP_HAND_L"))
                    || (slot == "EQP_ACC" && (wearer.slot == "EQP_ACC_L" || wearer.slot == "EQP_ACC_R"))
                    || (slot == "EQP_HELM"
                        && (wearer.slot == "EQP_HEAD_TOP" || wearer.slot == "EQP_HEAD_MID" || wearer.slot == "EQP_HEAD_LOW"))
            });
            if !matches {
                return Some(EquipDenial::Location);
            }
        }
        if row.sex != Sex::Any && row.sex != wearer.sex {
            return Some(EquipDenial::Sex);
        }
        if wearer.base_level < row.min_level || (row.max_level > 0 && wearer.base_level > row.max_level) {
            return Some(EquipDenial::Level);
        }
        if !row.jobs.is_empty() && !row.jobs.contains(&wearer.job_id) {
            return Some(EquipDenial::Job);
        }
        None
    }
}

pub const BUNDLED_ELIGIBILITY: &str = include_str!("equipment_eligibility.tsv");

pub fn bundled_table() -> EligibilityTable {
    EligibilityTable::parse(BUNDLED_ELIGIBILITY).expect("bundled eligibility is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_stale_schema() {
        let err = EligibilityTable::parse("# schema=0\n1101\t1\tany\t1\tEQP_WEAPON").unwrap_err();
        assert!(err.contains("incompatible"));
    }

    #[test]
    fn sword_allowed_for_knight_denied_for_mage() {
        let table = bundled_table();
        let knight = Wearer {
            job_id: 7,
            base_level: 10,
            sex: Sex::Male,
            slot: "EQP_WEAPON",
        };
        let mage = Wearer {
            job_id: 2,
            base_level: 10,
            sex: Sex::Female,
            slot: "EQP_WEAPON",
        };
        assert_eq!(table.denial(1101, knight), None);
        assert_eq!(table.denial(1101, mage), Some(EquipDenial::Job));
        assert_eq!(table.denial(1101, mage).unwrap().tooltip(), "Cannot equip: job");
    }

    #[test]
    fn level_and_sex_and_location() {
        let table = bundled_table();
        let novice = Wearer {
            job_id: 1,
            base_level: 1,
            sex: Sex::Male,
            slot: "EQP_WEAPON",
        };
        assert_eq!(table.denial(1101, novice), Some(EquipDenial::Level));
        let female = Wearer {
            job_id: 19,
            base_level: 10,
            sex: Sex::Female,
            slot: "EQP_WEAPON",
        };
        assert_eq!(table.denial(1901, female), Some(EquipDenial::Sex));
        let wrong_slot = Wearer {
            job_id: 7,
            base_level: 10,
            sex: Sex::Male,
            slot: "EQP_ARMOR",
        };
        assert_eq!(table.denial(1101, wrong_slot), Some(EquipDenial::Location));
    }

    #[test]
    fn armor_and_accessory_and_unusable_potion() {
        let table = bundled_table();
        let wearer = Wearer {
            job_id: 7,
            base_level: 10,
            sex: Sex::Male,
            slot: "EQP_ARMOR",
        };
        assert_eq!(table.denial(2301, wearer), None);
        let acc = Wearer {
            job_id: 0,
            base_level: 1,
            sex: Sex::Female,
            slot: "EQP_ACC",
        };
        assert_eq!(table.denial(2607, acc), None);
        assert_eq!(table.denial(501, wearer), None);
    }
}
