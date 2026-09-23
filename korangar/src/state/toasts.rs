//! Bounded HUD notices (GDD slice 1).
//!
//! At most two toasts are visible. A third waits in a short queue, or
//! replaces an existing toast that shares its coalesce key. Callers still
//! write the same text into chat; this queue is only the on-screen notice.

use korangar_interface::element::StateElement;
use rust_state::RustState;

/// How many notices may be on screen at once.
pub const MAX_VISIBLE: usize = 2;
/// Notices waiting behind the visible pair.
const MAX_QUEUED: usize = 8;
/// Default on-screen lifetime.
pub const DEFAULT_TTL_MS: u64 = 4_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToastPriority {
    Normal = 0,
    High = 1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    pub key: String,
    pub summary: String,
    pub priority: ToastPriority,
    /// Milliseconds remaining while this toast is visible.
    pub remaining_ms: u64,
}

#[derive(Clone, Debug, RustState, StateElement)]
pub struct ToastQueue {
    #[hidden_element]
    visible: Vec<Toast>,
    #[hidden_element]
    waiting: Vec<Toast>,
    display_text: String,
    #[hidden_element]
    last_now_ms: u64,
}

impl Default for ToastQueue {
    fn default() -> Self {
        Self {
            visible: Vec::new(),
            waiting: Vec::new(),
            display_text: String::new(),
            last_now_ms: 0,
        }
    }
}

impl ToastQueue {
    pub fn push(&mut self, key: impl Into<String>, summary: impl Into<String>, priority: ToastPriority) {
        self.push_with_ttl(key, summary, priority, DEFAULT_TTL_MS);
    }

    pub fn push_with_ttl(
        &mut self,
        key: impl Into<String>,
        summary: impl Into<String>,
        priority: ToastPriority,
        ttl_ms: u64,
    ) {
        let key = key.into();
        let summary = summary.into();
        if let Some(existing) = self
            .visible
            .iter_mut()
            .chain(self.waiting.iter_mut())
            .find(|toast| toast.key == key)
        {
            existing.summary = summary;
            existing.priority = priority;
            existing.remaining_ms = ttl_ms;
            self.rebuild_display();
            return;
        }

        let toast = Toast {
            key,
            summary,
            priority,
            remaining_ms: ttl_ms,
        };
        if self.visible.len() < MAX_VISIBLE {
            self.visible.push(toast);
            self.sort_visible();
        } else if self.waiting.len() < MAX_QUEUED {
            self.waiting.push(toast);
        } else if let Some(lowest) = self
            .waiting
            .iter()
            .enumerate()
            .min_by_key(|(_, toast)| toast.priority)
            .map(|(index, _)| index)
        {
            if priority > self.waiting[lowest].priority {
                self.waiting[lowest] = toast;
            }
        }
        self.rebuild_display();
    }

    pub fn clear(&mut self) {
        self.visible.clear();
        self.waiting.clear();
        self.display_text.clear();
    }

    /// Advance timers from a monotonic millisecond clock (server client tick).
    pub fn tick(&mut self, now_ms: u64) {
        let elapsed_ms = if self.last_now_ms == 0 {
            0
        } else {
            now_ms.saturating_sub(self.last_now_ms)
        };
        self.last_now_ms = now_ms;
        if elapsed_ms == 0 || self.visible.is_empty() {
            return;
        }
        for toast in &mut self.visible {
            toast.remaining_ms = toast.remaining_ms.saturating_sub(elapsed_ms);
        }
        self.visible.retain(|toast| toast.remaining_ms > 0);
        while self.visible.len() < MAX_VISIBLE {
            if self.waiting.is_empty() {
                break;
            }
            let next = self.waiting.remove(0);
            self.visible.push(next);
        }
        self.sort_visible();
        self.rebuild_display();
    }

    pub fn visible(&self) -> &[Toast] {
        &self.visible
    }

    fn sort_visible(&mut self) {
        self.visible.sort_by(|left, right| right.priority.cmp(&left.priority));
    }

    fn rebuild_display(&mut self) {
        self.display_text = self
            .visible
            .iter()
            .map(|toast| toast.summary.as_str())
            .collect::<Vec<_>>()
            .join("\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn third_notice_waits_until_one_expires() {
        let mut queue = ToastQueue::default();
        queue.push("a", "one", ToastPriority::Normal);
        queue.push("b", "two", ToastPriority::Normal);
        queue.push("c", "three", ToastPriority::Normal);
        assert_eq!(queue.visible().len(), 2);
        assert!(queue.visible().iter().all(|toast| toast.summary != "three"));

        queue.tick(1);
        queue.tick(1 + DEFAULT_TTL_MS);
        let summaries: Vec<_> = queue.visible().iter().map(|toast| toast.summary.as_str()).collect();
        assert!(summaries.contains(&"three"));
    }

    #[test]
    fn same_key_coalesces_instead_of_stacking() {
        let mut queue = ToastQueue::default();
        queue.push("drop", "Poring Card", ToastPriority::High);
        queue.push("drop", "Poring Card x2", ToastPriority::High);
        assert_eq!(queue.visible().len(), 1);
        assert_eq!(queue.visible()[0].summary, "Poring Card x2");
    }
}
