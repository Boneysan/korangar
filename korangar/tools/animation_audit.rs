fn main() {
    let lines = korangar::audit_animation_structure().unwrap_or_else(|error| {
        eprintln!("animation structure audit failed: {error}");
        std::process::exit(2);
    });

    for line in &lines {
        println!("{line}");
    }

    let mismatches = lines.iter().filter(|line| line.contains("MISMATCH")).count();
    println!("== {mismatches} layer mismatches ==");
}
