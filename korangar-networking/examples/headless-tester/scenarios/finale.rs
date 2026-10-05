//! Phase 9 — the Arc 19 finale (planning/design/finale-test-script.md).
//!
//! The finale is the campaign's last and most consequential piece of logic: a
//! readiness matrix over five endings, explicit preparation, and a once-only
//! commit. These drive the real NPCs through real dialogue and judge the
//! outcome by the server's own flags (`@dmflag get`), never by wording alone.
//!
//! Dialogue is driven by a policy, not a fixed script: the board's menus loop
//! and the preparation flows list their affirmative option first, so a policy
//! that names the one thing to do and otherwise answers "yes" reaches the end
//! of every flow without encoding each page.

use std::time::Duration;

use korangar_networking::NetworkEvent;
use ragnarok_packets::{EntityId, ExperienceType};

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;
use crate::scenarios::dm::{say_expect, wait_for_text};
use crate::scenarios::social::{connect_pair, ensure_no_party, form_party, leave_party_both};

const RUINS: &str = "moc_ruins";
const BOARD: (u16, u16) = (154, 150);
const BOARD_STAND: (u16, u16) = (152, 150);
const FIELD: &str = "moc_fild22";
const CHOICE: (u16, u16) = (175, 140);
const CHOICE_STAND: (u16, u16) = (173, 140);

const ENDINGS: [&str; 5] = [
    "The Shared Seal",
    "The Reforged Seal",
    "The Queen's Bargain",
    "Thanatos's Road",
    "Ragnarok Unbound",
];
/// The flag each ending sets on commit, in `ENDINGS` order.
const FINALE_FLAGS: [&str; 5] = [
    "dm_finale_shared_seal",
    "dm_finale_reforged_seal",
    "dm_finale_queens_bargain",
    "dm_finale_thanatos_road",
    "dm_finale_ragnarok_unbound",
];

pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario::new("finale-readiness-and-preparation", 9, readiness_and_preparation),
        Scenario::new("campaign-reward-paid-once", 9, campaign_reward_paid_once),
        Scenario::new("finale-prerequisites-absent", 9, prerequisites_absent),
        Scenario::new("finale-commit-shared-seal", 9, commit_shared_seal),
        Scenario::new("finale-commit-reforged-seal", 9, commit_reforged_seal),
        Scenario::new("finale-commit-queens-bargain", 9, commit_queens_bargain),
        Scenario::new("finale-commit-thanatos-road", 9, commit_thanatos_road),
        Scenario::new("finale-commit-ragnarok-unbound", 9, commit_ragnarok_unbound),
        Scenario::new("finale-two-conversations-one-ending", 9, two_conversations_one_ending),
        Scenario::new("finale-volunteer-absent-then-present", 9, volunteer_absent_then_present),
        Scenario::new("finale-volunteer-lysandra", 9, volunteer_lysandra),
        Scenario::new("finale-resume-after-relogin", 9, resume_after_relogin),
        Scenario::new("finale-loki-briefing", 9, loki_briefing),
    ]
}

// ---- driving an NPC -------------------------------------------------------

/// Everything one conversation showed.
#[derive(Default)]
struct Talk {
    text: String,
    menus: Vec<Vec<String>>,
    /// Base-experience grants that arrived for our account during the
    /// conversation.
    base_exp: Vec<usize>,
    /// Quests added during the conversation.
    quests: Vec<u32>,
}

enum Step {
    Text(String),
    Next,
    Choices(Vec<String>),
    Close,
    BaseExp(usize),
    Quest(u32),
}

/// What to do at a menu.
enum Reply {
    Choose(usize),
    /// Leave the menu unanswered and hand control back (two conversations at
    /// once).
    Pause,
}

enum Drive {
    Closed,
    Paused,
}

/// Choices that decline, defer or leave. Everything else is treated as "yes".
const DECLINING: [&str; 10] = [
    "not now",
    "skip",
    "no.",
    "leave",
    "wait.",
    "return to",
    "we need to discuss",
    "hold it",
    "not me",
    "hold",
];

fn affirmative(choices: &[String]) -> Option<usize> {
    choices
        .iter()
        .position(|choice| {
            let lower = choice.to_lowercase();
            !DECLINING.iter().any(|declining| lower.starts_with(declining))
        })
        .map(|index| index + 1)
}

/// The 1-based index of the first choice containing `needle`
/// (case-insensitive).
fn pick(choices: &[String], needle: &str) -> Option<usize> {
    let needle = needle.to_lowercase();
    choices
        .iter()
        .position(|choice| choice.to_lowercase().contains(&needle))
        .map(|index| index + 1)
}

/// The board's own top-level menu is the one that opens with this.
fn is_board_menu(choices: &[String]) -> bool {
    choices.first().is_some_and(|choice| choice.starts_with("Show readiness"))
}

/// The Central Choice's own top-level menu opens with the first ending.
fn is_choice_menu(choices: &[String]) -> bool {
    choices.first().is_some_and(|choice| choice.starts_with(ENDINGS[0]))
}

fn is_confirm_menu(choices: &[String]) -> bool {
    choices.first().is_some_and(|choice| choice.starts_with("Confirm "))
}

/// Do one thing on the board, then leave: `first` names the main-menu option;
/// every sub-menu is answered "yes".
fn board_do(first: &'static str) -> impl FnMut(&[String]) -> Option<usize> {
    let mut done = false;
    move |choices| {
        if is_board_menu(choices) {
            if done {
                return pick(choices, "Leave");
            }
            done = true;
            return pick(choices, first);
        }
        affirmative(choices)
    }
}

/// Like [`board_do`], but every sub-menu is declined: the player is asked and
/// says no each time.
fn board_do_declining(first: &'static str) -> impl FnMut(&[String]) -> Option<usize> {
    let mut done = false;
    move |choices| {
        if is_board_menu(choices) {
            if done {
                return pick(choices, "Leave");
            }
            done = true;
            return pick(choices, first);
        }
        // The decline is the last option on every prompt the board offers.
        Some(choices.len())
    }
}

/// At the Central Choice: pick ending `ending` (0-based) once, then at its
/// confirmation either confirm or wait, and on coming back to the list ask to
/// return to preparation.
fn choose_ending(ending: usize, confirm: bool) -> impl FnMut(&[String]) -> Option<usize> {
    let mut picked = false;
    move |choices| {
        if is_choice_menu(choices) {
            if !picked {
                picked = true;
                return Some(ending + 1);
            }
            return pick(choices, "Return to preparation");
        }
        if is_confirm_menu(choices) {
            return if confirm { Some(1) } else { pick(choices, "Wait.") };
        }
        affirmative(choices)
    }
}

/// Talk to an NPC until it closes, answering each menu with `choose`.
fn talk(context: &mut TestContext, npc: EntityId, mut choose: impl FnMut(&[String]) -> Option<usize>) -> Result<Talk, String> {
    let mut talk = Talk::default();
    context.flush();
    context.net.start_dialog(npc).map_err(|_| "disconnected")?;
    let mut policy = |choices: &[String]| match choose(choices) {
        Some(option) => Ok(Reply::Choose(option)),
        None => Err(format!("the policy had no answer for the menu {choices:?}")),
    };
    match drive(context, npc, &mut talk, &mut policy)? {
        Drive::Closed => Ok(talk),
        Drive::Paused => Err("a plain conversation paused".to_owned()),
    }
}

/// Run the conversation forward until it closes, or until `choose` says to
/// pause.
fn drive(
    context: &mut TestContext,
    npc: EntityId,
    talk: &mut Talk,
    choose: &mut dyn FnMut(&[String]) -> Result<Reply, String>,
) -> Result<Drive, String> {
    let account = context.account_id.0;
    for _ in 0..160 {
        let step = context.wait_for("the next dialogue step", |event| match event {
            NetworkEvent::OpenDialog { npc_id, text } if *npc_id == npc => Some(Step::Text(text.clone())),
            NetworkEvent::AddNextButton { npc_id } if *npc_id == npc => Some(Step::Next),
            NetworkEvent::AddChoiceButtons { choices, npc_id } if *npc_id == npc => Some(Step::Choices(choices.clone())),
            NetworkEvent::AddCloseButton { npc_id } if *npc_id == npc => Some(Step::Close),
            NetworkEvent::GainedExperience {
                account_id,
                amount,
                experience_type: ExperienceType::BaseExperience,
                ..
            } if account_id.0 == account => Some(Step::BaseExp(*amount as usize)),
            NetworkEvent::QuestAdded { quest_id, .. } => Some(Step::Quest(*quest_id)),
            _ => None,
        })?;
        match step {
            Step::Text(text) => {
                talk.text.push_str(&text);
                talk.text.push('\n');
            }
            Step::Next => {
                context.net.next_dialog(npc).map_err(|_| "disconnected")?;
            }
            Step::Choices(choices) => {
                let reply = choose(&choices)?;
                talk.menus.push(choices);
                match reply {
                    Reply::Choose(option) => {
                        context.net.choose_dialog_option(npc, option as i8).map_err(|_| "disconnected")?;
                    }
                    Reply::Pause => return Ok(Drive::Paused),
                }
            }
            Step::Close => {
                context.net.close_dialog(npc).map_err(|_| "disconnected")?;
                return Ok(Drive::Closed);
            }
            Step::BaseExp(amount) => talk.base_exp.push(amount),
            Step::Quest(id) => talk.quests.push(id),
        }
    }
    // Say whether this was a long page or a loop: the last menus and the tail
    // of the text are what tell the two apart.
    let tail: String = talk.text.chars().rev().take(400).collect::<Vec<_>>().into_iter().rev().collect();
    let last_menus: Vec<_> = talk.menus.iter().rev().take(3).collect();
    Err(format!(
        "the dialogue did not finish within 160 steps ({} menus answered; last menus {last_menus:?}; text tail {tail:?})",
        talk.menus.len()
    ))
}

/// The NPC standing on (or beside) `cell`, from what the map has told us.
fn npc_at(context: &TestContext, cell: (u16, u16)) -> Result<EntityId, String> {
    context
        .entities
        .iter()
        .find(|(_, entity)| {
            let tile = entity.position.tile_position();
            tile.x == cell.0 && tile.y == cell.1
        })
        .map(|(id, _)| *id)
        .ok_or_else(|| format!("no NPC is visible at {cell:?}"))
}

fn go(context: &mut TestContext, map: &str, stand: (u16, u16), npc_cell: (u16, u16)) -> Result<EntityId, String> {
    context.warp(map, stand.0, stand.1)?;
    context.pump(Duration::from_millis(500));
    npc_at(context, npc_cell)
}

// ---- reading state --------------------------------------------------------

fn flag(context: &mut TestContext, name: &str) -> Result<i64, String> {
    context.flush();
    context.say(&format!("@dmflag get {name}"))?;
    let line = wait_for_text(context, &format!("value of {name}"), &format!("{name} = "))?;
    let value = line
        .split(&format!("{name} = "))
        .nth(1)
        .ok_or_else(|| format!("no value in {line:?}"))?;
    value
        .trim()
        .parse()
        .map_err(|error| format!("could not read {name} from {line:?}: {error}"))
}

fn expect_flag(context: &mut TestContext, name: &str, expected: i64) -> Result<(), String> {
    let actual = flag(context, name)?;
    if actual != expected {
        return Err(format!("{name} is {actual}, expected {expected}"));
    }
    Ok(())
}

/// The five status lines from a "Show readiness" conversation, in ending order.
fn readiness(talk: &Talk) -> Result<[String; 5], String> {
    let mut statuses: [String; 5] = Default::default();
    for (index, ending) in ENDINGS.iter().enumerate() {
        let prefix = format!("{ending}: ");
        let line = talk
            .text
            .lines()
            .find_map(|line| line.find(&prefix).map(|at| line[at + prefix.len()..].to_owned()))
            .ok_or_else(|| format!("no readiness line for {ending}: {:?}", talk.text))?;
        statuses[index] = line;
    }
    Ok(statuses)
}

fn status_kind(status: &str) -> &'static str {
    if status.starts_with("Ready.") {
        "Ready"
    } else if status.starts_with("Needs preparation") {
        "Needs preparation"
    } else if status.starts_with("Unavailable") {
        "Unavailable"
    } else {
        "unrecognised"
    }
}

fn expect_kinds(statuses: &[String; 5], expected: [&str; 5], when: &str) -> Result<(), String> {
    let actual: Vec<&str> = statuses.iter().map(|status| status_kind(status)).collect();
    if actual != expected {
        return Err(format!(
            "readiness {when}: expected {expected:?}, got {actual:?} ({statuses:?})"
        ));
    }
    Ok(())
}

// ---- session ----------------------------------------------------------------

/// A party of two, a clean campaign, DM mode on, and a preset applied.
fn begin(config: &Config, preset: &str) -> Result<(TestContext, TestContext), String> {
    let (mut primary, mut partner) = connect_pair(config)?;
    ensure_no_party(&mut primary);
    ensure_no_party(&mut partner);
    form_party(&mut primary, &mut partner)?;
    say_expect(&mut primary, "@dm reset confirm", "Campaign reset complete")?;
    say_expect(&mut primary, "@dm mode on", "Campaign NPCs are active")?;
    say_expect(&mut primary, &format!("@dmpreset {preset}"), "set")?;
    Ok((primary, partner))
}

fn finish(mut primary: TestContext, mut partner: TestContext) {
    let _ = primary.say("@dmpreset finale-reset");
    primary.pump(Duration::from_millis(200));
    let _ = primary.say("@dm reset confirm");
    primary.pump(Duration::from_millis(500));
    let _ = primary.say("@dm mode off");
    primary.pump(Duration::from_millis(200));
    leave_party_both(&mut primary, &mut partner);
}

// ---- scenarios --------------------------------------------------------------

/// A campaign reward pays once, however the player comes back for it.
///
/// Arc 1's Mira rescue pays EXP and 1,500 zeny behind `DM_ClaimGrant`. A
/// re-talk is refused by the quest state (20004 complete); the real test is
/// the latch, so the quest is then reopened -- a member whose quest state
/// lags, or a script path that forgets to check -- and the same choice made
/// again, then once more after a reconnect. Until Hercules 9a3752c6c the
/// zeny sat outside the latch and this paid twice. Finally `@dm reset` starts
/// a new run, in which the reward must pay again.
fn campaign_reward_paid_once(config: &Config) -> Result<(), String> {
    const QUEST: u32 = 20004;
    const ZENY: u32 = 1500;
    const MOTHER_STAND: (u16, u16) = (156, 42);
    const MOTHER: (u16, u16) = (156, 40);

    let (primary, partner) = begin(config, "finale-bare")?;
    let mut primary = Some(primary);
    let reopen_quest = |context: &mut TestContext| -> Result<(), String> {
        let _ = context.say(&format!("@dmquest erase {QUEST}"));
        context.pump(Duration::from_millis(300));
        context.flush();
        context.say(&format!("@dmquest start {QUEST}"))?;
        context.wait_for("quest 20004 active", |event| match event {
            NetworkEvent::QuestAdded { quest_id, active: true } if *quest_id == QUEST => Some(()),
            _ => None,
        })
    };
    let choose_rescue = |choices: &[String]| pick(choices, "Walk Mira home").or_else(|| affirmative(choices));

    let result: Result<(), String> = (|| {
        let context = primary.as_mut().ok_or("primary disconnected")?;
        say_expect(context, "@dmflag set dm_arc01_started 1", "set to 1")?;
        say_expect(context, "@dmflag set dm_arc01_child_found 1", "set to 1")?;
        reopen_quest(context)?;
        let mother = go(context, "prontera", MOTHER_STAND, MOTHER)?;

        // `context.zeny` reads 0 until the session sees a zeny update; on a
        // character holding 2M from earlier scenarios that read as "paid 2M".
        let before = context.known_zeny()?;
        talk(context, mother, choose_rescue)?;
        context.pump(Duration::from_millis(500));
        if context.zeny != before + ZENY {
            return Err(format!(
                "the rescue paid {} zeny, expected {ZENY}",
                context.zeny as i64 - before as i64
            ));
        }
        let paid = context.zeny;

        talk(context, mother, choose_rescue)?;
        context.pump(Duration::from_millis(500));
        if context.zeny != paid {
            return Err(format!("a plain re-talk paid again: {} -> {}", paid, context.zeny));
        }

        reopen_quest(context)?;
        talk(context, mother, choose_rescue)?;
        context.pump(Duration::from_millis(500));
        if context.zeny != paid {
            return Err(format!(
                "with the quest reopened, the same choice paid again: {} -> {}",
                paid, context.zeny
            ));
        }

        drop(primary.take());
        std::thread::sleep(Duration::from_millis(900));
        primary = Some(TestContext::connect(config)?);
        let context = primary.as_mut().ok_or("primary reconnect failed")?;
        context.pump(Duration::from_millis(500));
        // A fresh login does not report zeny until it next changes, so the
        // context would read 0 here. Nudge it by one to get the real total.
        context.say("@zeny 1")?;
        context.pump(Duration::from_millis(500));
        if context.zeny < paid {
            return Err(format!(
                "could not read zeny after the reconnect (read {}, had {paid})",
                context.zeny
            ));
        }
        let after_relog = context.zeny;
        reopen_quest(context)?;
        let mother = go(context, "prontera", MOTHER_STAND, MOTHER)?;
        talk(context, mother, choose_rescue)?;
        context.pump(Duration::from_millis(500));
        if context.zeny != after_relog {
            return Err(format!(
                "after a reconnect the same choice paid again: {} -> {}",
                after_relog, context.zeny
            ));
        }

        // A reset starts a new run: the same reward must be earnable again.
        // Until the reset cleared the claim latches, a replayed campaign paid
        // nothing for any beat paid in the previous run.
        say_expect(context, "@dm reset confirm", "Campaign reset complete")?;
        say_expect(context, "@dm mode on", "Campaign NPCs are active")?;
        say_expect(context, "@dmflag set dm_arc01_started 1", "set to 1")?;
        say_expect(context, "@dmflag set dm_arc01_child_found 1", "set to 1")?;
        reopen_quest(context)?;
        let before_new_run = context.zeny;
        talk(context, mother, choose_rescue)?;
        context.pump(Duration::from_millis(500));
        if context.zeny != before_new_run + ZENY {
            return Err(format!(
                "after @dm reset the rescue paid {} zeny in the new run, expected {ZENY}",
                context.zeny as i64 - before_new_run as i64
            ));
        }
        Ok(())
    })();

    if let Some(primary) = primary {
        finish(primary, partner);
    }
    result
}

/// Tests A and B of the runbook: in a bare world only Unbound can be prepared;
/// in a generous world nothing is Ready until the party prepares it; each
/// preparation sets exactly its own bit; and a party that declines every
/// question prepares nothing.
fn readiness_and_preparation(config: &Config) -> Result<(), String> {
    let (mut primary, partner) = begin(config, "finale-bare")?;
    let result: Result<(), String> = (|| {
        let board = go(&mut primary, RUINS, BOARD_STAND, BOARD)?;

        // A: the bare world. Four endings need work; Unbound is reachable.
        let talk_a = talk(&mut primary, board, board_do("Show readiness"))?;
        let statuses = readiness(&talk_a)?;
        expect_kinds(&statuses, ["Needs preparation"; 5], "in a bare world")?;
        expect_flag(&mut primary, "dm_arc19_prepared_mask", 0)?;

        talk(&mut primary, board, board_do("Prepare Ragnarok Unbound"))?;
        expect_flag(&mut primary, "dm_arc19_prepared_mask", 16)?;
        let statuses = readiness(&talk(&mut primary, board, board_do("Show readiness"))?)?;
        expect_kinds(
            &statuses,
            [
                "Needs preparation",
                "Needs preparation",
                "Needs preparation",
                "Needs preparation",
                "Ready",
            ],
            "after preparing Unbound",
        )?;

        // B: the generous world. History is set, but nothing is prepared yet.
        say_expect(&mut primary, "@dmpreset finale-all", "set")?;
        let statuses = readiness(&talk(&mut primary, board, board_do("Show readiness"))?)?;
        expect_kinds(
            &statuses,
            ["Needs preparation"; 5],
            "in a generous world before any preparation",
        )?;
        expect_flag(&mut primary, "dm_arc19_prepared_mask", 0)?;

        // Skipping every group leaves the Shared Seal unprepared, with the count named.
        talk(&mut primary, board, board_do_declining("Prepare The Shared Seal"))?;
        expect_flag(&mut primary, "dm_arc19_prepared_mask", 0)?;
        let statuses = readiness(&talk(&mut primary, board, board_do("Show readiness"))?)?;
        if !statuses[0].contains("(0 so far)") {
            return Err(format!(
                "declining every group should leave 0 agreed, but the Shared Seal reads: {:?}",
                statuses[0]
            ));
        }

        // Prepare each in turn; the mask gains exactly one bit each time.
        let mut mask = 0;
        for (bit, name) in [
            (1, "Prepare The Shared Seal"),
            (2, "Prepare The Reforged Seal"),
            (4, "Prepare The Queen's Bargain"),
            (8, "Prepare Thanatos's Road"),
            (16, "Prepare Ragnarok Unbound"),
        ] {
            talk(&mut primary, board, board_do(name))?;
            mask |= bit;
            expect_flag(&mut primary, "dm_arc19_prepared_mask", mask)?;
        }
        let statuses = readiness(&talk(&mut primary, board, board_do("Show readiness"))?)?;
        expect_kinds(&statuses, ["Ready"; 5], "with all five prepared")?;
        Ok(())
    })();
    finish(primary, partner);
    result
}

// ---- the rest of the runbook
// ------------------------------------------------------

const BOARD_OPTIONS: [&str; 5] = [
    "Prepare The Shared Seal",
    "Prepare The Reforged Seal",
    "Prepare The Queen's Bargain",
    "Prepare Thanatos's Road",
    "Prepare Ragnarok Unbound",
];

/// Prepare all five endings as the speaking character (who is therefore Road's
/// volunteer).
fn prepare_everything(primary: &mut TestContext, board: EntityId) -> Result<(), String> {
    for option in BOARD_OPTIONS {
        talk(primary, board, board_do(option))?;
    }
    expect_flag(primary, "dm_arc19_prepared_mask", 31)
}

fn expect_only_finale_flag(context: &mut TestContext, only: Option<usize>) -> Result<(), String> {
    for (index, name) in FINALE_FLAGS.iter().enumerate() {
        expect_flag(context, name, i64::from(Some(index) == only))?;
    }
    Ok(())
}

/// Poll a flag until it has `bit` set, for at most `seconds`: a flag set by the
/// other member reaches this character when the party's next push is applied.
fn wait_for_bit(context: &mut TestContext, name: &str, bit: i64, seconds: u64) -> Result<(), String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(seconds);
    loop {
        if flag(context, name)? & bit == bit {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(format!("{name} never gained bit {bit} on this character within {seconds} s"));
        }
        context.pump(Duration::from_millis(700));
    }
}

/// Run one "a prerequisite is missing" case from a fresh preset (runbook test C
/// and D).
// Eight arguments: each names one axis of the prerequisite matrix, and a struct
// would only rename them at every call site.
#[allow(clippy::too_many_arguments)]
fn prerequisite_case(
    primary: &mut TestContext,
    board: EntityId,
    preset: &str,
    setup: &[&str],
    option: &'static str,
    says: &str,
    bit: i64,
    prepared_afterwards: bool,
) -> Result<(), String> {
    let case = format!("[{preset} {setup:?} -> {option}]");
    say_expect(primary, &format!("@dmpreset {preset}"), "set")?;
    for command in setup {
        say_expect(primary, command, "cleared")?;
    }
    expect_flag(primary, "dm_arc19_prepared_mask", 0)?;
    let said = talk(primary, board, board_do(option))?;
    if !said.text.contains(says) {
        return Err(format!("{case}: the board did not say {says:?}; it said {:?}", said.text));
    }
    let mask = flag(primary, "dm_arc19_prepared_mask")?;
    if (mask & bit != 0) != prepared_afterwards {
        return Err(format!(
            "{case}: the preparation bit {bit} is {} but should be {}",
            mask & bit != 0,
            prepared_afterwards
        ));
    }
    Ok(())
}

/// Tests C and D: each missing prerequisite blocks (or is offered to be
/// satisfied), a fallen Queen is Unavailable and cannot be prepared, and a
/// contradictory record is refused rather than guessed at.
fn prerequisites_absent(config: &Config) -> Result<(), String> {
    let (mut primary, partner) = begin(config, "finale-all")?;
    let result: Result<(), String> = (|| {
        let board = go(&mut primary, RUINS, BOARD_STAND, BOARD)?;
        let all = "finale-all";

        // Not preparable from the board.
        prerequisite_case(
            &mut primary,
            board,
            all,
            &["@dmflag clear dm_arc17_prototype_verified"],
            "Prepare The Reforged Seal",
            "never verified",
            2,
            false,
        )?;
        prerequisite_case(
            &mut primary,
            board,
            all,
            &["@dmflag clear dm_arc18_pact_valid"],
            "Prepare The Queen's Bargain",
            "No pact has been agreed",
            4,
            false,
        )?;
        prerequisite_case(
            &mut primary,
            board,
            "finale-no-cost",
            &[],
            "Prepare Thanatos's Road",
            "cost has not been disclosed",
            8,
            false,
        )?;

        // Offered, and preparable once the player agrees to do it.
        prerequisite_case(
            &mut primary,
            board,
            all,
            &["@dmflag clear dm_arc17_design_exported"],
            "Prepare The Reforged Seal",
            "design is not exported",
            2,
            true,
        )?;
        prerequisite_case(
            &mut primary,
            board,
            all,
            &["@dmflag clear dm_arc17_maintenance_secured"],
            "Prepare The Reforged Seal",
            "ask a team of engineers",
            2,
            true,
        )?;

        // The Queen, killed: Unavailable, and not restorable from a menu.
        prerequisite_case(
            &mut primary,
            board,
            "finale-queen-dead",
            &[],
            "Prepare The Queen's Bargain",
            "Himmelmez fell",
            4,
            false,
        )?;
        let statuses = readiness(&talk(&mut primary, board, board_do("Show readiness"))?)?;
        if !statuses[2].starts_with("Unavailable: Himmelmez fell") {
            return Err(format!("with the Queen dead her ending reads {:?}", statuses[2]));
        }

        // A record that says both "pact valid" and "killed" is flagged, never read as
        // Ready.
        say_expect(&mut primary, "@dmpreset finale-contradiction", "set")?;
        let statuses = readiness(&talk(&mut primary, board, board_do("Show readiness"))?)?;
        if !statuses[2].contains("contradictory") {
            return Err(format!("a contradictory record reads {:?}", statuses[2]));
        }
        talk(&mut primary, board, board_do("Prepare The Queen's Bargain"))?;
        let mask = flag(&mut primary, "dm_arc19_prepared_mask")?;
        if mask & 4 != 0 {
            return Err("a contradictory Queen record was prepared".to_owned());
        }
        Ok(())
    })();
    finish(primary, partner);
    result
}

/// Tests E and F for one ending: look and wait (nothing commits), then confirm
/// (exactly that ending is set, the campaign is complete, EXP lands once), then
/// a repeat visit (nothing changes, nothing is granted again).
fn commit_case(config: &Config, ending: usize) -> Result<(), String> {
    let (mut primary, partner) = begin(config, "finale-all")?;
    let result: Result<(), String> = (|| {
        let board = go(&mut primary, RUINS, BOARD_STAND, BOARD)?;
        prepare_everything(&mut primary, board)?;
        say_expect(&mut primary, "@dmflag set dm_arc19_surt_defeated 1", "set to 1")?;
        let choice = go(&mut primary, FIELD, CHOICE_STAND, CHOICE)?;

        // 1. The cost is shown BEFORE the question, and "wait" commits nothing.
        let looked = talk(&mut primary, choice, choose_ending(ending, false))?;
        if !looked.text.contains("Cost: ") {
            return Err(format!(
                "the cost of {} was not shown before the question: {:?}",
                ENDINGS[ending], looked.text
            ));
        }
        expect_flag(&mut primary, "dm_arc19_final_choice", 0)?;
        expect_flag(&mut primary, "dm_campaign_complete", 0)?;
        expect_only_finale_flag(&mut primary, None)?;
        if !looked.base_exp.is_empty() {
            return Err(format!(
                "experience was granted before anything was confirmed: {:?}",
                looked.base_exp
            ));
        }

        // 2. Confirm.
        let done = talk(&mut primary, choice, choose_ending(ending, true))?;
        if !done.text.contains("The Seal Cascade ends") {
            return Err(format!("the ending did not play out: {:?}", done.text));
        }
        expect_flag(&mut primary, "dm_arc19_final_choice", ending as i64 + 1)?;
        expect_only_finale_flag(&mut primary, Some(ending))?;
        expect_flag(&mut primary, "dm_campaign_complete", 1)?;
        let big: Vec<_> = done.base_exp.iter().filter(|amount| **amount >= 1_000_000).collect();
        if big.len() != 1 {
            return Err(format!(
                "the finale's 1,000,000 base EXP arrived {} time(s): {:?}",
                big.len(),
                done.base_exp
            ));
        }

        // 3. A second visit changes nothing and grants nothing.
        let again = talk(&mut primary, choice, affirmative)?;
        if !again.menus.is_empty() {
            return Err(format!(
                "after the choice was made the NPC still offered menus: {:?}",
                again.menus
            ));
        }
        if !again.text.contains("The choice was made") && !again.text.contains("You chose this") {
            return Err(format!("a repeat visit did not say the choice was made: {:?}", again.text));
        }
        if !again.base_exp.is_empty() {
            return Err(format!("a repeat visit granted experience again: {:?}", again.base_exp));
        }
        expect_flag(&mut primary, "dm_arc19_final_choice", ending as i64 + 1)?;
        expect_only_finale_flag(&mut primary, Some(ending))?;
        Ok(())
    })();
    finish(primary, partner);
    result
}

fn commit_shared_seal(config: &Config) -> Result<(), String> {
    commit_case(config, 0)
}
fn commit_reforged_seal(config: &Config) -> Result<(), String> {
    commit_case(config, 1)
}
fn commit_queens_bargain(config: &Config) -> Result<(), String> {
    commit_case(config, 2)
}
fn commit_thanatos_road(config: &Config) -> Result<(), String> {
    commit_case(config, 3)
}
fn commit_ragnarok_unbound(config: &Config) -> Result<(), String> {
    commit_case(config, 4)
}

/// Pause at the confirmation prompt: pick `ending`, then stop before answering
/// it.
fn pause_at_confirm(ending: usize) -> impl FnMut(&[String]) -> Result<Reply, String> {
    let mut picked = false;
    move |choices| {
        if is_choice_menu(choices) {
            if picked {
                return pick(choices, "Return to preparation")
                    .map(Reply::Choose)
                    .ok_or_else(|| "no way back to preparation".to_owned());
            }
            picked = true;
            return Ok(Reply::Choose(ending + 1));
        }
        if is_confirm_menu(choices) {
            return Ok(Reply::Pause);
        }
        affirmative(choices)
            .map(Reply::Choose)
            .ok_or_else(|| format!("no answer for {choices:?}"))
    }
}

fn say_yes() -> impl FnMut(&[String]) -> Result<Reply, String> {
    |choices| {
        affirmative(choices)
            .map(Reply::Choose)
            .ok_or_else(|| format!("no answer for {choices:?}"))
    }
}

/// Test G: two members stand at the Central Choice, each at the "Confirm"
/// prompt for a DIFFERENT ending. The first to confirm commits; the second is
/// told only one is given.
fn two_conversations_one_ending(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = begin(config, "finale-all")?;
    let result: Result<(), String> = (|| {
        let board = go(&mut primary, RUINS, BOARD_STAND, BOARD)?;
        prepare_everything(&mut primary, board)?;
        say_expect(&mut primary, "@dmflag set dm_arc19_surt_defeated 1", "set to 1")?;
        say_expect(&mut primary, "@dm catchup", "Catch-up ran")?;
        // The partner must hold the same state, or the race proves nothing.
        wait_for_bit(&mut partner, "dm_arc19_prepared_mask", 31, 10)?;
        wait_for_bit(&mut partner, "dm_arc19_surt_defeated", 1, 10)?;

        let a_npc = go(&mut primary, FIELD, CHOICE_STAND, CHOICE)?;
        let b_npc = go(&mut partner, FIELD, (CHOICE_STAND.0, CHOICE_STAND.1 + 1), CHOICE)?;

        let (mut a, mut b) = (Talk::default(), Talk::default());
        primary.flush();
        primary.net.start_dialog(a_npc).map_err(|_| "disconnected")?;
        if !matches!(drive(&mut primary, a_npc, &mut a, &mut pause_at_confirm(0))?, Drive::Paused) {
            return Err("A's conversation closed before reaching the confirmation".to_owned());
        }
        partner.flush();
        partner.net.start_dialog(b_npc).map_err(|_| "disconnected")?;
        if !matches!(drive(&mut partner, b_npc, &mut b, &mut pause_at_confirm(1))?, Drive::Paused) {
            return Err("B's conversation closed before reaching the confirmation".to_owned());
        }

        // A confirms the Shared Seal, then B confirms the Reforged Seal.
        primary.net.choose_dialog_option(a_npc, 1).map_err(|_| "disconnected")?;
        drive(&mut primary, a_npc, &mut a, &mut say_yes())?;
        partner.net.choose_dialog_option(b_npc, 1).map_err(|_| "disconnected")?;
        drive(&mut partner, b_npc, &mut b, &mut say_yes())?;

        if !a.text.contains("The Seal Cascade ends") {
            return Err(format!("the first to confirm did not get their ending: {:?}", a.text));
        }
        if !b.text.contains("Only one is given") {
            return Err(format!("the second to confirm was not told only one is given: {:?}", b.text));
        }
        if b.text.contains("The Seal Cascade ends") {
            return Err("both conversations played an ending".to_owned());
        }
        expect_flag(&mut primary, "dm_arc19_final_choice", 1)?;
        expect_only_finale_flag(&mut primary, Some(0))?;
        Ok(())
    })();
    finish(primary, partner);
    result
}

/// Test H: the volunteer for Thanatos's Road is the partner. With them logged
/// out the choice pauses and nothing is spent; when they return it commits
/// normally. The volunteer flag is theirs alone.
fn volunteer_absent_then_present(config: &Config) -> Result<(), String> {
    let (mut primary, partner) = begin(config, "finale-all")?;
    let mut partner = Some(partner);
    let result: Result<(), String> = (|| {
        say_expect(&mut primary, "@dm catchup", "Catch-up ran")?;
        {
            let other = partner.as_mut().ok_or("partner gone")?;
            wait_for_bit(other, "dm_arc15_cost_disclosed", 1, 10)?;

            // The partner is the one who volunteers.
            let board = go(other, RUINS, BOARD_STAND, BOARD)?;
            talk(other, board, board_do("Prepare Thanatos's Road"))?;
            expect_flag(other, "dm_finale_road_volunteer", 1)?;
        }
        wait_for_bit(&mut primary, "dm_arc19_prepared_mask", 8, 10)?;

        // The primary prepares the other four.
        let board = go(&mut primary, RUINS, BOARD_STAND, BOARD)?;
        for option in [BOARD_OPTIONS[0], BOARD_OPTIONS[1], BOARD_OPTIONS[2], BOARD_OPTIONS[4]] {
            talk(&mut primary, board, board_do(option))?;
        }
        expect_flag(&mut primary, "dm_arc19_prepared_mask", 31)?;
        say_expect(&mut primary, "@dmflag set dm_arc19_surt_defeated 1", "set to 1")?;
        say_expect(&mut primary, "@dm catchup", "Catch-up ran")?;

        // The volunteer logs out.
        drop(partner.take().ok_or("partner gone")?);
        std::thread::sleep(Duration::from_millis(1200));

        let choice = go(&mut primary, FIELD, CHOICE_STAND, CHOICE)?;
        let paused = talk(&mut primary, choice, choose_ending(3, true))?;
        if !paused.text.contains("Your volunteer is not here") {
            return Err(format!(
                "with the volunteer offline the choice did not pause: {:?}",
                paused.text
            ));
        }
        expect_flag(&mut primary, "dm_arc19_final_choice", 0)?;
        expect_only_finale_flag(&mut primary, None)?;
        expect_flag(&mut primary, "dm_campaign_complete", 0)?;

        // The volunteer returns; nothing was consumed, so it now commits.
        let mut returned = TestContext::connect_partner(config)?;
        returned.pump(Duration::from_millis(1500));
        let committed = talk(&mut primary, choice, choose_ending(3, true))?;
        if !committed.text.contains("The Seal Cascade ends") {
            return Err(format!("with the volunteer back the Road did not commit: {:?}", committed.text));
        }
        expect_flag(&mut primary, "dm_arc19_final_choice", 4)?;
        expect_only_finale_flag(&mut primary, Some(3))?;
        // The volunteer flag belongs to the volunteer and never to the rest of the
        // party.
        expect_flag(&mut returned, "dm_finale_road_volunteer", 1)?;
        expect_flag(&mut primary, "dm_finale_road_volunteer", 0)?;
        partner = Some(returned);
        Ok(())
    })();
    match partner {
        Some(partner) => finish(primary, partner),
        None => {
            let _ = primary.say("@dm reset confirm");
            primary.pump(Duration::from_millis(300));
        }
    }
    result
}

/// The volunteer menu offers Keeper Lysandra as an alternative to a party
/// member.
fn board_do_lysandra() -> impl FnMut(&[String]) -> Option<usize> {
    let mut done = false;
    move |choices| {
        if is_board_menu(choices) {
            if done {
                return pick(choices, "Leave");
            }
            done = true;
            return pick(choices, "Prepare Thanatos's Road");
        }
        if choices
            .first()
            .is_some_and(|choice| choice.starts_with("A member of this party volunteers"))
        {
            return pick(choices, "Keeper Lysandra offers");
        }
        affirmative(choices)
    }
}

/// Variant of test H: Keeper Lysandra is recorded as the volunteer, so nobody
/// has to be online for the Road, the epilogue names her, and no party member
/// carries the flag.
fn volunteer_lysandra(config: &Config) -> Result<(), String> {
    let (mut primary, partner) = begin(config, "finale-all")?;
    let result: Result<(), String> = (|| {
        let board = go(&mut primary, RUINS, BOARD_STAND, BOARD)?;
        for option in [BOARD_OPTIONS[0], BOARD_OPTIONS[1], BOARD_OPTIONS[2], BOARD_OPTIONS[4]] {
            talk(&mut primary, board, board_do(option))?;
        }
        talk(&mut primary, board, board_do_lysandra())?;
        expect_flag(&mut primary, "dm_arc19_prepared_mask", 31)?;
        expect_flag(&mut primary, "dm_arc19_volunteer_kind", 2)?;
        expect_flag(&mut primary, "dm_finale_road_volunteer", 0)?;
        say_expect(&mut primary, "@dmflag set dm_arc19_surt_defeated 1", "set to 1")?;

        let choice = go(&mut primary, FIELD, CHOICE_STAND, CHOICE)?;
        let done = talk(&mut primary, choice, choose_ending(3, true))?;
        if !done.text.contains("The Seal Cascade ends") {
            return Err(format!("the Road with Lysandra did not commit: {:?}", done.text));
        }
        if !done.text.contains("Lysandra") {
            return Err(format!("the epilogue does not name the volunteer: {:?}", done.text));
        }
        expect_flag(&mut primary, "dm_arc19_final_choice", 4)?;
        expect_flag(&mut primary, "dm_finale_road_volunteer", 0)?;
        Ok(())
    })();
    finish(primary, partner);
    result
}

/// Test I: both characters log out and back in after Surt, before the choice.
/// The preparation, the defeat and the readiness all survive, and the choice
/// still works.
fn resume_after_relogin(config: &Config) -> Result<(), String> {
    let (mut primary, partner) = begin(config, "finale-all")?;
    let board = go(&mut primary, RUINS, BOARD_STAND, BOARD)?;
    let prepared: Result<(), String> = (|| {
        prepare_everything(&mut primary, board)?;
        say_expect(&mut primary, "@dmflag set dm_arc19_surt_defeated 1", "set to 1")?;
        say_expect(&mut primary, "@dm catchup", "Catch-up ran")?;
        Ok(())
    })();
    if let Err(error) = prepared {
        finish(primary, partner);
        return Err(error);
    }
    drop(primary);
    drop(partner);
    std::thread::sleep(Duration::from_millis(1500));

    let (mut primary, partner) = connect_pair(config)?;
    let result: Result<(), String> = (|| {
        expect_flag(&mut primary, "dm_arc19_prepared_mask", 31)?;
        expect_flag(&mut primary, "dm_arc19_surt_defeated", 1)?;
        expect_flag(&mut primary, "dm_arc19_final_choice", 0)?;
        let choice = go(&mut primary, FIELD, CHOICE_STAND, CHOICE)?;
        let looked = talk(&mut primary, choice, |choices| {
            if is_choice_menu(choices) {
                pick(choices, "Return to preparation")
            } else {
                affirmative(choices)
            }
        })?;
        expect_kinds(&readiness(&looked)?, ["Ready"; 5], "after relogging in")?;
        expect_only_finale_flag(&mut primary, None)?;
        Ok(())
    })();
    finish(primary, partner);
    result
}

/// Test J: Loki's required briefing is short and carries no hunt turn-in; the
/// long history appears only if asked for; the allies page says so plainly in a
/// bare world.
fn loki_briefing(config: &Config) -> Result<(), String> {
    const JOURNEY: &str = "You made most of those decisions early";
    const NO_ALLIES: &str = "No one has come to speak";

    let (mut primary, partner) = begin(config, "finale-bare")?;
    let result: Result<(), String> = (|| {
        say_expect(&mut primary, "@dmflag set dm_arc19_started 0", "set to 0")?;
        let loki = go(&mut primary, RUINS, (148, 150), (150, 150))?;

        // The required account: no journey, no allies, no hunt turn-in.
        // The audience menu loops until "We are ready" (Hercules f6974e0ad, so
        // the journey and allies pages can be reread); the later progress menu
        // ends on "Nothing more". Taking the first option looped forever.
        let first = talk(&mut primary, loki, |choices| {
            pick(choices, "We are ready")
                .or_else(|| pick(choices, "Nothing more"))
                .or_else(|| affirmative(choices))
        })?;
        expect_flag(&mut primary, "dm_arc19_started", 1)?;
        if first.text.contains(JOURNEY) || first.text.contains(NO_ALLIES) {
            return Err(format!("the required briefing included an optional page: {:?}", first.text));
        }
        if first.quests.contains(&20232) {
            return Err("Loki handed out the retired hunt turn-in (quest 20232)".to_owned());
        }

        // Asked for, the allies page is honest about an empty world.
        let mut asked_allies = false;
        let allies = talk(&mut primary, loki, |choices| {
            if !asked_allies && let Some(option) = pick(choices, "Hear from our allies") {
                asked_allies = true;
                return Some(option);
            }
            if pick(choices, "Nothing more").is_some() {
                return pick(choices, "Nothing more");
            }
            pick(choices, "We are ready").or_else(|| affirmative(choices))
        })?;
        if !allies.text.contains(NO_ALLIES) {
            return Err(format!("a bare world's allies page did not say no one came: {:?}", allies.text));
        }

        // Asked for, the journey appears.
        let mut asked_journey = false;
        let journey = talk(&mut primary, loki, |choices| {
            if !asked_journey && let Some(option) = pick(choices, "Review our journey") {
                asked_journey = true;
                return Some(option);
            }
            if pick(choices, "Nothing more").is_some() {
                return pick(choices, "Nothing more");
            }
            pick(choices, "We are ready").or_else(|| affirmative(choices))
        })?;
        if !journey.text.contains(JOURNEY) {
            return Err(format!("asking for the journey did not show it: {:?}", journey.text));
        }
        if journey.quests.contains(&20232) {
            return Err("Loki handed out the retired hunt turn-in (quest 20232)".to_owned());
        }
        Ok(())
    })();
    finish(primary, partner);
    result
}
