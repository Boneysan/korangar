fn main() {
    let report = korangar::audit_weapon_sprites().unwrap_or_else(|error| {
        eprintln!("weapon sprite audit failed: {error}");
        std::process::exit(2);
    });

    println!("== weapon sprite files listed by the archives ==");
    for path in &report.discovered {
        println!("  {path}");
    }

    println!("== per-job probe results ==");
    for (folder, sex, found) in &report.probed {
        match found.is_empty() {
            true => println!("  {folder} ({sex}): none"),
            false => println!("  {folder} ({sex}): {}", found.join(", ")),
        }
    }
}
