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
/// than "Swordsman". That covers monsters and NPCs, which the server never
/// names.
///
/// **Player classes are then overlaid from Hercules**, because the `JT_`
/// constants are sprite identities and go wrong at rebirth: Gravity kept the
/// pre-rebirth name and suffixed it, so job 4010 is `JT_WIZARD_H` where both
/// the server and the player say *High Wizard*, and 4016 is `JT_MONK_H` for a
/// Champion. `hercules_job_names.tsv` is generated from the connected server's
/// own `db/constants.conf` by `tools/export_job_names.py`, so those labels
/// cannot drift from the ids arriving on the wire.
pub struct JobName(Cow<'static, str>);

/// Player class names, `id\tname` per line. See `tools/export_job_names.py`.
const HERCULES_JOB_NAMES: &str = include_str!("hercules_job_names.tsv");

impl Display for JobName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Bundled Hercules class names as `(job id, name)`, comments skipped.
fn hercules_job_names() -> impl Iterator<Item = (u16, &'static str)> {
    HERCULES_JOB_NAMES.lines().filter_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (job_id, name) = line.split_once('\t')?;
        Some((job_id.trim().parse().ok()?, name.trim()))
    })
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

        // Gravity renamed this global between client revisions and both spellings
        // are in the wild, so take whichever the installed GRF actually defines.
        // Reading only `jobtbl` silently produced an *empty* table against our
        // GRF — which names it `JTtbl` — and every class in the client fell back
        // to "Adventurer". `JobIdentity` reads both for the same reason.
        for table_name in ["jobtbl", "JTtbl"] {
            let Ok(table) = globals.get::<mlua::Table>(table_name) else {
                continue;
            };

            for (key, value) in table.pairs::<String, u16>().flatten() {
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

        if result.is_empty() {
            #[cfg(feature = "debug")]
            {
                use korangar_debug::logging::{Colorize, print_debug};
                print_debug!(
                    "[{}] jobidentity.lub defined neither `jobtbl` nor `JTtbl` — every class will read as \"{}\"",
                    "warning".yellow(),
                    "Adventurer"
                );
            }
        }

        // Authoritative for player classes; the lub keeps the ids Hercules does
        // not name, which is every monster and NPC view.
        for (job_id, name) in hercules_job_names() {
            result.insert(JobId(job_id), JobName(name.to_owned().into()));
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
    use hashbrown::HashMap;

    use super::{hercules_job_names, prettify};

    /// The rebirth classes are the whole reason this overlay exists: the `JT_`
    /// constants behind them prettify to "Wizard H" / "Monk H" / "Dancer H".
    #[test]
    fn hercules_names_cover_the_rebirth_classes() {
        let names: HashMap<u16, &str> = hercules_job_names().collect();

        assert_eq!(names.get(&0), Some(&"Novice"));
        assert_eq!(names.get(&4010), Some(&"High Wizard"));
        assert_eq!(names.get(&4016), Some(&"Champion"));
        assert_eq!(names.get(&4021), Some(&"Gypsy"));
        // A trailing digit is a mount view, not a class: 4022 is a Paladin on a
        // grand peco, and must not read as "Paladin2".
        assert_eq!(names.get(&4022), Some(&"Paladin"));
        assert!(names.len() > 100, "expected the whole job table, got {}", names.len());
    }

    #[test]
    fn prettifies_job_constants() {
        assert_eq!(prettify("SWORDMAN"), "Swordman");
        assert_eq!(prettify("HIGH_SWORDMAN"), "High Swordman");
        assert_eq!(prettify("SOUL_LINKER"), "Soul Linker");
        // Trailing or doubled separators must not produce empty words.
        assert_eq!(prettify("NOVICE_"), "Novice");
    }
}
