use std::sync::LazyLock;

use hashbrown::HashMap;
use mlua::Lua;
use ragnarok_packets::{SkillId, SkillLevel};

use super::{HashMapExt, Library, Table, fix_encoding, needs_ascii_fallback};
use crate::loaders::GameFileLoader;
use crate::state::skills::SkillAcquisition;
use crate::world::library::LuaExt;

static NOT_FOUND_ENTRY: LazyLock<SkillListInformation> = LazyLock::new(|| SkillListInformation {
    file_name: "notfound".to_owned(),
    name: "notfound".to_owned(),
    maximum_level: SkillLevel(100),
    can_select_level: false,
    // To make it unskillable.
    acquisition: SkillAcquisition::Quest,
});

pub struct SkillListInformation {
    pub file_name: String,
    pub name: String,
    pub maximum_level: SkillLevel,
    pub can_select_level: bool,
    pub acquisition: SkillAcquisition,
}

impl Table for SkillListInformation {
    type Key<'a> = SkillId;
    type Storage = HashMap<SkillId, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &[
            // Needed to get the `JOBID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub",
            // Needed to get the `SKID` table.
            "data\\luafiles514\\lua files\\skillinfoz\\skillid.lub",
            "data\\luafiles514\\lua files\\skillinfoz\\skillinfolist.lub",
        ])?;

        let globals = state.globals();
        let skill_info_list = globals.get::<mlua::Table>("SKILL_INFO_LIST")?;
        let skill_ids = globals.get::<mlua::Table>("SKID")?;
        let skill_names_by_id = skill_ids
            .pairs::<String, u16>()
            .flatten()
            .fold(HashMap::new(), |mut names, (skill_name, skill_id)| {
                names.entry(SkillId(skill_id)).or_insert(skill_name);
                names
            });

        let mut result = HashMap::new();

        for (skill_id, table) in skill_info_list.pairs::<u16, mlua::Table>().flatten() {
            let file_name = table.get(1)?;
            let name = table.get("SkillName").map(fix_encoding)?;
            let name = match needs_ascii_fallback(&name) {
                true => skill_names_by_id
                    .get(&SkillId(skill_id))
                    .and_then(|name| skill_name_from_identifier(name))
                    .unwrap_or_else(|| format!("Skill {skill_id}")),
                false => name,
            };
            let maximum_level = table.get("MaxLv")?;
            let can_select_level = table.get("bSeperateLv")?;
            let acquisition = match table.get::<String>("Type").ok().as_deref() {
                Some("Quest") => SkillAcquisition::Quest,
                Some("Soul") => SkillAcquisition::SoulLink,
                None => SkillAcquisition::Job,
                Some(unknown) => panic!("unknown skill type {}", unknown),
            };

            result.insert(SkillId(skill_id), SkillListInformation {
                file_name,
                name,
                maximum_level: SkillLevel(maximum_level),
                can_select_level,
                acquisition,
            });
        }

        Ok(result.compact())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self>
    where
        Self: Sized,
    {
        library.skill_information_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self
    where
        Self: Sized,
    {
        Self::try_get(library, key).unwrap_or(&*NOT_FOUND_ENTRY)
    }
}

fn skill_name_from_identifier(identifier: &str) -> Option<String> {
    let mut words = identifier.split('_').filter(|part| !part.is_empty()).collect::<Vec<_>>();

    if words.first().is_some_and(|part| part.len() <= 4) {
        words.remove(0);
    }

    let name = match words.as_slice() {
        ["BASIC"] => "Basic Skill".to_owned(),
        ["DECAGI"] => "Decrease AGI".to_owned(),
        ["INCAGI"] => "Increase AGI".to_owned(),
        ["FIRSTAID"] => "First Aid".to_owned(),
        ["PLAYDEAD"] => "Play Dead".to_owned(),
        ["FIREBOLT"] => "Fire Bolt".to_owned(),
        ["COLDBOLT"] => "Cold Bolt".to_owned(),
        ["LIGHTNINGBOLT"] => "Lightning Bolt".to_owned(),
        ["FIREBALL"] => "Fire Ball".to_owned(),
        ["FIREWALL"] => "Fire Wall".to_owned(),
        ["FROSTDIVER"] => "Frost Diver".to_owned(),
        ["THUNDERSTORM"] => "Thunderstorm".to_owned(),
        ["ORIDEOCON"] => "Oridecon Research".to_owned(),
        ["HILTBINDING"] => "Hilt Binding".to_owned(),
        ["SKINTEMPER"] => "Skin Tempering".to_owned(),
        ["MELTDOWN"] => "Shattering Strike".to_owned(),
        ["WEAPONREFINE"] => "Upgrade Weapon".to_owned(),
        ["OVERTHRUSTMAX"] => "Maximum Power Thrust".to_owned(),
        _ => words.into_iter().map(format_identifier_word).collect::<Vec<_>>().join(" "),
    };

    (!name.is_empty()).then_some(name)
}

fn format_identifier_word(word: &str) -> String {
    match word {
        "AGI" | "ASPD" | "ATK" | "CRIT" | "DEF" | "DEX" | "FLEE" | "HP" | "INT" | "LUK" | "MATK" | "MDEF" | "NPC" | "SP" | "STR" => {
            word.to_owned()
        }
        _ => {
            let mut characters = word.chars();
            let Some(first) = characters.next() else {
                return String::new();
            };

            let first = first.to_uppercase().to_string();
            let rest = characters.as_str().to_lowercase();

            format!("{first}{rest}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::skill_name_from_identifier;

    #[test]
    fn skill_name_from_identifier_removes_job_prefix() {
        assert_eq!(skill_name_from_identifier("NV_BASIC").as_deref(), Some("Basic Skill"));
        assert_eq!(skill_name_from_identifier("AL_HEAL").as_deref(), Some("Heal"));
    }

    #[test]
    fn skill_name_from_identifier_expands_common_compact_names() {
        assert_eq!(skill_name_from_identifier("AL_INCAGI").as_deref(), Some("Increase AGI"));
        assert_eq!(skill_name_from_identifier("NV_FIRSTAID").as_deref(), Some("First Aid"));
        assert_eq!(skill_name_from_identifier("SU_BASIC_SKILL").as_deref(), Some("Basic Skill"));
        assert_eq!(skill_name_from_identifier("BS_HILTBINDING").as_deref(), Some("Hilt Binding"));
        assert_eq!(skill_name_from_identifier("WS_WEAPONREFINE").as_deref(), Some("Upgrade Weapon"));
        assert_eq!(skill_name_from_identifier("WS_MELTDOWN").as_deref(), Some("Shattering Strike"));
        assert_eq!(skill_name_from_identifier("WS_OVERTHRUSTMAX").as_deref(), Some("Maximum Power Thrust"));
    }
}
