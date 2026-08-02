use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use hashbrown::HashMap;
use mlua::Lua;
use ragnarok_packets::JobId;

use super::{HashMapExt, Library, LuaExt, Table};
use crate::loaders::GameFileLoader;

/// Human-readable class name for a job id, e.g. `JobId(1)` → *"Swordman"*.
///
/// Deliberately **not** derived from [`JobIdentity`](super::JobIdentity), which
/// resolves *sprite resource* names: `jobname.lub` overrides those with the
/// paths the original client loads, and for player classes many are Korean
/// (`초보자`). Those are correct for finding a sprite and useless as a label.
///
/// The `JT_` constants in `jobidentity.lub` are English and stable, so this
/// table reads them directly and prettifies: `JT_HIGH_SWORDMAN` → *"High
/// Swordman"*. Spelling follows Gravity's constants, so it is "Swordman" rather
/// than "Swordsman".
pub struct JobName(Cow<'static, str>);

impl Display for JobName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `HIGH_SWORDMAN` → `High Swordman`.
fn prettify(constant: &str) -> String {
    constant
        .split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().chain(characters.flat_map(char::to_lowercase)).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

impl Table for JobName {
    type Key<'a> = JobId;
    type Storage = HashMap<JobId, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &["data\\luafiles514\\lua files\\datainfo\\jobidentity.lub"])?;

        let globals = state.globals();
        let mut result = HashMap::new();

        if let Ok(jobtbl) = globals.get::<mlua::Table>("jobtbl") {
            for (key, value) in jobtbl.pairs::<String, u16>().flatten() {
                // Every entry is `JT_<NAME>`; guild variants (`JT_G_`) are
                // monsters and never a player's class.
                let Some(constant) = key.strip_prefix("JT_") else {
                    continue;
                };
                if constant.starts_with("G_") {
                    continue;
                }

                result.insert(JobId(value), JobName(prettify(constant).into()));
            }
        }

        Ok(result.compact())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        library.job_name_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: JobName = JobName(Cow::Borrowed("Adventurer"));
        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}

#[cfg(test)]
mod tests {
    use super::prettify;

    #[test]
    fn prettifies_job_constants() {
        assert_eq!(prettify("SWORDMAN"), "Swordman");
        assert_eq!(prettify("HIGH_SWORDMAN"), "High Swordman");
        assert_eq!(prettify("SOUL_LINKER"), "Soul Linker");
        // Trailing or doubled separators must not produce empty words.
        assert_eq!(prettify("NOVICE_"), "Novice");
    }
}
