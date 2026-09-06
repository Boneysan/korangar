fn main() -> Result<(), String> {
    // `composed` prints the body-vs-head geometry the 9/12/3 head-detach
    // investigation needs; the default run keeps the original attach dump.
    match std::env::args().nth(1).as_deref() {
        Some("composed") => korangar::playtest_audit::dump_composed_geometry(),
        Some("sweep") => korangar::playtest_audit::sweep_composed_geometry(),
        Some("attach") => korangar::playtest_audit::sweep_attach_points(),
        Some("acc-tables") => korangar::playtest_audit::list_accessory_tables(),
        Some("hat-attach") => korangar::playtest_audit::compare_headgear_attach(),
        Some("hat-lookup") => korangar::playtest_audit::verify_headgear_lookup(),
        _ => korangar::playtest_audit::run(),
    }
}
