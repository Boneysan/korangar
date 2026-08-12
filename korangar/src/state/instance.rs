//! Memorial dungeon / instance window state.

use korangar_interface::element::StateElement;
use rust_state::RustState;

/// What the instance information window shows.
///
/// Hercules drives this with three packets — create (0x02CB), join with timers
/// (0x02CD) and delete (0x02CE) — which previously all fell through the length
/// fallback, so an instance was completely invisible client-side.
#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct InstanceState {
    instance_name: String,
    /// Cached line for the window; timers are seconds remaining.
    display_text: String,
}

#[allow(dead_code)]
impl InstanceState {
    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }

    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    /// The instance exists and is waiting to be entered.
    pub fn set_pending(&mut self, instance_name: String) {
        self.instance_name = instance_name;
        self.rebuild();
    }

    /// Entered. Exactly one timer is non-zero: `progress` while the instance is
    /// running, `idle` while it waits for someone to enter.
    pub fn set_joined(&mut self, instance_name: String, progress_remaining: u32, idle_remaining: u32) {
        self.instance_name = instance_name;
        self.display_text = match (progress_remaining, idle_remaining) {
            (0, 0) => format!("{}\nNo time limit.", self.instance_name),
            (progress, 0) => format!("{}\nTime remaining: {}", self.instance_name, format_duration(progress)),
            (_, idle) => format!("{}\nIdle timeout: {}", self.instance_name, format_duration(idle)),
        };
    }

    pub fn clear(&mut self) {
        self.instance_name.clear();
        self.display_text.clear();
    }

    fn rebuild(&mut self) {
        self.display_text = match self.instance_name.is_empty() {
            true => String::new(),
            false => format!("{}\nWaiting to enter.", self.instance_name),
        };
    }
}

/// Seconds as `1h 02m` / `5m 09s` / `42s`.
fn format_duration(seconds: u32) -> String {
    let (hours, minutes, seconds) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    match (hours, minutes) {
        (0, 0) => format!("{seconds}s"),
        (0, _) => format!("{minutes}m {seconds:02}s"),
        _ => format!("{hours}h {minutes:02}m"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_timers() {
        assert_eq!(format_duration(42), "42s");
        assert_eq!(format_duration(309), "5m 09s");
        assert_eq!(format_duration(3720), "1h 02m");
    }

    #[test]
    fn join_reports_whichever_timer_is_set() {
        let mut state = InstanceState::default();
        state.set_pending("Endless Tower".to_owned());
        assert!(state.display_text().contains("Waiting to enter"));

        state.set_joined("Endless Tower".to_owned(), 3720, 0);
        assert!(state.display_text().contains("Time remaining"));

        state.set_joined("Endless Tower".to_owned(), 0, 60);
        assert!(state.display_text().contains("Idle timeout"));

        state.clear();
        assert!(state.display_text().is_empty());
    }
}
