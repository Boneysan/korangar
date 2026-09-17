//! Authoritative exploration and campaign chest manifest.
//!
//! Inventory generated from Hercules scripts (`achievement_treasures.txt` and
//! `dm_treasures.txt`). Loads and validates unique chest IDs, coordinates, and
//! schema versions.

#![allow(dead_code)]

use hashbrown::HashMap;

pub const CHEST_SCHEMA_VERSION: u32 = 1;

/// One authored or exploration chest record from the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChestRecord {
    pub id: u32,
    pub map: String,
    pub x: u16,
    pub y: u16,
    pub region: Option<u8>,
    pub title: String,
    pub hat_id: u32,
}

impl ChestRecord {
    pub fn is_act1(&self) -> bool {
        self.region.is_some()
    }
}

/// Lookup table and validator for world chests.
#[derive(Debug, Clone)]
pub struct ChestTable {
    schema_version: u32,
    by_id: HashMap<u32, ChestRecord>,
    by_coord: HashMap<(String, u16, u16), u32>,
    records: Vec<ChestRecord>,
}

impl ChestTable {
    pub fn parse(content: &str) -> Result<Self, String> {
        let mut schema_version = None;
        let mut by_id = HashMap::new();
        let mut by_coord = HashMap::new();
        let mut records = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with('#') {
                if let Some(ver_str) = line.strip_prefix("# schema_version:") {
                    let v: u32 = ver_str.trim().parse().map_err(|e| format!("invalid schema version: {e}"))?;
                    schema_version = Some(v);
                }
                continue;
            }

            if line.starts_with("id\t") {
                continue; // Header row
            }

            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 7 {
                return Err(format!("insufficient columns in row: {line}"));
            }

            let id: u32 = parts[0].parse().map_err(|e| format!("invalid chest id {}: {e}", parts[0]))?;
            let map = parts[1].to_string();
            let x: u16 = parts[2].parse().map_err(|e| format!("invalid x coordinate {}: {e}", parts[2]))?;
            let y: u16 = parts[3].parse().map_err(|e| format!("invalid y coordinate {}: {e}", parts[3]))?;
            let region_val: i8 = parts[4].parse().map_err(|e| format!("invalid region {}: {e}", parts[4]))?;
            let region = if region_val >= 0 { Some(region_val as u8) } else { None };
            let title = parts[5].to_string();
            let hat_id: u32 = parts[6].parse().map_err(|e| format!("invalid hat id {}: {e}", parts[6]))?;

            let record = ChestRecord {
                id,
                map: map.clone(),
                x,
                y,
                region,
                title,
                hat_id,
            };

            if by_id.contains_key(&id) {
                return Err(format!("duplicate chest id: {id}"));
            }

            let coord_key = (map, x, y);
            if by_coord.contains_key(&coord_key) {
                return Err(format!("duplicate coordinate on map {}: ({}, {})", coord_key.0, x, y));
            }

            by_coord.insert(coord_key, id);
            by_id.insert(id, record.clone());
            records.push(record);
        }

        let version = schema_version.ok_or_else(|| "missing schema version in chest manifest".to_string())?;
        if version != CHEST_SCHEMA_VERSION {
            return Err(format!(
                "incompatible chest manifest schema: expected {}, got {}",
                CHEST_SCHEMA_VERSION, version
            ));
        }

        Ok(Self {
            schema_version: version,
            by_id,
            by_coord,
            records,
        })
    }

    pub fn load() -> Self {
        const BUNDLED_CHESTS: &str = include_str!("chests.tsv");
        Self::parse(BUNDLED_CHESTS).expect("bundled chest manifest must be valid")
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn get_by_id(&self, id: u32) -> Option<&ChestRecord> {
        self.by_id.get(&id)
    }

    pub fn find_at(&self, map: &str, x: u16, y: u16) -> Option<&ChestRecord> {
        let clean_map = map.strip_suffix(".gat").or_else(|| map.strip_suffix(".rsw")).unwrap_or(map);
        self.by_coord.get(&(clean_map.to_string(), x, y)).and_then(|id| self.by_id.get(id))
    }

    pub fn act1_chests(&self) -> impl Iterator<Item = &ChestRecord> {
        self.records.iter().filter(|r| r.is_act1())
    }

    pub fn region_chests(&self, region: u8) -> impl Iterator<Item = &ChestRecord> {
        self.records.iter().filter(move |r| r.region == Some(region))
    }

    pub fn total_chests(&self) -> usize {
        self.records.len()
    }

    pub fn act1_total_chests(&self) -> usize {
        self.records.iter().filter(|r| r.is_act1()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_manifest_loads_and_verifies_structure() {
        let table = ChestTable::load();
        assert_eq!(table.schema_version(), 1);
        assert_eq!(table.total_chests(), 146);
        assert_eq!(table.act1_total_chests(), 38);

        // Spot-check Act I chests in each region
        let prt = table.get_by_id(120001).expect("120001 exists");
        assert_eq!(prt.map, "prt_fild01");
        assert_eq!(prt.x, 146);
        assert_eq!(prt.y, 126);
        assert_eq!(prt.region, Some(0));
        assert_eq!(prt.title, "The Milestone That Moved");
        assert_eq!(prt.hat_id, 5108);

        let gef = table.get_by_id(120011).expect("120011 exists");
        assert_eq!(gef.region, Some(1));
        assert_eq!(gef.title, "Apprentice's Practice Sheet");

        let moc = table.get_by_id(120018).expect("120018 exists");
        assert_eq!(moc.region, Some(2));
        assert_eq!(moc.title, "Water Ration Book");

        let pay = table.get_by_id(120024).expect("120024 exists");
        assert_eq!(pay.region, Some(3));
        assert_eq!(pay.title, "Lantern Oil Account");

        let alb = table.get_by_id(120131).expect("120131 exists");
        assert_eq!(alb.region, Some(4));
        assert_eq!(alb.title, "Diver's Slate");

        // Coordinate lookup
        let found = table.find_at("prt_fild01", 146, 126);
        assert_eq!(found, Some(prt));

        let found_with_gat = table.find_at("prt_fild01.gat", 146, 126);
        assert_eq!(found_with_gat, Some(prt));
    }

    #[test]
    fn rejects_incompatible_schema() {
        let bad = "# schema_version: 999\nid\tmap\tx\ty\tregion\ttitle\that_id\n";
        let err = ChestTable::parse(bad).unwrap_err();
        assert!(err.contains("incompatible chest manifest schema"));
    }

    #[test]
    fn rejects_missing_schema_version() {
        let bad = "id\tmap\tx\ty\tregion\ttitle\that_id\n120001\tprt_fild01\t146\t126\t0\tTest\t0\n";
        let err = ChestTable::parse(bad).unwrap_err();
        assert!(err.contains("missing schema version"));
    }

    #[test]
    fn rejects_duplicate_chest_id() {
        let bad = concat!(
            "# schema_version: 1\n",
            "id\tmap\tx\ty\tregion\ttitle\that_id\n",
            "120001\tprt_fild01\t146\t126\t0\tTest\t0\n",
            "120001\tprt_fild02\t100\t100\t0\tTest 2\t0\n",
        );
        let err = ChestTable::parse(bad).unwrap_err();
        assert!(err.contains("duplicate chest id: 120001"));
    }

    #[test]
    fn rejects_duplicate_coordinates_on_same_map() {
        let bad = concat!(
            "# schema_version: 1\n",
            "id\tmap\tx\ty\tregion\ttitle\that_id\n",
            "120001\tprt_fild01\t146\t126\t0\tTest\t0\n",
            "120002\tprt_fild01\t146\t126\t0\tTest 2\t0\n",
        );
        let err = ChestTable::parse(bad).unwrap_err();
        assert!(err.contains("duplicate coordinate on map prt_fild01"));
    }
}
