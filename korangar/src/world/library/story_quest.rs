//! New-player quests from the renewal Izlude arrival chain.
//!
//! iRO Wiki's Izlude category names this chain (First Step, Cool Drink, the
//! guide-staff hunt, Hold Your Breath). The cells, counts, and rewards below
//! are the declarations and `getexp` / `getitem` calls in
//! `npc/re/jobs/novice/academy.txt`, not the wiki's older numbers. A quest
//! with no row here still has no map cell.

/// Where to go while a new-player quest is active, and what the script does.
#[derive(Debug, Clone, Copy)]
pub struct NewbieQuest {
    pub quest_id: u32,
    /// Visible name of the NPC the active quest is waiting on.
    pub npc: &'static str,
    pub map: &'static str,
    pub x: u16,
    pub y: u16,
    /// The NPC completes the quest. A contact who only gives directions is
    /// false.
    pub is_turn_in: bool,
    pub steps: &'static [&'static str],
}

const NEWBIE_QUESTS: &[NewbieQuest] = &[
    NewbieQuest {
        quest_id: 21001,
        npc: "Captain Carocc",
        map: "int_land",
        x: 78,
        y: 103,
        is_turn_in: true,
        steps: &[
            "The wounded swordsman on the wreck (iz_int 56,32) sends you to the captain.",
            "Captain Carocc stands on the island shore. Talking to him finishes Escape the Wreck.",
            "He grants 600 base EXP and a short Increase Agility and Blessing, then asks for lumber.",
        ],
    },
    NewbieQuest {
        quest_id: 21002,
        npc: "Sailor",
        map: "int_land",
        x: 58,
        y: 69,
        is_turn_in: true,
        steps: &[
            "Porings on the island drop Lumber. Bring 2 to the sailor on the south shore.",
            "He grants 600 base EXP and 5 Magnifiers. The ship can then leave for Izlude.",
        ],
    },
    NewbieQuest {
        quest_id: 7471,
        npc: "Lumin",
        map: "int_land",
        x: 73,
        y: 100,
        is_turn_in: true,
        steps: &[
            "Talk to Lumin beside the captain. That conversation finishes this quest.",
            "Leave the island through the shining portal on the ship. Captain Carocc continues at the Izlude harbor.",
        ],
    },
    NewbieQuest {
        quest_id: 7472,
        npc: "Hun",
        map: "izlude",
        x: 122,
        y: 207,
        is_turn_in: true,
        steps: &[
            "Captain Carocc at the Izlude harbor (izlude 198,213) sends you to Hun.",
            "Hun is at the Criatura Academy entrance. He finishes this quest.",
            "He grants 200 base EXP and 1 Apple Juice, then starts Cool drink.",
        ],
    },
    NewbieQuest {
        quest_id: 7473,
        npc: "Hun",
        map: "izlude",
        x: 122,
        y: 207,
        is_turn_in: true,
        steps: &[
            "Drink the Apple Juice, then talk to Hun again.",
            "He grants 30 Novice Potions and 200 base EXP.",
            "The Academy is north of him. The receptionist is inside at iz_ac01 100,39.",
            "Information Staff stands next to Hun at izlude 120,207 and can mark places in town.",
        ],
    },
    NewbieQuest {
        quest_id: 7474,
        npc: "Information Staff",
        map: "izlude",
        x: 120,
        y: 207,
        is_turn_in: true,
        steps: &[
            "Read the three boards, in any order: the airship board at izlude 179,75, the arena board at izlude 207,167, and the Prontera \
             field board at izlude 45,94.",
            "Lumin reaches each board first. Come back here after all three.",
            "The staff grants 300 base EXP, 20 job EXP, 20 Novice Fly Wings, 10 Novice Butterfly Wings, and 20 Novice Potions.",
        ],
    },
    NewbieQuest {
        quest_id: 7475,
        npc: "Information Staff",
        map: "izlude",
        x: 120,
        y: 207,
        is_turn_in: true,
        steps: &[
            "You have read the airship board at izlude 179,75.",
            "Read any board you have not visited yet, then return to Information Staff.",
        ],
    },
    NewbieQuest {
        quest_id: 7476,
        npc: "Information Staff",
        map: "izlude",
        x: 120,
        y: 207,
        is_turn_in: true,
        steps: &[
            "You have read the arena board at izlude 207,167.",
            "Read any board you have not visited yet, then return to Information Staff.",
        ],
    },
    NewbieQuest {
        quest_id: 7477,
        npc: "Information Staff",
        map: "izlude",
        x: 120,
        y: 207,
        is_turn_in: true,
        steps: &[
            "You have read the Prontera field board at izlude 45,94.",
            "Read any board you have not visited yet, then return to Information Staff.",
        ],
    },
    NewbieQuest {
        quest_id: 15001,
        npc: "Instructor Argos",
        map: "izlude",
        x: 140,
        y: 260,
        is_turn_in: true,
        steps: &[
            "Instructor Argos is on the ground just east of the Academy entrance.",
            "Ask to learn Play Dead. Wait 20 seconds, then talk to him again. Talking sooner starts the 20 seconds over.",
            "He teaches Play Dead. This quest grants no item and no EXP.",
        ],
    },
];

/// The new-player row for a quest id, if this chain includes it.
pub fn newbie_quest_guide(quest_id: u32) -> Option<&'static NewbieQuest> {
    NEWBIE_QUESTS.iter().find(|quest| quest.quest_id == quest_id)
}

#[cfg(test)]
mod tests {
    use super::newbie_quest_guide;

    #[test]
    fn first_step_routes_to_hun_at_the_academy_entrance() {
        let quest = newbie_quest_guide(7472).expect("First step towards a new world");
        assert_eq!(quest.npc, "Hun");
        assert_eq!((quest.map, quest.x, quest.y), ("izlude", 122, 207));
        assert!(quest.is_turn_in);
        let text = quest.steps.join("\n");
        assert!(text.contains("200 base EXP"));
        assert!(text.contains("198,213"));
        assert!(!text.contains("62, 51"));
    }

    #[test]
    fn cool_drink_and_the_treasure_hunt_use_the_script_rewards() {
        let drink = newbie_quest_guide(7473).unwrap().steps.join("\n");
        assert!(drink.contains("30 Novice Potions"));
        assert!(drink.contains("200 base EXP"));
        assert!(drink.contains("iz_ac01 100,39"));
        assert!(!drink.contains("300 base"));

        let hunt = newbie_quest_guide(7474).unwrap().steps.join("\n");
        assert!(hunt.contains("izlude 179,75"));
        assert!(hunt.contains("izlude 207,167"));
        assert!(hunt.contains("izlude 45,94"));
        assert!(hunt.contains("10 Novice Butterfly Wings"));
        assert!(hunt.contains("20 Novice Fly Wings"));
        assert!(hunt.contains("300 base EXP"));
        assert!(hunt.contains("20 job EXP"));
    }

    #[test]
    fn the_wreck_and_play_dead_name_their_script_cells() {
        let wreck = newbie_quest_guide(21001).unwrap();
        assert_eq!((wreck.map, wreck.x, wreck.y), ("int_land", 78, 103));
        let lumber = newbie_quest_guide(21002).unwrap();
        assert_eq!((lumber.npc, lumber.map, lumber.x, lumber.y), ("Sailor", "int_land", 58, 69));
        assert!(lumber.steps.join("\n").contains("2"));
        let argos = newbie_quest_guide(15001).unwrap();
        assert_eq!(
            (argos.npc, argos.map, argos.x, argos.y),
            ("Instructor Argos", "izlude", 140, 260)
        );
        assert!(argos.steps.join("\n").contains("20 seconds"));
    }

    #[test]
    fn a_quest_outside_the_chain_has_no_cell() {
        assert!(newbie_quest_guide(1).is_none());
        assert!(newbie_quest_guide(4269).is_none());
    }
}
