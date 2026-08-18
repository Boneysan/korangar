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
    /// Cached line for the window, with timers already resolved to a duration.
    display_text: String,
    /// The timers as they arrived: **absolute Unix seconds**, 0 meaning unset.
    ///
    /// Kept rather than discarded after formatting, because the label has to be
    /// rebuilt as the clock moves. Without these the window showed a correct
    /// duration that then sat frozen — reported live 2026-08-17 as "29m 28s but
    /// that doesn't count down".
    progress_timeout: u32,
    idle_timeout: u32,
    /// Seconds remaining at the last rebuild, so the string is rebuilt once a
    /// second rather than once a frame.
    last_remaining: Option<u32>,
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
        self.progress_timeout = 0;
        self.idle_timeout = 0;
        self.last_remaining = None;
        self.rebuild();
    }

    /// Whether there is a timer that needs re-rendering as the clock moves.
    ///
    /// Checked before ticking so a client with no instance open does not dirty
    /// the state every frame for nothing.
    pub fn is_counting_down(&self) -> bool {
        self.progress_timeout != 0 || self.idle_timeout != 0
    }

    /// Re-render the label for the current time. Cheap and idempotent within a
    /// second: the string is only rebuilt when the displayed value changes.
    pub fn tick(&mut self, now: u64) {
        if !self.is_counting_down() {
            return;
        }

        let (progress, idle) = (self.progress_timeout, self.idle_timeout);
        let remaining = match (progress, idle) {
            (0, idle) => seconds_until(idle, now),
            (progress, _) => seconds_until(progress, now),
        };

        if self.last_remaining != Some(remaining) {
            self.render(now);
        }
    }

    /// Entered. Exactly one timer is non-zero: `progress` while the instance is
    /// running, `idle` while it waits for someone to enter.
    ///
    /// **Both arrive as absolute Unix seconds, not durations.** Hercules stores
    /// `progress_timeout = now + value` (`instance.c:709`) and
    /// `idle_timeout = now + value` (`instance.c:685`), and `clif_instance_join`
    /// puts the field on the wire untouched, so the client has to do the
    /// subtraction. Printing the raw value read as **"Time remaining: 496397h
    /// 45m"** live on 2026-08-17 — 1.787 billion seconds, which is simply the
    /// clock.
    pub fn set_joined(&mut self, instance_name: String, progress_timeout: u32, idle_timeout: u32) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since_epoch| since_epoch.as_secs())
            .unwrap_or_default();

        self.set_joined_at(instance_name, progress_timeout, idle_timeout, now);
    }

    /// [`Self::set_joined`] against a supplied clock, so the arithmetic is
    /// testable without depending on when the test runs.
    pub fn set_joined_at(&mut self, instance_name: String, progress_timeout: u32, idle_timeout: u32, now: u64) {
        self.instance_name = instance_name;
        self.progress_timeout = progress_timeout;
        self.idle_timeout = idle_timeout;
        self.render(now);
    }

    /// Build the label from the stored timeouts and a clock.
    fn render(&mut self, now: u64) {
        let (text, remaining) = match (self.progress_timeout, self.idle_timeout) {
            (0, 0) => (format!("{}\nNo time limit.", self.instance_name), None),
            (progress, 0) => {
                let remaining = seconds_until(progress, now);
                (
                    format!("{}\nTime remaining: {}", self.instance_name, format_duration(remaining)),
                    Some(remaining),
                )
            }
            (_, idle) => {
                let remaining = seconds_until(idle, now);
                (
                    format!("{}\nIdle timeout: {}", self.instance_name, format_duration(remaining)),
                    Some(remaining),
                )
            }
        };

        self.display_text = text;
        self.last_remaining = remaining;
    }

    pub fn clear(&mut self) {
        self.instance_name.clear();
        self.display_text.clear();
        self.progress_timeout = 0;
        self.idle_timeout = 0;
        self.last_remaining = None;
    }

    fn rebuild(&mut self) {
        self.display_text = match self.instance_name.is_empty() {
            true => String::new(),
            false => format!("{}\nWaiting to enter.", self.instance_name),
        };
    }
}

/// How long until an absolute timestamp, saturating at zero.
///
/// Saturating rather than wrapping on purpose: a timeout already in the past is
/// a spent instance, and if some other server ever sent a *duration* here
/// instead the answer would be a harmless `0s` rather than the 56-year figure
/// that started this.
fn seconds_until(timeout: u32, now: u64) -> u32 {
    u64::from(timeout).saturating_sub(now).min(u64::from(u32::MAX)) as u32
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

    /// The wire carries an absolute time, so the window must show the
    /// difference. Printing the raw field gave "496397h 45m" live, which is the
    /// Unix clock rendered as a duration.
    #[test]
    fn timers_are_absolute_timestamps_not_durations() {
        let mut state = InstanceState::default();

        state.set_joined_at("Endless Tower".to_owned(), 1_787_031_900, 0, 1_787_028_300);
        assert!(
            state.display_text().contains("Time remaining: 1h 00m"),
            "{}",
            state.display_text()
        );

        // Already expired, and a duration sent by mistake, both land on zero
        // rather than on a number of years.
        assert_eq!(seconds_until(1_000, 2_000), 0);
        assert_eq!(seconds_until(3_600, 1_787_028_300), 0);
    }

    /// The label has to follow the clock, not freeze at the value it arrived
    /// with. Live 2026-08-17 it read "29m 28s" and stayed there.
    #[test]
    fn the_countdown_actually_counts_down() {
        let mut state = InstanceState::default();
        state.set_joined_at("Endless Tower".to_owned(), 0, 1_000_000 + 1800, 1_000_000);
        assert!(state.display_text().contains("30m 00s"), "{}", state.display_text());

        state.tick(1_000_000 + 60);
        assert!(state.display_text().contains("29m 00s"), "{}", state.display_text());

        // Past the timeout it floors rather than wrapping.
        state.tick(1_000_000 + 9_999);
        assert!(state.display_text().contains("0s"), "{}", state.display_text());

        // A pending instance has nothing to tick, so the frame loop can skip it.
        state.set_pending("Endless Tower".to_owned());
        assert!(!state.is_counting_down());
    }

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

        state.set_joined_at("Endless Tower".to_owned(), 1_000_000 + 3720, 0, 1_000_000);
        assert!(state.display_text().contains("Time remaining"));

        state.set_joined_at("Endless Tower".to_owned(), 0, 1_000_000 + 60, 1_000_000);
        assert!(state.display_text().contains("Idle timeout"));

        state.clear();
        assert!(state.display_text().is_empty());
    }
}
