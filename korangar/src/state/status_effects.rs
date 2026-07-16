use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;
use std::time::Instant;

use rust_state::RustState;

/// Index → English name for every status icon Hercules defines (M1-010).
///
/// Generated from Hercules' own `db/constants.conf` by
/// `tools/export_status_names.py`; re-run it if the server's constants change
/// (`--check` fails when this file is stale). Sourced from Hercules (GPL) rather
/// than the official client's efst tables to stay clear of the No-Upstream-IP
/// rule in `CLAUDE.md`.
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

/// Cap on effects listed in the status window. One line each, so this bounds the
/// window's height — keep it in step with the `WindowClass::StatusBar` default
/// size in `interface/windows/cache.rs`.
const MAXIMUM_DISPLAYED_EFFECTS: usize = 8;

/// A single active status effect (buff or debuff) on the player.
#[derive(Clone, Debug, PartialEq, RustState)]
pub struct StatusEffect {
    /// Server status change index; resolved to a name via [`status_name`].
    pub index: u16,
    /// When the effect expires (None = infinite / no timer).
    pub expires_at: Option<Instant>,
    /// Original full duration in ms (for visual depletion ratio if desired).
    pub duration_ms: u32,
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
                let name = status_name(e.index);
                if let Some(exp) = e.expires_at {
                    let secs = exp.saturating_duration_since(now).as_secs() as u32;
                    format!("{name} {secs}s")
                } else {
                    name
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    /// Apply or refresh an effect. If an effect with the same index exists, it is updated.
    ///
    /// `remaining_ms` of [`u32::MAX`] means infinite: Hercules' `INFINITE_DURATION` is
    /// `-1` (`src/common/mmo.h`), which arrives here as `u32::MAX`. A *zero* duration is
    /// **not** infinite — it means the effect is already over.
    pub fn apply(&mut self, index: u16, duration_ms: u32, remaining_ms: u32) {
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
        } else {
            self.effects.push(StatusEffect {
                index,
                expires_at,
                duration_ms,
            });
        }
        self.refresh_display();
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

        effects.apply(10, 240_000, 218_000);
        let shown = effects.to_string();
        assert!(shown.contains("Blessing"), "expected a name, got: {shown}");
        assert!(!shown.contains("10:"), "raw index leaked into display: {shown}");

        effects.remove(10);
        assert_eq!(effects.to_string(), "No active effects");
    }

    #[test]
    fn each_effect_gets_its_own_line() {
        // Names contain spaces, so any inline separator made effects read as one
        // phrase: "Hiding  Blessing 223s".
        let mut effects = StatusEffects::default();
        effects.apply(4, 0, u32::MAX); // Hiding, permanent — renders without a timer
        effects.apply(10, 240_000, 223_000); // Blessing — renders with one

        let lines: Vec<_> = effects.to_string().lines().map(str::to_owned).collect();
        assert_eq!(lines.len(), 2, "expected one line per effect, got: {lines:?}");
        assert_eq!(lines[0], "Hiding");
        assert!(lines[1].starts_with("Blessing "), "expected timer on line 2: {lines:?}");
    }

    #[test]
    fn zero_duration_effect_expires_instead_of_sticking() {
        // M1-011: "Skill Delay" hung on screen forever. Hercules fires SC_POSTDELAY via a
        // direct clif->status_change with delay_fix() as the duration and no sc_start, so
        // no end packet ever arrives — a skill with no after-cast delay sends 0/0 and the
        // client must expire it itself.
        let mut effects = StatusEffects::default();
        effects.apply(46, 0, 0); // Skill Delay, zero delay
        assert!(effects.to_string().contains("Skill Delay"), "should appear on apply");

        effects.tick(Instant::now() + std::time::Duration::from_millis(1));
        assert_eq!(effects.to_string(), "No active effects", "zero-duration effect must expire");
    }

    #[test]
    fn infinite_effect_is_retained() {
        // Hercules INFINITE_DURATION is -1 (mmo.h), arriving as u32::MAX. Those are
        // genuinely permanent and end only via an explicit server removal.
        let mut effects = StatusEffects::default();
        effects.apply(4, 0, u32::MAX); // Hiding, permanent

        effects.tick(Instant::now() + std::time::Duration::from_secs(3600));
        assert!(effects.to_string().contains("Hiding"), "infinite effect must survive tick");

        effects.remove(4);
        assert_eq!(effects.to_string(), "No active effects", "server end must still clear it");
    }

    #[test]
    fn timed_effect_expires_on_schedule() {
        let mut effects = StatusEffects::default();
        let start = Instant::now();
        effects.apply(10, 240_000, 218_000); // Blessing

        effects.tick(start + std::time::Duration::from_secs(217));
        assert!(effects.to_string().contains("Blessing"), "must survive before expiry");

        effects.tick(start + std::time::Duration::from_secs(219));
        assert_eq!(effects.to_string(), "No active effects", "must clear after expiry");
    }

    #[test]
    fn display_is_capped_so_the_window_cannot_overflow() {
        let mut effects = StatusEffects::default();
        for index in 0..(MAXIMUM_DISPLAYED_EFFECTS as u16 + 5) {
            effects.apply(index, 60_000, 60_000);
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
