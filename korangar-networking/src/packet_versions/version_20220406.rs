use std::cell::RefCell;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Instant;

use ragnarok_packets::handler::{DuplicateHandlerError, PacketCallback, PacketHandler};
use ragnarok_packets::*;

use crate::event::{NetworkEventList, NoNetworkEvents};
use crate::items::{IT_AMMO, ItemQuantity};
use crate::{
    CharacterServerLoginData, HotkeyState, InventoryItem, InventoryItemDetails, LoginServerLoginData, MessageColor, NetworkEvent,
    NoMetadata, ShopItem, UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
};

pub fn register_login_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|packet: LoginServerLoginSuccessPacket| NetworkEvent::LoginServerConnected {
        character_servers: packet.character_server_information,
        login_data: LoginServerLoginData {
            account_id: packet.account_id,
            login_id1: packet.login_id1,
            login_id2: packet.login_id2,
            sex: packet.sex,
        },
    })?;
    packet_handler.register(|packet: LoginFailedPacket| {
        let (reason, message) = match packet.reason {
            LoginFailedReason::ServerClosed => (UnifiedLoginFailedReason::ServerClosed, "Server closed"),
            LoginFailedReason::AlreadyLoggedIn => (
                UnifiedLoginFailedReason::AlreadyLoggedIn,
                "Someone has already logged in with this id",
            ),
            // A stale session (e.g. a failed map handoff) leaves the account
            // flagged online; this login attempt has asked the server to
            // clear it, so a retry is expected to succeed.
            LoginFailedReason::AlreadyOnline => (
                UnifiedLoginFailedReason::AlreadyOnline,
                "Account was still flagged online - the server is clearing it, try logging in again",
            ),
        };

        NetworkEvent::LoginServerConnectionFailed { reason, message }
    })?;
    packet_handler.register(|packet: LoginFailedPacket2| {
        let (reason, message) = unify_login_failed_reason2(packet.reason);
        NetworkEvent::LoginServerConnectionFailed { reason, message }
    })?;
    // Servers built with PACKETVER >= 20180627 (like our Hercules) send the
    // same refusal under the AC_REFUSE_LOGIN_R3 header instead.
    packet_handler.register(|packet: LoginFailedPacket3| {
        let (reason, message) = unify_login_failed_reason2(packet.reason);
        NetworkEvent::LoginServerConnectionFailed { reason, message }
    })?;

    // Safety net: consume any known-length packet without a dedicated handler
    // instead of desyncing the read buffer (0x0B02 used to be exactly such a
    // packet — a rejected login was silently swallowed). Registered last so it
    // never shadows a real handler.
    packet_handler.register_length_fallbacks(super::lengths_20220406::PACKET_LENGTHS);

    Ok(())
}

fn unify_login_failed_reason2(reason: LoginFailedReason2) -> (UnifiedLoginFailedReason, &'static str) {
    match reason {
        LoginFailedReason2::UnregisteredId => (UnifiedLoginFailedReason::UnregisteredId, "Incorrect username or password"),
        LoginFailedReason2::IncorrectPassword => (UnifiedLoginFailedReason::IncorrectPassword, "Incorrect username or password"),
        LoginFailedReason2::IdExpired => (UnifiedLoginFailedReason::IdExpired, "Id has expired"),
        LoginFailedReason2::RejectedFromServer => (UnifiedLoginFailedReason::RejectedFromServer, "Rejected from server"),
        LoginFailedReason2::BlockedByGMTeam => (UnifiedLoginFailedReason::BlockedByGMTeam, "Blocked by gm team"),
        LoginFailedReason2::GameOutdated => (UnifiedLoginFailedReason::GameOutdated, "Game outdated"),
        LoginFailedReason2::LoginProhibitedUntil => (UnifiedLoginFailedReason::LoginProhibitedUntil, "Login prohibited until"),
        LoginFailedReason2::ServerFull => (UnifiedLoginFailedReason::ServerFull, "Server is full"),
        LoginFailedReason2::CompanyAccountLimitReached => (
            UnifiedLoginFailedReason::CompanyAccountLimitReached,
            "Company account limit reached",
        ),
    }
}

pub fn register_character_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|packet: LoginFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            LoginFailedReason::ServerClosed => "Server closed",
            LoginFailedReason::AlreadyLoggedIn => "Someone has already logged in with this id",
            LoginFailedReason::AlreadyOnline => "Already online",
        };

        NetworkEvent::CharacterServerConnectionFailed { reason, message }
    })?;
    packet_handler.register(
        |packet: CharacterServerLoginSuccessPacket| NetworkEvent::CharacterServerConnected {
            normal_slot_count: packet.normal_slot_count as usize,
        },
    )?;
    packet_handler.register(|packet: RequestCharacterListSuccessPacket| NetworkEvent::CharacterList {
        characters: packet.character_information,
    })?;
    // Servers built with PACKETVER < 20201007 (like our Hercules at 20190605)
    // send the character list under the older header with 155-byte entries.
    packet_handler.register(|packet: RequestCharacterListLegacySuccessPacket| NetworkEvent::CharacterList {
        characters: packet.character_information.into_iter().map(Into::into).collect(),
    })?;
    packet_handler.register_noop::<CharacterListPacket>()?;
    packet_handler.register_noop::<CharacterSlotPagePacket>()?;
    packet_handler.register_noop::<CharacterBanListPacket>()?;
    packet_handler.register_noop::<LoginPincodePacket>()?;
    packet_handler.register_noop::<Packet0b18>()?;
    packet_handler.register(|packet: CharacterSelectionSuccessPacket| {
        let login_data = CharacterServerLoginData {
            server_ip: IpAddr::V4(packet.map_server_ip.into()),
            server_port: packet.map_server_port,
            character_id: packet.character_id,
        };

        NetworkEvent::CharacterSelected { login_data }
    })?;
    packet_handler.register(|packet: CharacterSelectionFailedPacket| {
        let (reason, message) = match packet.reason {
            CharacterSelectionFailedReason::RejectedFromServer => (
                UnifiedCharacterSelectionFailedReason::RejectedFromServer,
                "Rejected from server",
            ),
        };

        NetworkEvent::CharacterSelectionFailed { reason, message }
    })?;
    packet_handler.register(|_: MapServerUnavailablePacket| {
        let reason = UnifiedCharacterSelectionFailedReason::MapServerUnavailable;
        let message = "Map server currently unavailable";

        NetworkEvent::CharacterSelectionFailed { reason, message }
    })?;
    packet_handler.register(|packet: CreateCharacterSuccessPacket| NetworkEvent::CharacterCreated {
        character_information: packet.character_information,
    })?;
    // PACKETVER < 20201007 variant (see RequestCharacterListLegacySuccessPacket).
    packet_handler.register(|packet: CreateCharacterLegacySuccessPacket| NetworkEvent::CharacterCreated {
        character_information: packet.character_information.into(),
    })?;
    packet_handler.register(|packet: CharacterCreationFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            CharacterCreationFailedReason::CharacterNameAlreadyUsed => "Character name is already used",
            CharacterCreationFailedReason::NotOldEnough => "You are not old enough to create a character",
            CharacterCreationFailedReason::NotAllowedToUseSlot => "You are not allowed to use this character slot",
            CharacterCreationFailedReason::CharacterCerationFailed => "Character creation failed",
        };

        NetworkEvent::CharacterCreationFailed { reason, message }
    })?;
    packet_handler.register(|_: CharacterDeletionSuccessPacket| NetworkEvent::CharacterDeleted)?;
    packet_handler.register(|packet: CharacterDeletionFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            CharacterDeletionFailedReason::NotAllowed => "You are not allowed to delete this character",
            CharacterDeletionFailedReason::CharacterNotFound => "Character was not found",
            CharacterDeletionFailedReason::NotEligible => "Character is not eligible for deletion",
        };
        NetworkEvent::CharacterDeletionFailed { reason, message }
    })?;
    packet_handler.register(|packet: SwitchCharacterSlotResponsePacket| match packet.status {
        SwitchCharacterSlotResponseStatus::Success => NetworkEvent::CharacterSlotSwitched,
        SwitchCharacterSlotResponseStatus::Error => NetworkEvent::CharacterSlotSwitchFailed,
    })?;

    // Safety net: consume any known-length packet without a dedicated handler
    // instead of desyncing the read buffer (0x099D used to be exactly such a
    // packet — the character list from our PACKETVER 20190605 server was
    // dropped and the character selection screen hung forever). Registered
    // last so it never shadows a real handler.
    packet_handler.register_length_fallbacks(super::lengths_20220406::PACKET_LENGTHS);

    Ok(())
}

pub fn register_map_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    // Inventory / storage lists share the same Start → item* → End framing.
    // Transient buffer holds (inventory_type, items) until End.
    let inventory_items: Rc<RefCell<Option<(u8, Vec<InventoryItem<NoMetadata>>)>>> = Rc::new(RefCell::new(None));
    // Equipped ammunition seen while a list is still being accumulated.
    //
    // Hercules sends `clif_arrowequip` from *inside* `clif_inventoryItems`, between
    // the stackable list and the equippable list — so at login it arrives before the
    // End packet that publishes the inventory. Emitting it immediately would apply the
    // AMMO flag to the outgoing inventory and then lose it to the `SetInventory` that
    // follows, leaving the Ammo slot empty every login. Hold it and apply it at End.
    let pending_equipped_ammunition: Rc<RefCell<Option<InventoryIndex>>> = Rc::new(RefCell::new(None));

    packet_handler.register(|_: MapServerPingPacket| NoNetworkEvents)?;
    packet_handler.register(|packet: BroadcastMessagePacket| NetworkEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Broadcast,
    })?;
    packet_handler.register(|packet: Broadcast2MessagePacket| {
        // Drop the alpha channel because it might be 0.
        let color = MessageColor::Rgb {
            red: packet.font_color.red,
            green: packet.font_color.green,
            blue: packet.font_color.blue,
        };
        NetworkEvent::ChatMessage {
            text: packet.message,
            color,
        }
    })?;
    packet_handler.register(|packet: OverheadMessagePacket| {
        // FIX: This should be a different event.
        NetworkEvent::ChatMessage {
            text: packet.message,
            color: MessageColor::Broadcast,
        }
    })?;
    packet_handler.register(|packet: ServerMessagePacket| NetworkEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Server,
    })?;
    // Hercules `dispbottom` / `clif_disp_onlyself` (atcommands, @dm*, script
    // feedback).
    packet_handler.register(|packet: DisplayBottomMessagePacket| NetworkEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Server,
    })?;
    packet_handler.register(|packet: MessageTablePacket| NetworkEvent::MessageTable {
        message_id: packet.message_id,
        color: MessageColor::Error,
    })?;
    packet_handler.register(|packet: ServiceMessagePacket| NetworkEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Rgb {
            red: packet.color.red,
            green: packet.color.green,
            blue: packet.color.blue,
        },
    })?;
    packet_handler.register(|packet: MoveItemFailedPacket| NetworkEvent::ItemMoveFailed {
        item_index: packet.item_index,
        amount: packet.amount,
    })?;
    // Production skills report success and failure only here. The message ids
    // live in the same table as `MessageTablePacket`, so this reuses that event
    // rather than inventing a second rendering path; the skill id adds nothing
    // the text does not already say.
    packet_handler.register(|packet: SkillMessageTablePacket| NetworkEvent::MessageTable {
        message_id: packet.message_id as u16,
        color: MessageColor::Error,
    })?;
    packet_handler.register(|packet: MessageTableColorPacket| {
        // Hercules packs RGB in the low 24 bits (0x00RRGGBB on many clients;
        // attendance “not event” uses COLOR_RED).
        let c = packet.message_color;
        let color = MessageColor::Rgb {
            red: ((c >> 16) & 0xFF) as u8,
            green: ((c >> 8) & 0xFF) as u8,
            blue: (c & 0xFF) as u8,
        };
        NetworkEvent::MessageTable {
            message_id: packet.message_id,
            color,
        }
    })?;
    packet_handler.register_noop::<OpenUiPacket>()?;
    packet_handler.register(|packet: EntityMessagePacket| {
        // Drop the alpha channel because it might be 0.
        let color = MessageColor::Rgb {
            red: packet.color.red,
            green: packet.color.green,
            blue: packet.color.blue,
        };
        NetworkEvent::ChatMessage {
            text: packet.message,
            color,
        }
    })?;
    packet_handler.register(|packet: DisplayEmotionPacket| NetworkEvent::DisplayEmotion {
        entity_id: packet.entity_id,
        emotion: packet.emotion,
    })?;
    packet_handler.register(|packet: EntityMovePacket| {
        let EntityMovePacket {
            entity_id,
            from_to,
            starting_timestamp,
        } = packet;

        let (origin, destination) = from_to.to_origin_destination();

        NetworkEvent::EntityMove {
            entity_id,
            origin,
            destination,
            starting_timestamp,
        }
    })?;
    packet_handler.register(|packet: EntityStopMovePacket| NetworkEvent::EntityStopMove {
        entity_id: packet.entity_id,
        position: packet.position,
    })?;
    // `direction` is the wire's 0..7 facing and shares `Direction`'s numbering,
    // which is why the spawn packet can feed `Common::direction` directly.
    packet_handler.register(|packet: ChangeDirectionPacket| NetworkEvent::EntityDirection {
        entity_id: packet.entity_id,
        direction: Direction::from(packet.direction as u16),
        head_direction: packet.head_direction,
    })?;
    packet_handler.register(|packet: PlayerMovePacket| {
        let PlayerMovePacket {
            starting_timestamp,
            from_to,
        } = packet;

        let (origin, destination) = from_to.to_origin_destination();

        NetworkEvent::PlayerMove {
            origin,
            destination,
            starting_timestamp,
        }
    })?;
    packet_handler.register(|packet: ChangeMapPacket| {
        let ChangeMapPacket { map_name, position } = packet;

        // 16-byte fixed field is already null-trimmed by FromBytes; still strip
        // extension variants so loaders/minimap use a clean base name.
        let map_name = map_name
            .trim()
            .trim_end_matches('\0')
            .trim_end_matches(".gat")
            .trim_end_matches(".GAT")
            .trim_end_matches(".rsw")
            .trim_end_matches(".RSW")
            .to_owned();

        NetworkEvent::ChangeMap { map_name, position }
    })?;
    packet_handler.register(|packet: ResurrectionPacket| NetworkEvent::ResurrectPlayer {
        entity_id: packet.entity_id,
    })?;
    packet_handler.register(|packet: EntityAppearPacket| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: EntityAppear2Packet| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: MovingEntityAppearPacket| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: EntityDisAppearPacket| NetworkEvent::RemoveEntity {
        entity_id: packet.entity_id,
        reason: packet.reason,
    })?;
    packet_handler.register(|packet: GroundItemAppearPacket| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: GroundItemAppear2Packet| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: GroundItemAppear3Packet| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: GroundItemAppear4Packet| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: ItemDisappearPacket| NetworkEvent::RemoveGroundItem {
        entity_id: packet.entity_id,
    })?;
    packet_handler.register(|packet: UpdateStatPacket| {
        let UpdateStatPacket { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket1| {
        let UpdateStatPacket1 { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket2| {
        let UpdateStatPacket2 { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket3| {
        let UpdateStatPacket3 { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateAttackRangePacket| NetworkEvent::UpdateAttackRange {
        attack_range: packet.attack_range,
    })?;
    packet_handler.register_noop::<NewMailStatusPacket>()?;
    packet_handler.register_noop::<AchievementUpdatePacket>()?;
    packet_handler.register_noop::<AchievementListPacket>()?;
    packet_handler.register(|packet: CriticalWeightUpdatePacket| NetworkEvent::CriticalWeightPercent { percent: packet.weight })?;
    // This match is deliberately EXHAUSTIVE — no `_` arm. It used to end in
    // `_ => None`, which silently dropped nine of the fourteen look types the
    // server broadcasts (headgear ×3, hair colour, clothes colour, shoes, body,
    // robe, body style). Nothing downstream could see them, so no amount of
    // client testing could find it.
    //
    // Matching on a reference keeps `packet.sprite_type` available to forward.
    packet_handler.register(|packet: SpriteChangePacket| {
        let account_id = packet.account_id;
        let value = packet.value;

        match &packet.sprite_type {
            SpriteChangeType::Base => NetworkEvent::ChangeJob {
                account_id,
                job_id: JobId(value as u16),
            },
            SpriteChangeType::Hair => NetworkEvent::ChangeHair {
                account_id,
                hair_id: value,
            },
            SpriteChangeType::Weapon => NetworkEvent::ChangeWeapon {
                account_id,
                weapon_id: value,
            },
            SpriteChangeType::Shield => NetworkEvent::ChangeShield {
                account_id,
                shield_id: value,
            },
            // Korangar-fork broadcast riding the unused `LOOK_FLOOR` slot; see the
            // `Ammunition` variant in `ragnarok-packets`.
            SpriteChangeType::Ammunition => NetworkEvent::ChangeAmmunition {
                account_id,
                item_id: ItemId(value),
            },
            SpriteChangeType::HeadBottom
            | SpriteChangeType::HeadTop
            | SpriteChangeType::HeadMiddle
            | SpriteChangeType::HairCollor
            | SpriteChangeType::ClothesColor
            | SpriteChangeType::Shoes
            | SpriteChangeType::Body
            | SpriteChangeType::Robe
            | SpriteChangeType::Body2 => NetworkEvent::ChangeLook {
                account_id,
                look_type: packet.sprite_type.clone(),
                value,
            },
        }
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |packet: InventoyStartPacket| {
            *inventory_items.borrow_mut() = Some((packet.inventory_type, Vec::new()));
            NoNetworkEvents
        }
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |packet: RegularItemListPacket| {
            let mut borrowed = inventory_items.borrow_mut();
            let (inv_type, items) = borrowed.as_mut().expect("Unexpected inventory packet");
            let is_storage = matches!(*inv_type, inventory_type::STORAGE | inventory_type::GUILD_STORAGE);

            items.extend(packet.item_information.into_iter().map(|item_information| {
                let RegularItemInformation {
                    index,
                    item_id,
                    item_type,
                    amount,
                    equippable_position: _,
                    slot,
                    hire_expiration_date,
                    flags,
                } = item_information;

                let actual_index = match is_storage {
                    true => InventoryIndex(index.0.saturating_sub(1)),
                    false => InventoryIndex(index.0.saturating_sub(2)),
                };

                // Ammo is stackable but occupies the AMMO slot; the normal list
                // omits the equippable fields, so build them explicitly.
                //
                // Everything starts **unequipped**: this packet cannot say what is
                // worn. Its one slot field is the item database's mask, so every
                // arrow stack claims `AMMO` — feeding that in as the worn state
                // marked them all equipped, offered "Unequip" on stacks that were
                // not equipped, and made the *first* stack win as the active
                // ammunition. The truth arrives separately, in `EquipAmmunitionPacket`.
                let details = if item_type == IT_AMMO {
                    InventoryItemDetails::ammo(amount, EquipPosition::empty(), flags.contains(RegularItemFlags::IDENTIFIED))
                } else {
                    InventoryItemDetails::Regular {
                        amount,
                        equipped_position: EquipPosition::empty(),
                        flags,
                    }
                };

                InventoryItem {
                    index: actual_index,
                    metadata: NoMetadata,
                    item_id,
                    item_type,
                    slot,
                    hire_expiration_date,
                    details,
                }
            }));
            NoNetworkEvents
        }
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |packet: EquippableItemListPacket| {
            let mut borrowed = inventory_items.borrow_mut();
            let (inv_type, items) = borrowed.as_mut().expect("Unexpected inventory packet");
            let is_storage = matches!(*inv_type, inventory_type::STORAGE | inventory_type::GUILD_STORAGE);

            items.extend(packet.item_information.into_iter().map(|item| {
                let EquippableItemInformation {
                    index,
                    item_id,
                    item_type,
                    equip_position,
                    equipped_position,
                    slot,
                    hire_expiration_date,
                    bind_on_equip_type,
                    w_item_sprite_number,
                    option_count,
                    option_data,
                    refinement_level,
                    enchantment_level,
                    flags,
                } = item;

                let actual_index = match is_storage {
                    true => InventoryIndex(index.0.saturating_sub(1)),
                    false => InventoryIndex(index.0.saturating_sub(2)),
                };

                InventoryItem {
                    index: actual_index,
                    metadata: NoMetadata,
                    item_id,
                    item_type,
                    slot,
                    hire_expiration_date,
                    details: InventoryItemDetails::Equippable {
                        // The equip inventory list is real gear only (always 1);
                        // stackable ammo arrives via the normal list / pickup.
                        amount: 1,
                        equip_position,
                        equipped_position,
                        bind_on_equip_type,
                        w_item_sprite_number,
                        option_count,
                        option_data,
                        refinement_level,
                        enchantment_level,
                        flags,
                    },
                }
            }));
            NoNetworkEvents
        }
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();
        let pending_equipped_ammunition = pending_equipped_ammunition.clone();

        move |_packet: InventoyEndPacket| {
            let (inv_type, mut items) = inventory_items.borrow_mut().take().expect("Unexpected inventory end packet");

            // Apply the ammo equip that arrived mid-list (see the buffer's comment).
            // Storage lists never carry one, and taking it unconditionally would strand
            // a real inventory equip behind an unrelated storage window.
            if !matches!(inv_type, inventory_type::STORAGE | inventory_type::GUILD_STORAGE)
                && let Some(index) = pending_equipped_ammunition.borrow_mut().take()
                && let Some(item) = items.iter_mut().find(|item| item.index == index)
                && let InventoryItemDetails::Equippable { equipped_position, .. } = &mut item.details
            {
                *equipped_position = EquipPosition::AMMO;
            }

            match inv_type {
                inventory_type::STORAGE | inventory_type::GUILD_STORAGE => NetworkEvent::SetStorage { items },
                _ => NetworkEvent::SetInventory { items },
            }
        }
    })?;
    packet_handler.register_noop::<EquippableSwitchItemListPacket>()?;
    packet_handler.register_noop::<MapTypePacket>()?;
    packet_handler.register_noop::<EquipmentEffectPacket>()?;
    packet_handler.register_noop::<PersonalInformationPacket>()?;
    packet_handler.register_noop::<NotifyActorInitPacket>()?;
    packet_handler.register(|packet: UpdateSkillTreePacket| {
        let UpdateSkillTreePacket { skill_information } = packet;
        NetworkEvent::SkillTree { skill_information }
    })?;
    packet_handler.register(|packet: AutoRunSkillPacket| NetworkEvent::AutoRunSkill {
        skill_id: packet.skill_id,
        skill_type: match packet.skill_type {
            0 => SkillType::Passive,
            1 => SkillType::Attack,
            2 => SkillType::Ground,
            4 => SkillType::SelfCast,
            16 => SkillType::Support,
            32 => SkillType::Trap,
            _ => SkillType::SelfCast,
        },
        skill_level: packet.skill_level,
        spell_point_cost: packet.skill_sp,
        attack_range: AttackRange(packet.skill_range),
        skill_name: String::from_utf8_lossy(&packet.skill_name).trim_end_matches('\0').to_owned(),
        upgradable: packet.up_flag != 0,
    })?;
    packet_handler.register(|packet: UpdateHotkeysPacket| NetworkEvent::SetHotkeyData {
        tab: packet.tab,
        hotkeys: packet
            .hotkeys
            .into_iter()
            .map(|hotkey_data| match hotkey_data == HotkeyData::UNBOUND {
                true => HotkeyState::Unbound,
                false => HotkeyState::Bound(hotkey_data),
            })
            .collect(),
    })?;
    packet_handler.register(|packet: InitialStatsPacket| {
        let InitialStatsPacket {
            strength_stat_points_cost,
            agility_stat_points_cost,
            vitality_stat_points_cost,
            intelligence_stat_points_cost,
            dexterity_stat_points_cost,
            luck_stat_points_cost,
            ..
        } = packet;

        NetworkEvent::InitialStats {
            strength_stat_points_cost,
            agility_stat_points_cost,
            vitality_stat_points_cost,
            intelligence_stat_points_cost,
            dexterity_stat_points_cost,
            luck_stat_points_cost,
        }
    })?;
    packet_handler.register(|packet: UpdatePartyInvitationStatePacket| NetworkEvent::PartyInvitationState {
        deny_party_invites: packet.allowed != 0,
    })?;
    packet_handler.register_noop::<UpdateShowEquipPacket>()?;
    packet_handler.register_noop::<UpdateConfigurationPacket>()?;
    packet_handler.register_noop::<NavigateToMonsterPacket>()?;
    packet_handler.register(|packet: MarkMinimapPositionPacket| NetworkEvent::MarkMinimap {
        npc_id: packet.npc_id,
        marker_type: packet.marker_type,
        position: packet.position,
        id: packet.id,
        color: packet.color,
    })?;
    packet_handler.register(|packet: NextButtonPacket| {
        let NextButtonPacket { npc_id } = packet;

        NetworkEvent::AddNextButton { npc_id }
    })?;
    packet_handler.register(|packet: CloseButtonPacket| {
        let CloseButtonPacket { npc_id } = packet;

        NetworkEvent::AddCloseButton { npc_id }
    })?;
    packet_handler.register(|packet: DialogMenuPacket| {
        let DialogMenuPacket { npc_id, message } = packet;

        let choices = message.split(':').map(String::from).filter(|text| !text.is_empty()).collect();

        NetworkEvent::AddChoiceButtons { choices, npc_id }
    })?;
    packet_handler.register(|packet: NpcOpenNumberInputPacket| NetworkEvent::NpcRequestNumberInput { npc_id: packet.npc_id })?;
    packet_handler.register(|packet: NpcOpenStringInputPacket| NetworkEvent::NpcRequestStringInput { npc_id: packet.npc_id })?;
    // ZC_NOTIFY_EFFECT2 (0x01F3): entity + native effect id from effect_list.
    packet_handler.register(|packet: DisplaySpecialEffectPacket| NetworkEvent::SpecialEffect {
        entity_id: packet.entity_id,
        effect_id: packet.effect_id,
    })?;
    packet_handler.register(|packet: DisplaySkillCooldownPacket| NetworkEvent::SkillCooldown {
        skill_id: packet.skill_id,
        until: packet.until,
    })?;
    packet_handler.register(|packet: DisplaySkillEffectAndDamagePacket| {
        // Skill damage reuses the same floating-number path as basic attacks.
        // `damage == 0` is treated as a miss (e.g. skill type "no damage" end).
        Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            skill_id: Some(packet.skill_id),
            packet_tick: packet.start_time,
            damage_amount: (packet.damage > 0).then_some(packet.damage as usize),
            hit_count: (packet.div as usize).max(1),
            attack_duration: packet.soruce_delay,
            damage_delay: packet.destination_delay,
            // skill_type 8 is the multi-hit critical family on official clients.
            is_critical: packet.skill_type == 8,
        })
    })?;
    packet_handler.register(|packet: DisplaySkillEffectNoDamagePacket| NetworkEvent::SkillEffectNoDamage {
        skill_id: packet.skill_id,
        source_entity_id: packet.source_entity_id,
        destination_entity_id: packet.destination_entity_id,
        effect_value: packet.heal_amount,
        successful: packet.result != 0,
    })?;
    // Always targets the receiving client; entity_id 0 falls back to local player
    // in the lib.rs HealEffect arm.
    packet_handler.register(|packet: DisplayPlayerHealEffect| NetworkEvent::HealEffect {
        entity_id: EntityId(0),
        heal_amount: packet.heal_amount as usize,
    })?;
    packet_handler.register(|packet: StatusChangePacket| NetworkEvent::StatusChange {
        entity_id: packet.entity_id,
        index: packet.index,
        gained: packet.state == 1,
        duration_ms: packet.duration_in_milliseconds,
        remaining_ms: packet.remaining_in_milliseconds,
        values: packet.value,
    })?;
    packet_handler.register(|packet: StatusChange2Packet| NetworkEvent::StatusChange {
        entity_id: packet.entity_id,
        index: packet.index,
        gained: packet.state == 1,
        duration_ms: packet.remaining_in_milliseconds,
        remaining_ms: packet.remaining_in_milliseconds,
        values: packet.value,
    })?;
    packet_handler.register(|packet: QuestNotificationPacket1| NetworkEvent::QuestAdded {
        quest_id: packet.quest_id,
        active: packet.active != 0,
    })?;
    packet_handler.register(|packet: QuestNotificationPacket4| NetworkEvent::QuestAdded {
        quest_id: packet.quest_id,
        active: packet.active != 0,
    })?;
    packet_handler.register_noop::<HuntingQuestNotificationPacket>()?;
    packet_handler.register_noop::<HuntingQuestUpdateObjectivePacket>()?;
    packet_handler.register_noop::<HuntingQuestUpdateObjectivePacket4>()?;
    packet_handler.register(|packet: QuestRemovedPacket| NetworkEvent::QuestRemoved { quest_id: packet.quest_id })?;
    packet_handler.register(|packet: QuestListPacket| NetworkEvent::QuestList {
        quest_ids: packet.quests.iter().map(|quest| quest.quest_id).collect(),
    })?;
    packet_handler.register(|packet: QuestListPacket4| NetworkEvent::QuestList {
        quest_ids: packet.quests.iter().map(|quest| quest.quest_id).collect(),
    })?;
    packet_handler.register(|packet: VisualEffectPacket| {
        let VisualEffectPacket { entity_id, effect } = packet;

        let effect_path = match effect {
            VisualEffect::BaseLevelUp => "angel.str",
            VisualEffect::JobLevelUp => "joblvup.str",
            VisualEffect::RefineFailure => "bs_refinefailed.str",
            VisualEffect::RefineSuccess => "bs_refinesuccess.str",
            VisualEffect::GameOver => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::PharmacySuccess => "p_success.str",
            VisualEffect::PharmacyFailure => "p_failed.str",
            VisualEffect::BaseLevelUpSuperNovice => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::JobLevelUpSuperNovice => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::BaseLevelUpTaekwon => "help_angel\\help_angel\\help_angel.str",
        };

        NetworkEvent::VisualEffect { effect_path, entity_id }
    })?;
    packet_handler.register(|packet: DisplayGainedExperiencePacket| NetworkEvent::GainedExperience {
        account_id: packet.account_id,
        amount: packet.amount,
        experience_type: packet.experience_type,
        experience_source: packet.experience_source,
    })?;
    packet_handler.register_noop::<DisplayImagePacket>()?;
    // M1-007: promoted from noop 2026-07-15. `effect_state` is `sc->option`, which
    // carries hide/cloak; dropping it left the player with no way to see whether
    // Hiding was active, and made hide-gated skills look broken.
    packet_handler.register(|packet: StateChangePacket| NetworkEvent::StateChange {
        entity_id: packet.entity_id,
        option: packet.effect_state,
        body_state: packet.body_state,
        health_state: packet.health_state,
        is_pk_mode_on: packet.is_pk_mode_on != 0,
    })?;

    packet_handler.register(|packet: QuestEffectPacket| match packet.effect {
        QuestEffect::None => NetworkEvent::RemoveQuestEffect {
            entity_id: packet.entity_id,
        },
        _ => NetworkEvent::AddQuestEffect { quest_effect: packet },
    })?;
    packet_handler.register(|packet: ItemPickupPacket| {
        let ItemPickupPacket {
            index,
            quantity,
            item_id,
            is_identified,
            is_broken,
            cards,
            equip_position,
            item_type,
            result,
            hire_expiration_date,
            bind_on_equip_type,
            option_data,
            favorite,
            look,
            refinement_level,
            enchantment_level,
        } = packet;

        if result != ItemPickupResult::Success {
            return vec![NetworkEvent::ChatMessage {
                text: "Failed to pick up item.".to_string(),
                color: MessageColor::Error,
            }];
        }

        // TODO: Not sure where to store these, since the *InventoryItem packets are not
        // sending these either. We will certainly use them at some point though.
        let _ = (favorite, look);

        let details = if item_type == IT_AMMO {
            // Stackable-but-equippable ammo; model it consistently with the
            // inventory list so it keeps its equip option and stack count.
            InventoryItemDetails::ammo(quantity, EquipPosition::empty(), is_identified != 0)
        } else {
            match equip_position.is_empty() {
                true => InventoryItemDetails::Regular {
                    amount: quantity,
                    equipped_position: equip_position,
                    flags: {
                        let mut flags = RegularItemFlags::empty();
                        flags.set(RegularItemFlags::IDENTIFIED, is_identified != 0);
                        flags
                    },
                },
                false => InventoryItemDetails::Equippable {
                    amount: quantity,
                    equip_position,
                    equipped_position: EquipPosition::empty(),
                    bind_on_equip_type,
                    w_item_sprite_number: 0,
                    option_count: option_data.len() as u8,
                    option_data,
                    refinement_level,
                    enchantment_level,
                    flags: {
                        let mut flags = EquippableItemFlags::empty();
                        flags.set(EquippableItemFlags::IDENTIFIED, is_identified != 0);
                        flags.set(EquippableItemFlags::IS_BROKEN, is_broken != 0);
                        flags
                    },
                },
            }
        };

        let item = InventoryItem {
            metadata: NoMetadata,
            index,
            item_id,
            item_type,
            slot: cards,
            hire_expiration_date,
            details,
        };

        let is_identified = is_identified != 0;

        vec![NetworkEvent::IventoryItemAdded { item }, NetworkEvent::ItemObtained {
            item_id,
            quantity,
            is_identified,
        }]
    })?;
    packet_handler.register(|packet: RemoveItemFromInventoryPacket| NetworkEvent::InventoryItemRemoved {
        reason: packet.remove_reason,
        index: packet.index,
        amount: packet.amount,
    })?;
    packet_handler.register(|packet: UseItemAckPacket| -> NetworkEventList {
        if packet.result != 0 && packet.amount == 0 {
            NetworkEvent::InventoryItemRemoved {
                reason: RemoveItemReason::Normal,
                index: packet.index,
                amount: 1,
            }
            .into()
        } else {
            NetworkEventList::default()
        }
    })?;
    // ZC_ITEM_THROW_ACK (0x00AF). Success usually also sends 0x07FA; amount 0 means
    // rejected.
    packet_handler.register(|packet: DropItemAckPacket| -> NetworkEventList {
        if packet.amount == 0 {
            NetworkEvent::ChatMessage {
                text: "You can't drop that item.".to_owned(),
                color: MessageColor::Error,
            }
            .into()
        } else {
            // Fallback inventory sync if 0x07FA was not processed (safe if already
            // removed).
            NetworkEvent::InventoryItemRemoved {
                reason: RemoveItemReason::Normal,
                index: packet.inventory_index,
                amount: packet.amount,
            }
            .into()
        }
    })?;
    packet_handler.register(|packet: ServerTickPacket| NetworkEvent::UpdateClientTick {
        client_tick: packet.client_tick,
        received_at: Instant::now(),
    })?;
    packet_handler.register(|packet: RequestPlayerDetailsSuccessPacket| NetworkEvent::UpdateEntityDetails {
        entity_id: EntityId(packet.character_id.0),
        name: packet.name,
    })?;
    packet_handler.register(|packet: RequestEntityDetailsSuccessPacket| NetworkEvent::UpdateEntityDetails {
        entity_id: packet.entity_id,
        name: packet.name,
    })?;
    packet_handler.register(|packet: UpdateEntityHealthPointsPacket| {
        let UpdateEntityHealthPointsPacket {
            entity_id,
            health_points,
            maximum_health_points,
        } = packet;

        NetworkEvent::UpdateEntityHealth {
            entity_id,
            health_points: health_points as usize,
            maximum_health_points: maximum_health_points as usize,
        }
    })?;
    packet_handler.register(|packet: RequestPlayerAttackFailedPacket| {
        let RequestPlayerAttackFailedPacket {
            target_entity_id,
            target_position,
            player_position,
            attack_range,
        } = packet;

        NetworkEvent::AttackFailed {
            target_entity_id,
            target_position,
            player_position,
            attack_range,
        }
    })?;
    packet_handler.register(|packet: EntitySlidePacket| NetworkEvent::EntitySlide {
        entity_id: packet.entity_id,
        position: packet.position,
    })?;
    packet_handler.register(|packet: MonsterInformationPacket| NetworkEvent::MonsterInformation {
        job_id: packet.job_id,
        level: packet.level,
        size: packet.size,
        health_points: packet.health_points,
        defense: packet.defense,
        race: packet.race,
        magic_defense: packet.magic_defense,
        element: packet.element,
        elemental_effectiveness: packet.elemental_effectiveness,
    })?;
    packet_handler.register(|packet: WarpListPacket| NetworkEvent::WarpList {
        skill_id: packet.skill_id,
        destinations: packet
            .destinations
            .into_iter()
            .map(|destination| {
                let end = destination
                    .map_name
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(destination.map_name.len());
                String::from_utf8_lossy(&destination.map_name[..end]).into_owned()
            })
            .collect(),
    })?;
    packet_handler.register(|packet: SkillCooldownListPacket| NetworkEvent::SkillCooldownList {
        cooldowns: packet.cooldowns,
    })?;
    packet_handler.register(|packet: RefinableWeaponListPacket| NetworkEvent::RefinableWeaponList { weapons: packet.weapons })?;
    packet_handler.register(|packet: WeaponRefineResultPacket| NetworkEvent::WeaponRefineResult {
        result: packet.result,
        item_id: packet.item_id,
    })?;
    packet_handler.register(|packet: RepairableItemListPacket| NetworkEvent::RepairableItemList { items: packet.items })?;
    packet_handler.register(|packet: ItemRepairResultPacket| NetworkEvent::ItemRepairResult {
        inventory_index: packet.inventory_index,
        success: packet.result == 0,
    })?;
    packet_handler.register(|packet: DamagePacket1| match packet.damage_type {
        // Assassin dual-wield / Double Attack normal hits arrive as a multi-hit
        // damage type carrying a second damage value (`damage_amount_2`). These
        // must still surface a `DamageEffect`; otherwise the auto-attack loop
        // (which re-fires on the player's own damage ack) stalls after one swing.
        DamageType::MultiHitDamage | DamageType::MultiHitDamageEndure => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            skill_id: None,
            packet_tick: packet.client_tick,
            damage_amount: {
                let total = packet.damage_amount as i32 + packet.damage_amount_2 as i32;
                (total > 0).then_some(total as usize)
            },
            hit_count: (packet.number_of_hits as usize).max(2),
            attack_duration: packet.attack_duration,
            damage_delay: packet.damage_delay,
            is_critical: false,
        }),
        DamageType::CriticalMultiHit => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            skill_id: None,
            packet_tick: packet.client_tick,
            damage_amount: {
                let total = packet.damage_amount as i32 + packet.damage_amount_2 as i32;
                (total > 0).then_some(total as usize)
            },
            hit_count: (packet.number_of_hits as usize).max(2),
            attack_duration: packet.attack_duration,
            damage_delay: packet.damage_delay,
            is_critical: true,
        }),
        DamageType::Damage => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            skill_id: None,
            packet_tick: packet.client_tick,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            hit_count: 1,
            attack_duration: packet.attack_duration,
            damage_delay: packet.damage_delay,
            is_critical: false,
        }),
        DamageType::CriticalHit => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            skill_id: None,
            packet_tick: packet.client_tick,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            hit_count: 1,
            attack_duration: packet.attack_duration,
            damage_delay: packet.damage_delay,
            is_critical: true,
        }),
        DamageType::PickUpItem => Some(NetworkEvent::EntityPickUpItem {
            entity_id: packet.source_entity_id,
            item_entity_id: packet.destination_entity_id,
        }),
        // Hercules' clif_sitting/clif_standing put the acting entity in the
        // SOURCE field (destination is zeroed) — see clif.c `WBUFL(buf,2) = bl->id`.
        DamageType::SitDown => Some(NetworkEvent::PlayerSitDown {
            entity_id: packet.source_entity_id,
        }),
        DamageType::StandUp => Some(NetworkEvent::PlayerStandUp {
            entity_id: packet.source_entity_id,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: DamagePacket3| match packet.damage_type {
        // See DamagePacket1 above: dual-wield / Double Attack multi-hit normals
        // must produce a DamageEffect so the auto-attack loop keeps firing.
        DamageType::MultiHitDamage | DamageType::MultiHitDamageEndure => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            skill_id: None,
            packet_tick: packet.client_tick,
            damage_amount: {
                let total = packet.damage_amount as usize + packet.damage_amount_2 as usize;
                (total > 0).then_some(total)
            },
            hit_count: (packet.number_of_hits as usize).max(2),
            attack_duration: packet.attack_duration,
            damage_delay: packet.damage_delay,
            is_critical: false,
        }),
        DamageType::CriticalMultiHit => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            skill_id: None,
            packet_tick: packet.client_tick,
            damage_amount: {
                let total = packet.damage_amount as usize + packet.damage_amount_2 as usize;
                (total > 0).then_some(total)
            },
            hit_count: (packet.number_of_hits as usize).max(2),
            attack_duration: packet.attack_duration,
            damage_delay: packet.damage_delay,
            is_critical: true,
        }),
        DamageType::Damage => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            skill_id: None,
            packet_tick: packet.client_tick,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            hit_count: 1,
            attack_duration: packet.attack_duration,
            damage_delay: packet.damage_delay,
            is_critical: false,
        }),
        DamageType::CriticalHit => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            skill_id: None,
            packet_tick: packet.client_tick,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            hit_count: 1,
            attack_duration: packet.attack_duration,
            damage_delay: packet.damage_delay,
            is_critical: true,
        }),
        DamageType::PickUpItem => Some(NetworkEvent::EntityPickUpItem {
            entity_id: packet.source_entity_id,
            item_entity_id: packet.destination_entity_id,
        }),
        // Hercules' clif_sitting/clif_standing put the acting entity in the
        // SOURCE field (destination is zeroed) — see clif.c `WBUFL(buf,2) = bl->id`.
        DamageType::SitDown => Some(NetworkEvent::PlayerSitDown {
            entity_id: packet.source_entity_id,
        }),
        DamageType::StandUp => Some(NetworkEvent::PlayerStandUp {
            entity_id: packet.source_entity_id,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: NpcDialogPacket| {
        let NpcDialogPacket { npc_id, text } = packet;

        NetworkEvent::OpenDialog { text, npc_id }
    })?;
    // Yes, Hercules sends `n + 2` here (`clif_equipitemack` / `clif_unequipitemack`),
    // but do **not** subtract it in the handler: both fields are typed
    // `InventoryIndex`, whose `FromBytes` already does the `- 2` (see the type in
    // `ragnarok-packets`). Subtracting again lands two slots early. Only the
    // inventory *list* adjusts by hand, because its items carry `RawIndex`.
    packet_handler.register(|packet: RequestEquipItemStatusPacket| match packet.result {
        RequestEquipItemStatus::Success => Some(NetworkEvent::UpdateEquippedPosition {
            index: packet.inventory_index,
            equipped_position: packet.equipped_position,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: RequestUnequipItemStatusPacket| match packet.result {
        RequestUnequipItemStatus::Success => Some(NetworkEvent::UpdateEquippedPosition {
            index: packet.inventory_index,
            equipped_position: EquipPosition::NONE,
        }),
        _ => None,
    })?;
    packet_handler.register_noop::<Packet8302>()?;
    packet_handler.register_noop::<Packet0b18>()?;
    packet_handler.register_noop::<ConnectionRefusedPacket>()?;
    packet_handler.register(|packet: MapServerLoginSuccessPacket| NetworkEvent::UpdateClientTick {
        client_tick: packet.client_tick,
        received_at: Instant::now(),
    })?;
    packet_handler.register(|packet: RestartResponsePacket| match packet.result {
        RestartResponseStatus::Ok => NetworkEvent::LoggedOut,
        RestartResponseStatus::Nothing => NetworkEvent::ChatMessage {
            text: "Failed to log out.".to_string(),
            color: MessageColor::Error,
        },
    })?;
    packet_handler.register(|packet: DisconnectResponsePacket| match packet.result {
        DisconnectResponseStatus::Ok => NetworkEvent::LoggedOut,
        DisconnectResponseStatus::Wait10Seconds => NetworkEvent::ChatMessage {
            text: "Please wait 10 seconds before trying to log out.".to_string(),
            color: MessageColor::Error,
        },
    })?;
    packet_handler.register(|packet: UseSkillSuccessPacket| {
        (packet.delay_time > 0).then_some(NetworkEvent::SkillCast {
            source_entity_id: packet.source_entity,
            skill_id: packet.skill_id,
            cast_ms: packet.delay_time,
        })
    })?;
    packet_handler.register(|packet: UseSkillAckPacket| {
        (packet.delay_time > 0).then_some(NetworkEvent::SkillCast {
            source_entity_id: packet.source_entity,
            skill_id: packet.skill_id,
            cast_ms: packet.delay_time,
        })
    })?;
    packet_handler.register(|packet: SkillCastCancelledPacket| NetworkEvent::SkillCastCancelled {
        source_entity_id: Some(packet.entity_id),
    })?;
    // ZC_ACK_TOUSESKILL is only sent by Hercules on skill *failure* (flag 0),
    // including gameplay rejections like "party creation requires Basic Skill
    // 7". Without this the rejection is completely silent.
    packet_handler.register(|packet: ToUseSkillSuccessPacket| -> NetworkEventList {
        if packet.flag != 0 {
            return NetworkEventList::default();
        }

        let reported = match skill_failed_reason(&packet) {
            SkillFailure::Text(text) => NetworkEvent::ChatMessage {
                text,
                color: MessageColor::Error,
            },
            SkillFailure::MissingItem {
                item_id,
                amount,
                equipment,
            } => NetworkEvent::SkillFailedMissingItem {
                item_id,
                amount,
                equipment,
            },
        };

        vec![NetworkEvent::SkillCastCancelled { source_entity_id: None }, reported].into()
    })?;
    // `ZC_NOTIFY_MAPINFO` — a map-zone restriction refused the action. Hercules
    // deliberately sends this *instead of* `clif->skill_fail`, so without a
    // handler here the refusal is silent and the skill just appears to do
    // nothing. Same reasoning as 0x0110 above: a rejection the player caused
    // must be visible, never dropped.
    packet_handler.register(|packet: NotifyMapInfoPacket| NetworkEvent::ChatMessage {
        text: match packet.info_type {
            0 => "You cannot teleport in this area.".to_owned(),
            1 => "This location cannot be memorized as a save point.".to_owned(),
            2 => "This skill cannot be used in this area.".to_owned(),
            3 => "This item cannot be used in this area.".to_owned(),
            // Hercules only defines 0-3; surface anything else rather than
            // dropping it, so a new type shows up instead of going quiet.
            other => format!("This action is not allowed in this area. (code {other})"),
        },
        color: MessageColor::Error,
    })?;
    packet_handler.register(|packet: NotifySkillUnitPacket| {
        let NotifySkillUnitPacket {
            entity_id,
            position,
            unit_id,
            ..
        } = packet;

        NetworkEvent::AddSkillUnit {
            entity_id,
            unit_id,
            position,
        }
    })?;
    packet_handler.register(|packet: SkillUnitDisappearPacket| {
        let SkillUnitDisappearPacket { entity_id } = packet;
        NetworkEvent::RemoveSkillUnit { entity_id }
    })?;
    packet_handler.register(|packet: NotifyGroundSkillPacket| NetworkEvent::GroundSkillEffect {
        skill_id: packet.skill_id,
        source_entity_id: packet.entity_id,
        level: packet.level,
        position: packet.position,
        start_tick: packet.start_time,
    })?;
    packet_handler.register(|packet: FriendListPacket| NetworkEvent::SetFriendList {
        friend_list: packet.friend_list,
    })?;
    packet_handler.register(|packet: FriendOnlineStatusPacket| NetworkEvent::FriendOnlineStatus {
        account_id: packet.account_id,
        character_id: packet.character_id,
        online: matches!(packet.state, OnlineState::Online),
        name: packet.name,
    })?;
    packet_handler.register(|packet: FriendRequestPacket| NetworkEvent::FriendRequest {
        requestee: packet.requestee,
    })?;
    packet_handler.register(|packet: FriendRequestResultPacket| {
        let text = match packet.result {
            FriendRequestResult::Accepted => format!("You have become friends with {}.", packet.friend.name),
            FriendRequestResult::Rejected => format!("{} does not want to be friends with you.", packet.friend.name),
            FriendRequestResult::OwnFriendListFull => "Your Friend List is full.".to_owned(),
            FriendRequestResult::OtherFriendListFull => format!("{}'s Friend List is full.", packet.friend.name),
        };

        let mut events = vec![NetworkEvent::ChatMessage {
            text,
            color: MessageColor::Information,
        }];

        if matches!(packet.result, FriendRequestResult::Accepted) {
            events.push(NetworkEvent::FriendAdded { friend: packet.friend });
        }

        events
    })?;
    packet_handler.register(|packet: NotifyFriendRemovedPacket| NetworkEvent::FriendRemoved {
        account_id: packet.account_id,
        character_id: packet.character_id,
    })?;
    packet_handler.register(|packet: PartyInvitePacket| NetworkEvent::PartyInvite {
        party_id: packet.party_id,
        party_name: packet.party_name,
    })?;
    packet_handler.register(|packet: CreatePartyResultPacket| NetworkEvent::CreatePartyResult { result: packet.result })?;
    packet_handler.register(|packet: PartyInviteResultPacket| NetworkEvent::PartyInviteResult {
        character_name: packet.character_name,
        result: packet.result,
    })?;
    packet_handler.register(|packet: PartyListPacket| NetworkEvent::PartyList {
        party_name: packet.party_name,
        members: packet.members,
    })?;
    packet_handler.register(|packet: PartyMemberInfoPacket| NetworkEvent::PartyMemberAdded { member: packet })?;
    packet_handler.register(|packet: PartyMemberPositionPacket| NetworkEvent::PartyMemberPosition {
        account_id: packet.account_id,
        position: packet.position,
    })?;
    packet_handler.register(|packet: PartyMemberHealthPacket| NetworkEvent::PartyMemberHealth {
        account_id: packet.account_id,
        health_points: packet.health_points as usize,
        maximum_health_points: packet.maximum_health_points as usize,
        spell_points: None,
    })?;
    packet_handler.register(|packet: PartyOptionsChangedPacket| NetworkEvent::PartyShareOptions {
        experience_share: packet.experience_share != 0,
        item_pickup_share: Some(packet.item_pickup_share != 0),
        item_division_share: Some(packet.item_division_share != 0),
    })?;
    packet_handler.register(|packet: PartyExperienceOptionPacket| NetworkEvent::PartyShareOptions {
        experience_share: packet.experience_share != 0,
        item_pickup_share: None,
        item_division_share: None,
    })?;
    packet_handler.register(|packet: NpcRefineResultPacket| NetworkEvent::NpcRefineResult {
        result: packet.result,
        // `clif_refine` sends `index + 2`; undo it here so no consumer has to
        // remember the off-by-two.
        inventory_index: InventoryIndex(packet.index.saturating_sub(2)),
        refine_level: packet.refine_level,
    })?;
    packet_handler.register(|packet: PartyMemberAlivePacket| NetworkEvent::PartyMemberAlive {
        account_id: packet.account_id,
        is_dead: packet.is_dead != 0,
    })?;
    packet_handler.register(|packet: AutoSpellListPacket| NetworkEvent::AutoSpellList {
        // Zero entries pad the list out to a fixed size on some servers.
        skills: packet
            .skills
            .into_iter()
            .filter(|skill_id| *skill_id != 0)
            .map(|skill_id| SkillId(skill_id as u16))
            .collect(),
    })?;
    packet_handler.register(|packet: SpiritSpheresPacket| NetworkEvent::SpiritSpheres {
        entity_id: packet.entity_id,
        amount: packet.amount,
    })?;
    packet_handler.register(|packet: SkillUnitUpdatePacket| NetworkEvent::SkillUnitUpdated {
        entity_id: packet.entity_id,
    })?;
    packet_handler.register(|packet: EntitySnapPacket| NetworkEvent::EntitySnapped {
        entity_id: packet.entity_id,
        position: packet.position,
    })?;
    packet_handler.register(|packet: EntityEffectStatePacket| NetworkEvent::EntityEffectState {
        entity_id: packet.entity_id,
        effect_state: packet.effect_state,
        level: packet.level,
    })?;
    packet_handler.register(|packet: StarPlacePacket| NetworkEvent::StarPlace {
        map_name: packet.map_name,
        monster_id: packet.monster_id,
        star: packet.star,
        result: packet.result,
    })?;
    packet_handler.register(|packet: FeelRequestPacket| NetworkEvent::FeelRequest { which: packet.which })?;
    packet_handler.register(|packet: InstanceInfoPacket| NetworkEvent::InstanceInfo {
        instance_name: packet.instance_name,
    })?;
    packet_handler.register(|packet: InstanceJoinPacket| NetworkEvent::InstanceJoined {
        instance_name: packet.instance_name,
        progress_remaining: packet.progress_remaining,
        idle_remaining: packet.idle_remaining,
    })?;
    packet_handler.register(|_: InstanceLeavePacket| NetworkEvent::InstanceLeft)?;
    packet_handler.register(|packet: IgnorePlayerResultPacket| NetworkEvent::IgnoreResult {
        ignore_type: packet.ignore_type,
        result: packet.result,
    })?;
    packet_handler.register(|packet: PartyLeaderChangedPacket| NetworkEvent::PartyLeaderChanged {
        previous_leader_account_id: packet.previous_leader_account_id,
        new_leader_account_id: packet.new_leader_account_id,
    })?;
    packet_handler.register(|packet: PartyInviteSenderPacket| NetworkEvent::PartyInviteSender {
        party_id: packet.party_id,
        character_name: packet.character_name,
    })?;
    packet_handler.register(|packet: PartyMemberVitalsPacket| NetworkEvent::PartyMemberHealth {
        account_id: packet.account_id,
        health_points: packet.health_points as usize,
        maximum_health_points: packet.maximum_health_points as usize,
        spell_points: Some((packet.spell_points as usize, packet.maximum_spell_points as usize)),
    })?;
    packet_handler.register(|packet: PartyMemberJobAndLevelPacket| NetworkEvent::PartyMemberJobAndLevel {
        account_id: packet.account_id,
        job_id: packet.job_id,
        base_level: packet.base_level,
    })?;
    packet_handler.register(|packet: PartyMemberRemovedPacket| NetworkEvent::PartyMemberRemoved {
        account_id: packet.account_id,
        character_name: packet.character_name,
        result: packet.result,
    })?;
    packet_handler.register(|packet: NotifyPartyChatMessagePacket| NetworkEvent::PartyChatMessage {
        account_id: packet.account_id,
        text: packet.message,
    })?;
    packet_handler.register(|packet: WhisperMessagePacket| NetworkEvent::WhisperReceived {
        sender_character_id: packet.sender_character_id,
        sender_name: packet.sender_name,
        is_admin: packet.is_admin != 0,
        message: packet.message,
    })?;
    packet_handler.register(|packet: WhisperResultPacket| NetworkEvent::WhisperResult { result: packet.result })?;
    packet_handler.register(|packet: WhisperResult2Packet| NetworkEvent::WhisperResult { result: packet.result })?;
    // M1-012: promoted from noop 2026-07-15. Despite the name, this is how a status
    // *ends*: Hercules' `clif_status_change_end` sends 0x0196
    // (`status_change_endType`) with `state = 0`, while starts arrive on
    // 0x0983. Dropping it meant a buff could never be cleared *by the server* —
    // it only ever vanished when its own client-side timer ran out, so
    // cancelling early (un-hiding) left the timer on screen forever.
    packet_handler.register(|packet: StatusChangeSequencePacket| NetworkEvent::StatusChange {
        entity_id: EntityId(packet.id),
        index: packet.index,
        gained: packet.state == 1,
        // The end packet carries no timings or values; `gained: false` routes to
        // `remove()`, which ignores them.
        duration_ms: 0,
        remaining_ms: 0,
        values: [0; 3],
    })?;
    packet_handler.register_noop::<ReputationPacket>()?;
    packet_handler.register_noop::<ClanInfoPacket>()?;
    packet_handler.register_noop::<ClanOnlineCountPacket>()?;
    packet_handler.register(|packet: ChangeMapCellPacket| NetworkEvent::MapCellChanged {
        position: packet.position,
        cell_type: packet.cell_type,
    })?;
    packet_handler.register_noop::<OpenMarketPacket>()?;
    packet_handler.register(|packet: BuyOrSellPacket| NetworkEvent::AskBuyOrSell { shop_id: packet.shop_id })?;
    packet_handler.register(|packet: ShopItemListPacket| {
        let items = packet
            .items
            .into_iter()
            .map(|item| ShopItem {
                metadata: NoMetadata,
                item_id: item.item_id,
                item_type: item.item_type,
                price: item.price,
                quantity: ItemQuantity::Infinite,
                weight: 0,
                location: item.location,
            })
            .collect();

        NetworkEvent::OpenShop { items }
    })?;
    // NPC shop purchase result (0x00CA). Market shops use 0x0B4E separately.
    packet_handler.register(|packet: BuyItemsResultPacket| {
        let result = match packet.result {
            BuyItemResult::Successful | BuyItemResult::ExchangeWellDone => BuyShopItemsResult::Success,
            _ => BuyShopItemsResult::Error,
        };
        NetworkEvent::BuyingCompleted { result }
    })?;
    packet_handler.register(|packet: BuyShopItemsResultPacket| NetworkEvent::BuyingCompleted { result: packet.result })?;
    // ZC_LONGPAR_CHANGE (0x00B1) — legacy long-parameter updates (exp/zeny as u32).
    // Modern 20220406 traffic prefers 0x0ACB (UpdateStatPacket2); keep this for any
    // remaining senders and for completeness of the §8.3 stats MVP row.
    packet_handler.register(|packet: ParameterChangePacket| {
        let ParameterChangePacket { variable_id, value } = packet;
        let stat_type = match variable_id {
            1 => StatType::BaseExperience(u64::from(value)),
            2 => StatType::JobExperience(u64::from(value)),
            20 => StatType::Zeny(value),
            22 => StatType::NextBaseExperience(u64::from(value)),
            23 => StatType::NextJobExperience(u64::from(value)),
            24 => StatType::Weight(value),
            25 => StatType::MaximumWeight(value),
            0 => StatType::MovementSpeed(value),
            3 => StatType::Karma(value),
            4 => StatType::Manner(value),
            5 => StatType::HealthPoints(value),
            6 => StatType::MaximumHealthPoints(value),
            7 => StatType::SpellPoints(value),
            8 => StatType::MaximumSpellPoints(value),
            9 => StatType::StatPoints(value),
            11 => StatType::BaseLevel(value),
            12 => StatType::SkillPoints(value),
            _ => return None,
        };
        Some(NetworkEvent::UpdateStat { stat_type })
    })?;
    packet_handler.register(|packet: SellListPacket| NetworkEvent::SellItemList { items: packet.items })?;
    packet_handler.register(|packet: SellItemsResultPacket| NetworkEvent::SellingCompleted { result: packet.result })?;
    packet_handler.register(|packet: RequestStatUpResponsePacket| {
        // Success path is already reflected by subsequent UpdateStat packets.
        // Surface failures so stat allocation is not silent.
        match packet.success {
            RequestStatUpResult::Success => None,
            RequestStatUpResult::Failure => Some(NetworkEvent::ChatMessage {
                text: "Failed to increase that stat.".to_owned(),
                color: MessageColor::Error,
            }),
        }
    })?;
    // Ammo (arrows) equips via its own packet, not the normal equip ack; mark
    // the item equipped in the AMMO slot so it shows in the equipment window.
    //
    // `clif_arrowequip` does send `val + 2`, but the field is an `InventoryIndex`
    // and that type's `FromBytes` has already removed the `+ 2` by the time this
    // handler runs — so use it as-is. Subtracting again flags the item two slots
    // below the arrows, which leaves the Ammo box empty when that slot is free
    // (looking exactly like "the equip never happened") and picks the wrong
    // projectile sprite when it is not.
    packet_handler.register({
        let inventory_items = inventory_items.clone();
        let pending_equipped_ammunition = pending_equipped_ammunition.clone();

        move |packet: EquipAmmunitionPacket| -> Option<NetworkEvent> {
            // At login this packet lands *between* the inventory list packets, so the
            // event would be applied to an inventory that is about to be replaced.
            // Buffer it for the End handler instead of emitting into the void.
            if inventory_items.borrow().is_some() {
                *pending_equipped_ammunition.borrow_mut() = Some(packet.inventory_index);
                return None;
            }

            Some(NetworkEvent::UpdateEquippedPosition {
                index: packet.inventory_index,
                equipped_position: EquipPosition::AMMO,
            })
        }
    })?;
    packet_handler.register_noop::<AmmunitionActionPacket>()?;
    packet_handler.register(|packet: UpdateSkillPacket| {
        let UpdateSkillPacket {
            skill_id,
            skill_level,
            spell_point_cost,
            attack_range,
            upgradable,
        } = packet;

        NetworkEvent::UpdateSkill {
            skill_id,
            skill_level,
            spell_point_cost,
            attack_range,
            upgradable: upgradable != 0,
        }
    })?;
    packet_handler.register(|packet: RemoveSkillPacket| NetworkEvent::RemoveSkill { skill_id: packet.skill_id })?;

    // Identify
    packet_handler.register(|packet: ItemIdentifyListPacket| NetworkEvent::ItemIdentifyList { indices: packet.indices })?;
    packet_handler.register(|packet: ItemIdentifyResultPacket| NetworkEvent::ItemIdentified {
        inventory_index: packet.inventory_index,
        success: packet.result == 0,
    })?;

    // Trade
    packet_handler.register(|packet: TradeRequestPacket| NetworkEvent::TradeRequest {
        name: packet.name,
        character_id: packet.character_id,
        base_level: packet.base_level,
    })?;
    packet_handler.register(|packet: TradeStartPacket| NetworkEvent::TradeStart {
        result: packet.result,
        character_id: packet.character_id,
        base_level: packet.base_level,
    })?;
    packet_handler.register(|packet: TradeAddItemGradeNotifyPacket| NetworkEvent::TradePartnerItem {
        item_id: packet.item_id,
        item_type: packet.item_type,
        amount: packet.amount,
        identified: packet.identified == 1,
        refine: packet.refine,
    })?;
    packet_handler.register(|packet: TradeAddItemNotifyPacket| NetworkEvent::TradePartnerItem {
        item_id: packet.item_id,
        item_type: packet.item_type,
        amount: packet.amount,
        identified: packet.identified != 0,
        refine: packet.refine,
    })?;
    packet_handler.register(|packet: TradeAddItemResultPacket| NetworkEvent::TradeAddItemResult {
        inventory_index: packet.inventory_index,
        result: packet.result,
    })?;
    packet_handler.register(|packet: TradeLockPacket| NetworkEvent::TradeLocked { who: packet.who })?;
    packet_handler.register(|_: TradeCancelledPacket| NetworkEvent::TradeCancelled)?;
    packet_handler.register(|packet: TradeCompletedPacket| NetworkEvent::TradeCompleted {
        success: packet.result == 0,
    })?;

    // Storage
    packet_handler.register(|packet: StorageAmountPacket| NetworkEvent::StorageAmount {
        amount: packet.amount,
        max_amount: packet.max_amount,
    })?;
    packet_handler.register(|packet: StorageItemAddedPacket| {
        let mut flags = RegularItemFlags::empty();
        flags.set(RegularItemFlags::IDENTIFIED, packet.identified != 0);
        let mut equip_flags = EquippableItemFlags::empty();
        equip_flags.set(EquippableItemFlags::IDENTIFIED, packet.identified != 0);
        equip_flags.set(EquippableItemFlags::IS_BROKEN, packet.damaged != 0);

        // Stackable types use Regular; non-stackable use Equippable-ish details.
        let details = if packet.item_type == IT_AMMO {
            // Stackable-but-equippable ammo: keep it Equippable like the other sources.
            InventoryItemDetails::ammo(
                packet.amount.min(u32::from(u16::MAX)) as u16,
                EquipPosition::empty(),
                packet.identified != 0,
            )
        } else if packet.item_type == 4 || packet.item_type == 5 || packet.item_type == 7 || packet.item_type == 8 {
            // Armor / weapon / bothside / pet-armor equippables (Hercules type enum).
            InventoryItemDetails::Equippable {
                amount: packet.amount.min(u32::from(u16::MAX)) as u16,
                equip_position: EquipPosition::empty(),
                equipped_position: EquipPosition::empty(),
                bind_on_equip_type: 0,
                w_item_sprite_number: 0,
                option_count: 0,
                option_data: packet.option_data,
                refinement_level: packet.refine,
                enchantment_level: packet.grade,
                flags: equip_flags,
            }
        } else {
            InventoryItemDetails::Regular {
                amount: packet.amount.min(u32::from(u16::MAX)) as u16,
                equipped_position: EquipPosition::empty(),
                flags,
            }
        };

        NetworkEvent::StorageItemAdded {
            item: InventoryItem {
                index: InventoryIndex(packet.index.0),
                metadata: NoMetadata,
                item_id: packet.item_id,
                item_type: packet.item_type,
                slot: packet.slot,
                hire_expiration_date: 0,
                details,
            },
        }
    })?;
    packet_handler.register(|packet: StorageItemRemovedPacket| NetworkEvent::StorageItemRemoved {
        index: InventoryIndex(packet.index.0),
        amount: packet.amount,
    })?;
    packet_handler.register(|_: StorageClosedPacket| NetworkEvent::StorageClosed)?;

    // Consume any remaining server packet whose length is known (from Hercules'
    // own tables) but that has no dedicated handler yet, instead of desyncing
    // the read buffer. Registered last so it never shadows a real handler.
    packet_handler.register_length_fallbacks(super::lengths_20220406::PACKET_LENGTHS);

    Ok(())
}

/// Outcome of reading a `ZC_ACK_TOUSESKILL` failure: either final text, or a
/// missing-item rejection that only the client can finish rendering because
/// the item name table lives over there.
enum SkillFailure {
    Text(String),
    MissingItem { item_id: ItemId, amount: u16, equipment: bool },
}

const PR_BENEDICTIO: u16 = 69;

/// Every skill flagged `Ensemble: true` in `db/re/skill_db.conf`. These are the
/// only skills whose cause 0 means "no valid partner" rather than an unmet
/// condition on the caster.
const ENSEMBLE_SKILL_IDS: [u16; 11] = [
    PR_BENEDICTIO, // B.S. Sacramenti
    306,           // BD_LULLABY
    307,           // BD_RICHMANKIM
    308,           // BD_ETERNALCHAOS
    309,           // BD_DRUMBATTLEFIELD
    310,           // BD_RINGNIBELUNGEN
    311,           // BD_ROKISWEIL
    312,           // BD_INTOABYSS
    313,           // BD_SIEGFRIED
    395,           // CG_MOONLIT
    488,           // CG_HERMODE
];

/// Reason for a `ZC_ACK_TOUSESKILL` failure (Hercules `useskill_fail_cause`).
/// Hercules also reuses this packet for gameplay rejections, e.g. party
/// creation without Basic Skill 7 arrives as skill 1 / cause 0.
fn skill_failed_reason(packet: &ToUseSkillSuccessPacket) -> SkillFailure {
    match packet.cause {
        // Hercules' catch-all for a required item with no dedicated cause —
        // Yellow Gemstone (Land Protector) lands here, not on 7/8. It sends the
        // required count in `btype` and the item in `item_id`.
        71 | 72 => SkillFailure::MissingItem {
            item_id: packet.item_id,
            amount: packet.btype.clamp(0, i32::from(u16::MAX)) as u16,
            equipment: packet.cause == 72,
        },
        _ => SkillFailure::Text(skill_failed_text(packet)),
    }
}

/// Text for every `ZC_ACK_TOUSESKILL` cause the networking crate can render on
/// its own (i.e. everything that needs no item lookup).
fn skill_failed_text(packet: &ToUseSkillSuccessPacket) -> String {
    // Hercules overloads USESKILL_FAIL_LEVEL for outcomes that have nothing to do
    // with skill level, so a maxed skill reports "level not high enough". These are
    // the ones verified in `skill.c` to mean the target resisted or the roll missed
    // — extend only after checking the source, since most of the ~60 other cause-0
    // emitters really are unmet conditions.
    if packet.cause == 0 {
        // Ensemble songs and dances report a *missing partner* as cause 0
        // (`unit.c:1566`, and `skill.c:16015` for Benedictio), so the default
        // text sends the player to their skill level, which is never the
        // problem. Hercules has a dedicated `USESKILL_FAIL_ENSEMBLE_PARTYNER`
        // (94) and simply does not use it here.
        if ENSEMBLE_SKILL_IDS.contains(&packet.skill_id.0) {
            return match packet.skill_id.0 {
                PR_BENEDICTIO => "That needs two Acolyte-class helpers standing to your left and right.".to_owned(),
                _ => concat!(
                    "That needs a partner within 4 cells: a Bard or Dancer class of the opposite sex, ",
                    "in your party, holding an instrument or whip, who knows the same skill and is not ",
                    "already performing."
                )
                .to_owned(),
            };
        }

        let resisted = match packet.skill_id.0 {
            // `skill.c:8325` — the petrify roll (`skill_lv*4+20` percent, so 60% at
            // level 10) missed; `skill.c:8306` — the target is MD_BOSS. Both arrive
            // as cause 0 and are indistinguishable from here.
            16 => Some("Stone Curse didn't take hold — the target resisted or is immune."),
            // `skill.c:8298` — `pc->steal_coin` returned nothing.
            211 => Some("Failed to steal any Zeny."),
            // `skill.c:8275` — target out of level range, wrong race, or a boss.
            1011 => Some("The target was unaffected by the charm."),
            _ => None,
        };

        if let Some(resisted) = resisted {
            return resisted.to_owned();
        }
    }

    match packet.cause {
        0 if packet.skill_id.0 == 1 => "You need to learn the basic skills first.".to_owned(),
        0 => "Skill level is not high enough.".to_owned(),
        1 => "Not enough SP.".to_owned(),
        2 => "Not enough HP.".to_owned(),
        3 => "Required item or material is missing.".to_owned(),
        4 => "Skill is still on cooldown.".to_owned(),
        5 => "Not enough Zeny.".to_owned(),
        6 => "This skill cannot be used with this weapon.".to_owned(),
        7 => "Red Gemstone required.".to_owned(),
        8 => "Blue Gemstone required.".to_owned(),
        9 => "You are overweight.".to_owned(),
        10 => "You can't use that skill right now.".to_owned(),
        11 => "That target is invalid for this skill.".to_owned(),
        13 => "Holy Water required.".to_owned(),
        14 => "An Ancilla is required.".to_owned(),
        15 => "Another one of these is already in range.".to_owned(),
        16 => "You need another skill first.".to_owned(),
        22 => "That is already active.".to_owned(),
        23 => "The conditions for this skill are not met.".to_owned(),
        26 => "You can't place it there.".to_owned(),
        // Everything below is a cause Hercules genuinely emits (audited against
        // all 202 `clif->skill_fail` sites in `src/map/`), each worded from its
        // emitting site rather than from the enum name.
        12 => "You are already carrying three Ancillae.".to_owned(),                   // skill.c:16174
        17 => "That needs a partner from your party nearby.".to_owned(),               // skill.c:6539, chorus
        19 => "You already have the maximum number of these summoned.".to_owned(),     // skill.c:16230
        20 => "You have nothing summoned to release.".to_owned(),                      // skill.c:16224
        21 => "No skill has been copied yet.".to_owned(),                              // clif.c:20978
        25 => "That requires riding a dragon.".to_owned(),                             // skill.c:16537
        31 => "That must follow Weapon Blocking.".to_owned(),                          // skill.c:5756
        32 => "That requires a poisoned weapon.".to_owned(),                           // skill.c:12977
        33 => "That requires a Mado Gear.".to_owned(),                                 // skill.c:16555
        35 => "That cannot be used on monsters or boss monsters.".to_owned(),          // skill.c:11647
        36 => "That only works on players, and only where PvP is allowed.".to_owned(), // skill.c:11596
        37 => "A cannonball must be equipped in the ammunition slot.".to_owned(),      // skill.c:17016
        43 => "You have nothing to poison the weapon with.".to_owned(),                // clif.c:20927
        51 => "You have no spellbook to read.".to_owned(),                             // clif.c:20850
        52 => "You do not know that spellbook's skill, and it puts you to sleep.".to_owned(), // skill.c:20924
        53 => "That would exceed the spell points you can preserve.".to_owned(),       // skill.c:20933
        54 => "You cannot memorise any more spells.".to_owned(),                       // skill.c:10516
        57 => "That requires a pushcart.".to_owned(),                                  // skill.c:16491
        // Hercules emits this from exactly one site (`skill.c:16257`, Wug
        // Mastery under SC__GROOMY) and sends **no** accompanying message,
        // despite the name meaning "the server notifies manually". Printing
        // nothing would leave the player with silence, so name that one cause.
        70 => "You are too gloomy to handle your wolf.".to_owned(),
        73 => "That must follow another skill in its combo.".to_owned(),               // skill.c:15889
        74 => "Not enough soul spheres.".to_owned(),                                   // skill.c:16674
        75 => "That requires Fury.".to_owned(),                                        // skill.c:16509
        79 => "You have no elemental summoned.".to_owned(),                            // skill.c:16367
        80 => "Your homunculus is not intimate enough.".to_owned(),                    // skill.c:1432
        83 => "You are standing too close to an NPC.".to_owned(),                      // clif.c:13088
        // 71 / 72 (a required item / equipment) never reach here — they are
        // returned as `SkillFailure::MissingItem` so the client can name the item.
        84 => "Not enough ammunition.".to_owned(),
        85 => "Not enough coins.".to_owned(), // skill.c:16683, Gunslinger
        // Hercules does not currently emit this, but it is the cause that
        // *should* carry an ensemble partner failure — see the cause-0 branch.
        94 => "That needs an ensemble partner.".to_owned(),
        cause => format!("Skill failed (reason {cause})."),
    }
}

#[cfg(test)]
mod skill_failure_text_tests {
    use ragnarok_packets::{ItemId, SkillId, ToUseSkillSuccessPacket};

    use super::{ENSEMBLE_SKILL_IDS, SkillFailure, skill_failed_reason, skill_failed_text};

    fn failure(skill_id: u16, cause: u8) -> ToUseSkillSuccessPacket {
        ToUseSkillSuccessPacket {
            skill_id: SkillId(skill_id),
            btype: 0,
            item_id: ItemId(0),
            flag: 0,
            cause,
        }
    }

    /// Cause 0 on an ensemble skill is a *missing partner* (`unit.c:1566`), and
    /// the default text sends the player to their skill level instead — which
    /// cost a live session twice.
    #[test]
    fn ensemble_cause_zero_names_the_partner_requirement() {
        for skill_id in ENSEMBLE_SKILL_IDS {
            let text = skill_failed_text(&failure(skill_id, 0));
            assert!(
                !text.contains("level is not high enough"),
                "skill {skill_id} still blames the skill level: {text}"
            );
        }

        assert!(skill_failed_text(&failure(395, 0)).contains("partner"));
        // Benedictio is an ensemble too, but its helpers are Acolytes, not a
        // Bard/Dancer partner.
        assert!(skill_failed_text(&failure(69, 0)).contains("Acolyte"));
    }

    /// A non-ensemble skill must keep the generic cause-0 wording.
    #[test]
    fn ordinary_cause_zero_is_unchanged() {
        assert_eq!(skill_failed_text(&failure(28, 0)), "Skill level is not high enough.");
    }

    /// Every cause Hercules actually emits should read as a sentence, not as
    /// "Skill failed (reason N)".
    #[test]
    fn every_emitted_cause_has_real_text() {
        // Audited from all `clif->skill_fail` sites in Hercules `src/map/`.
        // 71 and 72 are absent deliberately: they carry an item id, so
        // `skill_failed_reason` routes them to `MissingItem` and they never
        // reach this function.
        const EMITTED: [u8; 45] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 19, 20, 21, 22, 23, 25, 26, 31, 32, 33, 35, 36, 37, 43,
            51, 52, 53, 54, 57, 73, 74, 75, 79, 80, 83, 84, 85,
        ];

        for cause in EMITTED {
            let text = skill_failed_text(&failure(28, cause));
            assert!(!text.contains("reason"), "cause {cause} has no text: {text}");
        }

        for cause in [71, 72] {
            assert!(
                matches!(skill_failed_reason(&failure(28, cause)), SkillFailure::MissingItem { .. }),
                "cause {cause} must name the item instead of printing text"
            );
        }
    }
}

#[cfg(test)]
mod length_agreement_tests {
    use ragnarok_bytes::ByteReader;
    use ragnarok_packets::handler::{HandlerResult, NoPacketCallback, PacketHandler};

    use super::*;
    use crate::packet_versions::lengths_20220406::PACKET_LENGTHS;

    /// Every registered server packet must consume exactly the number of bytes
    /// Hercules' own length table says it sends.
    ///
    /// A struct that is even one byte short leaves a stray byte in the read
    /// buffer, and the next header is then read misaligned. That does **not**
    /// surface as a deserialization failure: the bogus header simply is not in
    /// the table, so `process_one` reports `UnhandledPacket` and the rest of the
    /// buffer — real packets included — is dropped silently. A full suite run
    /// showed exactly that, as pseudo-headers `0x8600` / `0xD700` / `0xFE00` /
    /// `0x7F00`, each of which decodes as a leading `0x00` followed by a genuine
    /// packet (`ZC_NOTIFY_MOVE`, `ZC_SPRITE_CHANGE2`, a spawn, `ZC_NOTIFY_TIME`).
    ///
    /// Zeros are used as the payload, so packets that reject a zeroed body are
    /// skipped rather than failed — this checks framing, not validation.
    #[test]
    fn registered_packets_consume_their_declared_length() {
        let mut mismatches = Vec::new();

        // Some handlers are stateful (the inventory list accumulator) and panic
        // on a zeroed body. Silence the hook and give each packet a fresh
        // handler, so one hostile input neither spams the log nor leaves a
        // half-borrowed `RefCell` that would poison every later packet.
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));

        for &(header, length) in PACKET_LENGTHS {
            // Variable-length packets carry their own size; nothing to compare.
            if length < 2 {
                continue;
            }
            let length = length as usize;

            let mut bytes: Vec<u8> = header.to_le_bytes().to_vec();
            bytes.resize(length, 0);

            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut packet_handler: PacketHandler<NetworkEventList, NoPacketCallback> = PacketHandler::default();
                register_map_server_packets(&mut packet_handler).expect("registration must not duplicate");

                let mut byte_reader = ByteReader::without_metadata(&bytes);
                match packet_handler.process_one(&mut byte_reader) {
                    HandlerResult::Ok(_) => Some(byte_reader.get_offset()),
                    // Not registered (the fallback consumes it exactly), cut
                    // off, or the body rejected zeros — none of which is a
                    // framing disagreement.
                    _ => None,
                }
            }));

            if let Ok(Some(consumed)) = outcome
                && consumed != length
            {
                mismatches.push(format!("0x{header:04X}: consumed {consumed}, table says {length}"));
            }
        }

        std::panic::set_hook(previous_hook);

        assert!(
            mismatches.is_empty(),
            "packets disagree with the length table, so the reader will misalign:\n  {}",
            mismatches.join("\n  ")
        );
    }
}
