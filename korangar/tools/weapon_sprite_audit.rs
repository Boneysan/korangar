fn main() {
    let report = korangar::audit_weapon_sprites().unwrap_or_else(|error| {
        eprintln!("weapon sprite audit failed: {error}");
        std::process::exit(2);
    });

    println!("== weapon sprite files listed by the archives ==");
    for path in &report.discovered {
        println!("  {path}");
    }

    println!("== per-item weapon sprites (numbered, archive listing lower bound) ==");
    println!("  count: {}", report.per_item.len());
    for path in report.per_item.iter().take(40) {
        println!("  {path}");
    }
    if report.per_item.len() > 40 {
        println!("  ... ({} more)", report.per_item.len() - 40);
    }

    println!("== _검광 sword-trail sprites (archive listing lower bound) ==");
    println!("  count: {}", report.trails.len());
    for path in report.trails.iter().take(40) {
        println!("  {path}");
    }
    if report.trails.len() > 40 {
        println!("  ... ({} more)", report.trails.len() - 40);
    }

    println!("== per-job probe results ==");
    for (folder, sex, found) in &report.probed {
        match found.is_empty() {
            true => println!("  {folder} ({sex}): none"),
            false => println!("  {folder} ({sex}): {}", found.join(", ")),
        }
    }
}
