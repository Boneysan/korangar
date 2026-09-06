mod accessory_name;
mod baby_job;
mod campaign_quest;
mod item_info;
mod item_name;
mod item_resource;
mod item_stats;
mod job_identity;
mod job_name;
mod map_sky_data;
mod msgstringtable;
mod skill_info;
mod skill_information;
mod skill_requirements;
mod skill_tree;
mod towninfo;

use std::hash::Hash;

use encoding_rs::EUC_KR;
use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::{Lua, LuaOptions, StdLib};

pub use self::accessory_name::{AccessoryName, AccessoryNameKey};
pub use self::baby_job::IsBabyJob;
pub use self::campaign_quest::{CampaignQuest, CampaignQuestTable, quest_display_name};
pub use self::item_info::ItemInfo;
pub use self::item_name::{ItemName, ItemNameKey};
pub use self::item_resource::{ItemResource, ItemResourceKey};
pub use self::item_stats::{item_stats, item_tooltip_text};
pub use self::job_identity::JobIdentity;
pub use self::job_name::JobName;
pub use self::map_sky_data::MapSkyData;
pub use self::msgstringtable::MsgStringTable;
pub use self::skill_info::{skill_layout_value, skill_tooltip_text};
pub(crate) use self::skill_information::skill_asset_file_names;
pub use self::skill_tree::SkillTreeLayout;
pub use self::towninfo::{TownInfoTable, TownPoi, TownPoiKind};
use crate::loaders::GameFileLoader;
pub use crate::world::library::skill_information::SkillListInformation;
pub use crate::world::library::skill_requirements::{SkillListKey, SkillListRequirements};

pub struct Library {
    accessory_name_table: <AccessoryName as Table>::Storage,
    job_identity_table: <JobIdentity as Table>::Storage,
    job_name_table: <JobName as Table>::Storage,
    item_info_table: <ItemInfo as Table>::Storage,
    map_sky_data_table: <MapSkyData as Table>::Storage,
    skill_information_table: <SkillListInformation as Table>::Storage,
    skill_requirements_table: <SkillListRequirements as Table>::Storage,
    skill_tree_table: <SkillTreeLayout as Table>::Storage,
    baby_job_table: <IsBabyJob as Table>::Storage,
    towninfo_table: TownInfoTable,
    campaign_quest_table: CampaignQuestTable,
    msgstringtable: MsgStringTable,
}

impl Library {
    pub fn new(game_file_loader: &GameFileLoader) -> mlua::Result<Self> {
        let accessory_name_table = AccessoryName::load(game_file_loader)?;
        let job_identity_table = JobIdentity::load(game_file_loader)?;
        let job_name_table = JobName::load(game_file_loader)?;
        let item_info_table = ItemInfo::load(game_file_loader)?;
        let map_sky_data_table = MapSkyData::load(game_file_loader)?;
        let skill_information_table = SkillListInformation::load(game_file_loader)?;
        let skill_requirements_table = SkillListRequirements::load(game_file_loader)?;
        let skill_tree_table = SkillTreeLayout::load(game_file_loader)?;
        let baby_job_table = IsBabyJob::load(game_file_loader)?;
        let towninfo_table = TownInfoTable::load(game_file_loader);
        let campaign_quest_table = CampaignQuestTable::load();
        let msgstringtable = MsgStringTable::load(game_file_loader);

        Ok(Self {
            accessory_name_table,
            job_identity_table,
            job_name_table,
            item_info_table,
            map_sky_data_table,
            skill_information_table,
            skill_requirements_table,
            skill_tree_table,
            baby_job_table,
            towninfo_table,
            campaign_quest_table,
            msgstringtable,
        })
    }

    #[inline(always)]
    pub fn get<T: Table>(&self, key: T::Key<'_>) -> &T {
        T::get(self, key)
    }

    /// Non-panicking lookup for keys that may legitimately be absent (e.g. a
    /// job id with no skill-tree entry — crashed the client on warp-portal
    /// map change as a @jobchange'd Priest, 2026-07-23).
    #[inline(always)]
    pub fn try_get<T: Table>(&self, key: T::Key<'_>) -> Option<&T> {
        T::try_get(self, key)
    }

    /// Facility POIs (shops, kafra, guides, …) for a map base name.
    #[inline]
    pub fn town_pois(&self, map_name: &str) -> &[TownPoi] {
        self.towninfo_table.pois_for_map(map_name)
    }

    /// Whether the map is a town / safe map (has Towninfo facilities). Used to
    /// relax the battle-ready stance to peaceful idle where there are no
    /// monsters.
    pub fn is_town_map(&self, map_name: &str) -> bool {
        self.towninfo_table.is_town(map_name)
    }

    /// The Seal Cascade hunting contract for a quest id, if it is one.
    #[inline]
    pub fn campaign_quest(&self, quest_id: u32) -> Option<&CampaignQuest> {
        self.campaign_quest_table.get(quest_id)
    }

    /// Resolve a `ZC_MSG` / `ZC_MSG_COLOR` id via msgstringtable.
    #[inline]
    pub fn message_string(&self, message_id: u16) -> String {
        self.msgstringtable.resolve(message_id)
    }

    pub(crate) fn job_identity_entries(&self) -> Vec<(ragnarok_packets::JobId, String)> {
        self.job_identity_table
            .iter()
            .map(|(job_id, identity)| (*job_id, identity.to_string()))
            .collect()
    }

    pub(crate) fn skill_asset_entries(&self) -> Vec<(ragnarok_packets::SkillId, &str, &str)> {
        let visible_skill_ids = self
            .skill_tree_table
            .values()
            .flat_map(|layout| &layout.tabs)
            .flat_map(|tab| tab.skills.values())
            .copied()
            .collect::<hashbrown::HashSet<_>>();

        self.skill_information_table
            .iter()
            .filter(|(skill_id, _)| visible_skill_ids.contains(*skill_id))
            .map(|(skill_id, information)| (*skill_id, information.name.as_str(), information.file_name.as_str()))
            .collect()
    }
}

/// Trait for compacting a hash map after it is completely populated.
trait HashMapExt {
    /// Compact the hash map, possibly by creating a second one.
    fn compact(self) -> Self;
}

impl<K, V> HashMapExt for HashMap<K, V>
where
    K: Eq + Hash,
{
    fn compact(self) -> Self {
        HashMap::from_iter(self)
    }
}

trait LuaExt: Sized {
    fn load_from_game_files(game_file_loader: &GameFileLoader, files: &[&str]) -> mlua::Result<Self>;
}

pub(crate) fn new_sandboxed_lua() -> mlua::Result<Lua> {
    // Official RO lua is tables, strings, and math. io/os/package would make a
    // swapped lua_files.7z into host code execution.
    Lua::new_with(StdLib::TABLE | StdLib::STRING | StdLib::MATH, LuaOptions::default())
}

impl LuaExt for Lua {
    fn load_from_game_files(game_file_loader: &GameFileLoader, files: &[&str]) -> mlua::Result<Self> {
        let state = new_sandboxed_lua()?;

        for file in files {
            let data = game_file_loader
                .get(file)
                .unwrap_or_else(|_| panic!("failed to open lua file {}", file));

            state.load(&data).exec()?;
        }

        Ok(state)
    }
}

/// Trait for data that can be stored in a table and retrieved using a key.
pub trait Table {
    type Key<'a>;
    type Storage;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage>;

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self>
    where
        Self: Sized;

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self
    where
        Self: Sized;
}

fn fix_encoding(broken: String) -> String {
    let bytes: Vec<u8> = broken.chars().map(|char| char as u8).collect();
    match EUC_KR.decode_without_bom_handling_and_without_replacement(&bytes) {
        None => broken.to_string(),
        Some(char) => char.to_string(),
    }
}

fn needs_ascii_fallback(value: &str) -> bool {
    !value.is_ascii()
}
