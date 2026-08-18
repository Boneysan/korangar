use std::cell::RefCell;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::{Duration, Instant};

use ragnarok_packets::handler::{DuplicateHandlerError, PacketCallback, PacketHandler};
use ragnarok_packets::*;

use crate::event::{NetworkEventList, NoNetworkEvents};
use crate::items::{IT_AMMO, ItemQuantity};
use crate::{
    CharacterServerLoginData, HotkeyState, InventoryItem, InventoryItemDetails, LoginServerLoginData, MessageColor, NetworkEvent,
    NoMetadata, ShopItem, UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
};

type PendingInventoryItems = Rc<RefCell<Option<(u8, Vec<InventoryItem<NoMetadata>>)>>>;

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
    let inventory_items: PendingInventoryItems = Rc::new(RefCell::new(None));
    // Equipped ammunition seen while a list is still being accumulated.
    //
    // Hercules sends `clif_arrowequip` from *inside* `clif_inventoryItems`, between
    // the stackable list and the equippable list — so at login it arrives before
    // the End packet that publishes the inventory. Emitting it immediately
    // would apply the AMMO flag to the outgoing inventory and then lose it to
    // the `SetInventory` that follows, leaving the Ammo slot empty every login.
    // Hold it and apply it at End.
    let pending_equipped_ammunition: Rc<RefCell<Option<InventoryIndex>>> = Rc::new(RefCell::new(None));
    // The runtime reason for the cause-0 failure that is about to arrive
    // (`ZC_SKILL_FAIL_REASON`, our fork packet). Hercules sends it immediately
    // before `ZC_ACK_TOUSESKILL`, so this only ever holds a value across those
    // two reads. Keyed by skill id and taken on use, so a reason that somehow
    // arrives without its failure cannot be attached to a later, unrelated one.
    let pending_skill_fail_reason: Rc<RefCell<Option<(SkillId, SkillFailReason)>>> = Rc::new(RefCell::new(None));

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
    // Reason table from `clif.c:770-800`. Only the codes Hercules actually
    // reaches are named; the rest fall through rather than inventing text for
    // Gravity's regional billing states.
    packet_handler.register(|packet: MapDisconnectReasonPacket| NetworkEvent::MapDisconnectReason {
        message: match packet.reason {
            0 => "Disconnected from the server.".to_owned(),
            1 => "The server is closing.".to_owned(),
            2 => "Someone else logged in with this account.".to_owned(),
            3 => "Disconnected: the connection timed out.".to_owned(),
            4 => "Disconnected: the server is full.".to_owned(),
            8 => "The server still recognises your last connection.".to_owned(),
            9 => "Too many connections from this address.".to_owned(),
            10 => "Disconnected: out of paid time.".to_owned(),
            15 => "A GM disconnected you.".to_owned(),
            108 => "Disconnected: this address is blocked.".to_owned(),
            109 => "Disconnected: too many invalid password attempts.".to_owned(),
            110 => "Disconnected: this job class is not allowed here.".to_owned(),
            113 => "Access is restricted between midnight and 6:00am.".to_owned(),
            115 => "You are in a connection ban period.".to_owned(),
            reason => format!("Disconnected by the server (reason {reason})."),
        },
    })?;
    packet_handler.register(|packet: MessageTableNumberPacket| NetworkEvent::MessageTableNumber {
        message_id: packet.message_id,
        value: packet.value,
    })?;
    packet_handler.register(|packet: IgnoreAllResultPacket| NetworkEvent::IgnoreAllResult {
        ignore_type: packet.ignore_type,
        result: packet.result,
    })?;
    packet_handler.register(|packet: WarpMemoResultPacket| NetworkEvent::ChatMessage {
        text: match packet.result {
            0 => "Destination memorised.".to_owned(),
            1 => "Your Warp Portal level is not high enough to memorise a destination.".to_owned(),
            _ => "You have not learned Warp Portal.".to_owned(),
        },
        color: match packet.result {
            0 => MessageColor::Server,
            _ => MessageColor::Error,
        },
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
    // Hercules emits a legacy 0x0078 entity-shaped packet solely as an
    // invisible script-dialog anchor. Parsing it removes it from the unknown
    // packet backlog; publishing AddEntity here would create a phantom actor.
    packet_handler.register_noop::<FakeNpcDialogAnchorPacket>()?;
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
    // Yes, Hercules sends `n + 2` here (`clif_equipitemack` /
    // `clif_unequipitemack`), but do **not** subtract it in the handler: both
    // fields are typed `InventoryIndex`, whose `FromBytes` already does the `-
    // 2` (see the type in `ragnarok-packets`). Subtracting again lands two
    // slots early. Only the inventory *list* adjusts by hand, because its items
    // carry `RawIndex`.
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
    // The only acknowledgement a kicking DM ever gets. `ACMD(kick)` prints
    // nothing on success (`atcommand.c:3450` returns straight after
    // `clif->GM_kick`), and that function's entire feedback path is
    // `clif->GM_kickack(sd, 1)` (`clif.c:9410`) — so while this was a no-op a DM
    // typed `@kick`, watched the target vanish, and was told nothing. Failure
    // (`0`) comes only from the right-click "force to quit" path
    // (`clif_parse_GMKick`): no such target, or no permission for the
    // `@killmonster` / `@unloadnpc` it delegates to. The command's *own* failures
    // — no name given, character not found, outranked — already arrive as
    // ordinary `clif->message` lines, so this must not restate them.
    packet_handler.register(|packet: GmKickResponsePacket| match packet.result {
        GmKickResponseStatus::Success => NetworkEvent::ChatMessage {
            text: "The player has been disconnected.".to_owned(),
            color: MessageColor::Server,
        },
        GmKickResponseStatus::Failure => NetworkEvent::ChatMessage {
            text: "That target could not be disconnected.".to_owned(),
            color: MessageColor::Error,
        },
    })?;
    // Hunter Talkie Box trap text (`ZC_TALKBOX_CHATCONTENTS`). This was a no-op
    // under the claim that the text is "rendered at the trap"; nothing in this
    // tree renders it anywhere — the only Talkie Box in `korangar/src` is the
    // trap's own prop model, so the prop drew and the message it exists to carry
    // was dropped.
    //
    // Chat, for the reason `ShowScript` and `OverheadMessagePacket` go there:
    // there is no overhead-text surface yet. The label is not decoration. The
    // packet's `aid` is the **skill unit's** block id (`clif_talkiebox` is called
    // with `&src->bl`, `skill.c:14511`), not a player or anything in `entities`,
    // so the id can name nobody and an unlabelled line would arrive from
    // no one. The trap also never talks to its own owner (`sg->src_id == bl->id`
    // breaks first), so whoever sees this did not write it.
    packet_handler.register(|packet: TalkieBoxMessagePacket| NetworkEvent::ChatMessage {
        text: format!("Talkie Box: {}", packet.message),
        color: MessageColor::Broadcast,
    })?;
    // Gospel names the buff it just handed you (`ZC_GOSPEL_INFO`). This was
    // consumed by the length fallback and dropped, so a Paladin running Gospel
    // was silently receiving a stream of major effects -- +100% ATK, +100% MaxHP,
    // a Holy weapon -- with nothing on screen to say which. The fallback ate it
    // *cleanly*, so it never appeared in the unknown-packet ledger either, and no
    // headless scenario casts Gospel with a party to expose it.
    //
    // An unrecognised code is reported rather than swallowed: the codes are
    // Gravity's and a gap here should be visible, not silent.
    packet_handler.register(|packet: GospelInfoPacket| NetworkEvent::ChatMessage {
        text: match gospel_info_text(packet.info_type) {
            Some(text) => text.to_owned(),
            // Deliberately does not name Gospel: 0x28 already proves this packet
            // carries non-Gospel notices, so an unknown code must not be
            // attributed to a skill that may have nothing to do with it.
            None => format!("Unrecognised server notice ({:#04x}).", packet.info_type),
        },
        color: MessageColor::Information,
    })?;
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
    let reason_slot = pending_skill_fail_reason.clone();
    packet_handler.register(move |packet: SkillFailReasonPacket| {
        // Resolved here, not on the wire: an unknown reason must degrade to
        // `None`, never fail the packet.
        *reason_slot.borrow_mut() = SkillFailReason::from_wire(packet.reason).map(|reason| (packet.skill_id, reason));
        NoNetworkEvents
    })?;
    let reason_slot = pending_skill_fail_reason.clone();
    packet_handler.register(move |packet: ToUseSkillSuccessPacket| -> NetworkEventList {
        // Take it either way: a reason left behind by a suppressed failure must
        // not survive to explain a different skill.
        let reason = reason_slot
            .borrow_mut()
            .take()
            .filter(|&(skill_id, _)| skill_id == packet.skill_id)
            .map(|(_, reason)| reason);

        if packet.flag != 0 {
            return NetworkEventList::default();
        }

        // A refusal that names its reason is usually the whole diagnosis, and
        // until 2026-08-17 none of this reached the log at all -- a live pass
        // could see the message on screen but had no way to tell which cause
        // produced it, which is the difference between "the server refused" and
        // "we worded it wrong". Cause and item id are the raw wire values.
        if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
            eprintln!(
                "[skill-fail] skill={} cause={} btype={} item={} reason={:?}",
                packet.skill_id.0, packet.cause, packet.btype, packet.item_id.0, reason
            );
        }

        let reported = match skill_failed_reason(&packet, reason) {
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
    // Graffiti-only skill unit (`ZC_SKILL_ENTRY2`). Maps to the same AddSkillUnit
    // event ordinary units use so RG_GRAFFITI placement is observable.
    packet_handler.register(|packet: NotifySkillUnitGraffitiPacket| {
        let NotifySkillUnitGraffitiPacket {
            entity_id,
            position,
            unit_id,
            ..
        } = packet;
        NetworkEvent::AddSkillUnit {
            entity_id,
            // Only graffiti uses this header; keep the enum variant explicit so a
            // future non-0xB0 sender is still a visible skill unit.
            unit_id: if unit_id == 0xB0 { UnitId::Graffiti } else { UnitId::Dummyskill },
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
    packet_handler.register(|packet: SoundEffectPacket| NetworkEvent::PlaySoundEffect {
        file_name: packet.file_name,
        // Hercules zeroes the id for a repeating sound on purpose, so there is
        // nothing to attach it to in space.
        entity_id: (packet.entity_id.0 != 0).then_some(packet.entity_id),
    })?;
    packet_handler.register(|packet: ShowScriptPacket| NetworkEvent::ShowScript {
        entity_id: packet.entity_id,
        message: packet.message,
    })?;
    packet_handler.register(|packet: ProgressBarPacket| NetworkEvent::ProgressBar {
        duration: Some(Duration::from_secs(u64::from(packet.seconds))),
    })?;
    packet_handler.register(|_: ProgressBarAbortPacket| NetworkEvent::ProgressBar { duration: None })?;
    // `specialeffectnum`. The value rides along but nothing displays it yet, so
    // the effect itself is what matters — same visual as plain `specialeffect`.
    packet_handler.register(|packet: SpecialEffectValuePacket| NetworkEvent::SpecialEffect {
        entity_id: packet.entity_id,
        effect_id: packet.effect_id,
    })?;
    packet_handler.register(|packet: AddSkillPacket| NetworkEvent::SkillAdded {
        skill_information: packet.skill_information,
    })?;
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
const ALL_PARTYFLEE: u16 = 693;
const PR_REDEMPTIO: u16 = 1014;

/// The `State:` precondition a skill declares in `skill_db.conf`, or `None`
/// when it declares one whose failure Hercules reports with a real cause.
///
/// Hercules checks all of these in one shared switch and reports every one as
/// cause 0, so the skill id is the only thing that distinguishes them. The
/// table is generated (`tools/generate_skill_states.py`) rather than written
/// out here, because a hand-kept copy goes stale in silence the moment a skill
/// gains or loses a state — the previous hand-written shield list had that
/// problem, and it also missed Brandish Spear, Blitz Beat, Raid and Cart
/// Termination.
fn skill_state(skill_id: u16) -> Option<super::skill_states::SkillState> {
    super::skill_states::SKILL_STATES
        .binary_search_by_key(&skill_id, |&(id, _)| id)
        .ok()
        .map(|index| super::skill_states::SKILL_STATES[index].1)
}

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
fn skill_failed_reason(packet: &ToUseSkillSuccessPacket, reason: Option<SkillFailReason>) -> SkillFailure {
    match packet.cause {
        // Hercules' catch-all for a required item with no dedicated cause —
        // Yellow Gemstone (Land Protector) lands here, not on 7/8. It sends the
        // required count in `btype` and the item in `item_id`.
        71 | 72 => SkillFailure::MissingItem {
            item_id: packet.item_id,
            amount: packet.btype.clamp(0, i32::from(u16::MAX)) as u16,
            equipment: packet.cause == 72,
        },
        _ => SkillFailure::Text(skill_failed_text(packet, reason)),
    }
}

/// Text for a runtime reason the server named outright, over our fork packet.
///
/// These are the outcomes no static table can reach — a roll that missed, an
/// empty splash, a partner who is not there — so before this packet existed the
/// client could only key on the skill id and enumerate every condition the
/// skill has.
fn skill_fail_reason_text(reason: SkillFailReason) -> &'static str {
    match reason {
        SkillFailReason::EnsemblePartner => concat!(
            "That needs a partner within 4 cells: a Bard or Dancer class of the opposite sex, ",
            "in your party, holding an instrument or whip, who knows the same skill and is not ",
            "already performing."
        ),
        SkillFailReason::BenedictioHelpers => "That needs two Acolyte-class helpers standing to your left and right.",
        SkillFailReason::NoParty => "You have to be in a party to use that.",
        SkillFailReason::NoOneInRange => "No dead party member was in range.",
        SkillFailReason::NotEnoughExperience => "That spends 1% of your base and job experience, and you do not have it.",
        SkillFailReason::TargetResisted => "The target resisted.",
        SkillFailReason::NothingToSteal => "There was nothing to steal.",
        SkillFailReason::SuppressedByKyomu => "Kyomu suppressed the skill.",
        SkillFailReason::TargetImmune => "The target cannot be affected by that.",
        SkillFailReason::NeedsWarpPortal => "That has to be cast beside a warp portal.",
    }
}

/// The whole sentence behind a `ZC_GOSPEL_INFO` code, or `None` for one this
/// build does not know.
///
/// **Despite the name, this packet is not Gospel-only.** Upstream reuses it as
/// a general info channel: `skill.c:8613` sends `0x28` from `ST_FULLSTRIP` when
/// Full Chemical Protection blocks a strip, which has nothing to do with
/// Gospel. So the text is returned complete, prefix included, rather than the
/// caller pasting "Gospel:" in front of whatever arrives -- doing that
/// announced a Stripper's failure as a Gospel buff, which is the
/// confidently-wrong-message failure this tree keeps repeating.
///
/// The Gospel codes come from `skill.c`'s `UNT_GOSPEL` support branch, which
/// rolls one of thirteen effects. **Only ten report:** the heal, Blessing and
/// Increase AGI cases send no packet, because each already announces itself --
/// a heal number floats and both statuses have their own icon. So a silent buff
/// is not necessarily a missing code. The offensive branch reports nothing at
/// all, so a Gospel line always means *you received* something.
fn gospel_info_text(info_type: u32) -> Option<&'static str> {
    let text = match info_type {
        0x15 => "Gospel: all negative statuses cleared.",
        0x16 => "Gospel: immune to all status effects.",
        0x17 => "Gospel: max HP doubled.",
        0x18 => "Gospel: max SP doubled.",
        0x19 => "Gospel: all stats +20.",
        0x1C => "Gospel: your weapon is enchanted with Holy.",
        0x1D => "Gospel: your armour is enchanted with Holy.",
        0x1E => "Gospel: DEF +25%.",
        0x1F => "Gospel: ATK +100%.",
        0x20 => "Gospel: HIT and Flee +50.",
        // Not Gospel: Full Chemical Protection blocked a strip attempt.
        0x28 => "Full Chemical Protection blocked the strip.",
        _ => return None,
    };

    Some(text)
}

/// Text for every `ZC_ACK_TOUSESKILL` cause the networking crate can render on
/// its own (i.e. everything that needs no item lookup).
fn skill_failed_text(packet: &ToUseSkillSuccessPacket, reason: Option<SkillFailReason>) -> String {
    // Hercules overloads USESKILL_FAIL_LEVEL for outcomes that have nothing to do
    // with skill level, so a maxed skill reports "level not high enough". These are
    // the ones verified in `skill.c` to mean the target resisted or the roll missed
    // — extend only after checking the source, since most of the ~60 other cause-0
    // emitters really are unmet conditions.
    if packet.cause == 0 {
        // The server said what actually failed. Everything below this point is
        // inference from the skill id, kept only so a stock Hercules — or one
        // that lost the delta in a merge — still reads better than "level".
        if let Some(reason) = reason {
            return skill_fail_reason_text(reason).to_owned();
        }

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

        // A *static* precondition: the skill declares it in `skill_db.conf`, so
        // both sides already know it and nothing needs to be sent. 42 skills
        // across 13 states, none of which the protocol ever gave a code.
        //
        // A skill can also fail cause 0 for a runtime reason it happens to share
        // with its precondition — Shield Reflect has a 5%-per-level `SC_KYOMU`
        // roll (`skill.c:16379`) — and no static table can tell those apart.
        // That is what `ZC_SKILL_FAIL_REASON` is for, and it is consulted first,
        // above.
        if let Some(state) = skill_state(packet.skill_id.0) {
            return state.requirement().to_owned();
        }

        let resisted = match packet.skill_id.0 {
            // `skill.c:10003` — the only cause-0 path this skill has.
            ALL_PARTYFLEE => Some("You have to be in a party to use that."),
            // Three indistinguishable cause-0 paths, so the message names all
            // three conditions: `skill.c:7004` (no party), `skill.c:7013` (the
            // splash reached nobody), `skill.c:16056` (under 1% of base or job
            // experience, which is what the skill spends).
            PR_REDEMPTIO => Some(concat!(
                "Redemptio needs a party, at least one dead party member in range, ",
                "and 1% of your base and job experience to spend."
            )),
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
        12 => "You are already carrying three Ancillae.".to_owned(), // skill.c:16174
        17 => "That needs a partner from your party nearby.".to_owned(), // skill.c:6539, chorus
        19 => "You already have the maximum number of these summoned.".to_owned(), // skill.c:16230
        20 => "You have nothing summoned to release.".to_owned(),    // skill.c:16224
        21 => "No skill has been copied yet.".to_owned(),            // clif.c:20978
        25 => "That requires riding a dragon.".to_owned(),           // skill.c:16537
        31 => "That must follow Weapon Blocking.".to_owned(),        // skill.c:5756
        32 => "That requires a poisoned weapon.".to_owned(),         // skill.c:12977
        33 => "That requires a Mado Gear.".to_owned(),               // skill.c:16555
        35 => "That cannot be used on monsters or boss monsters.".to_owned(), // skill.c:11647
        36 => "That only works on players, and only where PvP is allowed.".to_owned(), // skill.c:11596
        37 => "A cannonball must be equipped in the ammunition slot.".to_owned(), // skill.c:17016
        43 => "You have nothing to poison the weapon with.".to_owned(), // clif.c:20927
        51 => "You have no spellbook to read.".to_owned(),           // clif.c:20850
        52 => "You do not know that spellbook's skill, and it puts you to sleep.".to_owned(), // skill.c:20924
        53 => "That would exceed the spell points you can preserve.".to_owned(), // skill.c:20933
        54 => "You cannot memorise any more spells.".to_owned(),     // skill.c:10516
        57 => "That requires a pushcart.".to_owned(),                // skill.c:16491
        // Hercules emits this from exactly one site (`skill.c:16257`, Wug
        // Mastery under SC__GROOMY) and sends **no** accompanying message,
        // despite the name meaning "the server notifies manually". Printing
        // nothing would leave the player with silence, so name that one cause.
        70 => "You are too gloomy to handle your wolf.".to_owned(),
        73 => "That must follow another skill in its combo.".to_owned(), // skill.c:15889
        74 => "Not enough soul spheres.".to_owned(),                     // skill.c:16674
        75 => "That requires Fury.".to_owned(),                          // skill.c:16509
        79 => "You have no elemental summoned.".to_owned(),              // skill.c:16367
        80 => "Your homunculus is not intimate enough.".to_owned(),      // skill.c:1432
        83 => "You are standing too close to an NPC.".to_owned(),        // clif.c:13088
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

    use super::{ALL_PARTYFLEE, ENSEMBLE_SKILL_IDS, PR_REDEMPTIO, SkillFailure, skill_failed_reason, skill_failed_text, skill_state};

    /// Rendering with no fork packet in play — i.e. what a stock Hercules, or
    /// one that lost the delta, still produces.
    fn text(packet: &ToUseSkillSuccessPacket) -> String {
        skill_failed_text(packet, None)
    }

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
            let text = text(&failure(skill_id, 0));
            assert!(
                !text.contains("level is not high enough"),
                "skill {skill_id} still blames the skill level: {text}"
            );
        }

        assert!(text(&failure(395, 0)).contains("partner"));
        // Benedictio is an ensemble too, but its helpers are Acolytes, not a
        // Bard/Dancer partner.
        assert!(text(&failure(69, 0)).contains("Acolyte"));
    }

    /// A non-ensemble skill must keep the generic cause-0 wording.
    #[test]
    fn ordinary_cause_zero_is_unchanged() {
        assert_eq!(text(&failure(28, 0)), "Skill level is not high enough.");
    }

    /// Every `State:` precondition is checked in one shared place and reported
    /// as cause 0, so a Crusader with no shield was told to level the skill up.
    /// `USESKILL_FAIL_NEED_SHIELD_WEAPON` (110) exists and Hercules never sends
    /// it; the other twelve states have no cause at all.
    #[test]
    fn state_cause_zero_names_the_precondition() {
        for &(skill_id, state) in super::super::skill_states::SKILL_STATES {
            let text = text(&failure(skill_id, 0));
            assert_eq!(text, state.requirement(), "skill {skill_id}");
            assert!(!text.contains("level is not high enough"), "skill {skill_id}: {text}");
        }

        // Spot-checks that the generated table is actually reaching the text,
        // including the four the hand-written shield list never had.
        assert_eq!(text(&failure(249, 0)), "That needs a shield equipped."); // CR_AUTOGUARD
        assert_eq!(text(&failure(129, 0)), "That needs a falcon."); // HT_BLITZBEAT
        assert_eq!(text(&failure(214, 0)), "That needs you to be hiding."); // RG_RAID
        assert_eq!(text(&failure(57, 0)), "That needs you to be riding."); // KN_BRANDISHSPEAR

        // A skill with no state precondition must not pick any of this up.
        assert_eq!(text(&failure(28, 0)), "Skill level is not high enough.");
    }

    /// Every reason the server can send needs text; an append-only enum is easy
    /// to extend on one side only.
    #[test]
    fn every_skill_fail_reason_has_text() {
        use ragnarok_packets::SkillFailReason::*;

        for reason in [
            EnsemblePartner,
            BenedictioHelpers,
            NoParty,
            NoOneInRange,
            NotEnoughExperience,
            TargetResisted,
            NothingToSteal,
            SuppressedByKyomu,
            TargetImmune,
            NeedsWarpPortal,
        ] {
            assert!(super::skill_fail_reason_text(reason).ends_with('.'), "{reason:?}");
        }
    }

    /// The wire values are a table spanning two repositories, and a drift shows
    /// up as a confident wrong sentence rather than as silence — the shape this
    /// project keeps getting caught by. Pinned against Hercules' `clif.h`.
    #[test]
    fn wire_reasons_match_the_server_enum() {
        use ragnarok_packets::SkillFailReason as R;

        for (wire, expected) in [
            (1, R::EnsemblePartner),
            (2, R::BenedictioHelpers),
            (3, R::NoParty),
            (4, R::NoOneInRange),
            (5, R::NotEnoughExperience),
            (6, R::TargetResisted),
            (7, R::NothingToSteal),
            (8, R::SuppressedByKyomu),
            (9, R::TargetImmune),
            (10, R::NeedsWarpPortal),
        ] {
            assert_eq!(R::from_wire(wire), Some(expected), "wire value {wire}");
        }

        // 0 is SKILLFAILREASON_NONE, and anything above the last known reason is
        // a server newer than this build. Both must resolve to `None` — *not* to
        // a deserialization failure, which would cost the whole read buffer.
        // NOTE: this boundary moves every time a reason is appended. Adding one
        // without moving it leaves the guard asserting the opposite of the truth.
        assert_eq!(R::from_wire(0), Option::None);
        assert_eq!(R::from_wire(11), Option::None);
        assert_eq!(R::from_wire(u16::MAX), Option::None);
    }

    /// A reason the server named beats anything inferred from the skill id —
    /// that is the whole point of the packet.
    #[test]
    fn a_named_reason_overrides_the_skill_id_inference() {
        use ragnarok_packets::SkillFailReason;

        // Shield Reflect has a shield precondition *and* a Kyomu roll.
        let inferred = text(&failure(252, 0));
        assert_eq!(inferred, "That needs a shield equipped.");
        let named = skill_failed_text(&failure(252, 0), Some(SkillFailReason::SuppressedByKyomu));
        assert!(named.contains("Kyomu"), "{named}");

        // Redemptio's three conditions collapse to the one that actually failed.
        let inferred = text(&failure(PR_REDEMPTIO, 0));
        assert!(inferred.contains("party") && inferred.contains("experience"), "{inferred}");
        let named = skill_failed_text(&failure(PR_REDEMPTIO, 0), Some(SkillFailReason::NotEnoughExperience));
        assert!(named.contains("experience") && !named.contains("dead party member"), "{named}");
    }

    /// The generated table is binary-searched, so it has to stay sorted.
    #[test]
    fn generated_skill_state_table_is_sorted_and_reachable() {
        let table = super::super::skill_states::SKILL_STATES;
        assert!(
            table.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "table is not sorted by skill id"
        );
        for &(skill_id, state) in table {
            assert_eq!(skill_state(skill_id), Some(state), "skill {skill_id} is not findable");
        }
        assert_eq!(skill_state(28), None);
    }

    /// The two party-conditional skills. Redemptio has three indistinguishable
    /// cause-0 paths, so its message has to name all three conditions; Party
    /// Flee has exactly one and can be precise.
    #[test]
    fn party_cause_zero_names_the_party_requirement() {
        let party_flee = text(&failure(ALL_PARTYFLEE, 0));
        assert!(party_flee.contains("party"), "{party_flee}");
        assert!(!party_flee.contains("level is not high enough"), "{party_flee}");

        let redemptio = text(&failure(PR_REDEMPTIO, 0));
        assert!(redemptio.contains("party"), "{redemptio}");
        assert!(redemptio.contains("experience"), "{redemptio}");
        assert!(!redemptio.contains("level is not high enough"), "{redemptio}");
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
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 19, 20, 21, 22, 23, 25, 26, 31, 32, 33, 35, 36, 37, 43, 51, 52,
            53, 54, 57, 73, 74, 75, 79, 80, 83, 84, 85,
        ];

        for cause in EMITTED {
            let text = text(&failure(28, cause));
            assert!(!text.contains("reason"), "cause {cause} has no text: {text}");
        }

        for cause in [71, 72] {
            assert!(
                matches!(skill_failed_reason(&failure(28, cause), None), SkillFailure::MissingItem { .. }),
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
    /// the table, so `process_one` reports `UnhandledPacket` and the rest of
    /// the buffer — real packets included — is dropped silently. A full
    /// suite run showed exactly that, as pseudo-headers `0x8600` / `0xD700`
    /// / `0xFE00` / `0x7F00`, each of which decodes as a leading `0x00`
    /// followed by a genuine packet (`ZC_NOTIFY_MOVE`, `ZC_SPRITE_CHANGE2`,
    /// a spawn, `ZC_NOTIFY_TIME`).
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

#[cfg(test)]
mod dropped_feature_tests {
    use ragnarok_bytes::ByteReader;
    use ragnarok_packets::handler::{HandlerResult, NoPacketCallback, PacketHandler};

    use super::*;

    /// Drive raw bytes through the **real** map-server registration and return
    /// what reaches the client.
    ///
    /// Registration is the thing under test, which is why this does not call
    /// the handler closures directly: both packets below spent time registered
    /// as `register_noop`, and that is a change no test of a closure in
    /// isolation can see. A no-op parses the packet perfectly and publishes
    /// nothing, so the tell is an empty event list, not an error.
    fn events(bytes: &[u8]) -> Vec<NetworkEvent> {
        let mut packet_handler: PacketHandler<NetworkEventList, NoPacketCallback> = PacketHandler::default();
        register_map_server_packets(&mut packet_handler).expect("registration must not duplicate");

        let mut byte_reader = ByteReader::without_metadata(bytes);
        match packet_handler.process_one(&mut byte_reader) {
            HandlerResult::Ok(events) => events.0,
            HandlerResult::UnhandledPacket => panic!("no handler is registered for this header at all"),
            HandlerResult::PacketCutOff => panic!("the test payload is shorter than the packet"),
            HandlerResult::InternalError(error) => panic!("the packet failed to deserialize: {error:?}"),
        }
    }

    /// Gospel's buff announcements (`ZC_GOSPEL_INFO`, 0x0215) were dropped by
    /// the length fallback, so a party under Gospel silently received one of
    /// ten major effects per interval with nothing on screen naming it.
    ///
    /// Driven through the **real registration** rather than the mapping
    /// function alone: a no-op parses perfectly and publishes nothing, and
    /// the only tell is an empty event list, which no test of the text
    /// table in isolation can see.
    #[test]
    fn gospel_names_the_buff_it_granted() {
        // 0x1f is ATK +100% -- one of the largest effects in the table.
        match events(&[0x15, 0x02, 0x1F, 0x00, 0x00, 0x00]).as_slice() {
            [NetworkEvent::ChatMessage { text, .. }] => {
                assert!(text.contains("ATK"), "0x1f should name the ATK buff, got {text:?}");
            }
            [] => panic!("0x0215 published nothing — Gospel's buffs are being dropped again"),
            other => panic!("0x0215 produced {other:?}"),
        }

        // An unknown code must still reach the player. The codes are Gravity's,
        // so a gap in our table should be visible rather than swallowed.
        match events(&[0x15, 0x02, 0xFE, 0x00, 0x00, 0x00]).as_slice() {
            [NetworkEvent::ChatMessage { text, .. }] => {
                assert!(text.contains("nrecognised"), "an unmapped code should say so: {text:?}");
                // It must NOT blame Gospel: this packet carries non-Gospel
                // notices too, so an unknown code has no attribution to give.
                assert!(
                    !text.contains("Gospel"),
                    "an unknown code must not be blamed on Gospel: {text:?}"
                );
            }
            other => panic!("an unknown code produced {other:?}"),
        }

        // 0x28 is NOT Gospel -- `ST_FULLSTRIP` sends it when Full Chemical
        // Protection blocks a strip. Announcing it as a Gospel buff is the
        // plausible-wrong-text failure, so it is pinned.
        match events(&[0x15, 0x02, 0x28, 0x00, 0x00, 0x00]).as_slice() {
            [NetworkEvent::ChatMessage { text, .. }] => {
                assert!(!text.contains("Gospel"), "0x28 is a strip failure, not Gospel: {text:?}");
                assert!(text.contains("Chemical"), "0x28 should name FCP: {text:?}");
            }
            other => panic!("0x28 produced {other:?}"),
        }

        // Every mapped code must produce a real sentence.
        for code in [0x15, 0x16, 0x17, 0x18, 0x19, 0x1C, 0x1D, 0x1E, 0x1F, 0x20, 0x28] {
            let text = super::gospel_info_text(code).unwrap_or_else(|| panic!("{code:#04x} has no text"));
            assert!(text.ends_with('.'), "{code:#04x}: {text:?}");
        }
    }

    /// `GmKickResponsePacket` (0x00CD) is the only acknowledgement a successful
    /// `@kick` produces anywhere — `ACMD(kick)` prints nothing and
    /// `clif_GM_kick`'s whole feedback path is `clif->GM_kickack(sd, 1)`. As a
    /// no-op it left a DM watching the target vanish with no confirmation.
    #[test]
    fn a_kick_confirmation_reaches_the_kicker() {
        match events(&[0xCD, 0x00, 0x01]).as_slice() {
            [NetworkEvent::ChatMessage { text, color }] => {
                assert!(
                    matches!(color, MessageColor::Server),
                    "a successful kick is not an error: {color:?}"
                );
                assert!(
                    !text.trim().is_empty(),
                    "the confirmation is blank, which reaches the DM as nothing at all"
                );
            }
            [] => panic!("0x00CD published nothing — it is registered as a no-op again, and a kicking DM is told nothing"),
            other => panic!("0x00CD produced {other:?}"),
        }

        // The failure result must be distinguishable, or the right-click
        // "force to quit" path reports a refusal as a success.
        match events(&[0xCD, 0x00, 0x00]).as_slice() {
            [NetworkEvent::ChatMessage { color, .. }] => {
                assert!(
                    matches!(color, MessageColor::Error),
                    "a refused kick must not read as a success: {color:?}"
                )
            }
            other => panic!("a failed kick produced {other:?}"),
        }
    }

    /// `TalkieBoxMessagePacket` (0x0191) carries the whole point of a Talkie
    /// Box. As a no-op the trap's prop drew and its message was dropped.
    #[test]
    fn talkie_box_text_reaches_the_player() {
        const MESSAGE: &str = "meet at the bridge";

        let mut bytes: Vec<u8> = vec![0x91, 0x01];
        bytes.extend(0x0000_2A2Au32.to_le_bytes());
        bytes.extend(MESSAGE.as_bytes());
        bytes.resize(2 + 4 + 21, 0);

        match events(&bytes).as_slice() {
            [NetworkEvent::ChatMessage { text, .. }] => {
                assert!(text.contains(MESSAGE), "the message did not survive to the player: {text:?}");
                // The packet's `aid` is the skill unit's block id, so nothing
                // can name the speaker. Without the label the line arrives from
                // nobody.
                assert!(
                    text.contains("Talkie Box"),
                    "an unlabelled line has no attributable source: {text:?}"
                );
            }
            [] => panic!("0x0191 published nothing — it is registered as a no-op again, and the trap's message is dropped"),
            other => panic!("0x0191 produced {other:?}"),
        }
    }
}
