use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use flate2::bufread::ZlibDecoder;
use ragnarok_bytes::{ByteReader, FixedByteSize, FromBytes};
use ragnarok_formats::archive::{AssetTable, FileTableRow, Header};
use ragnarok_formats::map::{GatData, GroundData, MapData};
use ragnarok_formats::version::MapFormatMetadata;
use walkdir::WalkDir;

#[path = "../src/loaders/archive/native/mixcrypt.rs"]
mod mixcrypt;

struct Grf {
    path: PathBuf,
    rows: BTreeMap<String, FileTableRow>,
}

impl Grf {
    fn open(path: PathBuf) -> Result<Self, String> {
        let mut file = File::open(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let mut header_bytes = vec![0; Header::size_in_bytes()];
        file.read_exact(&mut header_bytes).map_err(|error| error.to_string())?;
        let header = Header::from_bytes(&mut ByteReader::without_metadata(&header_bytes)).map_err(|error| format!("{error:?}"))?;
        if header.version != 0x200 {
            return Err(format!("{}: unsupported GRF version {:#x}", path.display(), header.version));
        }

        file.seek(SeekFrom::Current(header.file_table_offset as i64))
            .map_err(|error| error.to_string())?;
        let mut table_bytes = vec![0; AssetTable::size_in_bytes()];
        file.read_exact(&mut table_bytes).map_err(|error| error.to_string())?;
        let table = AssetTable::from_bytes(&mut ByteReader::without_metadata(&table_bytes)).map_err(|error| format!("{error:?}"))?;
        let mut compressed = vec![0; table.compressed_size as usize];
        file.read_exact(&mut compressed).map_err(|error| error.to_string())?;
        let mut decoder = ZlibDecoder::new(compressed.as_slice());
        let mut decompressed = Vec::with_capacity(table.uncompressed_size as usize);
        decoder.read_to_end(&mut decompressed).map_err(|error| error.to_string())?;

        let mut reader = ByteReader::without_metadata(&decompressed);
        let mut rows = BTreeMap::new();
        for _ in 0..header.get_file_count() {
            let row = FileTableRow::from_bytes(&mut reader).map_err(|error| format!("{error:?}"))?;
            rows.insert(row.file_name.to_lowercase(), row);
        }
        Ok(Self { path, rows })
    }

    fn get(&self, path: &str) -> Result<Option<Vec<u8>>, String> {
        let Some(row) = self.rows.get(&path.to_lowercase()) else {
            return Ok(None);
        };
        let mut file = File::open(&self.path).map_err(|error| error.to_string())?;
        file.seek(SeekFrom::Start(row.offset as u64 + Header::size_in_bytes() as u64))
            .map_err(|error| error.to_string())?;
        let mut compressed = vec![0; row.compressed_size_aligned as usize];
        file.read_exact(&mut compressed).map_err(|error| error.to_string())?;
        mixcrypt::decrypt_file(row, &mut compressed);
        let mut decoder = ZlibDecoder::new(compressed.as_slice());
        let mut bytes = Vec::with_capacity(row.uncompressed_size as usize);
        decoder.read_to_end(&mut bytes).map_err(|error| format!("{path}: {error}"))?;
        Ok(Some(bytes))
    }
}

struct Archives(Vec<Grf>);

impl Archives {
    fn get(&self, path: &str) -> Result<Option<Vec<u8>>, String> {
        for archive in self.0.iter().rev() {
            if let Some(bytes) = archive.get(path)? {
                return Ok(Some(bytes));
            }
        }
        Ok(None)
    }
}

fn parse<Data: FromBytes>(bytes: &[u8]) -> Result<Data, String> {
    Data::from_bytes(&mut ByteReader::with_default_metadata::<MapFormatMetadata>(bytes)).map_err(|error| format!("{error:?}"))
}

fn enabled_maps(path: &Path) -> Result<BTreeSet<String>, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(text
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            (!line.starts_with("//") && line.starts_with('"') && line.ends_with("\","))
                .then(|| line.trim_matches(|character| character == '"' || character == ',').to_lowercase())
        })
        .collect())
}

fn teleport_points(path: &Path) -> Result<Vec<(String, usize, usize, usize, PathBuf)>, String> {
    let files: Vec<PathBuf> = if path.is_dir() {
        WalkDir::new(path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file() && entry.path().extension().is_some_and(|extension| extension == "txt"))
            .map(|entry| entry.into_path())
            .collect()
    } else {
        vec![path.to_owned()]
    };
    let mut points = Vec::new();
    for file in files {
        points.extend(teleport_points_in_file(&file)?);
    }
    Ok(points)
}

fn teleport_points_in_file(path: &Path) -> Result<Vec<(String, usize, usize, usize, PathBuf)>, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let mut points = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if line.contains("DM_WarpParty\", \"") {
            let quoted: Vec<_> = line.split('"').collect();
            if quoted.len() >= 5 {
                push_coordinates(&mut points, quoted[3], quoted[4], line_index, path);
            }
        } else if let Some(warp) = line.find("warp \"") {
            let quoted: Vec<_> = line[warp..].split('"').collect();
            if quoted.len() >= 3 {
                push_coordinates(&mut points, quoted[1], quoted[2], line_index, path);
            }
        } else if line.contains("\twarp\t") {
            let fields: Vec<_> = line.split('\t').collect();
            if let Some(destination) = fields.last() {
                let parts: Vec<_> = destination.split(',').map(str::trim).collect();
                if parts.len() >= 3 {
                    let length = parts.len();
                    if let (Ok(x), Ok(y)) = (parts[length - 2].parse(), parts[length - 1].parse()) {
                        points.push((parts[length - 3].to_lowercase(), x, y, line_index + 1, path.to_owned()));
                    }
                }
            }
        }
    }
    Ok(points)
}

fn push_coordinates(
    points: &mut Vec<(String, usize, usize, usize, PathBuf)>,
    map: &str,
    coordinate_text: &str,
    line_index: usize,
    path: &Path,
) {
    let coordinates: Vec<_> = coordinate_text
        .trim_matches(|character: char| character == ',' || character == ')' || character == ';' || character.is_whitespace())
        .split(',')
        .map(str::trim)
        .collect();
    if coordinates.len() >= 2
        && let (Ok(x), Ok(y)) = (coordinates[0].parse(), coordinates[1].parse())
    {
        points.push((map.to_lowercase(), x, y, line_index + 1, path.to_owned()));
    }
}

fn main() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() < 2 {
        return Err("usage: map-asset-audit <Hercules maps.conf> <archive.grf> [archive.grf ...] [NPC file or directory ...]".to_owned());
    }
    let maps = enabled_maps(Path::new(&args[0]))?;
    let archive_args: Vec<_> = args[1..]
        .iter()
        .filter(|path| Path::new(path).extension().is_some_and(|ext| ext == "grf"))
        .collect();
    let teleport_sources: Vec<_> = args[1..]
        .iter()
        .filter(|path| Path::new(path).extension().is_none_or(|ext| ext != "grf"))
        .collect();
    let archives = Archives(
        archive_args
            .iter()
            .map(|path| Grf::open(PathBuf::from(path)))
            .collect::<Result<_, _>>()?,
    );

    let mut failures = BTreeMap::new();
    for map in &maps {
        let rsw_path = format!("data\\{map}.rsw");
        let result = (|| {
            let rsw = archives.get(&rsw_path)?.ok_or_else(|| "missing rsw".to_owned())?;
            let map_data: MapData = parse(&rsw).map_err(|error| format!("rsw parse: {error}"))?;
            let gnd_path = format!("data\\{}", map_data.ground_file.to_lowercase());
            let gat_path = format!("data\\{}", map_data.gat_file.to_lowercase());
            let gnd = archives.get(&gnd_path)?.ok_or_else(|| format!("missing referenced {gnd_path}"))?;
            let gat = archives.get(&gat_path)?.ok_or_else(|| format!("missing referenced {gat_path}"))?;
            let _: GroundData = parse(&gnd).map_err(|error| format!("gnd parse: {error}"))?;
            let _: GatData = parse(&gat).map_err(|error| format!("gat parse: {error}"))?;
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            failures.insert(map.clone(), error);
        }
    }

    println!("enabled maps: {}", maps.len());
    println!("maps with parse/reference failures: {}", failures.len());
    for (map, error) in &failures {
        println!("{map}: {error}");
    }

    let mut unsafe_warps = Vec::new();
    let mut checked_warps = 0usize;
    let mut gat_cache: BTreeMap<String, Option<(usize, usize, Vec<u8>)>> = BTreeMap::new();
    for source in teleport_sources {
        for (map, x, y, line, file) in teleport_points(Path::new(source))? {
            checked_warps += 1;
            if !gat_cache.contains_key(&map) {
                let gat_path = format!("data\\{map}.gat");
                let parsed = match archives.get(&gat_path)? {
                    Some(bytes) => {
                        let gat: GatData = parse(&bytes).map_err(|error| format!("{gat_path}: {error}"))?;
                        Some((
                            gat.map_width as usize,
                            gat.map_height as usize,
                            gat.tiles.iter().map(|tile| tile.flags.bits()).collect(),
                        ))
                    }
                    None => None,
                };
                gat_cache.insert(map.clone(), parsed);
            }
            let Some((width, height, flags)) = &gat_cache[&map] else {
                unsafe_warps.push(format!("{}:{line}: {map} ({x},{y}): missing gat", file.display()));
                continue;
            };
            let safe = x < *width && y < *height && flags[x + y * width] & 1 != 0;
            if !safe {
                let nearest = (0..=100usize).find_map(|radius| {
                    (y.saturating_sub(radius)..=(y + radius).min(height - 1))
                        .flat_map(|candidate_y| {
                            (x.saturating_sub(radius)..=(x + radius).min(width - 1)).map(move |candidate_x| (candidate_x, candidate_y))
                        })
                        .find(|(candidate_x, candidate_y)| {
                            candidate_x.abs_diff(x).max(candidate_y.abs_diff(y)) == radius
                                && flags[candidate_x + candidate_y * width] & 1 != 0
                        })
                });
                unsafe_warps.push(format!(
                    "{}:{line}: {map} ({x},{y}), nearest walkable: {nearest:?}",
                    file.display()
                ));
            }
        }
    }
    println!("static teleport destinations checked: {checked_warps}");
    println!("unsafe static teleport destinations: {}", unsafe_warps.len());
    for warp in &unsafe_warps {
        println!("{warp}");
    }
    if failures.is_empty() && unsafe_warps.is_empty() {
        Ok(())
    } else {
        Err("map asset audit failed".to_owned())
    }
}
