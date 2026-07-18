use std::cell::RefCell;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Instant;

use ragnarok_packets::handler::{DuplicateHandlerError, PacketCallback, PacketHandler};
use ragnarok_packets::*;

use crate::event::{NetworkEventList, NoNetworkEvents};
use crate::items::ItemQuantity;
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
    packet_handler.register_noop::<EntityStopMovePacket>()?;
    packet_handler.register_noop::<ChangeDirectionPacket>()?;
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
    packet_handler.register(|packet: SpriteChangePacket| match packet.sprite_type {
        SpriteChangeType::Base => Some(NetworkEvent::ChangeJob {
            account_id: packet.account_id,
            job_id: JobId(packet.value as u16),
        }),
        SpriteChangeType::Hair => Some(NetworkEvent::ChangeHair {
            account_id: packet.account_id,
            hair_id: packet.value,
        }),
        SpriteChangeType::Weapon => Some(NetworkEvent::ChangeWeapon {
            account_id: packet.account_id,
            weapon_id: packet.value,
        }),
        SpriteChangeType::Shield => Some(NetworkEvent::ChangeShield {
            account_id: packet.account_id,
            shield_id: packet.value,
        }),
        _ => None,
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
                    equipped_position,
                    slot,
                    hire_expiration_date,
                    flags,
                } = item_information;

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
                    details: InventoryItemDetails::Regular {
                        amount,
                        equipped_position,
                        flags,
                    },
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

        move |_packet: InventoyEndPacket| {
            let (inv_type, items) = inventory_items.borrow_mut().take().expect("Unexpected inventory end packet");
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
    packet_handler.register_noop::<DisplaySpecialEffectPacket>()?;
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
    })?;
    packet_handler.register(|packet: StatusChange2Packet| NetworkEvent::StatusChange {
        entity_id: packet.entity_id,
        index: packet.index,
        gained: packet.state == 1,
        duration_ms: packet.remaining_in_milliseconds,
        remaining_ms: packet.remaining_in_milliseconds,
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

        let details = match equip_position.is_empty() {
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

        vec![
            NetworkEvent::SkillCastCancelled { source_entity_id: None },
            NetworkEvent::ChatMessage {
                text: skill_failed_text(&packet),
                color: MessageColor::Error,
            },
        ]
        .into()
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
        // The end packet carries no timings; `gained: false` routes to `remove()`, which
        // ignores them.
        duration_ms: 0,
        remaining_ms: 0,
    })?;
    packet_handler.register_noop::<ReputationPacket>()?;
    packet_handler.register_noop::<ClanInfoPacket>()?;
    packet_handler.register_noop::<ClanOnlineCountPacket>()?;
    packet_handler.register_noop::<ChangeMapCellPacket>()?;
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
    packet_handler.register_noop::<EquipAmmunitionPacket>()?;
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
        let details = if packet.item_type == 4 || packet.item_type == 5 || packet.item_type == 7 || packet.item_type == 8 {
            // Armor / weapon / bothside / ammo-like equippables (Hercules type enum).
            InventoryItemDetails::Equippable {
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

/// Text for `ZC_ACK_TOUSESKILL` failures (Hercules `useskill_fail_cause`).
/// Hercules also reuses this packet for gameplay rejections, e.g. party
/// creation without Basic Skill 7 arrives as skill 1 / cause 0.
fn skill_failed_text(packet: &ToUseSkillSuccessPacket) -> String {
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
        cause => format!("Skill failed (reason {cause})."),
    }
}
