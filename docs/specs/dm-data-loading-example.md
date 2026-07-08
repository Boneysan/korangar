# DM Data Loading Example (Build-time + Runtime)

**Related**: DM_DATA_GUIDE.md, specs/bestiary-journal.md, specs/dm-loot-generator.md.

## Build-time Preprocessing (Recommended)

In `korangar/build.rs`:

```rust
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell cargo to re-run if data changes
    println!("cargo:rerun-if-changed=docs/bestiary.json");
    println!("cargo:rerun-if-changed=docs/items.json");
    println!("cargo:rerun-if-changed=docs/cards.json");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("dm_data.rs");

    let bestiary = fs::read_to_string("docs/bestiary.json").unwrap();
    let items = fs::read_to_string("docs/items.json").unwrap();
    // ... cards too

    let code = format!(
        r#"
pub const BESTIARY_JSON: &str = r#"{bestiary}"#;
pub const ITEMS_JSON: &str = r#"{items}"#;
// ... 
"#
    );

    fs::write(dest, code).unwrap();
}
```

Then in `src/dm/data/mod.rs`:

```rust
include!(concat!(env!("OUT_DIR"), "/dm_data.rs"));

pub fn load_bestiary() -> Vec<serde_json::Value> {  // or proper structs
    serde_json::from_str(BESTIARY_JSON).expect("valid bestiary")
}
```

## Runtime Alternative (Simpler for now)

```rust
use std::fs;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct BestiaryMonster { /* fields */ }

pub fn load_from_docs() -> Vec<BestiaryMonster> {
    let s = fs::read_to_string("docs/bestiary.json").unwrap();
    serde_json::from_str(&s).unwrap()
}
```

**Recommendation**: Start with runtime for fast iteration. Switch to build.rs consts once stable.

This pattern keeps data versioned with the docs and easy to regenerate from Hercules.
