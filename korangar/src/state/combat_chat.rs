//! Structured combat-log events from network facts, not parsed chat strings.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CombatCategory {
    DamageDealt,
    DamageReceived,
    Healing,
    StatusGain,
    StatusLoss,
    SkillFailure,
    Exp,
    Loot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatEntry {
    pub category: CombatCategory,
    pub source: String,
    pub target: String,
    pub value: i32,
}

pub fn from_damage(source: &str, target: &str, amount: i32, outgoing: bool) -> CombatEntry {
    CombatEntry {
        category: if outgoing {
            CombatCategory::DamageDealt
        } else {
            CombatCategory::DamageReceived
        },
        source: source.to_owned(),
        target: target.to_owned(),
        value: amount,
    }
}

pub fn from_heal(source: &str, target: &str, amount: i32) -> CombatEntry {
    CombatEntry {
        category: CombatCategory::Healing,
        source: source.to_owned(),
        target: target.to_owned(),
        value: amount,
    }
}

pub fn from_status(target: &str, gain: bool) -> CombatEntry {
    CombatEntry {
        category: if gain {
            CombatCategory::StatusGain
        } else {
            CombatCategory::StatusLoss
        },
        source: String::new(),
        target: target.to_owned(),
        value: 0,
    }
}

pub fn from_skill_fail(source: &str) -> CombatEntry {
    CombatEntry {
        category: CombatCategory::SkillFailure,
        source: source.to_owned(),
        target: String::new(),
        value: 0,
    }
}

pub fn from_exp(amount: i32) -> CombatEntry {
    CombatEntry {
        category: CombatCategory::Exp,
        source: String::new(),
        target: "self".into(),
        value: amount,
    }
}

pub fn from_loot(item: &str, amount: i32) -> CombatEntry {
    CombatEntry {
        category: CombatCategory::Loot,
        source: String::new(),
        target: item.to_owned(),
        value: amount,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_entry_per_category() {
        assert_eq!(from_damage("me", "poring", 12, true).category, CombatCategory::DamageDealt);
        assert_eq!(from_damage("poring", "me", 4, false).category, CombatCategory::DamageReceived);
        assert_eq!(from_heal("acolyte", "me", 20).value, 20);
        assert_eq!(from_status("me", true).category, CombatCategory::StatusGain);
        assert_eq!(from_status("me", false).category, CombatCategory::StatusLoss);
        assert_eq!(from_skill_fail("me").category, CombatCategory::SkillFailure);
        assert_eq!(from_exp(400).value, 400);
        assert_eq!(from_loot("Red Potion", 1).category, CombatCategory::Loot);
    }
}
