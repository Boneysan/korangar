//! The shared crafting commission board (GDD F31): Hercules
//! `npc/custom/commission_board.txt` against two real clients.
//!
//! The client-side unit tests pin the `[CMB]` format with hand-written lines.
//! Only this proves the server sends that format, that a request is seen by
//! a *different* player (the whole point: the 2026-10-03 board was local to
//! one client), and that only the poster can change it.

use korangar_networking::{MessageColor, NetworkEvent};
use serde_json::Value;

use crate::context::{Config, TestContext};
use crate::scenarios::Scenario;

pub fn scenarios() -> Vec<Scenario> {
    vec![Scenario::new("commission-board-shared", 8, commission_board_shared)]
}

/// What one `@commission` command returned: the plain result lines, then
/// the `[CMB]` rows of the list that follows every command.
struct Reply {
    notes: Vec<String>,
    rows: Vec<Value>,
}

impl Reply {
    fn row(&self, id: u64) -> Option<&Value> {
        self.rows.iter().find(|row| row["id"].as_u64() == Some(id))
    }

    fn says(&self, needle: &str) -> bool {
        self.notes.iter().any(|note| note.contains(needle))
    }
}

fn command(context: &mut TestContext, text: &str) -> Result<Reply, String> {
    context.flush();
    context.say(text)?;
    let mut notes = Vec::new();
    let mut rows = Vec::new();
    loop {
        let line = context.wait_for(&format!("the [CMB] list after {text}"), |event| match event {
            NetworkEvent::ChatMessage {
                text,
                color: MessageColor::Server,
            } => Some(text.clone()),
            _ => None,
        })?;
        let Some(body) = line.strip_prefix("[CMB]") else {
            notes.push(line);
            continue;
        };
        let message: Value = serde_json::from_str(body).map_err(|error| format!("unreadable [CMB] line {body:?}: {error}"))?;
        if message["v"].as_u64() != Some(1) {
            return Err(format!("[CMB] line with an unexpected version: {body}"));
        }
        match message["t"].as_str() {
            Some("begin") => rows.clear(),
            Some("row") => rows.push(message),
            Some("end") => return Ok(Reply { notes, rows }),
            other => return Err(format!("unknown [CMB] type {other:?}: {body}")),
        }
    }
}

/// Cancel whatever this character left open in an earlier, failed run: the
/// server allows five open requests, and test characters are shared.
fn cancel_own(context: &mut TestContext) -> Result<(), String> {
    let board = command(context, "@commission list")?;
    for row in board.rows.iter().filter(|row| row["mine"] == 1) {
        command(context, &format!("@commission cancel {}", row["id"]))?;
    }
    Ok(())
}

fn commission_board_shared(config: &Config) -> Result<(), String> {
    let (mut primary, mut partner) = TestContext::connect_pair(config)?;
    cancel_own(&mut primary)?;
    cancel_own(&mut partner)?;
    let partner_name = partner.character_name.clone();

    // 1. Post. The poster's own list marks it theirs.
    partner.flush();
    let posted = command(&mut primary, "@commission post 50000 Fire Damascus")?;
    let row = posted
        .rows
        .iter()
        .find(|row| row["item"] == "Fire Damascus" && row["mine"] == 1)
        .ok_or_else(|| {
            format!(
                "the poster's list lacks the new request: {:?} / {:?}",
                posted.notes, posted.rows
            )
        })?;
    let id = row["id"].as_u64().ok_or("row without an id")?;
    if row["fee"] != 50000 || row["status"] != "open" || row["by"] != primary.character_name.as_str() {
        return Err(format!("posted row is wrong: {row}"));
    }

    // 2. Everyone hears about it, and the other player's board shows it.
    partner.wait_for("the post announcement", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains("[Commission]") && text.contains("Fire Damascus") => Some(()),
        _ => None,
    })?;
    let seen = command(&mut partner, "@commission list")?;
    match seen.row(id) {
        Some(row) if row["mine"] == 0 && row["yours"] == 0 && row["item"] == "Fire Damascus" => {}
        other => {
            return Err(format!(
                "the other player does not see request #{id} as someone else's: {other:?}"
            ));
        }
    }

    // 3. Only the poster may change it.
    for action in ["complete", "cancel"] {
        let refused = command(&mut partner, &format!("@commission {action} {id}"))?;
        if !refused.says("Only the player who posted") || refused.row(id).is_none() {
            return Err(format!("a non-poster could {action} #{id}: {:?}", refused.notes));
        }
    }
    let refused = command(&mut partner, &format!("@commission assign {id} {partner_name}"))?;
    if !refused.says("Only the player who posted") || refused.row(id).map(|row| row["status"] == "open") != Some(true) {
        return Err(format!("a non-poster could assign #{id}: {:?}", refused.notes));
    }

    // 4. Input the server must refuse rather than store and show to everyone.
    let bad = command(&mut primary, "@commission post 10 Sword\"<script>")?;
    if !bad.says("Item names may use")
        || bad
            .rows
            .iter()
            .any(|row| row["item"].as_str().is_some_and(|item| item.contains('<')))
    {
        return Err(format!("an unsafe item name was accepted: {:?}", bad.notes));
    }
    let bad = command(&mut primary, "@commission post lots Sword")?;
    if !bad.says("Usage") {
        return Err(format!("a non-numeric fee was accepted: {:?}", bad.notes));
    }
    let bad = command(&mut primary, &format!("@commission assign {id} NoSuchCharacterXyz"))?;
    if !bad.says("No character is named") {
        return Err(format!("an unknown crafter was accepted: {:?}", bad.notes));
    }

    // 5. The poster assigns the partner, who is told and sees it as theirs.
    partner.flush();
    let assigned = command(&mut primary, &format!("@commission assign {id} {partner_name}"))?;
    match assigned.row(id) {
        Some(row) if row["status"] == "assigned" && row["crafter"] == partner_name.as_str() => {}
        other => return Err(format!("assign did not take: {other:?} / {:?}", assigned.notes)),
    }
    partner.wait_for("the assignment notice", |event| match event {
        NetworkEvent::ChatMessage { text, .. } if text.contains("[Commission]") && text.contains("assigned you") => Some(()),
        _ => None,
    })?;

    // 6. It is stored, not held in a session: the crafter logs out and back in and
    //    still has it.
    drop(partner);
    let mut partner = TestContext::connect_partner(config)?;
    let after_relog = command(&mut partner, "@commission list")?;
    match after_relog.row(id) {
        Some(row) if row["yours"] == 1 && row["status"] == "assigned" => {}
        other => return Err(format!("after a relog the crafter's board shows #{id} as {other:?}")),
    }

    // 7. Completing removes it from every board.
    let done = command(&mut primary, &format!("@commission complete {id}"))?;
    if done.row(id).is_some() || !done.says("completed") {
        return Err(format!("complete did not take: {:?}", done.notes));
    }
    let gone = command(&mut partner, "@commission list")?;
    if gone.row(id).is_some() {
        return Err(format!("#{id} is still on the other player's board after completion"));
    }
    let again = command(&mut primary, &format!("@commission cancel {id}"))?;
    if !again.says("is not on the board") {
        return Err(format!("a completed request could still be cancelled: {:?}", again.notes));
    }
    Ok(())
}
