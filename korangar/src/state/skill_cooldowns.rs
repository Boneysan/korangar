//! Skill cooldown deadlines from `ZC_SKILL_POSTDELAY` (0x043D).

use korangar_interface::element::StateElement;
use ragnarok_packets::{ClientTick, SkillId};
use rust_state::RustState;

#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct SkillCooldowns {
    /// skill_id → server client-tick when the skill is usable again.
    #[hidden_element]
    entries: Vec<(SkillId, ClientTick)>,
    display_text: String,
}

impl SkillCooldowns {
    pub fn set(&mut self, skill_id: SkillId, until: ClientTick) {
        if let Some(entry) = self.entries.iter_mut().find(|(id, _)| *id == skill_id) {
            entry.1 = until;
        } else {
            self.entries.push((skill_id, until));
        }
        self.rebuild_display();
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.display_text.clear();
    }

    /// Drop expired cooldowns and refresh the HUD line.
    pub fn tick(&mut self, now: ClientTick) {
        let before = self.entries.len();
        self.entries.retain(|(_, until)| until.0 > now.0);
        if self.entries.len() != before {
            self.rebuild_display_with_now(now);
        } else if !self.entries.is_empty() {
            self.rebuild_display_with_now(now);
        }
    }

    pub fn remaining_ms(&self, skill_id: SkillId, now: ClientTick) -> Option<u32> {
        self.entries
            .iter()
            .find(|(id, _)| *id == skill_id)
            .and_then(|(_, until)| until.0.checked_sub(now.0))
    }

    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn rebuild_display(&mut self) {
        if self.entries.is_empty() {
            self.display_text.clear();
            return;
        }
        // Absolute ticks until we know "now"; tick() refreshes with deltas.
        let parts: Vec<_> = self
            .entries
            .iter()
            .map(|(id, until)| format!("skill {} until {}", id.0, until.0))
            .collect();
        self.display_text = format!("CD: {}", parts.join(" | "));
    }

    fn rebuild_display_with_now(&mut self, now: ClientTick) {
        if self.entries.is_empty() {
            self.display_text.clear();
            return;
        }
        let parts: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(id, until)| {
                let remain = until.0.checked_sub(now.0)?;
                Some(format!("#{} {:.1}s", id.0, remain as f32 / 1000.0))
            })
            .collect();
        self.display_text = if parts.is_empty() {
            String::new()
        } else {
            format!("CD: {}", parts.join(" | "))
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_remaining() {
        let mut cds = SkillCooldowns::default();
        cds.set(SkillId(28), ClientTick(5000));
        assert_eq!(cds.remaining_ms(SkillId(28), ClientTick(2000)), Some(3000));
        assert_eq!(cds.remaining_ms(SkillId(1), ClientTick(2000)), None);
    }

    #[test]
    fn tick_expires_and_updates_display() {
        let mut cds = SkillCooldowns::default();
        cds.set(SkillId(28), ClientTick(5000));
        cds.tick(ClientTick(1000));
        assert!(cds.display_text().contains("#28"));
        assert!(cds.display_text().contains("4.0s"));

        cds.tick(ClientTick(5000));
        assert!(cds.is_empty());
        assert!(cds.display_text().is_empty());
    }

    #[test]
    fn set_replaces_same_skill() {
        let mut cds = SkillCooldowns::default();
        cds.set(SkillId(28), ClientTick(1000));
        cds.set(SkillId(28), ClientTick(9000));
        assert_eq!(cds.remaining_ms(SkillId(28), ClientTick(0)), Some(9000));
        assert_eq!(cds.entries.len(), 1);
    }
}
