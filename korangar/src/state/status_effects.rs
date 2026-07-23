use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;
use std::time::Instant;

use rust_state::RustState;

/// Index → English name for every status icon Hercules defines (M1-010).
///
/// Generated from Hercules' own `db/constants.conf` by
/// `tools/export_status_names.py`; re-run it if the server's constants change
/// (`--check` fails when this file is stale). Sourced from Hercules (GPL)
/// rather than the official client's efst tables to stay clear of the
/// No-Upstream-IP rule in `CLAUDE.md`.
const STATUS_NAMES_JSON: &str = include_str!("../../../docs/status_effects.json");

fn status_names() -> &'static HashMap<u16, String> {
    static NAMES: OnceLock<HashMap<u16, String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        serde_json::from_str::<HashMap<String, String>>(STATUS_NAMES_JSON)
            .expect("embedded status_effects.json is valid")
            .into_iter()
            .filter_map(|(index, name)| index.parse::<u16>().ok().map(|index| (index, name)))
            .collect()
    })
}

/// English name for a status index, falling back to the raw index.
///
/// The fallback matters: the server may send an icon this build's table doesn't
/// know, and showing `#123` is still better than dropping the effect.
fn status_name(index: u16) -> String {
    status_names().get(&index).cloned().unwrap_or_else(|| format!("#{index}"))
}

/// Compact glyph "icon" for a status (M1-010 icon half without GRF artwork).
///
/// Two-letter monograms from the English name, plus a role tag so buffs and
/// debuffs are visually distinct in the text bar. Real SC sprites remain a
/// follow-up once artwork is licensed/shipped.
fn status_icon_glyph(index: u16, name: &str) -> String {
    let monogram: String = name
        .split(|c: char| c.is_whitespace() || c == '-' || c == '/')
        .filter(|w| !w.is_empty())
        .take(2)
        .filter_map(|w| w.chars().next())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let monogram = if monogram.is_empty() {
        format!("{index}")
    } else {
        monogram
    };
    let role = status_role_tag(index, name);
    format!("[{monogram}{role}]")
}

/// `+` buff-like, `−` debuff-like, `·` neutral/utility (hide, post-delay, …).
///
/// Classification is name-based on the SI English table (not SC_* ids). Keep
/// it conservative: substring matches that would tag buffs as debuffs
/// (`Stonehardskin`, `Enchant Poison`, `Freeze Sp`) are excluded.
fn status_role_tag(index: u16, name: &str) -> char {
    // Explicit utility / non-buff SI indices.
    match index {
        4 | 5 => return '·',   // Hiding / Cloaking
        46 => return '·',      // Skill Delay (postdelay)
        27 | 28 => return '·', // Riding / Falcon
        35 | 36 => return '·', // Weightover
        // Not a buff on the player at all — a property of the ground they are
        // standing on.
        SYNTHETIC_LAND_PROTECTOR => return '·',
        _ => {}
    }

    let lower = name.to_ascii_lowercase();

    // Utility first so "Skill Delay" / "Hiding" don't fall through.
    if lower.contains("delay")
        || lower == "hiding"
        || lower == "cloaking"
        || lower.contains("weightover")
        || lower.contains("noequip")
        || lower.contains("riding")
        || lower.contains("falcon")
    {
        return '·';
    }

    // Known buff names that share debuff substrings.
    if lower.contains("enchant poison")
        || lower.contains("poison react")
        || lower.contains("poisoningweapon")
        || lower.contains("stonehard")
        || lower.contains("stone shield")
        || lower.contains("freeze sp")
        || lower.contains("slow poison") // support skill icon
    {
        return '+';
    }

    // Debuff / control cues (SI display names).
    if lower.contains("quagmire")
        || lower.contains("decrease agi")
        || lower.contains("lex aeterna")
        || lower.contains("anklesnare")
        || lower.contains("broken")
        || lower.contains("illusion")
        || lower.contains("bleed")
        || lower.contains("deepsleep")
        || lower.contains("oblivioncurse")
        || lower.contains("soulcurse")
        || lower.starts_with("gvg ")
        || (lower.contains("curse") && !lower.contains("cursed soil"))
        || lower.contains("stun")
        || lower.contains("silence")
        || lower.contains("blind")
        || (lower.contains("sleep") && !lower.contains("deep"))
        || lower.contains("strip")
    {
        return '−';
    }

    '+' // default: treat as buff / support
}

/// Cap on effects listed in the status window. Effects with a description take
/// **two** lines, so this bounds the window's height — keep it in step with the
/// `WindowClass::StatusBar` default size in `interface/windows/cache.rs`
/// (320×160). Lowered from 8 when descriptions were added.
const MAXIMUM_DISPLAYED_EFFECTS: usize = 5;

/// Index for the client-synthesised "you are standing in a Land Protector"
/// hint.
///
/// Land Protector deliberately grants no status — it acts on the ground, not
/// on people — so the server never sends anything and a modern player gets no
/// feedback that their ground magic is being suppressed. This is the client
/// telling them anyway, from the skill units it already tracks.
///
/// `u16::MAX` cannot collide: the highest index Hercules defines is 1149.
pub const SYNTHETIC_LAND_PROTECTOR: u16 = u16::MAX;

/// What an effect actually does, in the player's terms.
///
/// Deliberately **not** exhaustive: `docs/status_effects.json` holds 699
/// indices and most never reach a player's screen. Anything without an entry
/// renders as name + timer, exactly as before.
///
/// Numbers come from the server's own `val1`/`val2`/`val3` rather than
/// re-deriving Hercules' formulas, so they cannot drift out of sync with the
/// server the client is actually connected to.
fn status_description(index: u16, specific_name: Option<&str>, values: [u32; 3]) -> Option<String> {
    let bonus = values[1];

    match index {
        // SI_GROUNDMAGIC — shared by all three Sage elemental fields, which is
        // why the caller has to tell us which one this is. `val2` is the
        // server's computed bonus in each case (`status.c`: Watk/Matk for
        // Volcano, `deluge_eff[]` HP% for Deluge, Flee for Violent Gale).
        112 => match specific_name {
            Some("Volcano") => Some(format!("+{bonus} ATK & MATK, stronger Fire")),
            Some("Deluge") => Some(format!("+{bonus}% Max HP, stronger Water")),
            Some("Violent Gale") => Some(format!("+{bonus} Flee, stronger Wind")),
            // In a field whose unit we never saw spawn.
            _ => Some("Elemental ground field".to_owned()),
        },
        SYNTHETIC_LAND_PROTECTOR => Some("Ground magic suppressed here".to_owned()),
        _ => None,
    }
}

/// A single active status effect (buff or debuff) on the player.
#[derive(Clone, Debug, PartialEq, RustState)]
pub struct StatusEffect {
    /// Server status change index; resolved to a name via [`status_name`].
    pub index: u16,
    /// When the effect expires (None = infinite / no timer).
    pub expires_at: Option<Instant>,
    /// Original full duration in ms (for visual depletion ratio if desired).
    pub duration_ms: u32,
    /// Hercules' `val1`/`val2`/`val3` — the server's own computed numbers for
    /// this status, used by [`status_description`] so the UI never re-derives
    /// a server formula.
    pub values: [u32; 3],
    /// A more precise name than the icon index can give, resolved by the
    /// caller. Several statuses share one icon — `SC_VOLCANO`, `SC_DELUGE`
    /// and `SC_VIOLENTGALE` are all `SI_GROUNDMAGIC` — so the caller
    /// disambiguates from world state and passes the answer in.
    pub specific_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, RustState)]
pub struct StatusEffects {
    effects: Vec<StatusEffect>,
    /// Cached summary for simple UI binding (MVP).
    display_text: String,
}

impl Default for StatusEffects {
    fn default() -> Self {
        Self {
            effects: Vec::new(),
            display_text: "No active effects".to_owned(),
        }
    }
}

impl StatusEffects {
    fn refresh_display(&mut self) {
        if self.effects.is_empty() {
            self.display_text = "No active effects".to_owned();
            return;
        }
        let now = Instant::now();
        // One effect per line. Names contain spaces ("Skill Delay"), so any inline
        // separator blurs them together — "Skill Delay  Blessing 223s" reads as a
        // single phrase. cosmic-text breaks on '\n', so this lays out as real lines.
        // No "Effects:" prefix: the window is already titled "Status".
        self.display_text = self
            .effects
            .iter()
            .take(MAXIMUM_DISPLAYED_EFFECTS)
            .map(|e| {
                // A resolved specific name wins: the icon index alone would
                // render every Sage field as "Elemental Field".
                let name = e.specific_name.clone().unwrap_or_else(|| status_name(e.index));
                let icon = status_icon_glyph(e.index, &name);
                let headline = if let Some(exp) = e.expires_at {
                    let secs = exp.saturating_duration_since(now).as_secs() as u32;
                    format!("{icon} {name} {secs}s")
                } else {
                    format!("{icon} {name}")
                };

                match status_description(e.index, e.specific_name.as_deref(), e.values) {
                    // Indented so the description reads as subordinate to the
                    // name rather than as another effect.
                    Some(description) => format!("{headline}\n    {description}"),
                    None => headline,
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    /// Apply or refresh an effect. If an effect with the same index exists, it
    /// is updated.
    ///
    /// `remaining_ms` of [`u32::MAX`] means infinite: Hercules'
    /// `INFINITE_DURATION` is `-1` (`src/common/mmo.h`), which arrives here
    /// as `u32::MAX`. A *zero* duration is **not** infinite — it means the
    /// effect is already over.
    pub fn apply(&mut self, index: u16, duration_ms: u32, remaining_ms: u32, values: [u32; 3], specific_name: Option<String>) {
        // Previously `duration_ms == 0` was also treated as infinite, which stuck
        // zero-length effects on screen forever. `SC_POSTDELAY` ("Skill Delay") is the
        // case that bites: `skill.c` fires it via a direct `clif->status_change` with
        // `delay_fix()` as the duration and no `sc_start`, so there is no server-side
        // timer and *no end packet will ever arrive* — the client alone must expire it.
        // For a skill with no after-cast delay that duration is 0, so the icon hung
        // around permanently (M1-011).
        let expires_at = match remaining_ms {
            u32::MAX => None,
            remaining => Some(Instant::now() + std::time::Duration::from_millis(remaining as u64)),
        };

        if let Some(existing) = self.effects.iter_mut().find(|e| e.index == index) {
            existing.expires_at = expires_at;
            existing.duration_ms = duration_ms;
            existing.values = values;
            // A refresh may arrive when the caller cannot resolve the name
            // (walked out of the unit's cell mid-effect); keep what we had
            // rather than downgrading to the generic icon name.
            if specific_name.is_some() {
                existing.specific_name = specific_name;
            }
        } else {
            self.effects.push(StatusEffect {
                index,
                expires_at,
                duration_ms,
                values,
                specific_name,
            });
        }
        self.refresh_display();
    }

    /// Add or drop a client-synthesised effect — one the server never sends,
    /// derived from world state the client tracks itself (see
    /// [`SYNTHETIC_LAND_PROTECTOR`]).
    ///
    /// Called every frame, so it must be a no-op when already in the wanted
    /// state; otherwise the display string would be rebuilt continuously.
    /// These carry no timer: the client knows the player is standing in the
    /// area, not how long it will last.
    pub fn set_synthetic(&mut self, index: u16, name: &str, present: bool) {
        let existing = self.effects.iter().any(|effect| effect.index == index);

        match (present, existing) {
            (true, false) => self.apply(index, 0, u32::MAX, [0; 3], Some(name.to_owned())),
            (false, true) => self.remove(index),
            _ => {}
        }
    }

    /// Remove an effect by index (if present).
    pub fn remove(&mut self, index: u16) {
        self.effects.retain(|e| e.index != index);
        self.refresh_display();
    }

    /// Drop any expired effects. Call once per frame.
    pub fn tick(&mut self, now: Instant) {
        let before = self.effects.len();
        self.effects.retain(|e| match e.expires_at {
            Some(exp) => exp > now,
            None => true,
        });
        // Always refresh so countdowns in the summary update.
        if !self.effects.is_empty() || before != 0 {
            self.refresh_display();
        }
    }

    /// Clear all effects (e.g. on map change or logout).
    pub fn clear(&mut self) {
        self.effects.clear();
        self.refresh_display();
    }

    /// Borrow the current list (for direct reads, e.g. future tile rendering).
    #[allow(dead_code)]
    pub fn effects(&self) -> &[StatusEffect] {
        &self.effects
    }
}

impl fmt::Display for StatusEffects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M1-010: the buff bar rendered raw server indices ("10:218s"), so players
    // could not tell what was buffed. Indices below are from Hercules
    // `db/constants.conf`.

    #[test]
    fn known_indices_resolve_to_english() {
        assert_eq!(status_name(10), "Blessing"); // SI_BLESSING
        assert_eq!(status_name(4), "Hiding"); // SI_HIDING
        assert_eq!(status_name(0), "Provoke"); // SI_PROVOKE
    }

    #[test]
    fn overrides_beat_the_naive_humaniser() {
        // Would otherwise render "Postdelay" / "Twohandquicken" / "Inc Agi".
        assert_eq!(status_name(46), "Skill Delay"); // SI_POSTDELAY
        assert_eq!(status_name(2), "Two-Hand Quicken"); // SI_TWOHANDQUICKEN
        assert_eq!(status_name(12), "Increase AGI"); // SI_INC_AGI
    }

    #[test]
    fn unknown_index_falls_back_to_number() {
        // A newer server may send an icon this build doesn't know; showing
        // something is better than dropping the effect.
        assert_eq!(status_name(u16::MAX), "#65535");
    }

    #[test]
    fn display_uses_names_not_indices() {
        let mut effects = StatusEffects::default();
        assert_eq!(effects.to_string(), "No active effects");

        effects.apply(10, 240_000, 218_000, [0; 3], None);
        let shown = effects.to_string();
        assert!(shown.contains("Blessing"), "expected a name, got: {shown}");
        assert!(shown.contains("[B+]"), "expected buff monogram [B+], got: {shown}");
        assert!(!shown.contains("10:"), "raw index leaked into display: {shown}");

        effects.remove(10);
        assert_eq!(effects.to_string(), "No active effects");
    }

    #[test]
    fn role_tags_do_not_mark_common_buffs_as_debuffs() {
        assert_eq!(status_role_tag(10, "Blessing"), '+');
        assert_eq!(status_role_tag(3, "Concentration"), '+');
        assert_eq!(status_role_tag(6, "Enchant Poison"), '+');
        assert_eq!(status_role_tag(320, "Stonehardskin"), '+');
        assert_eq!(status_role_tag(4, "Hiding"), '·');
        assert_eq!(status_role_tag(46, "Skill Delay"), '·');
        assert_eq!(status_role_tag(13, "Decrease AGI"), '−');
        assert_eq!(status_role_tag(8, "Quagmire"), '−');
    }

    #[test]
    fn each_effect_gets_its_own_line() {
        // Names contain spaces, so any inline separator made effects read as one
        // phrase: "Hiding  Blessing 223s".
        let mut effects = StatusEffects::default();
        effects.apply(4, 0, u32::MAX, [0; 3], None); // Hiding, permanent — renders without a timer
        effects.apply(10, 240_000, 223_000, [0; 3], None); // Blessing — renders with one

        let lines: Vec<_> = effects.to_string().lines().map(str::to_owned).collect();
        assert_eq!(lines.len(), 2, "expected one line per effect, got: {lines:?}");
        assert!(lines[0].contains("Hiding"), "line 0: {lines:?}");
        assert!(lines[1].contains("Blessing"), "expected timer on line 2: {lines:?}");
    }

    #[test]
    fn zero_duration_effect_expires_instead_of_sticking() {
        // M1-011: "Skill Delay" hung on screen forever. Hercules fires SC_POSTDELAY via
        // a direct clif->status_change with delay_fix() as the duration and no
        // sc_start, so no end packet ever arrives — a skill with no after-cast
        // delay sends 0/0 and the client must expire it itself.
        let mut effects = StatusEffects::default();
        effects.apply(46, 0, 0, [0; 3], None); // Skill Delay, zero delay
        assert!(effects.to_string().contains("Skill Delay"), "should appear on apply");

        effects.tick(Instant::now() + std::time::Duration::from_millis(1));
        assert_eq!(effects.to_string(), "No active effects", "zero-duration effect must expire");
    }

    #[test]
    fn infinite_effect_is_retained() {
        // Hercules INFINITE_DURATION is -1 (mmo.h), arriving as u32::MAX. Those are
        // genuinely permanent and end only via an explicit server removal.
        let mut effects = StatusEffects::default();
        effects.apply(4, 0, u32::MAX, [0; 3], None); // Hiding, permanent

        effects.tick(Instant::now() + std::time::Duration::from_secs(3600));
        assert!(effects.to_string().contains("Hiding"), "infinite effect must survive tick");

        effects.remove(4);
        assert_eq!(effects.to_string(), "No active effects", "server end must still clear it");
    }

    #[test]
    fn timed_effect_expires_on_schedule() {
        let mut effects = StatusEffects::default();
        let start = Instant::now();
        effects.apply(10, 240_000, 218_000, [0; 3], None); // Blessing

        effects.tick(start + std::time::Duration::from_secs(217));
        assert!(effects.to_string().contains("Blessing"), "must survive before expiry");

        effects.tick(start + std::time::Duration::from_secs(219));
        assert_eq!(effects.to_string(), "No active effects", "must clear after expiry");
    }

    /// Hercules gives SC_VOLCANO, SC_DELUGE and SC_VIOLENTGALE the same icon
    /// (SI_GROUNDMAGIC, 112), so the index alone can only say "Elemental
    /// Field". The caller resolves the real one from the unit the player
    /// stands in and passes it in.
    #[test]
    fn elemental_fields_show_the_specific_name_not_the_shared_icon_name() {
        let mut effects = StatusEffects::default();
        effects.apply(112, 300_000, 245_000, [5, 30, 0], Some("Volcano".to_owned()));

        let shown = effects.to_string();
        assert!(shown.contains("Volcano"), "expected the resolved name, got: {shown}");
        assert!(!shown.contains("Elemental Field"), "generic name leaked: {shown}");
    }

    #[test]
    fn unresolved_field_still_names_itself_something_useful() {
        // Walking into a field whose units we never saw spawn.
        let mut effects = StatusEffects::default();
        effects.apply(112, 300_000, 245_000, [5, 30, 0], None);

        assert!(effects.to_string().contains("Elemental Field"));
    }

    /// The bonus is quoted from the server's own `val2`, never re-derived, so
    /// it cannot drift from the server the client is connected to.
    #[test]
    fn description_quotes_the_servers_own_numbers() {
        let cases = [
            ("Volcano", "+30 ATK & MATK, stronger Fire"),
            ("Deluge", "+30% Max HP, stronger Water"),
            ("Violent Gale", "+30 Flee, stronger Wind"),
        ];

        for (field, expected) in cases {
            let description = status_description(112, Some(field), [5, 30, 0]).expect("fields describe themselves");
            assert_eq!(description, expected);
        }
    }

    #[test]
    fn a_described_effect_renders_its_description_on_its_own_line() {
        let mut effects = StatusEffects::default();
        effects.apply(112, 300_000, 245_000, [5, 30, 0], Some("Volcano".to_owned()));

        let lines: Vec<_> = effects.to_string().lines().map(str::to_owned).collect();
        assert_eq!(lines.len(), 2, "name and description are separate lines: {lines:?}");
        assert!(lines[0].contains("Volcano"), "{lines:?}");
        assert!(lines[1].trim().starts_with("+30 ATK"), "{lines:?}");
    }

    #[test]
    fn effects_without_a_description_stay_one_line() {
        // Only a curated few have descriptions; the rest must not regress.
        let mut effects = StatusEffects::default();
        effects.apply(10, 240_000, 218_000, [0; 3], None); // Blessing

        assert_eq!(effects.to_string().lines().count(), 1);
    }

    #[test]
    fn a_refresh_without_a_resolved_name_keeps_the_one_we_had() {
        // The server refreshes the status while the player has stepped off the
        // unit's cell; downgrading to "Elemental Field" mid-effect would look
        // like a bug.
        let mut effects = StatusEffects::default();
        effects.apply(112, 300_000, 245_000, [5, 30, 0], Some("Deluge".to_owned()));
        effects.apply(112, 300_000, 200_000, [5, 30, 0], None);

        assert!(effects.to_string().contains("Deluge"));
    }

    /// Land Protector sends no status at all, so this line exists purely to
    /// tell a modern player why their ground magic stopped working.
    #[test]
    fn land_protector_hint_appears_and_clears_with_the_ground() {
        let mut effects = StatusEffects::default();

        effects.set_synthetic(SYNTHETIC_LAND_PROTECTOR, "Magnetic Earth", true);
        let shown = effects.to_string();
        assert!(shown.contains("Magnetic Earth"), "{shown}");
        assert!(shown.contains("Ground magic suppressed here"), "{shown}");
        // A property of the ground, not a buff on the player.
        assert!(shown.contains("[ME·]"), "expected the utility tag, got: {shown}");
        // The client knows the player is inside, not for how long, so the
        // headline must end at the name with no "123s" appended.
        let headline = shown.lines().next().unwrap();
        assert!(
            headline.trim_end().ends_with("Magnetic Earth"),
            "a synthetic hint must not invent a timer: {headline}"
        );

        effects.set_synthetic(SYNTHETIC_LAND_PROTECTOR, "Magnetic Earth", false);
        assert_eq!(effects.to_string(), "No active effects");
    }

    #[test]
    fn a_synthetic_hint_survives_ticking_and_does_not_duplicate() {
        // It is set every frame, so it must be idempotent and must not expire
        // on its own like a zero-duration server effect would.
        let mut effects = StatusEffects::default();
        for _ in 0..5 {
            effects.set_synthetic(SYNTHETIC_LAND_PROTECTOR, "Magnetic Earth", true);
        }

        effects.tick(Instant::now() + std::time::Duration::from_secs(600));

        let lines: Vec<_> = effects.to_string().lines().map(str::to_owned).collect();
        assert_eq!(lines.len(), 2, "one effect: name + description, got: {lines:?}");
    }

    #[test]
    fn display_is_capped_so_the_window_cannot_overflow() {
        let mut effects = StatusEffects::default();
        for index in 0..(MAXIMUM_DISPLAYED_EFFECTS as u16 + 5) {
            effects.apply(index, 60_000, 60_000, [0; 3], None);
        }
        assert_eq!(effects.to_string().lines().count(), MAXIMUM_DISPLAYED_EFFECTS);
    }

    #[test]
    fn every_exported_name_is_non_empty() {
        // Guards against the exporter emitting blanks for odd constants.
        assert!(!status_names().is_empty(), "status name table failed to load");
        assert!(
            status_names().values().all(|name| !name.trim().is_empty()),
            "exporter produced an empty status name"
        );
    }
}
