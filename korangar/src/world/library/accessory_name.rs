use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::Lua;

use super::{HashMapExt, Library, LuaExt, Table};
use crate::loaders::GameFileLoader;

/// Sprite resource name for an equipped headgear's *view id*.
///
/// The wire carries only a small view id per headgear slot (`accessory`,
/// `accessory2`, `accessory3`). Turning one into a file name is a two-table
/// lookup the original client also performs: `accessoryid.lub` defines the
/// `ACCESSORY_*` constants, and `accname.lub` maps those constants to the
/// Korean sprite base name under `data\sprite\악세사리\`.
pub struct AccessoryName(String);

impl AccessoryName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Female headgear sprites are a separate table where they differ; the male
/// table is the fallback, matching the original client's lookup order.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AccessoryNameKey {
    pub view_id: u16,
    pub female: bool,
}

impl Table for AccessoryName {
    type Key<'a> = AccessoryNameKey;
    type Storage = HashMap<AccessoryNameKey, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &["data\\luafiles514\\lua files\\datainfo\\accessoryid.lub"])?;
        let globals = state.globals();
        let mut result = HashMap::new();

        // `accname_f.lub` overrides only the entries whose female sprite has a
        // different base name, so the male table has to be read first.
        for (path, female) in [
            ("data\\luafiles514\\lua files\\datainfo\\accname.lub", false),
            ("data\\luafiles514\\lua files\\datainfo\\accname_f.lub", true),
        ] {
            let Ok(data) = game_file_loader.get(path) else {
                continue;
            };
            if state.load(&data).exec().is_err() {
                continue;
            }
            let Ok(names) = globals.get::<mlua::Table>("AccNameTable") else {
                continue;
            };

            for (key, value) in names.pairs::<u16, mlua::String>().flatten() {
                let bytes = value.as_bytes();
                // Sprite file names in these tables are EUC-KR, like the job
                // ones; a UTF-8 read succeeds only for the ASCII entries.
                let name = match std::str::from_utf8(&bytes) {
                    Ok(text) => text.to_owned(),
                    Err(_) => match decode_euc_kr(&bytes) {
                        Some(text) => text,
                        None => continue,
                    },
                };

                if name.is_empty() {
                    continue;
                }

                result.insert(AccessoryNameKey { view_id: key, female }, AccessoryName(name.clone()));
                if !female {
                    // Seed the female slot too so a missing `accname_f` entry
                    // falls back to the shared sprite instead of nothing.
                    result.insert(
                        AccessoryNameKey {
                            view_id: key,
                            female: true,
                        },
                        AccessoryName(name),
                    );
                }
            }
        }

        Ok(result.compact())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        library.accessory_name_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: AccessoryName = AccessoryName(String::new());
        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}

fn decode_euc_kr(bytes: &[u8]) -> Option<String> {
    encoding_rs::EUC_KR
        .decode_without_bom_handling_and_without_replacement(bytes)
        .map(|text| text.to_string())
}
