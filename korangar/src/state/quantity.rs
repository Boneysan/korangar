//! Exact-quantity chooser used by drop, trade, and later vendor flows.
//!
//! The model is independent of rendering: keyboard text, arrow steps, min 1,
//! stack maximum, cancel, invalid-input feedback, and overflow are all
//! asserted here. Consumers only receive a confirmed amount through
//! [`QuantityChooser::confirm`].
//!
//! Wired by QW-041 (drop) and QW-042 (trade).

use std::fmt;

use korangar_interface::element::StateElement;
use ragnarok_packets::InventoryIndex;
use rust_state::RustState;

/// Why a chooser cannot open or cannot confirm.
#[derive(Clone, Debug, PartialEq, Eq, RustState)]
pub enum QuantityError {
    /// Stack is empty; there is no legal amount.
    NothingAvailable,
    /// The draft is empty or not a base-10 integer.
    InvalidInput,
    /// Parsed amount is 0.
    BelowMinimum,
    /// Parsed amount is greater than the available stack.
    AboveMaximum { requested: u32, maximum: u32 },
    /// The player dismissed the chooser.
    Cancelled,
}

impl fmt::Display for QuantityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NothingAvailable => write!(f, "Nothing to choose a quantity from."),
            Self::InvalidInput => write!(f, "Enter a whole number."),
            Self::BelowMinimum => write!(f, "Amount must be at least 1."),
            Self::AboveMaximum { requested, maximum } => {
                write!(f, "{requested} is more than the {maximum} available.")
            }
            Self::Cancelled => write!(f, "Quantity choice cancelled."),
        }
    }
}

/// In-progress exact quantity choice for one stack.
#[derive(Clone, Debug, RustState, StateElement)]
pub struct QuantityChooser {
    maximum: u32,
    value: u32,
    draft: String,
    error: Option<QuantityError>,
    cancelled: bool,
}

impl QuantityChooser {
    /// Open a chooser for `maximum` available items. Starts at 1.
    pub fn open(maximum: u32) -> Result<Self, QuantityError> {
        if maximum == 0 {
            return Err(QuantityError::NothingAvailable);
        }
        Ok(Self {
            maximum,
            value: 1,
            draft: "1".to_owned(),
            error: None,
            cancelled: false,
        })
    }

    pub fn maximum(&self) -> u32 {
        self.maximum
    }

    pub fn value(&self) -> u32 {
        self.value
    }

    pub fn draft(&self) -> &str {
        &self.draft
    }

    pub fn error(&self) -> Option<&QuantityError> {
        self.error.as_ref()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Feedback string for the UI, if the last edit was invalid.
    pub fn error_text(&self) -> Option<String> {
        self.error.as_ref().map(ToString::to_string)
    }

    /// Arrow up: one more, clamped to the stack.
    pub fn increment(&mut self) {
        if self.cancelled {
            return;
        }
        let next = self.value.saturating_add(1).min(self.maximum);
        self.set_value(next);
    }

    /// Arrow down: one fewer, not below 1.
    pub fn decrement(&mut self) {
        if self.cancelled {
            return;
        }
        let next = self.value.saturating_sub(1).max(1);
        self.set_value(next);
    }

    /// Jump to the full stack.
    pub fn set_all(&mut self) {
        if self.cancelled {
            return;
        }
        self.set_value(self.maximum);
    }

    /// Apply typed text. Invalid drafts keep the last good value and set
    /// feedback; overflow is reported rather than silently wrapping.
    pub fn set_from_text(&mut self, text: &str) {
        if self.cancelled {
            return;
        }
        self.draft = text.to_owned();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.error = Some(QuantityError::InvalidInput);
            return;
        }
        if !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
            self.error = Some(QuantityError::InvalidInput);
            return;
        }
        match trimmed.parse::<u32>() {
            Ok(0) => {
                self.error = Some(QuantityError::BelowMinimum);
            }
            Ok(parsed) if parsed > self.maximum => {
                self.error = Some(QuantityError::AboveMaximum {
                    requested: parsed,
                    maximum: self.maximum,
                });
            }
            Ok(parsed) => {
                self.value = parsed;
                self.error = None;
            }
            Err(_) => {
                self.error = Some(QuantityError::AboveMaximum {
                    requested: u32::MAX,
                    maximum: self.maximum,
                });
            }
        }
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.error = Some(QuantityError::Cancelled);
    }

    /// Confirmed amount, or the reason it must not be submitted.
    pub fn confirm(&self) -> Result<u32, QuantityError> {
        if self.cancelled {
            return Err(QuantityError::Cancelled);
        }
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        if self.value == 0 {
            return Err(QuantityError::BelowMinimum);
        }
        if self.value > self.maximum {
            return Err(QuantityError::AboveMaximum {
                requested: self.value,
                maximum: self.maximum,
            });
        }
        Ok(self.value)
    }

    fn set_value(&mut self, value: u32) {
        self.value = value;
        self.draft = value.to_string();
        self.error = None;
    }
}

/// What confirming the chooser will do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, RustState, StateElement)]
pub enum QuantityPurpose {
    #[default]
    Drop,
    Trade,
}

/// Client-owned quantity dialog.
#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct QuantityState {
    chooser: Option<QuantityChooser>,
    inventory_index: Option<InventoryIndex>,
    purpose: QuantityPurpose,
    title: String,
}

impl QuantityState {
    pub fn is_open(&self) -> bool {
        self.chooser.is_some()
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn chooser(&self) -> Option<&QuantityChooser> {
        self.chooser.as_ref()
    }

    pub fn chooser_mut(&mut self) -> Option<&mut QuantityChooser> {
        self.chooser.as_mut()
    }

    pub fn purpose(&self) -> QuantityPurpose {
        self.purpose
    }

    pub fn inventory_index(&self) -> Option<InventoryIndex> {
        self.inventory_index
    }

    pub fn display_text(&self) -> String {
        match &self.chooser {
            None => String::new(),
            Some(chooser) => {
                let mut text = format!("Amount: {} / {}", chooser.draft(), chooser.maximum());
                if let Some(error) = chooser.error_text() {
                    text.push('\n');
                    text.push_str(&error);
                }
                text
            }
        }
    }

    pub fn open_drop(&mut self, inventory_index: InventoryIndex, maximum: u32, item_name: &str) -> Result<(), QuantityError> {
        self.open(inventory_index, maximum, item_name, QuantityPurpose::Drop)
    }

    pub fn open_trade(&mut self, inventory_index: InventoryIndex, maximum: u32, item_name: &str) -> Result<(), QuantityError> {
        self.open(inventory_index, maximum, item_name, QuantityPurpose::Trade)
    }

    fn open(
        &mut self,
        inventory_index: InventoryIndex,
        maximum: u32,
        item_name: &str,
        purpose: QuantityPurpose,
    ) -> Result<(), QuantityError> {
        let chooser = QuantityChooser::open(maximum)?;
        self.chooser = Some(chooser);
        self.inventory_index = Some(inventory_index);
        self.purpose = purpose;
        self.title = match purpose {
            QuantityPurpose::Drop => format!("Drop {item_name}"),
            QuantityPurpose::Trade => format!("Trade {item_name}"),
        };
        Ok(())
    }

    pub fn clear(&mut self) {
        self.chooser = None;
        self.inventory_index = None;
        self.title.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_open_for_empty_stack() {
        assert!(matches!(QuantityChooser::open(0), Err(QuantityError::NothingAvailable)));
    }

    #[test]
    fn starts_at_one() {
        let chooser = QuantityChooser::open(10).unwrap();
        assert_eq!(chooser.confirm(), Ok(1));
    }

    #[test]
    fn arrows_stay_inside_one_and_maximum() {
        let mut chooser = QuantityChooser::open(3).unwrap();
        chooser.decrement();
        assert_eq!(chooser.confirm(), Ok(1));
        chooser.increment();
        chooser.increment();
        chooser.increment();
        chooser.increment();
        assert_eq!(chooser.confirm(), Ok(3));
        chooser.set_all();
        assert_eq!(chooser.confirm(), Ok(3));
    }

    #[test]
    fn keyboard_middle_value_confirms() {
        let mut chooser = QuantityChooser::open(10).unwrap();
        chooser.set_from_text("5");
        assert_eq!(chooser.confirm(), Ok(5));
        assert!(chooser.error().is_none());
    }

    #[test]
    fn keyboard_zero_cannot_submit() {
        let mut chooser = QuantityChooser::open(10).unwrap();
        chooser.set_from_text("0");
        assert_eq!(chooser.confirm(), Err(QuantityError::BelowMinimum));
        assert_eq!(chooser.error_text().as_deref(), Some("Amount must be at least 1."));
    }

    #[test]
    fn overflow_cannot_submit() {
        let mut chooser = QuantityChooser::open(10).unwrap();
        chooser.set_from_text("11");
        assert_eq!(
            chooser.confirm(),
            Err(QuantityError::AboveMaximum {
                requested: 11,
                maximum: 10
            })
        );
        chooser.set_from_text("9999999999");
        assert!(matches!(
            chooser.confirm(),
            Err(QuantityError::AboveMaximum { maximum: 10, .. })
        ));
    }

    #[test]
    fn invalid_input_cannot_submit() {
        let mut chooser = QuantityChooser::open(10).unwrap();
        chooser.set_from_text("abc");
        assert_eq!(chooser.confirm(), Err(QuantityError::InvalidInput));
        chooser.set_from_text("");
        assert_eq!(chooser.confirm(), Err(QuantityError::InvalidInput));
        chooser.set_from_text("1.5");
        assert_eq!(chooser.confirm(), Err(QuantityError::InvalidInput));
    }

    #[test]
    fn cancel_blocks_submit_and_later_edits() {
        let mut chooser = QuantityChooser::open(10).unwrap();
        chooser.set_from_text("4");
        chooser.cancel();
        assert_eq!(chooser.confirm(), Err(QuantityError::Cancelled));
        chooser.set_from_text("2");
        chooser.increment();
        assert_eq!(chooser.confirm(), Err(QuantityError::Cancelled));
        assert!(chooser.is_cancelled());
    }

    #[test]
    fn recovering_from_invalid_input_allows_submit() {
        let mut chooser = QuantityChooser::open(8).unwrap();
        chooser.set_from_text("nope");
        assert!(chooser.confirm().is_err());
        chooser.set_from_text("8");
        assert_eq!(chooser.confirm(), Ok(8));
    }

    #[test]
    fn trade_purpose_opens_the_same_chooser() {
        let mut state = QuantityState::default();
        state.open_trade(InventoryIndex(3), 10, "Red Potion").unwrap();
        assert_eq!(state.purpose(), QuantityPurpose::Trade);
        assert_eq!(state.inventory_index(), Some(InventoryIndex(3)));
        assert_eq!(state.chooser().unwrap().confirm(), Ok(1));
        state.chooser_mut().unwrap().set_all();
        assert_eq!(state.chooser().unwrap().confirm(), Ok(10));
    }
}
