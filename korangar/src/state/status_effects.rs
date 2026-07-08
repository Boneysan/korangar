use std::fmt;
use std::time::Instant;

use rust_state::RustState;

/// A single active status effect (buff or debuff) on the player.
#[derive(Clone, Debug, PartialEq, RustState)]
pub struct StatusEffect {
    /// Server status change index (maps to icon/effect type in the future).
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
        let first = self
            .effects
            .iter()
            .take(8)
            .map(|e| {
                if let Some(exp) = e.expires_at {
                    let secs = exp.saturating_duration_since(now).as_secs() as u32;
                    format!("{}:{:02}s", e.index, secs)
                } else {
                    e.index.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("  ");
        self.display_text = format!("Effects: {}", first);
    }

    /// Apply or refresh an effect. If an effect with the same index exists, it is updated.
    pub fn apply(&mut self, index: u16, duration_ms: u32, remaining_ms: u32) {
        let expires_at = if duration_ms == 0 || remaining_ms == u32::MAX {
            None
        } else {
            Some(Instant::now() + std::time::Duration::from_millis(remaining_ms as u64))
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
