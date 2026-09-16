//! Personal chest discovery. Opened state is server-authoritative, not
//! click-local.

use std::collections::HashSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChestBook {
    discovered: HashSet<u32>,
    opened: HashSet<u32>,
}

impl ChestBook {
    pub fn visual(&self, chest_id: u32) -> &'static str {
        if self.opened.contains(&chest_id) {
            "opened"
        } else if self.discovered.contains(&chest_id) {
            "available"
        } else {
            "unopened"
        }
    }

    pub fn discover(&mut self, chest_id: u32) {
        self.discovered.insert(chest_id);
    }

    pub fn mark_opened_from_server(&mut self, chest_id: u32) {
        self.discovered.insert(chest_id);
        self.opened.insert(chest_id);
    }

    pub fn click_does_not_open(&mut self, chest_id: u32) {
        self.discover(chest_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_states() {
        let mut book = ChestBook::default();
        assert_eq!(book.visual(1), "unopened");
        book.discover(1);
        assert_eq!(book.visual(1), "available");
        book.click_does_not_open(1);
        assert_eq!(book.visual(1), "available");
        book.mark_opened_from_server(1);
        assert_eq!(book.visual(1), "opened");
        let other = ChestBook::default();
        assert_eq!(other.visual(1), "unopened");
    }
}
