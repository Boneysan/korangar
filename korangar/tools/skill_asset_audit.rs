fn main() {
    let (total, missing) = korangar::audit_skill_assets().unwrap_or_else(|error| {
        eprintln!("skill asset audit failed: {error}");
        std::process::exit(2);
    });

    println!("catalogued skills checked: {total}");
    println!("skills with missing icon assets: {}", missing.len());
    for asset in &missing {
        let missing_parts = match (asset.missing_sprite, asset.missing_actions) {
            (true, true) => "SPR+ACT",
            (true, false) => "SPR",
            (false, true) => "ACT",
            (false, false) => unreachable!(),
        };
        println!(
            "  #{} {} [{}] missing {missing_parts}",
            asset.skill_id.0, asset.skill_name, asset.file_name
        );
    }

    if !missing.is_empty() {
        std::process::exit(1);
    }
}
