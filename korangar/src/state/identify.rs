//! Item identify dialog state (magnifier / MC_IDENTIFY skill).

use korangar_interface::element::StateElement;
use ragnarok_packets::InventoryIndex;
use rust_state::RustState;

#[derive(Clone, Debug, Default, RustState, StateElement)]
pub struct IdentifyState {
    /// Inventory indices awaiting selection (server +2 encoded).
    indices: Vec<InventoryIndex>,
    display_text: String,
}

#[allow(dead_code)]
impl IdentifyState {
    pub fn is_open(&self) -> bool {
        !self.indices.is_empty()
    }

    pub fn indices(&self) -> &[InventoryIndex] {
        &self.indices
    }

    pub fn display_text(&self) -> &str {
        &self.display_text
    }

    pub fn set_list(&mut self, indices: Vec<InventoryIndex>) {
        self.indices = indices;
        if self.indices.is_empty() {
            self.display_text = "No items to identify.".to_owned();
        } else {
            self.display_text = format!(
                "Select an item to identify ({} candidates).\nDouble-click an unidentified item, or use /identify <index>.",
                self.indices.len()
            );
        }
    }

    pub fn clear(&mut self) {
        self.indices.clear();
        self.display_text.clear();
    }
}
