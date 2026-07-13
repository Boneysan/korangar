fn main() {
    let (total, missing) = korangar::audit_entity_sprites().unwrap_or_else(|error| {
        eprintln!("entity sprite audit failed: {error}");
        std::process::exit(2);
    });

    println!("monster/NPC identities checked: {total}");
    println!("identities with missing sprite assets: {}", missing.len());
    for entry in &missing {
        let missing_parts = match (entry.missing_sprite, entry.missing_actions) {
            (true, true) => "SPR+ACT",
            (true, false) => "SPR",
            (false, true) => "ACT",
            (false, false) => unreachable!(),
        };
        println!(
            "  #{} {}\\{} missing {missing_parts}",
            entry.job_id.0, entry.folder, entry.sprite_name
        );
    }

    if !missing.is_empty() {
        std::process::exit(1);
    }
}
