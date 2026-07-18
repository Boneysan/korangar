use korangar_interface::element::StateElement;
use ragnarok_packets::{CharacterId, CharacterInformation};
use rust_state::{Path, PathExt, RustState, Selector};

use crate::state::ClientState;

#[derive(Default, RustState, StateElement)]
pub struct CharacterSlots {
    slots: Vec<Option<CharacterInformation>>,
}

impl CharacterSlots {
    pub fn set_slot_count(&mut self, slot_count: usize) {
        self.slots.resize(slot_count, None);
    }

    pub fn get_slot_count(&self) -> usize {
        self.slots.len()
    }

    pub fn add_character(&mut self, character_information: CharacterInformation) {
        let Some(slot) = self.slots.get_mut(character_information.character_number as usize) else {
            panic!("attempted to add character to a slot that doesn't exist");
        };

        assert!(slot.is_none(), "attempted to add a character to an occupied slot");

        *slot = Some(character_information);
    }

    pub fn remove_with_id(&mut self, character_id: CharacterId) {
        self.slots.iter_mut().for_each(|slot| {
            if slot
                .as_ref()
                .is_some_and(|character_information| character_information.character_id == character_id)
            {
                *slot = None;
            }
        })
    }

    pub fn with_id(&self, character_id: CharacterId) -> Option<&CharacterInformation> {
        self.slots
            .iter()
            .find(|slot| {
                slot.as_ref()
                    .is_some_and(|character_information| character_information.character_id == character_id)
            })
            .and_then(|slot| slot.as_ref())
    }

    /// Replace the slot contents with `characters`.
    ///
    /// An **empty** list is ignored rather than treated as "no characters"
    /// (M1-013). With exactly 3 characters on the account, Hercules sends
    /// the list and then a *second, empty* `0x0B72` — `char.c`: *"send
    /// empty packet if chars count is 3, for trigger final code in
    /// client"*, a quirk of the official client's pagination. The
    /// two are indistinguishable by content, so without this guard the
    /// terminator wipes the list we just populated and the character select
    /// renders empty.
    ///
    /// Ignoring it is safe for an account that genuinely has no characters:
    /// slots are `None` from [`Self::set_slot_count`], so there is nothing
    /// to preserve and the outcome is identical either way.
    pub fn set_characters(&mut self, characters: Vec<CharacterInformation>) {
        if characters.is_empty() {
            return;
        }

        // Clear the character list.
        self.slots.iter_mut().for_each(|slot| *slot = None);

        characters
            .into_iter()
            .for_each(|character_information| self.add_character(character_information));
    }
}

#[derive(Clone, Copy)]
struct SlotPath<P>
where
    P: Copy,
{
    path: P,
    slot: usize,
}

impl<P> Path<ClientState, CharacterInformation, false> for SlotPath<P>
where
    P: Path<ClientState, CharacterSlots>,
{
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a CharacterInformation> {
        self.path.follow_safe(state).slots.get(self.slot).and_then(|slot| slot.as_ref())
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut CharacterInformation> {
        self.path
            .follow_mut_safe(state)
            .slots
            .get_mut(self.slot)
            .and_then(|slot| slot.as_mut())
    }
}

impl<P> Selector<ClientState, CharacterInformation, false> for SlotPath<P>
where
    P: Path<ClientState, CharacterSlots>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a CharacterInformation> {
        self.follow(state)
    }
}

pub trait CharacterSlotsExt {
    fn in_slot(self, slot: usize) -> impl Path<ClientState, CharacterInformation, false>;
}

impl<P> CharacterSlotsExt for P
where
    P: Path<ClientState, CharacterSlots>,
{
    fn in_slot(self, slot: usize) -> impl Path<ClientState, CharacterInformation, false> {
        SlotPath { path: self, slot }
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{JobId, Sex};

    use super::*;

    /// Minimal character in `slot`. `CharacterInformation` has no `Default`
    /// (and adding one would mean picking a default `Sex` in the shared
    /// protocol crate), so the fields are spelled out; only
    /// `character_number` and `name` matter here.
    fn character(slot: u8, name: &str) -> CharacterInformation {
        CharacterInformation {
            character_id: CharacterId(150_000 + slot as u32),
            experience: 0,
            money: 0,
            job_experience: 0,
            job_level: 1,
            body_state: 0,
            health_state: 0,
            effect_state: 0,
            virtue: 0,
            honor: 0,
            stat_points: 0,
            health_points: 40,
            maximum_health_points: 40,
            spell_points: 11,
            maximum_spell_points: 11,
            movement_speed: 150,
            job_id: JobId(0),
            head: 0,
            body: 0,
            weapon: 0,
            base_level: 1,
            sp_point: 0,
            accessory: 0,
            shield: 0,
            accessory2: 0,
            accessory3: 0,
            head_palette: 0,
            body_palette: 0,
            name: name.to_owned(),
            strength: 1,
            agility: 1,
            vitality: 1,
            intelligence: 1,
            dexterity: 1,
            luck: 1,
            character_number: slot,
            hair_color: 0,
            b_is_changed_char: 0,
            map_name: "new_1-1".to_owned(),
            deletion_reverse_date: 0,
            robe_palette: 0,
            character_slot_change_count: 0,
            character_name_change_count: 0,
            sex: Sex::Male,
        }
    }

    fn occupied_names(slots: &CharacterSlots) -> Vec<String> {
        slots
            .slots
            .iter()
            .filter_map(|slot| slot.as_ref().map(|character| character.name.clone()))
            .collect()
    }

    #[test]
    fn set_characters_populates_by_slot() {
        let mut slots = CharacterSlots::default();
        slots.set_slot_count(12);
        slots.set_characters(vec![character(0, "test"), character(2, "yoyo")]);

        assert_eq!(occupied_names(&slots), vec!["test", "yoyo"]);
        assert!(slots.slots[1].is_none(), "gap between slots must stay empty");
    }

    #[test]
    fn empty_list_does_not_wipe_existing_characters() {
        // M1-013: with exactly 3 characters Hercules sends the list, then a *second,
        // empty* 0x0B72 as an end-of-pagination marker (char.c: "send empty packet if
        // chars count is 3, for trigger final code in client"). The two are
        // indistinguishable by content, so an unguarded set_characters wiped the list
        // that had just been populated and the character select rendered empty.
        let mut slots = CharacterSlots::default();
        slots.set_slot_count(12);
        slots.set_characters(vec![character(0, "test"), character(1, "HlTmp9686"), character(2, "yoyo")]);
        assert_eq!(occupied_names(&slots).len(), 3);

        slots.set_characters(Vec::new()); // the terminator

        assert_eq!(
            occupied_names(&slots),
            vec!["test", "HlTmp9686", "yoyo"],
            "the empty terminator must not clear the list"
        );
    }

    #[test]
    fn empty_list_is_harmless_with_no_characters() {
        // An account with genuinely no characters gets a single empty list. Ignoring it
        // is identical to applying it, because slots start empty.
        let mut slots = CharacterSlots::default();
        slots.set_slot_count(12);
        slots.set_characters(Vec::new());

        assert!(occupied_names(&slots).is_empty());
        assert_eq!(slots.get_slot_count(), 12);
    }
}
