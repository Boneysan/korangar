use ragnarok_packets::*;

#[derive(Debug, Clone)]
pub struct EntityData {
    pub entity_id: EntityId,
    pub movement_speed: u16,
    pub job_id: JobId,
    pub head: u16,
    pub weapon: u32,
    pub shield: u32,
    pub position: WorldPosition,
    pub destination: Option<WorldPosition>,
    pub health_points: i32,
    pub maximum_health_points: i32,
    pub head_direction: usize,
    pub sex: Sex,
    /// `sc->opt1` / spawn `bodyState`.
    pub body_state: u16,
    /// `sc->opt2` / spawn `healthState`.
    pub health_state: u16,
    /// `sc->option` / spawn `effectState`.
    pub option: u32,
    /// Packet `isPKModeON`, stored by Ragexe at actor `+0x2C0`.
    pub is_pk_mode_on: bool,
    /// Idle-unit `dead_sit` value: 0 standing, 1 dead, 2 sitting.
    pub state: u8,
}

impl EntityData {
    pub fn from_character(account_id: AccountId, character_information: &CharacterInformation, position: WorldPosition) -> Self {
        Self {
            entity_id: EntityId(account_id.0),
            movement_speed: character_information.movement_speed as u16,
            job_id: character_information.job_id,
            head: character_information.head as u16,
            weapon: character_information.weapon as u32,
            shield: character_information.shield as u32,
            position,
            destination: None,
            health_points: character_information.health_points as i32,
            maximum_health_points: character_information.maximum_health_points as i32,
            head_direction: 0, // TODO: get correct rotation
            sex: character_information.sex,
            body_state: character_information.body_state as u16,
            health_state: character_information.health_state as u16,
            option: character_information.effect_state as u32,
            is_pk_mode_on: false,
            state: 0,
        }
    }
}

impl From<EntityAppearPacket> for EntityData {
    fn from(packet: EntityAppearPacket) -> Self {
        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job_id: packet.job_id,
            head: packet.head,
            weapon: packet.weapon,
            shield: packet.shield,
            position: packet.position,
            destination: None,
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
            body_state: packet.body_state,
            health_state: packet.health_state,
            option: packet.effect_state,
            is_pk_mode_on: packet.is_pk_mode_on != 0,
            state: 0,
        }
    }
}

impl From<EntityAppear2Packet> for EntityData {
    fn from(packet: EntityAppear2Packet) -> Self {
        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job_id: packet.job_id,
            head: packet.head,
            weapon: packet.weapon,
            shield: packet.shield,
            position: packet.position,
            destination: None,
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
            body_state: packet.body_state,
            health_state: packet.health_state,
            option: packet.effect_state,
            is_pk_mode_on: packet.is_pk_mode_on != 0,
            state: packet.state,
        }
    }
}

impl From<MovingEntityAppearPacket> for EntityData {
    fn from(packet: MovingEntityAppearPacket) -> Self {
        let (origin, destination) = packet.position.to_origin_destination();

        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job_id: packet.job_id,
            head: packet.head,
            weapon: packet.weapon,
            shield: packet.shield,
            position: origin,
            destination: Some(destination),
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
            body_state: packet.body_state,
            health_state: packet.health_state,
            option: packet.effect_state,
            is_pk_mode_on: packet.is_pk_mode_on != 0,
            state: 0,
        }
    }
}
