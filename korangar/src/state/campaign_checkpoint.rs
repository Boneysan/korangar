//! Party campaign checkpoint: forward-only, per-character carried items.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub step: u32,
    pub carried_by: Option<u32>,
}

impl Checkpoint {
    pub fn advance(&mut self, from: u32, actor: u32) -> Result<(), &'static str> {
        if from != self.step {
            return Err("ineligible or stale step");
        }
        self.step = self.step.saturating_add(1);
        self.carried_by = Some(actor);
        Ok(())
    }

    pub fn sync_member(&self, member_step: u32) -> Result<u32, &'static str> {
        if member_step > self.step {
            return Err("refuse backward sync of ahead member");
        }
        Ok(self.step)
    }

    pub fn consume_turn_in(&mut self, actor: u32) -> Result<(), &'static str> {
        if self.carried_by != Some(actor) {
            return Err("cannot consume another character's carried items");
        }
        self.carried_by = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_only_and_ownership() {
        let mut cp = Checkpoint { step: 1, carried_by: None };
        assert!(cp.advance(0, 10).is_err());
        assert!(cp.advance(1, 10).is_ok());
        assert_eq!(cp.step, 2);
        assert!(cp.consume_turn_in(11).is_err());
        assert!(cp.consume_turn_in(10).is_ok());
        assert_eq!(cp.sync_member(1).unwrap(), 2);
        assert!(cp.sync_member(9).is_err());
    }
}
