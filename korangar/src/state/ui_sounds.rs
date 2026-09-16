//! Restrained UI sounds: one shot per state transition.

#[derive(Clone, Debug, Default)]
pub struct UiSoundGate {
    last_activation: bool,
}

impl UiSoundGate {
    pub fn on_activation(&mut self, down: bool) -> bool {
        let play = down && !self.last_activation;
        self.last_activation = down;
        play
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_does_not_repeat() {
        let mut gate = UiSoundGate::default();
        assert!(gate.on_activation(true));
        assert!(!gate.on_activation(true));
        assert!(!gate.on_activation(true));
        assert!(!gate.on_activation(false));
        assert!(gate.on_activation(true));
    }
}
