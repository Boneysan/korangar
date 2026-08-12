//! Manages archives where game assets are stored and provides convenient
//! methods to retrieve each of them individually. The archives implement the
//! [`Archive`] trait.

mod cache;
mod list;

use core::panic;
use std::path::Path;
use std::sync::RwLock;

use blake3::Hash;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_loaders::{FileLoader, FileNotFoundError};

pub use self::cache::{sync_cache_archive, texture_file_dds_name, video_file_ivf_name};
use self::list::GameArchiveList;
use super::archive::folder::FolderArchive;
use super::archive::native::{NativeArchive, NativeArchiveBuilder};
use super::archive::{Archive, ArchiveType, Compression, Writable};
use crate::loaders::archive::seven_zip::{SevenZipArchive, SevenZipArchiveBuilder};

pub(crate) const CACHE_FILE_NAME: &str = "cache.7z";
pub(crate) const LUA_ARCHIVE_FILE_NAME: &str = "lua_files.7z";

pub(crate) const TEMPORARY_CACHE_FILE_NAME: &str = "cache.7z.tmp";
pub(crate) const HASH_FILE_PATH: &str = "game_file_hash.txt";

/// This string is used to derive an initialization vector for the game file
/// hash calculation. We can use this to trigger a de-sync of the cache files of
/// users.
const GAME_FILE_DERIVE_KEY: &str = "korangar 2025-03-09 14:17:23 game file key v1";

struct LoaderArchive {
    archive: Box<dyn Archive>,
    is_game_archive: bool,
}

/// Type implementing the game file loader.
///
/// Currently, there are two types implementing
/// [`Archive`]:
/// - [`NativeArchive`] - Retrieve assets from GRF files.
/// - [`FolderArchive`] - Retrieve assets from an OS folder.
/// - [`SevenZipArchive`] - Retrieve assets from ZIP files.
#[derive(Default)]
pub struct GameFileLoader {
    archives: RwLock<Vec<LoaderArchive>>,
}

impl FileLoader for GameFileLoader {
    fn get(&self, path: &str) -> Result<Vec<u8>, FileNotFoundError> {
        let lowercase_path = path.to_lowercase();
        self.archives
            .read()
            .unwrap()
            .iter()
            .find_map(|archive| archive.archive.get_file_by_path(&lowercase_path))
            .ok_or_else(|| FileNotFoundError::new(path.to_owned()))
    }
}

impl GameFileLoader {
    pub fn file_exists(&self, path: &str) -> bool {
        self.archives
            .read()
            .unwrap()
            .iter()
            .any(|archive| archive.archive.file_exists(path))
    }

    fn add_archive(&self, archive: Box<dyn Archive>, is_game_archive: bool) {
        self.archives.write().unwrap().insert(0, LoaderArchive { archive, is_game_archive });
    }

    fn get_archive_type_by_path(path: &Path) -> ArchiveType {
        if path.is_dir() || path.display().to_string().ends_with('/') {
            ArchiveType::Folder
        } else if let Some(extension) = path.extension()
            && let Some("grf") = extension.to_str()
        {
            ArchiveType::Native
        } else if let Some(extension) = path.extension()
            && let Some("7z") = extension.to_str()
        {
            ArchiveType::SevenZip
        } else {
            panic!("Provided archive must be a directory or have a .grf extension")
        }
    }

    fn load_archive_from_path(path: &str) -> Box<dyn Archive> {
        let path = Path::new(path);

        match GameFileLoader::get_archive_type_by_path(path) {
            ArchiveType::Folder => Box::new(FolderArchive::from_path(path)),
            ArchiveType::Native => Box::new(NativeArchive::from_path(path)),
            ArchiveType::SevenZip => Box::new(SevenZipArchive::from_path(path)),
        }
    }

    pub fn load_archives_from_settings(&self) {
        #[cfg(feature = "debug")]
        let timer = Timer::new("load game archives");

        let game_archive_list = GameArchiveList::load();

        game_archive_list.archives.iter().for_each(|path| {
            let game_archive = Self::load_archive_from_path(path);
            self.add_archive(game_archive, true);
        });

        #[cfg(feature = "debug")]
        timer.stop();
    }

    pub fn calculate_hash(&self) -> Hash {
        let mut hasher = blake3::Hasher::new_derive_key(GAME_FILE_DERIVE_KEY);
        self.archives
            .read()
            .unwrap()
            .iter()
            .filter(|archive| archive.is_game_archive)
            .for_each(|archive| archive.archive.hash(&mut hasher));
        hasher.finalize()
    }

    pub fn remove_patched_lua_files(&self) {
        if Path::new(LUA_ARCHIVE_FILE_NAME).exists() {
            std::fs::remove_file(LUA_ARCHIVE_FILE_NAME).unwrap();
        }
    }

    pub fn load_patched_lua_files(&self) {
        if !Path::new(LUA_ARCHIVE_FILE_NAME).exists() {
            self.patch_lua_files();
        }

        let lua_archive = Self::load_archive_from_path(LUA_ARCHIVE_FILE_NAME);
        self.add_archive(lua_archive, false);
    }

    /// Resolve a server-sent map name to a loadable `.rsw` resource name.
    ///
    /// Hercules instanced maps arrive as `<instance>#<base>` (e.g.
    /// `000#pronter`), and the 12-byte wire limit may additionally have
    /// truncated the base name. Strip the instance prefix and, when the
    /// stripped name has no `.rsw` of its own, complete it against the
    /// archive file table (first alphabetical match).
    pub fn resolve_map_name(&self, map_name: &str) -> String {
        if map_name.is_empty() || self.file_exists(&format!("data\\{map_name}.rsw")) {
            return map_name.to_owned();
        }

        let Some((_, base)) = map_name.rsplit_once('#') else {
            return map_name.to_owned();
        };

        if self.file_exists(&format!("data\\{base}.rsw")) {
            return base.to_owned();
        }

        let prefix = format!("data\\{}", base.to_lowercase());
        self.get_files_with_extension(&[".rsw"])
            .iter()
            .find(|path| path.to_lowercase().starts_with(&prefix))
            .map(|path| path["data\\".len()..path.len() - ".rsw".len()].to_owned())
            .unwrap_or_else(|| base.to_owned())
    }

    pub fn get_files_with_extension(&self, extensions: &[&str]) -> Vec<String> {
        let mut files = Vec::new();
        self.archives
            .read()
            .unwrap()
            .iter()
            .for_each(|archive| archive.archive.get_files_with_extension(&mut files, extensions));

        files.sort();
        files.dedup();

        files
    }
}

#[cfg(test)]
mod resolve_map_name_tests {
    use korangar_loaders::FileLoader;

    use super::GameFileLoader;

    /// Needs the configured game archives; run explicitly with
    /// `cargo test -p korangar resolve_map_name -- --ignored`.
    #[test]
    #[ignore]
    fn resolves_instanced_and_truncated_names() {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        // Plain maps resolve to themselves.
        assert_eq!(game_file_loader.resolve_map_name("prontera"), "prontera");
        // Instance prefix is stripped.
        assert_eq!(game_file_loader.resolve_map_name("001#izlude"), "izlude");
        // A wire-truncated base name is completed from the archive table.
        assert_eq!(game_file_loader.resolve_map_name("000#pronter"), "prontera");
        // Unknown names fall through unchanged.
        assert_eq!(game_file_loader.resolve_map_name("no_such_map"), "no_such_map");
    }

    /// Needs the configured game archives; run explicitly with
    /// `cargo test -p korangar loads_emote_assets -- --ignored`.
    #[test]
    #[ignore]
    fn loads_emote_assets() {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        game_file_loader
            .get("data\\sprite\\이팩트\\emotion.spr")
            .expect("emotion sprite should exist in the configured archives");
        game_file_loader
            .get("data\\sprite\\이팩트\\emotion.act")
            .expect("emotion actions should exist in the configured archives");
    }

    /// Diagnostic inventory for effect work; deliberately ignored because it
    /// opens the configured multi-gigabyte GRFs.
    #[test]
    #[ignore]
    fn reports_representative_skill_effect_assets() {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        let needles = [
            "fire", "cold", "frost", "ice", "soul", "storm", "light", "bolt", "bash", "magnum", "raid", "hiding",
        ];
        let effect_files = game_file_loader.get_files_with_extension(&[".str", ".spr", ".act"]);
        println!("{} STR/SPR/ACT assets", effect_files.len());
        for path in effect_files {
            let lowercase = path.to_lowercase();
            let legacy_effect = lowercase
                .strip_prefix("data\\sprite\\이팩트\\")
                .is_some_and(|relative| !relative.contains('\\'));
            let root_effect = lowercase
                .strip_prefix("data\\texture\\effect\\")
                .is_some_and(|relative| relative.ends_with(".str") && !relative.contains('\\'));
            if legacy_effect
                || root_effect
                || (needles.iter().any(|needle| lowercase.contains(needle))
                    && (lowercase.ends_with(".str") || lowercase.contains("\\이팩트\\") || lowercase.contains("\\effect\\")))
            {
                println!("{path}");
            }
        }
    }

    /// Opens the configured GRFs, so keep it out of the default fast suite.
    #[test]
    #[ignore]
    fn loads_classic_knight_spear_layer() {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        for extension in ["spr", "act"] {
            let path = format!("data\\sprite\\인간족\\기사\\기사_남_창.{extension}").to_lowercase();
            assert!(game_file_loader.file_exists(&path), "missing {path}");
        }

        for path in [
            "data\\texture\\effect\\ring_yellow.tga",
            "data\\texture\\effect\\대폭발.tga",
            "data\\texture\\effect\\lens1.tga",
            "data\\texture\\effect\\lens2.tga",
            "data\\texture\\effect\\pierce.str",
            "data\\texture\\effect\\earthhit.str",
            "data\\texture\\effect\\brandish.str",
            "data\\texture\\effect\\brandish2.str",
            "data\\texture\\effect\\spearstab.str",
            "data\\texture\\effect\\spearboomerang.str",
            "data\\texture\\effect\\bowling.str",
            "data\\wav\\effect\\ef_magnumbreak.wav",
            "data\\wav\\effect\\knight_brandish_spear.wav",
            "data\\wav\\effect\\knight_spear_boomerang.wav",
            "data\\wav\\_enemy_hit_normal1.wav",
            "data\\wav\\effect\\ef_hit2.wav",
        ] {
            let path = path.to_lowercase();
            assert!(game_file_loader.file_exists(&path), "missing {path}");
        }
    }

    /// Diagnostic: probe shield SPR paths in the original RO client GRFs
    /// (`../../RO/client/…` relative to korangar/korangar) vs configured pack.
    #[test]
    #[ignore]
    fn probes_original_client_shield_paths() {
        use std::path::PathBuf;

        let game_file_loader = GameFileLoader::default();
        // Original client archives from data.ini order (later overrides earlier
        // in native client; we load all and file_exists if any has it).
        let root = PathBuf::from("/Volumes/T7/GitHub/RO/client");
        for name in ["renewal2021.grf", "resources2021.grf", "data.grf", "rdata.grf"] {
            let path = root.join(name);
            if path.exists() {
                let archive = GameFileLoader::load_archive_from_path(path.to_str().unwrap());
                // insert so lookups hit it
                game_file_loader.add_archive(archive, true);
                println!("loaded {}", path.display());
            } else {
                println!("missing {}", path.display());
            }
        }

        // Classic patterns reported by community clients + this pack's job form
        let mut found = Vec::new();
        for sex in ["남", "여"] {
            for id in 1..=10u32 {
                for base in [
                    format!("방패\\{sex}\\{id}_{sex}"),
                    format!("방패\\{sex}\\방패_{sex}_{id}"),
                    format!("방패\\방패_{sex}_{id}"),
                    format!("인간족\\방패\\{sex}\\{id}_{sex}"),
                    format!("인간족\\방패\\{sex}\\방패_{sex}_{id}"),
                    format!("방패\\기사\\기사_{sex}_가드"),
                    format!("방패\\기사\\기사_{sex}_버클러"),
                    format!("방패\\기사\\기사_{sex}_쉴드"),
                    format!("방패\\기사\\기사_{sex}_실드"),
                    format!("방패\\검사\\검사_{sex}_가드"),
                    format!("방패\\검사\\검사_{sex}_버클러"),
                    format!("방패\\기사\\기사_{sex}_{id}_방패"),
                    format!("방패\\기사\\기사_{sex}_{id}"),
                ] {
                    let spr = format!("data\\sprite\\{base}.spr").to_lowercase();
                    if game_file_loader.file_exists(&spr) {
                        found.push(base);
                    }
                }
            }
        }
        found.sort();
        found.dedup();
        println!("classic/job pattern hits ({}):", found.len());
        for p in &found {
            println!("  {p}");
        }

        let files = game_file_loader.get_files_with_extension(&["spr"]);
        let bangpae: Vec<_> = files.iter().filter(|p| p.contains("방패")).cloned().collect();
        println!("total 방패 spr in original client archives: {}", bangpae.len());
        let mut sorted = bangpae;
        sorted.sort_by_key(|p| p.len());
        println!("shortest 40:");
        for p in sorted.iter().take(40) {
            println!("  {p}");
        }
        println!("기사 folder:");
        for p in files.iter().filter(|p| p.contains("방패\\기사\\") || p.contains("방패/기사/")) {
            println!("  {p}");
        }
        println!("sex-only folder 방패\\남:");
        for p in files.iter().filter(|p| p.contains("방패\\남\\") || p.contains("방패/남/")).take(30) {
            println!("  {p}");
        }

        // Also confirm korangar's configured archives (cwd-relative GRFs).
        let korangar = GameFileLoader::default();
        korangar.load_archives_from_settings();
        println!("korangar settings archives:");
        for base in [
            // Nested (job subfolder) — how files appear in GRF listing tools
            "방패\\기사\\기사_남_가드",
            "방패\\기사\\기사_남_버클러",
            "방패\\기사\\기사_남_쉴드",
            "방패\\검사\\검사_남_가드",
            // Flat native sprintf form from Ragexe 0x7C46C0:
            //   방패\%s_%s%s.%s  → 방패\{job}_{sex}_{suffix}.spr  (suffix includes leading _)
            //   방패\%s_%s_%d_방패.%s → 방패\{job}_{sex}_{id}_방패.spr
            "방패\\기사_남_가드",
            "방패\\기사_남_버클러",
            "방패\\기사_남_쉴드",
            "방패\\기사_남_1_방패",
            "방패\\기사_남_2101_방패",
            "방패\\기사_남_28901_방패",
        ] {
            let p = format!("data\\sprite\\{base}.spr").to_lowercase();
            println!("  {base}: {}", if korangar.file_exists(&p) { "YES" } else { "NO" });
        }
    }

    /// Diagnostic: body vs sword vs shield ACT action/clip counts (Knight
    /// male).
    #[test]
    #[ignore]
    fn dumps_sword_and_shield_action_counts() {
        use ragnarok_bytes::{ByteReader, FromBytes};
        use ragnarok_formats::action::ActionsData;
        use ragnarok_formats::version::GenericFormatMetadata;

        let l = GameFileLoader::default();
        l.load_archives_from_settings();
        for part in [
            "인간족\\몸통\\남\\기사_남",
            "인간족\\기사\\기사_남_검",
            "방패\\기사\\기사_남_가드",
            "인간족\\기사\\기사_남_창",
        ] {
            let path = format!("data\\sprite\\{part}.act");
            match l.get(&path) {
                Ok(bytes) => {
                    let mut br = ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes);
                    let act = ActionsData::from_bytes(&mut br).expect("parse");
                    println!("== {part} ==");
                    println!(
                        "actions {} events {:?}",
                        act.actions.len(),
                        act.events.iter().map(|e| e.name.as_str()).collect::<Vec<_>>()
                    );
                    for (i, a) in act.actions.iter().enumerate() {
                        if i % 8 != 0 {
                            continue;
                        }
                        let n = a.motions.len();
                        let clips: usize = a.motions.iter().map(|m| m.sprite_clips.len()).sum();
                        let nonempty = a.motions.iter().filter(|m| !m.sprite_clips.is_empty()).count();
                        println!(
                            "  group {:2}: {} motions, {} with clips, {} total clips",
                            i / 8,
                            n,
                            nonempty,
                            clips
                        );
                    }
                }
                Err(e) => println!("== {part} == MISSING {e:?}"),
            }
        }
    }

    /// Diagnostic: probe classic shield SPR path patterns under 방패.
    #[test]
    #[ignore]
    fn probes_shield_sprite_paths() {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        let mut found = 0usize;
        let mut seen = std::collections::BTreeSet::new();
        for sex in ["남", "여"] {
            for id in 0..=50u32 {
                let patterns = [
                    format!("인간족\\방패\\{sex}\\방패_{sex}_{id}"),
                    format!("인간족\\방패\\방패_{sex}_{id}"),
                    format!("방패\\{sex}\\방패_{sex}_{id}"),
                    format!("방패\\방패_{sex}_{id}"),
                    format!("방패\\{sex}\\{id}_{sex}"),
                    format!("인간족\\방패\\{sex}\\{id}_{sex}"),
                    format!("인간족\\방패\\{id}_{sex}"),
                    format!("방패\\{id}_{sex}"),
                    format!("인간족\\방패\\{sex}\\{id}"),
                    format!("방패\\{sex}\\{id}"),
                ];
                for base in patterns {
                    let spr = format!("data\\sprite\\{base}.spr").to_lowercase();
                    if game_file_loader.file_exists(&spr) && seen.insert(base.clone()) {
                        println!("FOUND {base}.spr");
                        found += 1;
                    }
                }
            }
            // named classics
            for name in ["가드", "버클러", "실드", "미러실드", "가아드", "buckler", "guard", "shield"] {
                for base in [
                    format!("인간족\\방패\\{sex}\\{name}"),
                    format!("인간족\\방패\\{sex}\\{name}_{sex}"),
                    format!("방패\\{sex}\\{name}"),
                    format!("방패\\{sex}\\{name}_{sex}"),
                    format!("방패\\{name}_{sex}"),
                ] {
                    let spr = format!("data\\sprite\\{base}.spr").to_lowercase();
                    if game_file_loader.file_exists(&spr) && seen.insert(base.clone()) {
                        println!("FOUND {base}.spr");
                        found += 1;
                    }
                }
            }
        }
        // extension listing for 방패 if the loader can
        let files = game_file_loader.get_files_with_extension(&["spr"]);
        let with_bangpae: Vec<_> = files.iter().filter(|p| p.contains("방패")).cloned().collect();
        println!("paths containing 방패: {}", with_bangpae.len());
        // Prefer short/classic paths first
        let mut sorted = with_bangpae;
        sorted.sort_by_key(|p| p.len());
        for p in sorted.iter().take(60) {
            println!("  {p}");
        }
        // Top-level folders under data\sprite\인간족\
        let mut folders = std::collections::BTreeSet::new();
        for p in files.iter().filter(|p| p.contains("인간족\\") || p.contains("인간족/")) {
            let rest = p.split("인간족\\").nth(1).or_else(|| p.split("인간족/").nth(1));
            if let Some(rest) = rest {
                let folder = rest.split(['\\', '/']).next().unwrap_or("");
                folders.insert(folder.to_string());
            }
        }
        println!("인간족 folders ({}):", folders.len());
        for f in &folders {
            println!("  {f}");
        }
        // Any path with shield-related hangul
        for needle in ["방패", "패", "쉴드", "가드", "버클"] {
            let n = files.iter().filter(|p| p.contains(needle)).count();
            println!("contains '{needle}': {n}");
        }
        println!("total unique pattern hits: {found}");
        println!("--- 기사 shield names ---");
        for p in files.iter().filter(|p| p.contains("방패\\기사\\") || p.contains("방패/기사/")) {
            println!("  {p}");
        }
        println!("--- 검사 shield names ---");
        for p in files.iter().filter(|p| p.contains("방패\\검사\\") || p.contains("방패/검사/")) {
            println!("  {p}");
        }
    }

    /// Diagnostic: parse the real Knight body/weapon/head ACT files and dump
    /// per-action frame counts, delays, and sound-event placement, to compare
    /// the client's animation playback against the original data. Run with
    /// `--ignored --nocapture`.
    #[test]
    #[ignore]
    fn reports_knight_attack_frame_structure() {
        use ragnarok_bytes::{ByteReader, FromBytes};
        use ragnarok_formats::action::ActionsData;
        use ragnarok_formats::version::GenericFormatMetadata;

        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        for part in [
            "인간족\\몸통\\남\\기사_남",
            "인간족\\기사\\기사_남_창",
            "인간족\\기사\\기사_남_양손검",
            "인간족\\머리통\\남\\1_남",
            // Empty event tables on these are why SinX needs a synthetic Attack
            // fallback at the native attack-event marker (see collect_crossed_events).
            "인간족\\몸통\\남\\어세신_남",
            "인간족\\몸통\\남\\어쌔신크로스_남",
            "인간족\\몸통\\여\\어세신_여",
        ] {
            let path = format!("data\\sprite\\{part}.act");
            let Ok(bytes) = game_file_loader.get(&path) else {
                println!("== {part} == MISSING");
                continue;
            };
            let mut byte_reader: ByteReader = ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes);
            let actions_data = ActionsData::from_bytes(&mut byte_reader).expect("action file should parse");

            println!("== {part} ==");
            println!(
                "actions: {} events: {:?}",
                actions_data.actions.len(),
                actions_data.events.iter().map(|event| event.name.as_str()).collect::<Vec<_>>()
            );
            let delays = actions_data.delays.clone().unwrap_or_default();
            for (action_index, action) in actions_data.actions.iter().enumerate() {
                // Only print direction 0 of each action group.
                if action_index % 8 != 0 {
                    continue;
                }
                let events: Vec<String> = action
                    .motions
                    .iter()
                    .enumerate()
                    .filter_map(|(motion_index, motion)| {
                        motion
                            .event_id
                            .filter(|event_id| *event_id != -1)
                            .map(|event_id| format!("motion {motion_index} → event {event_id}"))
                    })
                    .collect();
                println!(
                    "action group {:2} (act {:3}): {} motions, delay {:?}, events: {:?}",
                    action_index / 8,
                    action_index,
                    action.motions.len(),
                    delays.get(action_index),
                    events
                );
            }
        }
    }

    /// Diagnostic: probe candidate classic sound names for the mapped skill
    /// effects. Run with `--ignored --nocapture` and wire only confirmed
    /// names. Opens the configured GRFs.
    #[test]
    #[ignore]
    fn probes_classic_skill_sound_candidates() {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        let candidates = [
            "effect\\ef_firebolt.wav",
            "effect\\ef_coldbolt.wav",
            "effect\\ef_lightingbolt.wav",
            "effect\\ef_lightningbolt.wav",
            "effect\\ef_thunderstorm.wav",
            "effect\\ef_soulstrike.wav",
            "effect\\ef_frostdiver.wav",
            "effect\\ef_fireball.wav",
            "effect\\ef_firewall.wav",
            "effect\\ef_napalmbeat.wav",
            "effect\\ef_stormgust.wav",
            "effect\\ef_meteorstorm.wav",
            "effect\\ef_lordofvermilion.wav",
            "effect\\ef_frostnova.wav",
            "effect\\ef_firepillar.wav",
            "effect\\ef_firepillarbomb.wav",
            "effect\\ef_quagmire.wav",
            "effect\\ef_magnus.wav",
            "effect\\ef_magnuslight.wav",
            "effect\\ef_santuary.wav",
            "effect\\ef_sanctuary.wav",
            "effect\\ef_holylight.wav",
            "effect\\ef_turnundead.wav",
            "effect\\ef_hammerfall.wav",
            "effect\\ef_venomdust.wav",
            "effect\\ef_landmine.wav",
            "effect\\ef_sandman.wav",
            "effect\\ef_freezingtrap.wav",
            "effect\\ef_blastmine.wav",
            "effect\\ef_claymoretrap.wav",
            "effect\\ef_sonicblow.wav",
            "effect\\ef_ignitionbreak.wav",
            "effect\\ef_meteorassault.wav",
            "effect\\ef_raid.wav",
            "effect\\ef_firearrow1.wav",
            "effect\\ef_firearrow2.wav",
            "effect\\ef_firearrow3.wav",
            "effect\\ef_icearrow1.wav",
            "effect\\ef_icearrow.wav",
            "effect\\assasin_sonicblow.wav",
            "effect\\rogue_raid.wav",
            "effect\\t_돌붕괴.wav",
            "effect\\storm.wav",
            "effect\\stormgust.wav",
            "effect\\ef_snowstorm.wav",
            "effect\\meteor.wav",
            "effect\\ef_meteor.wav",
            "effect\\meteorstorm.wav",
            "effect\\lord.wav",
            "effect\\lordofvermilion.wav",
            "effect\\ef_lord.wav",
            "effect\\frostnova.wav",
            "effect\\icecrash.wav",
            "effect\\ef_icecrash.wav",
            "effect\\firepillar.wav",
            "effect\\quagmire.wav",
            "effect\\sanctuary.wav",
            "effect\\santuary.wav",
            "effect\\magnus.wav",
            "effect\\holylight.wav",
            "effect\\ef_holyhit.wav",
            "effect\\turnundead.wav",
            "effect\\hammerfall.wav",
            "effect\\landmine.wav",
            "effect\\sandman.wav",
            "effect\\freezingtrap.wav",
            "effect\\blastmine.wav",
            "effect\\claymore.wav",
            "effect\\skidtrap.wav",
            "effect\\venomdust.wav",
            "effect\\raid.wav",
            "effect\\meteorassault.wav",
            "effect\\ignitionbreak.wav",
            "effect\\rk_ignitionbreak.wav",
            "effect\\ef_lightning.wav",
            "effect\\lightningbolt.wav",
            "effect\\thunder.wav",
            "effect\\ef_thunder.wav",
            "effect\\ef_firehit.wav",
            "effect\\firehit.wav",
            "effect\\ef_windhit.wav",
            "effect\\ef_bash.wav",
        ];

        for candidate in candidates {
            let path = format!("data\\wav\\{candidate}").to_lowercase();
            let status = match game_file_loader.file_exists(&path) {
                true => "FOUND  ",
                false => "missing",
            };
            println!("{status} {candidate}");
        }

        for candidate in [
            "texture\\effect\\bash.str",
            "texture\\effect\\bash3d.str",
            "texture\\effect\\매지컬어택.str",
            "texture\\effect\\napalmbeat.str",
            "texture\\effect\\firesplashhit.str",
        ] {
            let path = format!("data\\{candidate}").to_lowercase();
            let status = match game_file_loader.file_exists(&path) {
                true => "FOUND  ",
                false => "missing",
            };
            println!("{status} {candidate}");
        }

        println!("== all effect wav files the archives list ==");
        for path in game_file_loader.get_files_with_extension(&[".wav"]) {
            if path.to_lowercase().contains("effect") {
                println!("listed {path}");
            }
        }
    }

    /// The exact weapon sprite layers the client will request for the
    /// provisioned effect roster and the classic folder aliases (transcendent
    /// classes reuse base folders, Priest weapons live under 프리스트).
    /// Opens the configured GRFs, so keep it out of the default fast suite.
    #[test]
    #[ignore]
    fn loads_classic_weapon_layers_for_roster() {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        for part_file in [
            // Knight: spear, two-handed spear, sword, two-handed sword.
            "인간족\\기사\\기사_남_창",
            "인간족\\기사\\기사_남_양손창",
            "인간족\\기사\\기사_남_검",
            "인간족\\기사\\기사_남_양손검",
            // Assassin Cross reuses the Assassin folder; katar pair sprite.
            "인간족\\어세신\\어세신_남_카타르_카타르",
            "인간족\\어세신\\어세신_여_단검_단검",
            // Stalker reuses the Rogue folder.
            "인간족\\로그\\로그_남_단검",
            "인간족\\로그\\로그_남_활",
            // Rune Knight ships its own third-class files.
            "인간족\\룬나이트\\룬나이트_남_창",
            "인간족\\룬나이트\\룬나이트_남_양손검",
            // Priest weapons live under 프리스트, not the body folder name.
            "인간족\\프리스트\\프리스트_남_클럽",
            "인간족\\프리스트\\프리스트_여_책",
            // Hunter bow.
            "인간족\\헌터\\헌터_여_활",
            // Phase D: per-item Mjolnir (item 1530) + its trail on Knight.
            "인간족\\기사\\기사_남_1530",
            "인간족\\기사\\기사_남_1530_검광",
            "인간족\\기사\\기사_여_1530",
            "인간족\\기사\\기사_여_1530_검광",
        ] {
            for extension in ["spr", "act"] {
                let path = format!("data\\sprite\\{part_file}.{extension}").to_lowercase();
                assert!(game_file_loader.file_exists(&path), "missing {path}");
            }
        }
    }

    /// Phase E1 texture hunt: probe classic effect TGA/BMP candidates for
    /// travel balls, soul orbs, and ground spikes. `file_exists` is
    /// authoritative (archive listing under-reports). Run:
    /// `cargo test -p korangar --lib probes_e1_procedural_effect_textures --
    /// --ignored --nocapture`
    #[test]
    #[ignore]
    fn probes_e1_procedural_effect_textures() {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        let candidates = [
            // Currently wired stand-ins
            "effect\\불화살1.tga",
            "effect\\icearrow.tga",
            "effect\\lens1.tga",
            "effect\\lens2.tga",
            "effect\\purpleslash.tga",
            "effect\\ring_yellow.tga",
            "effect\\ring2.bmp",
            // Balls / spheres / orbs
            "effect\\ball.tga",
            "effect\\ball1.tga",
            "effect\\ball2.tga",
            "effect\\sphere.tga",
            "effect\\sphere1.tga",
            "effect\\sphere2.tga",
            "effect\\magic_sphere.tga",
            "effect\\magic_sphere1.tga",
            "effect\\white_sphere.tga",
            "effect\\orb.tga",
            "effect\\orb1.tga",
            "effect\\soul.tga",
            "effect\\soul1.tga",
            "effect\\spirit.tga",
            "effect\\ghost.tga",
            "effect\\pok1.tga",
            "effect\\pok2.tga",
            "effect\\particle1.tga",
            "effect\\particle2.tga",
            "effect\\particle3.tga",
            "effect\\magic.tga",
            "effect\\magic1.tga",
            "effect\\magic2.tga",
            "effect\\magic3.tga",
            "effect\\alpha_down.tga",
            "effect\\circle1.tga",
            "effect\\circle2.tga",
            "effect\\ring_blue.tga",
            "effect\\ring_red.tga",
            "effect\\ring_violet.tga",
            "effect\\ring_green.tga",
            "effect\\ring_white.tga",
            // Fire / ice / electric travel
            "effect\\fireball.tga",
            "effect\\fireball1.tga",
            "effect\\fire.tga",
            "effect\\fire1.tga",
            "effect\\flame.tga",
            "effect\\firesplash.tga",
            "effect\\firesplashhit.tga",
            "effect\\iceball.tga",
            "effect\\ice.tga",
            "effect\\ice1.tga",
            "effect\\freezing.tga",
            "effect\\freeze.tga",
            "effect\\thunderball.tga",
            "effect\\thunder.tga",
            "effect\\thunder1.tga",
            "effect\\lightning.tga",
            "effect\\electric.tga",
            "effect\\plasma1.tga",
            "effect\\plasma2.tga",
            "effect\\jupitel.tga",
            "effect\\yufitel.tga",
            "effect\\yufitelhit.tga",
            // Spikes / earth
            "effect\\spike.tga",
            "effect\\spike1.tga",
            "effect\\earth.tga",
            "effect\\earth1.tga",
            "effect\\stone.tga",
            "effect\\stone1.tga",
            "effect\\rock.tga",
            "effect\\quake.tga",
            "effect\\stonecurse.tga",
            "effect\\elemental_earth.tga",
            "effect\\대폭발.tga",
            // Korean classic effect names often used by code-drawn effects
            "effect\\전기구.tga",
            "effect\\파이어볼.tga",
            "effect\\아이스볼.tga",
            "effect\\소울.tga",
            "effect\\정령.tga",
            "effect\\불.tga",
            "effect\\얼음.tga",
            "effect\\전기.tga",
            "effect\\마구.tga",
            "effect\\마구1.tga",
            "effect\\마구2.tga",
            "effect\\스피어.tga",
            "effect\\스피어1.tga",
            "effect\\돌.tga",
            "effect\\바위.tga",
            "effect\\땅.tga",
            "effect\\번개.tga",
            "effect\\번개구.tga",
            "effect\\영혼.tga",
            "effect\\영혼1.tga",
            "effect\\유령.tga",
            "effect\\나선.tga",
            "effect\\구슬.tga",
            "effect\\구슬1.tga",
            "effect\\빛.tga",
            "effect\\빛1.tga",
            "effect\\핑크.tga",
            "effect\\보라.tga",
            // Sprite-based projectiles (effect folder under sprite)
            "sprite\\이팩트\\전기구.spr",
            "sprite\\이팩트\\파이어볼.spr",
            "sprite\\이팩트\\아이스볼.spr",
            "sprite\\이팩트\\소울.spr",
            "sprite\\이팩트\\정령.spr",
            "sprite\\이팩트\\불.spr",
            "sprite\\이팩트\\얼음.spr",
            "sprite\\이팩트\\전기.spr",
            "sprite\\이팩트\\창.spr",
            "sprite\\이팩트\\스피어.spr",
            "sprite\\이팩트\\돌.spr",
            "sprite\\이팩트\\번개.spr",
            "sprite\\이팩트\\영혼.spr",
            "sprite\\이팩트\\구슬.spr",
            "sprite\\이팩트\\마구.spr",
        ];

        println!("== E1 procedural texture probe ==");
        let mut found = 0usize;
        for candidate in candidates {
            let path = if candidate.starts_with("sprite\\") {
                format!("data\\{candidate}").to_lowercase()
            } else {
                format!("data\\texture\\{candidate}").to_lowercase()
            };
            if game_file_loader.file_exists(&path) {
                found += 1;
                println!("FOUND   {candidate}");
            } else {
                println!("missing {candidate}");
            }
        }
        println!("== {found}/{} candidates found ==", candidates.len());

        // Keyword scan of listed TGA/BMP under texture\\effect (best-effort;
        // listing under-reports — treat as hints only).
        println!("== listed effect images matching ball|sphere|orb|spike|soul|ice|fire|thunder|electric|stone ==");
        let mut listed = 0usize;
        for path in game_file_loader.get_files_with_extension(&[".tga", ".bmp", ".TGA", ".BMP"]) {
            let lower = path.to_lowercase();
            if !lower.contains("texture\\effect\\") && !lower.contains("texture/effect/") {
                continue;
            }
            let keywords = [
                "ball",
                "sphere",
                "orb",
                "spike",
                "soul",
                "spirit",
                "ghost",
                "ice",
                "fire",
                "thunder",
                "electric",
                "stone",
                "earth",
                "plasma",
                "particle",
                "ring_",
                "구슬",
                "영혼",
                "전기",
                "파이어",
                "아이스",
                "돌",
                "번개",
            ];
            if keywords.iter().any(|k| lower.contains(k)) {
                listed += 1;
                println!("listed  {path}");
            }
        }
        println!("== {listed} listed keyword matches ==");
    }
}

impl GameFileLoader {
    fn patch_lua_files(&self) {
        use lunify::{Format, Settings, unify};

        const LUA_BYTECODE_EXTENSION: &str = ".lub";
        let lua_files = self.get_files_with_extension(&[LUA_BYTECODE_EXTENSION]);

        let path = Path::new(LUA_ARCHIVE_FILE_NAME);
        let mut lua_archive: Box<dyn Writable> = match GameFileLoader::get_archive_type_by_path(path) {
            ArchiveType::Folder => Box::new(FolderArchive::from_path(path)),
            ArchiveType::Native => Box::new(NativeArchiveBuilder::from_path(path)),
            ArchiveType::SevenZip => Box::new(SevenZipArchiveBuilder::from_path(path)),
        };

        let bytecode_format = Format::default();
        let settings = Settings::default();

        #[cfg(feature = "debug")]
        let mut total_count = lua_files.len();
        #[cfg(feature = "debug")]
        let mut failed_count = 0;

        for file_name in lua_files {
            let bytes = match self.get(&file_name) {
                Ok(bytes) => bytes,
                Err(_error) => {
                    #[cfg(feature = "debug")]
                    {
                        print_debug!(
                            "[{}] failed to extract file {} from the grf: {:?}",
                            "warning".yellow(),
                            file_name.magenta(),
                            _error
                        );
                        failed_count += 1;
                    }

                    continue;
                }
            };

            // Try to unify all bytecode to Lua 5.1 and possibly 64 bit.
            match unify(&bytes, &bytecode_format, &settings) {
                Ok(bytes) => lua_archive.add_file(&file_name, bytes, Compression::Default),
                // If the operation fails the file with this error, the Lua file is not actually a
                // pre-compiled binary but rather a source file, so we can safely ignore it.
                #[cfg(feature = "debug")]
                Err(lunify::LunifyError::IncorrectSignature) => total_count -= 1,
                Err(_error) => {
                    #[cfg(feature = "debug")]
                    {
                        print_debug!("[{}] error upcasting {}: {:?}", "warning".yellow(), file_name.magenta(), _error,);
                        failed_count += 1;
                    }
                }
            }
        }

        #[cfg(feature = "debug")]
        print_debug!(
            "converted a total of {} files of which {} failed.",
            total_count.yellow(),
            failed_count.red(),
        );

        lua_archive.finish().expect("can't save lua archive");
    }

    #[allow(unused_variables)]
    pub fn load_cache_archive(&self, game_file_hash: Hash) {
        let path = Path::new(CACHE_FILE_NAME);

        if !path.exists() && !path.is_dir() {
            return;
        }

        let archive = Box::new(SevenZipArchive::from_path(path));

        let Some(hash_file) = archive.get_file_by_path(HASH_FILE_PATH) else {
            #[cfg(feature = "debug")]
            print_debug!("Can't find game hash file. Using empty cache");
            return;
        };

        let Ok(_hash) = Hash::from_hex(hash_file) else {
            #[cfg(feature = "debug")]
            print_debug!("Can't read game hash file. Using empty cache");
            return;
        };

        #[cfg(feature = "debug")]
        if _hash != game_file_hash {
            print_debug!("[{}] Cache is out of sync. Please re-sync or delete the cache", "error".red());
        }

        self.add_archive(archive, false);
    }
}

pub fn fix_broken_texture_file_endings(path: &str) -> String {
    let mut path = path.to_string();

    if path.ends_with(".bm") {
        path.push('p');
    }

    if path.ends_with(".jp") {
        path.push('g');
    }

    if path.ends_with(".pn") {
        path.push('g');
    }

    if path.ends_with(".tg") {
        path.push('a');
    }

    path
}
