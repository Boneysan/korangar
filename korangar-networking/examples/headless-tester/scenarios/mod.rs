mod combat;
mod dialogue;
mod dm;
mod gm;
mod items;
mod movement;
mod observer;
mod session;
/// Generated from Hercules' own `skill_db.conf` — see
/// `tools/generate_skill_expectations.py`. Regenerate after any skill_db
/// change.
mod skill_expectations;
pub mod skills;
mod social;

use crate::context::Config;

/// Marker prefix for a scenario that could not run its assertions.
///
/// `Scenario::run` is `fn(&Config) -> Result<(), String>` and 114 functions
/// share that signature, so a third outcome as a return *type* would touch
/// every one of them. A skip is instead an `Err` whose message starts with this
/// prefix, which the runner classifies separately from a real failure.
///
/// **It is an `Err` and not an `Ok` on purpose.** The previous code returned
/// `Ok(())` for a skip, so a skipped scenario printed "skipped" and was still
/// tallied as PASS — which is how `skills-dancer` and `skills-gypsy` sat red
/// behind a green "114/114" for weeks. Putting a skip on the failure side means
/// the worst case is a visible row, never a false green. Only exact, reviewed
/// entries in [`EXPECTED_SKIPS`] avoid failing the gate.
pub const SKIPPED_PREFIX: &str = "SKIPPED: ";

/// Skips which are intentional for the checked-in server fixture.
///
/// A skip outside this list is a regression: it means a scenario silently
/// stopped asserting something. Reasons are exact-match on purpose so a changed
/// precondition has to be reviewed instead of inheriting an old exemption.
pub const EXPECTED_SKIPS: &[(&str, &str)] = &[("skills-novice", "Novice has no castable skills — its actives are quest-gated")];

/// Report that a scenario could not run — a precondition the harness cannot
/// establish, as opposed to a defect in the code under test.
pub fn skipped(reason: impl std::fmt::Display) -> Result<(), String> {
    Err(format!("{SKIPPED_PREFIX}{reason}"))
}

/// Whether a scenario result is a skip rather than a genuine failure.
pub fn is_skip(result: &Result<(), String>) -> bool {
    matches!(result, Err(message) if message.starts_with(SKIPPED_PREFIX))
}

/// Whether a scenario's skip reason exactly matches the reviewed baseline.
pub fn is_expected_skip(name: &str, result: &Result<(), String>) -> bool {
    let reason = match result {
        Err(message) => message.strip_prefix(SKIPPED_PREFIX),
        Ok(()) => None,
    };
    reason.is_some_and(|reason| EXPECTED_SKIPS.contains(&(name, reason)))
}

pub struct Scenario {
    pub name: &'static str,
    pub phase: u8,
    pub run: fn(&Config) -> Result<(), String>,
    /// Open findings-log entry this scenario is expected to fail on. Such a
    /// failure is reported as KNOWN-FAIL and does not affect the exit code;
    /// an unexpected PASS is flagged so the entry gets closed.
    pub known_issue: Option<&'static str>,
}

impl Scenario {
    pub const fn new(name: &'static str, phase: u8, run: fn(&Config) -> Result<(), String>) -> Self {
        Self {
            name,
            phase,
            run,
            known_issue: None,
        }
    }
}

pub fn all_scenarios() -> Vec<Scenario> {
    let mut scenarios = Vec::new();
    scenarios.extend(session::scenarios());
    scenarios.extend(gm::scenarios());
    scenarios.extend(movement::scenarios());
    scenarios.extend(combat::scenarios());
    scenarios.extend(skills::scenarios());
    scenarios.extend(items::scenarios());
    scenarios.extend(dialogue::scenarios());
    scenarios.extend(social::scenarios());
    scenarios.extend(dm::scenarios());
    scenarios.extend(observer::scenarios());
    scenarios
}

/// Public `NetworkingSystem` actions the suite claims to exercise, with at
/// least one scenario name (or an explicit exclusion reason).
///
/// **Why this exists.** The remaining-test design called for a coverage
/// manifest so new protocol APIs cannot land without either a scenario or a
/// stated exclusion. Without it, actions like `reject_trade` and
/// `cancel_item_identify` sat untested while the suite reported green.
///
/// When you add a public map/character action to `korangar-networking`, add a
/// row here. `"untested"` is not an acceptable exclusion.
const ACTION_COVERAGE: &[(&str, &str)] = &[
    // Session / connection
    ("connect_to_login_server", "smoke"),
    ("connect_to_character_server", "smoke"),
    ("connect_to_map_server", "smoke"),
    ("disconnect_from_login_server", "connection-state"),
    ("disconnect_from_character_server", "connection-state"),
    ("disconnect_from_map_server", "connection-state"),
    ("is_login_server_connected", "connection-state"),
    ("is_character_server_connected", "connection-state"),
    ("is_map_server_connected", "connection-state"),
    ("request_character_list", "smoke"),
    ("select_character", "character-select-invalid"),
    ("create_character", "character-create-delete"),
    ("delete_character", "character-create-delete"),
    ("switch_character_slot", "character-slot-switch"),
    ("map_loaded", "smoke"),
    ("request_client_tick", "tick-sync"),
    ("respawn", "respawn"),
    ("log_out", "logout-relogin"),
    // Movement / world
    ("player_move", "walk"),
    ("warp_to_map", "warp-crossmap"),
    ("entity_details", "entity-details"),
    ("player_sit", "sit-stand"),
    ("player_stand", "sit-stand"),
    // Combat / skills
    ("player_attack", "attack-kill"),
    ("cast_skill", "skills-mage"),
    ("cast_ground_skill", "skills-wizard"),
    ("cast_channeling_skill", "channeling-start-stop"),
    ("stop_channeling_skill", "channeling-start-stop"),
    ("cancel_cast", "cast-cancel"),
    ("select_warp_destination", "teleport-select"),
    ("cancel_warp_selection", "teleport-cancel"),
    ("select_auto_spell", "auto-spell-list"),
    ("request_weapon_refine", "weapon-refine-success"),
    ("cancel_weapon_refine", "weapon-refine-cancel"),
    ("request_item_repair", "repair-weapon-success"),
    ("cancel_item_repair", "repair-weapon-cancel"),
    ("level_up_skill", "stat-skill-points"),
    // Items / economy
    ("use_item", "use-drop-failures"),
    ("drop_item", "use-drop-failures"),
    ("pick_up_item", "drop-pickup"),
    ("request_item_equip", "equip-unequip"),
    ("request_item_unequip", "equip-unequip"),
    ("request_item_identify", "identify"),
    ("cancel_item_identify", "identify-cancel"),
    (
        "one_click_item_identify",
        "exclusion: not used by current Hercules fixture path",
    ),
    ("move_item_to_storage", "storage-persistence"),
    ("move_item_from_storage", "storage-persistence"),
    ("close_storage", "storage-persistence"),
    ("select_buy_or_sell", "shop-buy-sell"),
    ("purchase_items", "shop-buy-sell"),
    ("sell_items", "shop-buy-sell"),
    ("close_shop", "shop-close"),
    ("request_stat_up", "stat-skill-points"),
    ("set_hotkey_data", "hotkeys"),
    // Dialogue
    ("start_dialog", "dialogue-linear"),
    ("next_dialog", "dialogue-linear"),
    ("close_dialog", "dialogue-linear"),
    ("choose_dialog_option", "dialogue-choice"),
    ("submit_dialog_number", "dialogue-number"),
    ("submit_dialog_string", "dialogue-string"),
    // Social
    ("send_chat_message", "smoke"),
    ("send_party_chat_message", "party-lifecycle"),
    ("send_whisper_message", "whisper-emotion"),
    ("create_party", "party-lifecycle"),
    ("invite_to_party", "party-lifecycle"),
    ("accept_party_invite", "party-lifecycle"),
    ("reject_party_invite", "party-reject-block"),
    ("leave_party", "party-lifecycle"),
    ("kick_party_member", "party-kick"),
    ("change_party_leader", "party-promote-leader"),
    ("set_party_options", "party-share-options"),
    ("set_party_invitation_block", "party-reject-block"),
    ("set_player_ignored", "whisper-ignore"),
    ("set_all_ignored", "exclusion: client UI not wired; API only"),
    ("add_friend", "friend-lifecycle"),
    ("accept_friend_request", "friend-lifecycle"),
    ("reject_friend_request", "friend-reject"),
    ("remove_friend", "friend-lifecycle"),
    ("request_emotion", "whisper-emotion"),
    ("request_trade", "trade-commit"),
    ("accept_trade", "trade-commit"),
    ("reject_trade", "trade-reject"),
    ("trade_add_item", "trade-invalid-offers"),
    ("trade_add_zeny", "trade-invalid-offers"),
    ("trade_ok", "trade-commit"),
    ("trade_cancel", "trade-cancel"),
    ("trade_commit", "trade-commit"),
];

#[cfg(test)]
mod tests {
    use super::{ACTION_COVERAGE, all_scenarios, is_expected_skip, skipped};

    #[test]
    fn expected_skips_require_the_exact_reviewed_reason() {
        assert!(is_expected_skip(
            "skills-novice",
            &skipped("Novice has no castable skills — its actives are quest-gated")
        ));
        assert!(!is_expected_skip("skills-novice", &skipped("Novice has no castable skills")));
        assert!(!is_expected_skip(
            "skills-mage",
            &skipped("Novice has no castable skills — its actives are quest-gated")
        ));
    }

    #[test]
    fn action_coverage_points_at_registered_scenarios_or_exclusions() {
        let scenario_names: Vec<&str> = all_scenarios().iter().map(|scenario| scenario.name).collect();
        assert!(!ACTION_COVERAGE.is_empty());

        let mut seen_actions = std::collections::BTreeSet::new();
        for (action, coverage) in ACTION_COVERAGE {
            assert!(seen_actions.insert(*action), "duplicate action coverage row for {action}");
            if coverage.starts_with("exclusion:") {
                assert!(
                    coverage.len() > "exclusion:".len() + 3,
                    "exclusion for {action} needs a concrete reason"
                );
                assert!(
                    !coverage.contains("untested"),
                    "untested is not an acceptable exclusion for {action}"
                );
                continue;
            }
            assert!(
                scenario_names.contains(coverage),
                "action {action} claims scenario {coverage:?}, which is not registered"
            );
        }
    }

    #[test]
    fn suite_registers_at_least_the_known_scenario_count() {
        // Guard against accidental mass-deletion of scenarios. Bump when the
        // suite legitimately grows; do not lower it to silence a drop.
        assert!(
            all_scenarios().len() >= 148,
            "expected at least 148 scenarios, found {}",
            all_scenarios().len()
        );
    }
}
