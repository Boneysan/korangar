#![cfg_attr(feature = "interface", feature(negative_impls))]
#![cfg_attr(feature = "interface", feature(impl_trait_in_assoc_type))]

pub mod handler;
mod position;

use std::net::Ipv4Addr;

use ragnarok_bytes::{
    ByteConvertable, ByteReader, ByteWriter, ConversionError, ConversionResult, ConversionResultExt, FixedByteSize, FromBytes, ToBytes,
};
#[cfg(feature = "derive")]
pub use ragnarok_macros::{CharacterServer, ClientPacket, LoginServer, MapServer, Packet, ServerPacket};
#[cfg(not(feature = "derive"))]
use ragnarok_macros::{CharacterServer, ClientPacket, LoginServer, MapServer, Packet, ServerPacket};

pub use self::position::{Direction, WorldPosition, WorldPosition2};

// To make proc macros work in korangar_interface.
extern crate self as ragnarok_packets;

/// The header of a Ragnarok Online packet. It is always two bytes long.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ByteConvertable, PartialOrd, Ord, Hash)]
pub struct PacketHeader(pub u16);

/// Base trait that all packets implement.
/// All packets in Ragnarok online consist of a header, two bytes in size,
/// followed by the packet data. If the packet does not have a fixed size,
/// the first two bytes will be the size of the packet in bytes *including* the
/// header. Packets are sent in little endian.
pub trait Packet: std::fmt::Debug + Send + Clone + 'static {
    /// Any scheduled packet that does not depend on in-game events should be
    /// marked as a ping. This is mostly for filtering when logging
    /// packet traffic.
    const IS_PING: bool;
    /// The header of the Packet.
    const HEADER: PacketHeader;

    /// Read packet **without the header**. To read the packet with the header,
    /// use [`PacketExt::packet_from_bytes`].
    fn payload_from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self>;

    /// Write packet **without the header**. To write the packet with the
    /// header, use [`PacketExt::packet_to_bytes`].
    fn payload_to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize>;

    // Implementation detail of Korangar. Can be used to convert a packet to an
    // UI element in the packet inspector.
    #[cfg(feature = "packet-to-state-element")]
    fn to_element<App: korangar_interface::application::Application>(
        self_path: impl rust_state::Path<App, Self>,
        name: String,
    ) -> Box<dyn korangar_interface::element::Element<App, LayoutInfo = ()>>;
}

/// Extension trait for reading and writing packets with the header.
pub trait PacketExt: Packet {
    /// Read packet **with the header**. To read the packet without the header,
    /// use [`Packet::payload_from_bytes`].
    fn packet_from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self>;

    /// Write packet **with the header**. To write the packet without the
    /// header, use [`Packet::payload_to_bytes`].
    fn packet_to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize>;
}

impl<T> PacketExt for T
where
    T: Packet,
{
    fn packet_from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let header = PacketHeader::from_bytes(byte_reader)?;

        if header != Self::HEADER {
            return Err(ConversionError::from_message("mismatched header"));
        }

        Self::payload_from_bytes(byte_reader)
    }

    fn packet_to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        let mut written = Self::HEADER.to_bytes(byte_writer)?;
        written += self.payload_to_bytes(byte_writer)?;
        Ok(written)
    }
}

/// Marker trait for packets sent by the client.
pub trait ClientPacket: Packet {}

/// Marker trait for packets sent by the server.
pub trait ServerPacket: Packet {}

/// Marker trait for login server packets.
pub trait LoginServerPacket: Packet {}

/// Marker trait for character server packets.
pub trait CharacterServerPacket: Packet {}

/// Marker trait for map server packets.
pub trait MapServerPacket: Packet {}

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ClientTick(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct AccountId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct CharacterId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct PartyId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct EntityId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SkillId(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SkillLevel(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct HotbarTab(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct HotbarSlot(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ShopId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Price(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct AttackRange(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct JobId(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ServerAddress(pub [u8; 4]);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct UserId(pub [u8; 24]);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct AuthToken(pub [u8; 17]);

impl From<ServerAddress> for Ipv4Addr {
    fn from(value: ServerAddress) -> Self {
        value.0.into()
    }
}

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct TilePosition {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct LargeTilePosition {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ColorBGRA {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ColorRGBA {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

/// Item index is always actual index + 2.
#[derive(Clone, Copy, Debug, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct InventoryIndex(pub u16);

impl FromBytes for InventoryIndex {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        u16::from_bytes(byte_reader).map(|raw| Self(raw.wrapping_sub(2)))
    }
}

impl ToBytes for InventoryIndex {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        u16::to_bytes(&self.0.wrapping_add(2), byte_writer)
    }
}

/// Storage index is always actual index + 1.
#[derive(Clone, Copy, Debug, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct StorageIndex(pub u16);

impl FromBytes for StorageIndex {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        u16::from_bytes(byte_reader).map(|raw| Self(raw.saturating_sub(1)))
    }
}

impl ToBytes for StorageIndex {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        u16::to_bytes(&(self.0 + 1), byte_writer)
    }
}

/// Raw index without any server-side offset.
#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct RawIndex(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ItemId(pub u32);

#[derive(Copy, Debug, Clone, ByteConvertable, FixedByteSize, PartialEq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum Sex {
    Female,
    Male,
    Both,
    Server,
}

/// Sent by the client to the login server.
/// The very first packet sent when logging in, it is sent after the user has
/// entered email and password.
#[derive(Debug, Clone, Packet, ClientPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0064)]
pub struct LoginServerLoginPacket {
    /// Unused
    #[new_default]
    pub version: [u8; 4],
    #[length(24)]
    pub name: String,
    #[length(24)]
    pub password: String,
    /// Unused
    #[new_default]
    pub client_type: u8,
}

/// Sent by the login server as a response to [LoginServerLoginPacket]
/// succeeding. After receiving this packet, the client will connect to one of
/// the character servers provided by this packet.
#[derive(Debug, Clone, Packet, ServerPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AC4)]
#[variable_length]
pub struct LoginServerLoginSuccessPacket {
    pub login_id1: u32,
    pub account_id: AccountId,
    pub login_id2: u32,
    /// Deprecated and always 0 on rAthena
    #[new_default]
    pub ip_address: u32,
    /// Deprecated and always 0 on rAthena
    #[new_default]
    pub name: [u8; 24],
    /// Always 0 on rAthena
    #[new_default]
    pub unknown: u16,
    pub sex: Sex,
    pub auth_token: AuthToken,
    #[repeating_remaining]
    pub character_server_information: Vec<CharacterServerInformation>,
}

/// Sent by the character server as a response to [CharacterServerLoginPacket]
/// succeeding. Provides basic information about the number of available
/// character slots.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x082D)]
pub struct CharacterServerLoginSuccessPacket {
    /// Always 29 on rAthena
    pub unknown: u16,
    pub normal_slot_count: u8,
    pub vip_slot_count: u8,
    pub billing_slot_count: u8,
    pub producible_slot_count: u8,
    pub valid_slot: u8,
    #[new_default]
    pub unused: [u8; 20],
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006B)]
#[variable_length]
pub struct CharacterListPacket {
    pub maximum_slot_count: u8,
    pub available_slot_count: u8,
    pub vip_slot_count: u8,
    #[new_default]
    pub unknown: [u8; 20],
    /// Per-character entries. Kept as raw bytes because the entry layout
    /// depends on the server's PACKETVER (155 bytes on our Hercules
    /// 20190605, 175 bytes on >= 20201007) and this packet is only ever
    /// consumed as a noop — the client requests the list explicitly via
    /// [RequestCharacterListPacket] instead. Parsing typed entries here
    /// would corrupt the stream on the "wrong" server generation.
    #[repeating_remaining]
    pub character_information: Vec<u8>,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09A0)]
pub struct CharacterSlotPagePacket {
    pub page_quantity: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x020D)]
#[variable_length]
pub struct CharacterBanListPacket {
    #[repeating_remaining]
    pub character_information: Vec<CharacterBanInformation>,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct CharacterBanInformation {
    pub character_id: CharacterId,
    #[length(20)]
    pub ban_time: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08B9)]
pub struct LoginPincodePacket {
    pub pincode_seed: u32,
    pub account_id: AccountId,
    pub state: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B18)]
pub struct Packet0b18 {
    /// Possibly inventory related
    #[new_default]
    pub unknown: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum ConnectionRefusedReason {
    #[numeric_value(1)]
    Unknown1,
    InvalidAccountId,
    InvalidChracterId,
    #[numeric_value(6)]
    InvalidSex,
    Unkown2,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006A)]
pub struct ConnectionRefusedPacket {
    pub reason: ConnectionRefusedReason,
    pub unknown: [u8; 19],
}

/// Sent by the map server as a response to [MapServerLoginPacket] succeeding.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02EB)]
pub struct MapServerLoginSuccessPacket {
    pub client_tick: ClientTick,
    pub position: WorldPosition,
    /// Always [5, 5] on rAthena
    #[new_default]
    pub ignored: [u8; 2],
    pub font: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum LoginFailedReason {
    #[numeric_value(1)]
    ServerClosed,
    #[numeric_value(2)]
    AlreadyLoggedIn,
    #[numeric_value(8)]
    AlreadyOnline,
}

#[derive(Debug, Clone, Packet, ServerPacket, LoginServer, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0081)]
pub struct LoginFailedPacket {
    pub reason: LoginFailedReason,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0840)]
#[variable_length]
pub struct MapServerUnavailablePacket {
    #[length_remaining]
    pub unknown: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum LoginFailedReason2 {
    UnregisteredId,
    IncorrectPassword,
    IdExpired,
    RejectedFromServer,
    BlockedByGMTeam,
    GameOutdated,
    LoginProhibitedUntil,
    ServerFull,
    CompanyAccountLimitReached,
}

/// Hercules `AC_REFUSE_LOGIN_R2`: 26 bytes total, a 4-byte error code
/// followed by a 20-byte block date string.
#[derive(Debug, Clone, Packet, ServerPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x083E)]
pub struct LoginFailedPacket2 {
    pub reason: LoginFailedReason2,
    pub block_date: [u8; 20],
}

/// Hercules `AC_REFUSE_LOGIN_R3`: identical 26-byte layout to
/// [LoginFailedPacket2], but sent under a different header by servers built
/// with PACKETVER >= 20180627 (see `lclif_send_auth_failed` in
/// Hercules `src/login/lclif.c`). Without this packet a rejected login
/// (wrong password / unknown account) is consumed silently and the client
/// appears to do nothing.
#[derive(Debug, Clone, Packet, ServerPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B02)]
pub struct LoginFailedPacket3 {
    pub reason: LoginFailedReason2,
    pub block_date: [u8; 20],
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum CharacterSelectionFailedReason {
    RejectedFromServer,
}

/// Sent by the character server as a response to [SelectCharacterPacket]
/// failing. Provides a reason for the character selection failing.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006C)]
pub struct CharacterSelectionFailedPacket {
    pub reason: CharacterSelectionFailedReason,
}

/// Sent by the character server as a response to [SelectCharacterPacket]
/// succeeding. Provides a map server to connect to, along with the id of our
/// selected character.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AC5)]
pub struct CharacterSelectionSuccessPacket {
    pub character_id: CharacterId,
    #[length(16)]
    pub map_name: String,
    pub map_server_ip: ServerAddress,
    pub map_server_port: u16,
    // NOTE: Could be `new_default` but Rust doesn't implement `[u8; 128]: Default`.
    #[new_value([0; 128])]
    pub unknown: [u8; 128],
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum CharacterCreationFailedReason {
    CharacterNameAlreadyUsed,
    NotOldEnough,
    #[numeric_value(3)]
    NotAllowedToUseSlot,
    #[numeric_value(255)]
    CharacterCerationFailed,
}

/// Sent by the character server as a response to [CreateCharacterPacket]
/// failing. Provides a reason for the character creation failing.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006E)]
pub struct CharacterCreationFailedPacket {
    pub reason: CharacterCreationFailedReason,
}

/// Sent by the client to the login server every 60 seconds to keep the
/// connection alive.
#[derive(Debug, Clone, Packet, ClientPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0200)]
#[ping]
pub struct LoginServerKeepalivePacket {
    #[new_value(UserId([0; 24]))]
    pub user_id: UserId,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct CharacterServerInformation {
    pub server_ip: ServerAddress,
    pub server_port: u16,
    #[length(20)]
    pub server_name: String,
    pub user_count: u16,
    pub server_type: u16, // ServerType
    pub display_new: u16, // bool16 ?
    #[new_value([0; 128])]
    pub unknown: [u8; 128],
}

/// Sent by the client to the character server after after successfully logging
/// into the login server.
/// Attempts to log into the character server using the provided information.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0065)]
pub struct CharacterServerLoginPacket {
    pub account_id: AccountId,
    pub login_id1: u32,
    pub login_id2: u32,
    #[new_default]
    pub unknown: u16,
    pub sex: Sex,
}

/// Sent by the client to the map server after after successfully selecting a
/// character. Attempts to log into the map server using the provided
/// information.
///
/// This is the 23-byte `CZ_ENTER` layout of PACKETVER >= 20211103 servers
/// (which includes our reference Hercules build at 20220406). Servers built
/// below that (e.g. a default Hercules build, 20190605) expect a 19-byte
/// variant WITHOUT `login_id2` — sending this packet to one desyncs the map
/// connection immediately (the server consumes 19 bytes and parses the 4
/// leftover bytes as a garbage header). Build the server with
/// `./configure --enable-packetver=20220406`; see
/// `docs/PLATFORM_BRINGUP.md`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0436)]
pub struct MapServerLoginPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub login_id1: u32,
    pub login_id2: u32,
    pub client_tick: ClientTick,
    pub sex: Sex,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0283)]
pub struct Packet8302 {
    pub entity_id: EntityId,
}

/// Sent by the client to the character server when the player tries to create
/// a new character.
/// Attempts to create a new character in an empty slot using the provided
/// information.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A39)]
pub struct CreateCharacterPacket {
    #[length(24)]
    pub name: String,
    pub slot: u8,
    pub hair_color: u16,     // TODO: HairColor
    pub hair_style: u16,     // TODO: HairStyle
    pub start_job_id: JobId, // TODO: Job
    #[new_default]
    pub unknown: [u8; 2],
    pub sex: Sex,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct CharacterInformation {
    pub character_id: CharacterId,
    pub experience: i64,
    pub money: i32,
    pub job_experience: i64,
    pub job_level: i32,
    pub body_state: i32,
    pub health_state: i32,
    pub effect_state: i32,
    pub virtue: i32,
    pub honor: i32,
    pub stat_points: i16,
    pub health_points: i64,
    pub maximum_health_points: i64,
    pub spell_points: i64,
    pub maximum_spell_points: i64,
    pub movement_speed: i16,
    pub job_id: JobId,
    pub head: i16,
    pub body: i16,
    pub weapon: i16,
    pub base_level: i16,
    pub sp_point: i16,
    pub accessory: i16,
    pub shield: i16,
    pub accessory2: i16,
    pub accessory3: i16,
    pub head_palette: i16,
    pub body_palette: i16,
    #[length(24)]
    pub name: String,
    pub strength: u8,
    pub agility: u8,
    pub vitality: u8,
    pub intelligence: u8,
    pub dexterity: u8,
    pub luck: u8,
    pub character_number: u8,
    pub hair_color: u8,
    pub b_is_changed_char: i16,
    #[length(16)]
    pub map_name: String,
    pub deletion_reverse_date: i32,
    pub robe_palette: i32,
    pub character_slot_change_count: i32,
    pub character_name_change_count: i32,
    pub sex: Sex,
}

#[cfg(feature = "interface")]
impl rust_state::VecItem for CharacterInformation {
    // TODO: Use CharacterId
    type Id = u32;

    fn get_id(&self) -> Self::Id {
        self.character_id.0
    }
}

/// The 155-byte per-character entry layout used by servers built with
/// PACKETVER < 20201007 (like our Hercules at 20190605): health points are
/// 32-bit and spell points 16-bit, where [CharacterInformation] models the
/// newer 64-bit variants. Field order is otherwise identical — verified
/// against `char_mmo_char_tobuf` in Hercules `src/char/char.c`.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct CharacterInformationLegacy {
    pub character_id: CharacterId,
    pub experience: i64,
    pub money: i32,
    pub job_experience: i64,
    pub job_level: i32,
    pub body_state: i32,
    pub health_state: i32,
    pub effect_state: i32,
    pub virtue: i32,
    pub honor: i32,
    pub stat_points: i16,
    pub health_points: i32,
    pub maximum_health_points: i32,
    pub spell_points: i16,
    pub maximum_spell_points: i16,
    pub movement_speed: i16,
    pub job_id: JobId,
    pub head: i16,
    pub body: i16,
    pub weapon: i16,
    pub base_level: i16,
    pub sp_point: i16,
    pub accessory: i16,
    pub shield: i16,
    pub accessory2: i16,
    pub accessory3: i16,
    pub head_palette: i16,
    pub body_palette: i16,
    #[length(24)]
    pub name: String,
    pub strength: u8,
    pub agility: u8,
    pub vitality: u8,
    pub intelligence: u8,
    pub dexterity: u8,
    pub luck: u8,
    pub character_number: u8,
    pub hair_color: u8,
    pub b_is_changed_char: i16,
    #[length(16)]
    pub map_name: String,
    pub deletion_reverse_date: i32,
    pub robe_palette: i32,
    pub character_slot_change_count: i32,
    pub character_name_change_count: i32,
    pub sex: Sex,
}

impl From<CharacterInformationLegacy> for CharacterInformation {
    fn from(legacy: CharacterInformationLegacy) -> Self {
        Self {
            character_id: legacy.character_id,
            experience: legacy.experience,
            money: legacy.money,
            job_experience: legacy.job_experience,
            job_level: legacy.job_level,
            body_state: legacy.body_state,
            health_state: legacy.health_state,
            effect_state: legacy.effect_state,
            virtue: legacy.virtue,
            honor: legacy.honor,
            stat_points: legacy.stat_points,
            health_points: i64::from(legacy.health_points),
            maximum_health_points: i64::from(legacy.maximum_health_points),
            spell_points: i64::from(legacy.spell_points),
            maximum_spell_points: i64::from(legacy.maximum_spell_points),
            movement_speed: legacy.movement_speed,
            job_id: legacy.job_id,
            head: legacy.head,
            body: legacy.body,
            weapon: legacy.weapon,
            base_level: legacy.base_level,
            sp_point: legacy.sp_point,
            accessory: legacy.accessory,
            shield: legacy.shield,
            accessory2: legacy.accessory2,
            accessory3: legacy.accessory3,
            head_palette: legacy.head_palette,
            body_palette: legacy.body_palette,
            name: legacy.name,
            strength: legacy.strength,
            agility: legacy.agility,
            vitality: legacy.vitality,
            intelligence: legacy.intelligence,
            dexterity: legacy.dexterity,
            luck: legacy.luck,
            character_number: legacy.character_number,
            hair_color: legacy.hair_color,
            b_is_changed_char: legacy.b_is_changed_char,
            map_name: legacy.map_name,
            deletion_reverse_date: legacy.deletion_reverse_date,
            robe_palette: legacy.robe_palette,
            character_slot_change_count: legacy.character_slot_change_count,
            character_name_change_count: legacy.character_name_change_count,
            sex: legacy.sex,
        }
    }
}

/// Sent by the character server as a response to [CreateCharacterPacket]
/// succeeding. Provides all character information of the newly created
/// character.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B6F)]
pub struct CreateCharacterSuccessPacket {
    pub character_information: CharacterInformation,
}

/// `HC_ACCEPT_MAKECHAR` as sent by servers built with PACKETVER < 20201007
/// (like our Hercules at 20190605): same meaning as
/// [CreateCharacterSuccessPacket], but a different header and the 155-byte
/// legacy entry layout.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006D)]
pub struct CreateCharacterLegacySuccessPacket {
    pub character_information: CharacterInformationLegacy,
}

/// Sent by the client to the character server.
/// Requests a list of every character associated with the account.
#[derive(Debug, Clone, Default, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09A1)]
pub struct RequestCharacterListPacket {}

/// Sent by the character server as a response to [RequestCharacterListPacket]
/// succeeding. Provides the requested list of character information.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B72)]
#[variable_length]
pub struct RequestCharacterListSuccessPacket {
    #[repeating_remaining]
    pub character_information: Vec<CharacterInformation>,
}

/// `HC_ACK_CHARINFO_PER_PAGE` as sent by servers built with
/// PACKETVER < 20201007 (like our Hercules at 20190605): same meaning as
/// [RequestCharacterListSuccessPacket], but a different header and the
/// 155-byte legacy entry layout.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x099D)]
#[variable_length]
pub struct RequestCharacterListLegacySuccessPacket {
    #[repeating_remaining]
    pub character_information: Vec<CharacterInformationLegacy>,
}

/// Sent by the map server to the client.
#[derive(Debug, Clone, Default, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B1D)]
#[ping]
pub struct MapServerPingPacket {}

/// Sent by the client to the map server when the player wants to move.
/// Attempts to path the player towards the provided position.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x035F)]
pub struct RequestPlayerMovePacket {
    pub position: WorldPosition,
}

/// Sent by the client to the map server when the player wants to warp.
/// Attempts to warp the player to a specific position on a specific map using
/// the provided information.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0140)]
pub struct RequestWarpToMapPacket {
    #[length(16)]
    pub map_name: String,
    pub position: TilePosition,
}

/// Sent by the map server to the client.
/// Informs the client that an entity is pathing towards a new position.
/// Provides the initial position and destination of the movement, as well as a
/// timestamp of when it started (for synchronization).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0086)]
pub struct EntityMovePacket {
    pub entity_id: EntityId,
    pub from_to: WorldPosition2,
    pub starting_timestamp: ClientTick,
}

// TODO: Handle this to improve the combat system.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0088)]
pub struct EntityStopMovePacket {
    pub entity_id: EntityId,
    pub position: TilePosition,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x009C)]
pub struct ChangeDirectionPacket {
    pub entity_id: EntityId,
    pub head_direction: u16,
    pub direction: u8,
}

/// Sent by the map server to the client.
/// Informs the client that the player is pathing towards a new position.
/// Provides the initial position and destination of the movement, as well as a
/// timestamp of when it started (for synchronization).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0087)]
pub struct PlayerMovePacket {
    pub starting_timestamp: ClientTick,
    pub from_to: WorldPosition2,
}

/// Sent by the client to the character server when the user tries to delete a
/// character.
/// Attempts to delete a character from the user account using the provided
/// information.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01FB)]
pub struct DeleteCharacterPacket {
    pub character_id: CharacterId,
    /// This field can be used for email or date of birth, depending on the
    /// configuration of the character server.
    #[length(40)]
    pub email: String,
    /// Ignored by rAthena
    #[new_default]
    pub unknown: [u8; 10],
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum CharacterDeletionFailedReason {
    NotAllowed,
    CharacterNotFound,
    NotEligible,
}

/// Sent by the character server as a response to [DeleteCharacterPacket]
/// failing. Provides a reason for the character deletion failing.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0070)]
pub struct CharacterDeletionFailedPacket {
    pub reason: CharacterDeletionFailedReason,
}

/// Sent by the character server as a response to [DeleteCharacterPacket]
/// succeeding.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006F)]
pub struct CharacterDeletionSuccessPacket {}

/// Sent by the client to the character server when the user selects a
/// character. Attempts to select the character in the specified slot.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0066)]
pub struct SelectCharacterPacket {
    pub selected_slot: u8,
}

/// Sent by the map server to the client when there is a new chat message from
/// the server. Provides the message to be displayed in the chat window.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x008E)]
#[variable_length]
pub struct ServerMessagePacket {
    #[length_remaining]
    pub message: String,
}

/// Self-only / guild-style display message (`ZC_GUILD_CHAT` 0x017F).
///
/// Hercules uses this for `dispbottom` / `clif_disp_onlyself` (atcommand
/// feedback, DM console replies, script notifications). Layout matches
/// `ServerMessagePacket`: `<header>.W <len>.W <message>.?B` with NUL.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x017F)]
#[variable_length]
pub struct DisplayBottomMessagePacket {
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0291)]
pub struct MessageTablePacket {
    pub message_id: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09CD)]
pub struct MessageTableColorPacket {
    pub message_id: u16,
    pub message_color: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AE2)]
pub struct OpenUiPacket {
    pub ui_type: u8,
    pub data: i32,
}

/// Sent by the client to the map server when the user hovers over an entity.
/// Attempts to fetch additional information about the entity, such as the
/// display name.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0368)]
pub struct RequestDetailsPacket {
    pub entity_id: EntityId,
}

/// Sent by the map server to the client as a response to
/// [RequestDetailsPacket]. Provides additional information about the player.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A30)]
pub struct RequestPlayerDetailsSuccessPacket {
    pub character_id: CharacterId,
    #[length(24)]
    pub name: String,
    #[length(24)]
    pub party_name: String,
    #[length(24)]
    pub guild_name: String,
    #[length(24)]
    pub position_name: String,
    pub title_id: u32,
}

/// Sent by the map server to the client as a response to
/// [RequestDetailsPacket]. Provides additional information about the entity.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ADF)]
pub struct RequestEntityDetailsSuccessPacket {
    pub entity_id: EntityId,
    pub group_id: u32,
    #[length(24)]
    pub name: String,
    #[length(24)]
    pub title: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09E7)]
pub struct NewMailStatusPacket {
    pub new_available: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct AchievementData {
    pub acheivement_id: u32,
    pub is_completed: u8,
    pub objectives: [u32; 10],
    pub completion_timestamp: u32,
    pub got_rewarded: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A24)]
pub struct AchievementUpdatePacket {
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_to_next_level: u32, // "to_next_level" might be wrong
    pub acheivement_data: AchievementData,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A23)]
#[variable_length]
pub struct AchievementListPacket {
    #[new_derive]
    pub achievement_count: u32,
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_to_next_level: u32, // "to_next_level" might be wrong
    #[repeating(achievement_count)]
    pub acheivement_data: Vec<AchievementData>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ADE)]
pub struct CriticalWeightUpdatePacket {
    pub weight: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01D7)]
pub struct SpriteChangePacket {
    pub account_id: AccountId,
    pub sprite_type: SpriteChangeType,
    pub value: u32,
    pub value2: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum SpriteChangeType {
    Base,
    Hair,
    Weapon,
    HeadBottom,
    HeadTop,
    HeadMiddle,
    HairCollor,
    ClothesColor,
    Shield,
    Shoes,
    Body,
    /// Equipped ammunition item id — a **Korangar fork addition**, not official.
    ///
    /// Hercules calls this slot `LOOK_FLOOR` and never sends it ("unknown
    /// purpose"), so the fork reuses it to broadcast what ammunition a player has
    /// loaded; official Ragnarok reports that for nobody but yourself. See
    /// `LOOK_AMMO` in the server's `map/map.h` for why this rides an existing
    /// look type instead of a new packet.
    Ammunition,
    Robe,
    Body2,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B08)]
#[variable_length]
pub struct InventoyStartPacket {
    pub inventory_type: u8,
    #[length_remaining]
    pub inventory_name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B0B)]
pub struct InventoyEndPacket {
    pub inventory_type: u8,
    pub flag: u8, // maybe char ?
}

#[derive(Debug, Clone, Copy, Default, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ItemOptions {
    pub index: u16,
    pub value: u16,
    pub parameter: u8,
}

bitflags::bitflags! {
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
    pub struct RegularItemFlags: u8 {
        const IDENTIFIED = 0b01;
        const IN_ETC_TAB = 0b10;
    }
}

impl FixedByteSize for RegularItemFlags {
    fn size_in_bytes() -> usize {
        <<Self as bitflags::Flags>::Bits as FixedByteSize>::size_in_bytes()
    }
}

impl FromBytes for RegularItemFlags {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        <Self as bitflags::Flags>::Bits::from_bytes(byte_reader).map(|raw| Self::from_bits(raw).expect("Invalid equip position"))
    }
}

impl ToBytes for RegularItemFlags {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        self.bits().to_bytes(byte_writer)
    }
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct RegularItemInformation {
    pub index: RawIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub amount: u16,
    /// Where this item *can* be worn — **not** where it is worn.
    ///
    /// The wire field is called `WearState`, but for stackable items Hercules
    /// fills it from `id->equip`, the item database's slot mask
    /// (`clif_item_normal`). It is identical for every stack of a given item and
    /// never reflects the character. Contrast [`EquippableItemInformation`],
    /// which has *two* fields: `equip_position` (`location` = `pc->equippoint`)
    /// and `equipped_position` (`WearState` = `it->equip`, the real worn state).
    ///
    /// Consequence: every arrow stack reports `AMMO` here, equipped or not, so
    /// treating this as worn state makes them all look equipped and picks the
    /// wrong one as the active ammunition. The stackable list simply cannot say
    /// what is worn; only `EquipAmmunitionPacket` (0x013C) can.
    pub equippable_position: EquipPosition,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: u32,
    pub flags: RegularItemFlags,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B09)]
#[variable_length]
pub struct RegularItemListPacket {
    pub inventory_type: u8,
    #[repeating_remaining]
    pub item_information: Vec<RegularItemInformation>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
    pub struct EquippableItemFlags: u8 {
        const IDENTIFIED = 0b001;
        const IS_BROKEN = 0b010;
        const IN_ETC_TAB = 0b110;
    }
}

impl FixedByteSize for EquippableItemFlags {
    fn size_in_bytes() -> usize {
        <<Self as bitflags::Flags>::Bits as FixedByteSize>::size_in_bytes()
    }
}

impl FromBytes for EquippableItemFlags {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        <Self as bitflags::Flags>::Bits::from_bytes(byte_reader).map(|raw| Self::from_bits(raw).expect("Invalid equip position"))
    }
}

impl ToBytes for EquippableItemFlags {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        self.bits().to_bytes(byte_writer)
    }
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct EquippableItemInformation {
    pub index: RawIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub equip_position: EquipPosition,
    pub equipped_position: EquipPosition,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: u32,
    pub bind_on_equip_type: u16,
    pub w_item_sprite_number: u16,
    pub option_count: u8,
    pub option_data: [ItemOptions; 5], // fix count
    pub refinement_level: u8,
    pub enchantment_level: u8,
    pub flags: EquippableItemFlags,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B39)]
#[variable_length]
pub struct EquippableItemListPacket {
    pub inventory_type: u8,
    #[repeating_remaining]
    pub item_information: Vec<EquippableItemInformation>,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct EquippableSwitchItemInformation {
    pub index: InventoryIndex,
    pub position: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A9B)]
#[variable_length]
pub struct EquippableSwitchItemListPacket {
    #[repeating_remaining]
    pub item_information: Vec<EquippableSwitchItemInformation>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x099B)]
pub struct MapTypePacket {
    pub map_type: u16,
    pub flags: u32,
}

/// Sent by the map server to the client when there is a new chat message from
/// ??. Provides the message to be displayed in the chat window, as well as
/// information on how the message should be displayed.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01C3)]
#[variable_length]
pub struct Broadcast2MessagePacket {
    pub font_color: ColorRGBA,
    pub font_type: u16,
    pub font_size: u16,
    pub font_alignment: u16,
    pub font_y: u16,
    #[length_remaining]
    pub message: String,
}

/// Sent by the map server to the client when when someone uses the @broadcast
/// command. Provides the message to be displayed in the chat window.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x009A)]
#[variable_length]
pub struct BroadcastMessagePacket {
    #[length_remaining]
    pub message: String,
}

/// Sent by the map server to the client when when someone writes in proximity
/// chat. Provides the source player and message to be displayed in the chat
/// window and the speach bubble.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x008D)]
#[variable_length]
pub struct OverheadMessagePacket {
    pub entity_id: EntityId,
    #[length_remaining]
    pub message: String,
}

/// Sent by the map server to the client when there is a new chat message from
/// an entity. Provides the message to be displayed in the chat window, the
/// color of the message, and the id of the entity it originated from.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C1)]
#[variable_length]
pub struct EntityMessagePacket {
    pub entity_id: EntityId,
    pub color: ColorBGRA,
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C0)]
pub struct DisplayEmotionPacket {
    pub entity_id: EntityId,
    pub emotion: u8,
}

/// Client request to play an emote (`CZ_REQ_EMOTION` 0x00BF).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00BF)]
pub struct RequestEmotionPacket {
    pub emotion: u8,
}

/// Every value that can be set from the server through [UpdateStatPacket],
/// [UpdateStatPacket1], [UpdateStatPacket2], and [UpdateStatPacket3].
/// All UpdateStatPackets do the same, they just have different sizes
/// correlating to the space the updated value requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatType {
    Weight(u32),
    MaximumWeight(u32),
    MovementSpeed(u32),
    BaseLevel(u32),
    JobLevel(u32),
    Karma(u32),
    Manner(u32),
    StatPoints(u32),
    SkillPoints(u32),
    Hit(u32),
    Flee1(u32),
    Flee2(u32),
    MaximumHealthPoints(u32),
    MaximumSpellPoints(u32),
    HealthPoints(u32),
    SpellPoints(u32),
    AttackSpeed(u32),
    Attack1(u32),
    Defense1(u32),
    MagicDefense1(u32),
    Attack2(u32),
    Defense2(u32),
    MagicDefense2(u32),
    Critical(u32),
    MagicAttack1(u32),
    MagicAttack2(u32),
    Zeny(u32),
    BaseExperience(u64),
    JobExperience(u64),
    NextBaseExperience(u64),
    NextJobExperience(u64),
    StrengthStatPointCost(u8),
    AgilityStatPointCost(u8),
    VitalityStatPointCost(u8),
    IntelligenceStatPointCost(u8),
    DexterityStatPointCost(u8),
    LuckStatPointCost(u8),
    Strength(i32, i32),
    Agility(i32, i32),
    Vitality(i32, i32),
    Intelligence(i32, i32),
    Dexterity(i32, i32),
    Luck(i32, i32),
    CartInfo(u16, u32, u32),
    ActivityPoints(u32),
    TraitPoint(u32),
    MaximumActivityPoints(u32),
    Power(u32, u32),
    Stamina(u32, u32),
    Wisdom(u32, u32),
    Spell(u32, u32),
    Concentration(u32, u32),
    Creativity(u32, u32),
    PowerStatPointCost(u8),
    StaminaStatPointCost(u8),
    WisdomStatPointCost(u8),
    SpellStatPointCost(u8),
    ConcentrationStatPointCost(u8),
    CreativitySpellPointCost(u8),
    PhysicalAttack(u32),
    SpellMagicAttack(u32),
    Resistance(u32),
    MagicResistance(u32),
    HealingPlus(u32),
    CriticalDamageRate(u32),
}

impl FromBytes for StatType {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        /// For some reason, the stats are packed into the upper two bytes of an
        /// `i32`. I am unsure why that is but for now we will just work
        /// with what we got.
        fn weirdly_packed_stat(byte_reader: &mut ByteReader) -> ConversionResult<(i32, i32)> {
            let _ = i16::from_bytes(byte_reader)?;
            let base = i16::from_bytes(byte_reader)?;
            let _ = i16::from_bytes(byte_reader)?;
            let bonus = i16::from_bytes(byte_reader)?;

            Ok((base as i32, bonus as i32))
        }

        let stat = match u16::from_bytes(byte_reader).trace::<Self>()? {
            0 => u32::from_bytes(byte_reader).map(Self::MovementSpeed),
            1 => u64::from_bytes(byte_reader).map(Self::BaseExperience),
            2 => u64::from_bytes(byte_reader).map(Self::JobExperience),
            3 => u32::from_bytes(byte_reader).map(Self::Karma),
            4 => u32::from_bytes(byte_reader).map(Self::Manner),
            5 => u32::from_bytes(byte_reader).map(Self::HealthPoints),
            6 => u32::from_bytes(byte_reader).map(Self::MaximumHealthPoints),
            7 => u32::from_bytes(byte_reader).map(Self::SpellPoints),
            8 => u32::from_bytes(byte_reader).map(Self::MaximumSpellPoints),
            9 => u32::from_bytes(byte_reader).map(Self::StatPoints),
            11 => u32::from_bytes(byte_reader).map(Self::BaseLevel),
            12 => u32::from_bytes(byte_reader).map(Self::SkillPoints),
            13 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Strength(base, bonus)),
            14 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Agility(base, bonus)),
            15 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Vitality(base, bonus)),
            16 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Intelligence(base, bonus)),
            17 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Dexterity(base, bonus)),
            18 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Luck(base, bonus)),
            20 => u32::from_bytes(byte_reader).map(Self::Zeny),
            22 => u64::from_bytes(byte_reader).map(Self::NextBaseExperience),
            23 => u64::from_bytes(byte_reader).map(Self::NextJobExperience),
            24 => u32::from_bytes(byte_reader).map(Self::Weight),
            25 => u32::from_bytes(byte_reader).map(Self::MaximumWeight),
            32 => u8::from_bytes(byte_reader).map(Self::StrengthStatPointCost),
            33 => u8::from_bytes(byte_reader).map(Self::AgilityStatPointCost),
            34 => u8::from_bytes(byte_reader).map(Self::VitalityStatPointCost),
            35 => u8::from_bytes(byte_reader).map(Self::IntelligenceStatPointCost),
            36 => u8::from_bytes(byte_reader).map(Self::DexterityStatPointCost),
            37 => u8::from_bytes(byte_reader).map(Self::LuckStatPointCost),
            41 => u32::from_bytes(byte_reader).map(Self::Attack1),
            42 => u32::from_bytes(byte_reader).map(Self::Attack2),
            43 => u32::from_bytes(byte_reader).map(Self::MagicAttack1),
            44 => u32::from_bytes(byte_reader).map(Self::MagicAttack2),
            45 => u32::from_bytes(byte_reader).map(Self::Defense1),
            46 => u32::from_bytes(byte_reader).map(Self::Defense2),
            47 => u32::from_bytes(byte_reader).map(Self::MagicDefense1),
            48 => u32::from_bytes(byte_reader).map(Self::MagicDefense2),
            49 => u32::from_bytes(byte_reader).map(Self::Hit),
            50 => u32::from_bytes(byte_reader).map(Self::Flee1),
            51 => u32::from_bytes(byte_reader).map(Self::Flee2),
            52 => u32::from_bytes(byte_reader).map(Self::Critical),
            53 => u32::from_bytes(byte_reader).map(Self::AttackSpeed),
            55 => u32::from_bytes(byte_reader).map(Self::JobLevel),
            99 => u16::from_bytes(byte_reader)
                .and_then(|a| Ok(Self::CartInfo(a, u32::from_bytes(byte_reader)?, u32::from_bytes(byte_reader)?))),
            219 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Power(a, u32::from_bytes(byte_reader)?))),
            220 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Stamina(a, u32::from_bytes(byte_reader)?))),
            221 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Wisdom(a, u32::from_bytes(byte_reader)?))),
            222 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Spell(a, u32::from_bytes(byte_reader)?))),
            223 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Concentration(a, u32::from_bytes(byte_reader)?))),
            224 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Creativity(a, u32::from_bytes(byte_reader)?))),
            225 => u32::from_bytes(byte_reader).map(Self::PhysicalAttack),
            226 => u32::from_bytes(byte_reader).map(Self::SpellMagicAttack),
            227 => u32::from_bytes(byte_reader).map(Self::Resistance),
            228 => u32::from_bytes(byte_reader).map(Self::MagicResistance),
            229 => u32::from_bytes(byte_reader).map(Self::HealingPlus),
            230 => u32::from_bytes(byte_reader).map(Self::CriticalDamageRate),
            231 => u32::from_bytes(byte_reader).map(Self::TraitPoint),
            232 => u32::from_bytes(byte_reader).map(Self::ActivityPoints),
            233 => u32::from_bytes(byte_reader).map(Self::MaximumActivityPoints),
            247 => u8::from_bytes(byte_reader).map(Self::PowerStatPointCost),
            248 => u8::from_bytes(byte_reader).map(Self::StaminaStatPointCost),
            249 => u8::from_bytes(byte_reader).map(Self::WisdomStatPointCost),
            250 => u8::from_bytes(byte_reader).map(Self::SpellStatPointCost),
            251 => u8::from_bytes(byte_reader).map(Self::ConcentrationStatPointCost),
            252 => u8::from_bytes(byte_reader).map(Self::CreativitySpellPointCost),
            invalid => Err(ConversionError::from_message(format!("invalid stat id {invalid}"))),
        };

        stat.trace::<Self>()
    }
}

impl ToBytes for StatType {
    fn to_bytes(&self, _byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        panic!("this should be derived");
    }
}

impl std::fmt::Display for StatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Weight(value) => write!(f, "Weight: {}", value),
            Self::MaximumWeight(value) => write!(f, "Maximum Weight: {}", value),
            Self::MovementSpeed(value) => write!(f, "Movement Speed: {}", value),
            Self::BaseLevel(value) => write!(f, "Base Level: {}", value),
            Self::JobLevel(value) => write!(f, "Job Level: {}", value),
            Self::Karma(value) => write!(f, "Karma: {}", value),
            Self::Manner(value) => write!(f, "Manner: {}", value),
            Self::StatPoints(value) => write!(f, "Stat Points: {}", value),
            Self::SkillPoints(value) => write!(f, "Skill Point: {}", value),
            Self::Hit(value) => write!(f, "Hit: {}", value),
            Self::Flee1(value) => write!(f, "Flee1: {}", value),
            Self::Flee2(value) => write!(f, "Flee2: {}", value),
            Self::MaximumHealthPoints(value) => write!(f, "Maximum Health Points: {}", value),
            Self::MaximumSpellPoints(value) => write!(f, "Maximum Spell Points: {}", value),
            Self::HealthPoints(value) => write!(f, "Health Points: {}", value),
            Self::SpellPoints(value) => write!(f, "Spell Points: {}", value),
            Self::AttackSpeed(value) => write!(f, "Attack Speed: {}", value),
            Self::Attack1(value) => write!(f, "Attack1: {}", value),
            Self::Defense1(value) => write!(f, "Defense1: {}", value),
            Self::MagicDefense1(value) => write!(f, "Magic Defense1: {}", value),
            Self::Attack2(value) => write!(f, "Attack2: {}", value),
            Self::Defense2(value) => write!(f, "Defense2: {}", value),
            Self::MagicDefense2(value) => write!(f, "Magic Defense2: {}", value),
            Self::Critical(value) => write!(f, "Critical: {}", value),
            Self::MagicAttack1(value) => write!(f, "Magic Attack1: {}", value),
            Self::MagicAttack2(value) => write!(f, "Magic Attack2: {}", value),
            Self::Zeny(value) => write!(f, "Zeny: {}", value),
            Self::BaseExperience(value) => write!(f, "Base Experience: {}", value),
            Self::JobExperience(value) => write!(f, "Job Experience: {}", value),
            Self::NextBaseExperience(value) => write!(f, "Next Base Experience: {}", value),
            Self::NextJobExperience(value) => write!(f, "Next Job Experience: {}", value),
            Self::StrengthStatPointCost(value) => write!(f, "Strength Stat Point Cost: {}", value),
            Self::AgilityStatPointCost(value) => write!(f, "Agility Stat Point Cost: {}", value),
            Self::VitalityStatPointCost(value) => write!(f, "Vitality Stat Point Cost: {}", value),
            Self::IntelligenceStatPointCost(value) => write!(f, "Intelligence Stat Point Cost: {}", value),
            Self::DexterityStatPointCost(value) => write!(f, "Dexterity Stat Point Cost: {}", value),
            Self::LuckStatPointCost(value) => write!(f, "Luck Stat Point Cost: {}", value),
            Self::Strength(base, bonus) => write!(f, "Strength: {} (+{})", base, bonus),
            Self::Agility(base, bonus) => write!(f, "Agility: {} (+{})", base, bonus),
            Self::Vitality(base, bonus) => write!(f, "Vitality: {} (+{})", base, bonus),
            Self::Intelligence(base, bonus) => write!(f, "Intelligence: {} (+{})", base, bonus),
            Self::Dexterity(base, bonus) => write!(f, "Dexterity: {} (+{})", base, bonus),
            Self::Luck(base, bonus) => write!(f, "Luck: {} (+{})", base, bonus),
            Self::CartInfo(items, weight, max_weight) => write!(f, "Cart Info: {} items, {}/{} weight", items, weight, max_weight),
            Self::ActivityPoints(value) => write!(f, "Activity Points: {}", value),
            Self::TraitPoint(value) => write!(f, "Trait Point: {}", value),
            Self::MaximumActivityPoints(value) => write!(f, "Maximum Activity Points: {}", value),
            Self::Power(base, bonus) => write!(f, "Power: {} (+{})", base, bonus),
            Self::Stamina(base, bonus) => write!(f, "Stamina: {} (+{})", base, bonus),
            Self::Wisdom(base, bonus) => write!(f, "Wisdom: {} (+{})", base, bonus),
            Self::Spell(base, bonus) => write!(f, "Spell: {} (+{})", base, bonus),
            Self::Concentration(base, bonus) => write!(f, "Concentration: {} (+{})", base, bonus),
            Self::Creativity(base, bonus) => write!(f, "Creativity: {} (+{})", base, bonus),
            Self::PowerStatPointCost(value) => write!(f, "Power Stat Point Cost: {}", value),
            Self::StaminaStatPointCost(value) => write!(f, "Stamina Stat Point Cost: {}", value),
            Self::WisdomStatPointCost(value) => write!(f, "Wisdom Stat Point Cost: {}", value),
            Self::SpellStatPointCost(value) => write!(f, "Spell Stat Point Cost: {}", value),
            Self::ConcentrationStatPointCost(value) => write!(f, "Concentration Stat Point Cost: {}", value),
            Self::CreativitySpellPointCost(value) => write!(f, "Creativity Stat Point Cost: {}", value),
            Self::PhysicalAttack(value) => write!(f, "Physical Attack: {}", value),
            Self::SpellMagicAttack(value) => write!(f, "Spell Magic Attack: {}", value),
            Self::Resistance(value) => write!(f, "Resistance: {}", value),
            Self::MagicResistance(value) => write!(f, "Magic Resistance: {}", value),
            Self::HealingPlus(value) => write!(f, "Healing Plus: {}", value),
            Self::CriticalDamageRate(value) => write!(f, "Critical Damage Rate: {}", value),
        }
    }
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B0)]
pub struct UpdateStatPacket {
    #[length(6)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0196)]
pub struct StatusChangeSequencePacket {
    pub index: u16,
    pub id: u32,
    pub state: u8,
}

/// Sent by the character server to the client when loading onto a new map.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00BD)]
pub struct InitialStatsPacket {
    pub stat_points: u16,
    pub strength: u8,
    pub strength_stat_points_cost: u8,
    pub agility: u8,
    pub agility_stat_points_cost: u8,
    pub vitatity: u8,
    pub vitality_stat_points_cost: u8,
    pub intelligence: u8,
    pub intelligence_stat_points_cost: u8,
    pub dexterity: u8,
    pub dexterity_stat_points_cost: u8,
    pub luck: u8,
    pub luck_stat_points_cost: u8,
    pub left_attack: u16,
    pub rigth_attack: u16,
    pub rigth_magic_attack: u16,
    pub left_magic_attack: u16,
    pub left_defense: u16,
    pub rigth_defense: u16,
    pub rigth_magic_defense: u16,
    pub left_magic_defense: u16,
    pub hit: u16, // ?
    pub flee: u16,
    pub flee2: u16,
    pub crit: u16,
    pub attack_speed: u16,
    /// Always 0 on rAthena
    #[new_default]
    pub bonus_attack_speed: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0141)]
pub struct UpdateStatPacket1 {
    #[length(12)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ACB)]
pub struct UpdateStatPacket2 {
    #[length(10)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00BE)]
pub struct UpdateStatPacket3 {
    #[length(3)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x013A)]
pub struct UpdateAttackRangePacket {
    pub attack_range: AttackRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum StatUpType {
    Strength { amount: u8 },
    Agility { amount: u8 },
    Vitality { amount: u8 },
    Intelligence { amount: u8 },
    Dexterity { amount: u8 },
    Luck { amount: u8 },
}

impl FromBytes for StatUpType {
    fn from_bytes(_: &mut ByteReader) -> ConversionResult<Self> {
        todo!()
    }
}

impl ToBytes for StatUpType {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        match self {
            Self::Strength { amount } => {
                13u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
            Self::Agility { amount } => {
                14u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
            Self::Vitality { amount } => {
                15u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
            Self::Intelligence { amount } => {
                16u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
            Self::Dexterity { amount } => {
                17u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
            Self::Luck { amount } => {
                18u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
        }
    }
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00BB)]
pub struct RequestStatUpPacket {
    pub stat_type: StatUpType,
}

/// rAthena seems to always return [`Success`](RequestStatUpResult::Success),
/// even if the request fails.
#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum RequestStatUpResult {
    Failure,
    Success,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00BC)]
pub struct RequestStatUpResponsePacket {
    pub staus_type: u16,
    pub success: RequestStatUpResult,
    pub value: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08D4)]
pub struct SwitchCharacterSlotPacket {
    // TODO: Type this more strongly.
    pub origin_slot: u16,
    // TODO: Type this more strongly.
    pub destination_slot: u16,
    /// 1 instead of default, just in case the sever actually uses this value
    /// (rAthena does not)
    #[new_value(1)]
    pub remaining_moves: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum Action {
    Attack,
    PickUpItem,
    SitDown,
    StandUp,
    #[numeric_value(7)]
    ContinousAttack,
    /// Unsure what this does
    #[numeric_value(12)]
    TouchSkill,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0437)]
pub struct RequestActionPacket {
    pub npc_id: EntityId,
    pub action: Action,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0362)]
pub struct ItemPickupRequestPacket {
    pub entity_id: EntityId,
}

/// Drop an inventory item onto the ground (`CZ_ITEM_THROW2` 0x0363 at PACKETVER
/// 20220406). Wire layout: `<index>.W <amount>.W` (InventoryIndex serializes as
/// index + 2).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0363)]
pub struct DropItemPacket {
    pub inventory_index: InventoryIndex,
    pub amount: u16,
}

/// Server ack that an inventory item was dropped (`ZC_ITEM_THROW_ACK` 0x00AF).
/// Modern Hercules also sends `ZC_DELETE_ITEM_FROM_BODY` (0x07FA) on success;
/// this packet still arrives and must be consumed so the stream stays aligned.
/// `amount == 0` means the drop was rejected / ignored.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00AF)]
pub struct DropItemAckPacket {
    pub inventory_index: InventoryIndex,
    pub amount: u16,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F3)]
#[variable_length]
pub struct GlobalMessagePacket {
    #[length_remaining_off_by_one]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0139)]
pub struct RequestPlayerAttackFailedPacket {
    pub target_entity_id: EntityId,
    pub target_position: TilePosition,
    pub player_position: TilePosition,
    pub attack_range: AttackRange,
}

/// Instant same-map relocation (`ZC_HIGHJUMP`). Hercules also uses this
/// packet for combat slide/knockback effects such as Bowling Bash.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01FF)]
pub struct EntitySlidePacket {
    pub entity_id: EntityId,
    pub position: TilePosition,
}

/// Monster information shown by Wizard Sense/Estimation (`ZC_MONSTER_INFO`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x018C)]
pub struct MonsterInformationPacket {
    pub job_id: JobId,
    pub level: u16,
    pub size: u16,
    pub health_points: u32,
    pub defense: u16,
    pub race: u16,
    pub magic_defense: u16,
    pub element: u16,
    pub elemental_effectiveness: [u8; 9],
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct WarpDestination {
    pub map_name: [u8; 16],
}

/// Available destinations for Teleport/Warp Portal (`ZC_WARPLIST`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ABE)]
#[variable_length]
pub struct WarpListPacket {
    pub skill_id: SkillId,
    #[repeating_remaining]
    pub destinations: Vec<WarpDestination>,
}

/// Select a destination offered by [`WarpListPacket`] (`CZ_SELECT_WARPPOINT`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x011B)]
pub struct SelectWarpDestinationPacket {
    pub skill_id: SkillId,
    #[length(16)]
    pub map_name: String,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct RefinableWeaponInformation {
    pub inventory_index: InventoryIndex,
    pub item_id: ItemId,
    pub refinement_level: u8,
    pub cards: [ItemId; 4],
}

/// Inventory weapons eligible for Whitesmith Weapon Refine.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0221)]
#[variable_length]
pub struct RefinableWeaponListPacket {
    #[repeating_remaining]
    pub weapons: Vec<RefinableWeaponInformation>,
}

/// Select an inventory weapon offered by [`RefinableWeaponListPacket`].
/// The index is the server wire index (the client's normalized index + 2).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0222)]
pub struct RequestWeaponRefinePacket {
    pub inventory_index: u32,
}

/// Result of a Whitesmith Weapon Refine selection (`ZC_ACK_WEAPONREFINE`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0223)]
pub struct WeaponRefineResultPacket {
    pub result: i32,
    pub item_id: ItemId,
}

/// Broken equipment offered by Blacksmith Repair Weapon. Unlike most inventory
/// packets, Hercules sends and expects the raw zero-based inventory index here.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct RepairableItemInformation {
    pub inventory_index: RawIndex,
    pub item_id: ItemId,
    pub cards: [ItemId; 4],
    pub refinement_level: u8,
    pub grade: u8,
}

/// Broken-equipment selection list (`ZC_REPAIRITEMLIST`, modern 0x0B65 layout).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B65)]
#[variable_length]
pub struct RepairableItemListPacket {
    #[repeating_remaining]
    pub items: Vec<RepairableItemInformation>,
}

/// Select or cancel Repair Weapon (`CZ_REQ_ITEMREPAIR2`). A raw index of
/// `u16::MAX` is the official cancellation sentinel used by Hercules.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B66)]
pub struct RequestItemRepairPacket {
    pub item: RepairableItemInformation,
}

/// Repair Weapon result (`ZC_ACK_ITEMREPAIR`). The wire index uses the normal
/// inventory +2 offset, so `InventoryIndex` normalizes it for consumers.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01FE)]
pub struct ItemRepairResultPacket {
    pub inventory_index: InventoryIndex,
    pub result: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0977)]
pub struct UpdateEntityHealthPointsPacket {
    pub entity_id: EntityId,
    pub health_points: u32,
    pub maximum_health_points: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum DamageType {
    Damage,
    PickUpItem,
    SitDown,
    StandUp,
    DamageEndure, // Not confirmed
    Splash,       // Not confirmed
    Skill,        // Not confirmed
    RepeatDamage, // Not confirmed
    MultiHitDamage,
    MultiHitDamageEndure, // Not confirmed
    CriticalHit,
    LuckyDodge,
    TouchSkill, // Not confirmed
    CriticalMultiHit,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x008A)]
pub struct DamagePacket1 {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub client_tick: ClientTick,
    pub attack_duration: u32,
    pub damage_delay: u32,
    pub damage_amount: i16,
    pub number_of_hits: u16,
    pub damage_type: DamageType,
    /// Assassin dual wield damage.
    pub damage_amount_2: i16,
}

// FIX: This one is the attack animation one
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08C8)]
pub struct DamagePacket3 {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub client_tick: ClientTick,
    pub attack_duration: u32,
    pub damage_delay: u32,
    pub damage_amount: u32,
    pub is_special_damage: u8,
    pub number_of_hits: u16,
    pub damage_type: DamageType,
    /// Assassin dual wield damage.
    pub damage_amount_2: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x007F)]
#[ping]
pub struct ServerTickPacket {
    pub client_tick: ClientTick,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0360)]
#[ping]
pub struct RequestServerTickPacket {
    pub client_tick: ClientTick,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum SwitchCharacterSlotResponseStatus {
    Success,
    Error,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B70)]
pub struct SwitchCharacterSlotResponsePacket {
    #[new_default]
    pub unknown: u16, // is always 8 ?
    pub status: SwitchCharacterSlotResponseStatus,
    pub remaining_moves: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0091)]
pub struct ChangeMapPacket {
    #[length(16)]
    pub map_name: String,
    pub position: TilePosition,
}

#[derive(Debug, Clone, ByteConvertable, PartialEq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum DisappearanceReason {
    OutOfSight,
    Died,
    LoggedOut,
    Teleported,
    TrickDead,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0080)]
pub struct EntityDisAppearPacket {
    pub entity_id: EntityId,
    pub reason: DisappearanceReason,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09FD)]
#[variable_length]
pub struct MovingEntityAppearPacket {
    pub object_type: u8,
    pub entity_id: EntityId,
    pub group_id: u32, // may be reversed - or completely wrong
    pub movement_speed: u16,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub job_id: JobId,
    pub head: u16,
    pub weapon: u32,
    pub shield: u32,
    pub accessory: u16,
    pub move_start_time: u32,
    pub accessory2: u16,
    pub accessory3: u16,
    pub head_palette: u16,
    pub body_palette: u16,
    pub head_direction: u16,
    pub robe: u16,
    pub guild_id: u32, // may be reversed - or completely wrong
    pub emblem_version: u16,
    pub honor: u16,
    pub virtue: u32,
    pub is_pk_mode_on: u8,
    pub sex: Sex,
    pub position: WorldPosition2,
    pub x_size: u8,
    pub y_size: u8,
    pub c_level: u16,
    pub font: u16,
    pub maximum_health_points: i32,
    pub health_points: i32,
    pub is_boss: u8,
    pub body: u16,
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0148)]
pub struct ResurrectionPacket {
    pub entity_id: EntityId,
    /// Always 0 in rAthena.
    #[new_default]
    pub packet_type: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09FE)]
#[variable_length]
pub struct EntityAppearPacket {
    pub object_type: u8,
    pub entity_id: EntityId,
    pub group_id: u32, // may be reversed - or completely wrong
    pub movement_speed: u16,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub job_id: JobId,
    pub head: u16,
    pub weapon: u32,
    pub shield: u32,
    pub accessory: u16,
    pub accessory2: u16,
    pub accessory3: u16,
    pub head_palette: u16,
    pub body_palette: u16,
    pub head_direction: u16,
    pub robe: u16,
    pub guild_id: u32, // may be reversed - or completely wrong
    pub emblem_version: u16,
    pub honor: u16,
    pub virtue: u32,
    pub is_pk_mode_on: u8,
    pub sex: Sex,
    pub position: WorldPosition,
    pub x_size: u8,
    pub y_size: u8,
    pub c_level: u16,
    pub font: u16,
    pub maximum_health_points: i32,
    pub health_points: i32,
    pub is_boss: u8,
    pub body: u16,
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09FF)]
#[variable_length]
pub struct EntityAppear2Packet {
    pub object_type: u8,
    pub entity_id: EntityId,
    pub group_id: u32, // may be reversed - or completely wrong
    pub movement_speed: u16,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub job_id: JobId,
    pub head: u16,
    pub weapon: u32,
    pub shield: u32,
    pub accessory: u16,
    pub accessory2: u16,
    pub accessory3: u16,
    pub head_palette: u16,
    pub body_palette: u16,
    pub head_direction: u16,
    pub robe: u16,
    pub guild_id: u32, // may be reversed - or completely wrong
    pub emblem_version: u16,
    pub honor: u16,
    pub virtue: u32,
    pub is_pk_mode_on: u8,
    pub sex: Sex,
    pub position: WorldPosition,
    pub x_size: u8,
    pub y_size: u8,
    pub state: u8,
    pub c_level: u16,
    pub font: u16,
    pub maximum_health_points: i32,
    pub health_points: i32,
    pub is_boss: u8,
    pub body: u16,
    #[length(24)]
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum SkillType {
    #[numeric_value(0)]
    Passive,
    #[numeric_value(1)]
    Attack,
    #[numeric_value(2)]
    Ground,
    #[numeric_value(4)]
    SelfCast,
    #[numeric_value(16)]
    Support,
    #[numeric_value(32)]
    Trap,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SkillInformation {
    pub skill_id: SkillId,
    pub skill_type: SkillType,
    pub skill_level: SkillLevel,
    pub spell_point_cost: u16,
    pub attack_range: AttackRange,
    #[length(24)]
    pub skill_name: String,
    // TODO: bool
    pub upgradable: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x010F)]
#[variable_length]
pub struct UpdateSkillTreePacket {
    #[repeating_remaining]
    pub skill_information: Vec<SkillInformation>,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum HotkeyType {
    Item,
    Skill,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct HotkeyData {
    pub hotkey_type: HotkeyType,
    pub item_or_skill_id: u32,
    pub quantity_or_skill_level: u16,
}

impl HotkeyData {
    pub const UNBOUND: Self = Self {
        hotkey_type: HotkeyType::Item,
        item_or_skill_id: 0,
        quantity_or_skill_level: 0,
    };
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B20)]
pub struct UpdateHotkeysPacket {
    pub rotate: u8,
    pub tab: HotbarTab,
    pub hotkeys: [HotkeyData; 38],
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02BA)]
pub struct SetHotkeyData1Packet {
    pub slot: HotbarSlot,
    pub hotkey_data: HotkeyData,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B21)]
pub struct SetHotkeyData2Packet {
    pub tab: HotbarTab,
    pub slot: HotbarSlot,
    pub hotkey_data: HotkeyData,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0147)]
pub struct AutoRunSkillPacket {
    pub skill_id: SkillId,
    pub skill_type: u32,
    pub skill_level: SkillLevel,
    pub skill_sp: u16,
    pub skill_range: u16,
    pub skill_name: [u8; 24],
    pub up_flag: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C9)]
pub struct UpdatePartyInvitationStatePacket {
    pub allowed: u8, // always 0 on rAthena
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C8)]
pub struct SetPartyInvitationStatePacket {
    pub refuse_invite: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02DA)]
pub struct UpdateShowEquipPacket {
    pub open_equip_window: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02D9)]
pub struct UpdateConfigurationPacket {
    pub config_type: u32,
    pub value: u32, // only enabled and disabled ?
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08E2)]
pub struct NavigateToMonsterPacket {
    pub target_type: u8, // 3 - entity; 0 - coordinates; 1 - coordinates but fails if you're alweady on the map
    pub flags: u8,
    pub hide_window: u8,
    #[length(16)]
    pub map_name: String,
    pub target_position: TilePosition,
    pub target_monster_id: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum MarkerType {
    DisplayFor15Seconds,
    DisplayUntilLeave,
    RemoveMark,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0144)]
pub struct MarkMinimapPositionPacket {
    pub npc_id: EntityId,
    pub marker_type: MarkerType,
    pub position: LargeTilePosition,
    pub id: u8,
    pub color: ColorRGBA,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B5)]
pub struct NextButtonPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B6)]
pub struct CloseButtonPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B7)]
#[variable_length]
pub struct DialogMenuPacket {
    pub npc_id: EntityId,
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Copy, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum EffectId {
    Hit1,
    Hit2,
    Hit3,
    Hit4,
    Hit5,
    Hit6,
    Entry,
    Exit,
    Warp,
    Enhance,
    Coin,
    Endure,
    Beginspell,
    Glasswall,
    Healsp,
    Soulstrike,
    Bash,
    Magnumbreak,
    Steal,
    Hiding,
    Pattack,
    Detoxication,
    Sight,
    Stonecurse,
    Fireball,
    Firewall,
    Icearrow,
    Frostdiver,
    Frostdiver2,
    Lightbolt,
    Thunderstorm,
    Firearrow,
    Napalmbeat,
    Ruwach,
    Teleportation,
    Readyportal,
    Portal,
    Incagility,
    Decagility,
    Aqua,
    Signum,
    Angelus,
    Blessing,
    Incagidex,
    Smoke,
    Firefly,
    Sandwind,
    Torch,
    Spraypond,
    Firehit,
    Firesplashhit,
    Coldhit,
    Windhit,
    Poisonhit,
    Beginspell2,
    Beginspell3,
    Beginspell4,
    Beginspell5,
    Beginspell6,
    Beginspell7,
    Lockon,
    Warpzone,
    Sightrasher,
    Barrier,
    Arrowshot,
    Invenom,
    Cure,
    Provoke,
    Mvp,
    Skidtrap,
    Brandishspear,
    Cone,
    Sphere,
    Bowlingbash,
    Icewall,
    Gloria,
    Magnificat,
    Resurrection,
    Recovery,
    Earthspike,
    Spearbmr,
    Pierce,
    Turnundead,
    Sanctuary,
    Impositio,
    Lexaeterna,
    Aspersio,
    Lexdivina,
    Suffragium,
    Stormgust,
    Lord,
    Benedictio,
    Meteorstorm,
    Yufitel,
    Yufitelhit,
    Quagmire,
    Firepillar,
    Firepillarbomb,
    Hasteup,
    Flasher,
    Removetrap,
    Repairweapon,
    Crashearth,
    Perfection,
    Maxpower,
    Blastmine,
    Blastminebomb,
    Claymore,
    Freezing,
    Bubble,
    Gaspush,
    Springtrap,
    Kyrie,
    Magnus,
    Bottom,
    Blitzbeat,
    Waterball,
    Waterball2,
    Fireivy,
    Detecting,
    Cloaking,
    Sonicblow,
    Sonicblowhit,
    Grimtooth,
    Venomdust,
    Enchantpoison,
    Poisonreact,
    Poisonreact2,
    Overthrust,
    Splasher,
    Twohandquicken,
    Autocounter,
    Grimtoothatk,
    Freeze,
    Freezed,
    Icecrash,
    Slowpoison,
    Bottom2,
    Firepillaron,
    Sandman,
    Revive,
    Pneuma,
    Heavensdrive,
    Sonicblow2,
    Brandish2,
    Shockwave,
    Shockwavehit,
    Earthhit,
    Pierceself,
    Bowlingself,
    Spearstabself,
    Spearbmrself,
    Holyhit,
    Concentration,
    Refineok,
    Refinefail,
    Jobchange,
    Lvup,
    Joblvup,
    Toprank,
    Party,
    Rain,
    Snow,
    Sakura,
    StatusState,
    Banjjakii,
    Makeblur,
    Tamingsuccess,
    Tamingfailed,
    Energycoat,
    Cartrevolution,
    Venomdust2,
    Changedark,
    Changefire,
    Changecold,
    Changewind,
    Changeflame,
    Changeearth,
    Chaingeholy,
    Changepoison,
    Hitdark,
    Mentalbreak,
    Magicalatthit,
    SuiExplosion,
    Darkattack,
    Suicide,
    Comboattack1,
    Comboattack2,
    Comboattack3,
    Comboattack4,
    Comboattack5,
    Guidedattack,
    Poisonattack,
    Silenceattack,
    Stunattack,
    Petrifyattack,
    Curseattack,
    Sleepattack,
    Telekhit,
    Pong,
    Level99,
    Level99_2,
    Level99_3,
    Gumgang,
    Potion1,
    Potion2,
    Potion3,
    Potion4,
    Potion5,
    Potion6,
    Potion7,
    Potion8,
    Darkbreath,
    Deffender,
    Keeping,
    Summonslave,
    Blooddrain,
    Energydrain,
    PotionCon,
    Potion_,
    PotionBerserk,
    Potionpillar,
    Defender,
    Ganbantein,
    Wind,
    Volcano,
    Grandcross,
    Intimidate,
    Chookgi,
    Cloud,
    Cloud2,
    Mappillar,
    Linelink,
    Cloud3,
    Spellbreaker,
    Dispell,
    Deluge,
    Violentgale,
    Landprotector,
    BottomVo,
    BottomDe,
    BottomVi,
    BottomLa,
    Fastmove,
    Magicrod,
    Holycross,
    Shieldcharge,
    Mappillar2,
    Providence,
    Shieldboomerang,
    Spearquicken,
    Devotion,
    Reflectshield,
    Absorbspirits,
    Steelbody,
    Flamelauncher,
    Frostweapon,
    Lightningloader,
    Seismicweapon,
    Mappillar3,
    Mappillar4,
    Gumgang2,
    Teihit1,
    Gumgang3,
    Teihit2,
    Tanji,
    Teihit1x,
    Chimto,
    Stealcoin,
    Stripweapon,
    Stripshield,
    Striparmor,
    Striphelm,
    Chaincombo,
    RgCoin,
    Backstap,
    Teihit3,
    BottomDissonance,
    BottomLullaby,
    BottomRichmankim,
    BottomEternalchaos,
    BottomDrumbattlefield,
    BottomRingnibelungen,
    BottomRokisweil,
    BottomIntoabyss,
    BottomSiegfried,
    BottomWhistle,
    BottomAssassincross,
    BottomPoembragi,
    BottomAppleidun,
    BottomUglydance,
    BottomHumming,
    BottomDontforgetme,
    BottomFortunekiss,
    BottomServiceforyou,
    TalkFrostjoke,
    TalkScream,
    Pokjuk,
    Throwitem,
    Throwitem2,
    Chemicalprotection,
    PokjukSound,
    Demonstration,
    Chemical2,
    Teleportation2,
    PharmacyOk,
    PharmacyFail,
    Forestlight,
    Throwitem3,
    Firstaid,
    Sprinklesand,
    Loud,
    Heal,
    Heal2,
    Exit2,
    Glasswall2,
    Readyportal2,
    Portal2,
    BottomMag,
    BottomSanc,
    Heal3,
    Warpzone2,
    Forestlight2,
    Forestlight3,
    Forestlight4,
    Heal4,
    Foot,
    Foot2,
    Beginasura,
    Tripleattack,
    Hitline,
    Hptime,
    Sptime,
    Maple,
    Blind,
    Poison,
    Guard,
    Joblvup50,
    Angel2,
    Magnum2,
    Callzone,
    Portal3,
    Couplecasting,
    Heartcasting,
    Entry2,
    Saintwing,
    Spherewind,
    Colorpaper,
    Lightsphere,
    Waterfall,
    Waterfall90,
    WaterfallSmall,
    WaterfallSmall90,
    WaterfallT2,
    WaterfallT2_90,
    WaterfallSmallT2,
    WaterfallSmallT2_90,
    MiniTetris,
    Ghost,
    Bat,
    Bat2,
    Soulbreaker,
    Level99_4,
    Vallentine,
    Vallentine2,
    Pressure,
    Bash3d,
    Aurablade,
    Redbody,
    Lkconcentration,
    BottomGospel,
    Angel,
    Devil,
    Dragonsmoke,
    BottomBasilica,
    Assumptio,
    Hitline2,
    Bash3d2,
    Energydrain2,
    Transbluebody,
    Magiccrasher,
    Lightsphere2,
    Lightblade,
    Energydrain3,
    Linelink2,
    Linklight,
    Truesight,
    Falconassault,
    Tripleattack2,
    Portal4,
    Meltdown,
    Cartboost,
    Rejectsword,
    Tripleattack3,
    Spherewind2,
    Linelink3,
    Pinkbody,
    Level99_5,
    Level99_6,
    Bash3d3,
    Bash3d4,
    Napalmvalcan,
    Portal5,
    Magiccrasher2,
    BottomSpider,
    BottomFogwall,
    Soulburn,
    Soulchange,
    Baby,
    Soulbreaker2,
    Rainbow,
    Peong,
    Tanji2,
    Pressedbody,
    Spinedbody,
    Kickedbody,
    Airtexture,
    Hitbody,
    Doublegumgang,
    Reflectbody,
    Babybody,
    Babybody2,
    Giantbody,
    Giantbody2,
    Asurabody,
    _4waybody,
    Quakebody,
    AsurabodyMonster,
    Hitline3,
    Hitline4,
    Hitline5,
    Hitline6,
    Electric,
    Electric2,
    Hitline7,
    Stormkick,
    Halfsphere,
    Attackenergy,
    Attackenergy2,
    Chemical3,
    Assumptio2,
    Bluecasting,
    Run,
    Stoprun,
    Stopeffect,
    Jumpbody,
    Landbody,
    Foot3,
    Foot4,
    TaeReady,
    Grandcross2,
    Soulstrike2,
    Yufitel2,
    NpcStop,
    Darkcasting,
    Gumgangnpc,
    Agiup,
    Jumpkick,
    Quakebody2,
    Stormkick1,
    Stormkick2,
    Stormkick3,
    Stormkick4,
    Stormkick5,
    Stormkick6,
    Stormkick7,
    Spinedbody2,
    Beginasura1,
    Beginasura2,
    Beginasura3,
    Beginasura4,
    Beginasura5,
    Beginasura6,
    Beginasura7,
    Aurablade2,
    Devil1,
    Devil2,
    Devil3,
    Devil4,
    Devil5,
    Devil6,
    Devil7,
    Devil8,
    Devil9,
    Devil10,
    Doublegumgang2,
    Doublegumgang3,
    Blackdevil,
    Flowercast,
    Flowercast2,
    Flowercast3,
    Mochi,
    Lamadan,
    Edp,
    Shieldboomerang2,
    RgCoin2,
    Guard2,
    Slim,
    Slim2,
    Slim3,
    Chemicalbody,
    Castspin,
    Piercebody,
    Soullink,
    Chookgi2,
    Memorize,
    Soullight,
    Mapae,
    Itempokjuk,
    _05val,
    Beginasura11,
    Night,
    Chemical2dash,
    Groundsample,
    GiExplosion,
    Cloud4,
    Cloud5,
    BottomHermode,
    Cartter,
    Itemfast,
    Shieldboomerang3,
    Doublecastbody,
    Gravitation,
    Tarotcard1,
    Tarotcard2,
    Tarotcard3,
    Tarotcard4,
    Tarotcard5,
    Tarotcard6,
    Tarotcard7,
    Tarotcard8,
    Tarotcard9,
    Tarotcard10,
    Tarotcard11,
    Tarotcard12,
    Tarotcard13,
    Tarotcard14,
    Aciddemon,
    Greenbody,
    Throwitem4,
    BabybodyBack,
    Throwitem5,
    Bluebody,
    Hated,
    Redlightbody,
    Ro2year,
    SmaReady,
    Stin,
    RedHit,
    BlueHit,
    Quakebody3,
    Sma,
    Sma2,
    Stin2,
    Hittexture,
    Stin3,
    Sma3,
    Bluefall,
    Bluefall90,
    Fastbluefall,
    Fastbluefall90,
    BigPortal,
    BigPortal2,
    ScreenQuake,
    Homuncasting,
    Hflimoon1,
    Hflimoon2,
    Hflimoon3,
    HoUp,
    Hamidefence,
    Hamicastle,
    Hamiblood,
    Hated2,
    Twilight1,
    Twilight2,
    Twilight3,
    ItemThunder,
    ItemCloud,
    ItemCurse,
    ItemZzz,
    ItemRain,
    ItemLight,
    Angel3,
    M01,
    M02,
    M03,
    M04,
    M05,
    M06,
    M07,
    Kaizel,
    Kaahi,
    Cloud6,
    Food01,
    Food02,
    Food03,
    Food04,
    Food05,
    Food06,
    Shrink,
    Throwitem6,
    Sight2,
    Quakebody4,
    Firehit2,
    NpcStop2,
    NpcStop2Del,
    Fvoice,
    Wink,
    CookingOk,
    CookingFail,
    TempOk,
    TempFail,
    Hapgyeok,
    Throwitem7,
    Throwitem8,
    Throwitem9,
    Throwitem10,
    Bunsinjyutsu,
    Kouenka,
    Hyousensou,
    BottomSuiton,
    Stin4,
    Thunderstorm2,
    Chemical4,
    Stin5,
    MadnessBlue,
    MadnessRed,
    RgCoin3,
    Bash3d5,
    Chookgi3,
    Kirikage,
    Tatami,
    Kasumikiri,
    Issen,
    Kaen,
    Baku,
    Hyousyouraku,
    Desperado,
    LightningS,
    BlindS,
    PoisonS,
    FreezingS,
    FlareS,
    Rapidshower,
    Magicalbullet,
    Spreadattack,
    Trackcasting,
    Tracking,
    Tripleaction,
    Bullseye,
    MapMagiczone,
    MapMagiczone2,
    Damage1,
    Damage1_2,
    Damage1_3,
    Undeadbody,
    UndeadbodyDel,
    GreenNumber,
    BlueNumber,
    RedNumber,
    PurpleNumber,
    BlackNumber,
    WhiteNumber,
    YellowNumber,
    PinkNumber,
    BubbleDrop,
    NpcEarthquake,
    DaSpace,
    Dragonfear,
    Bleeding,
    Wideconfuse,
    BottomRunner,
    BottomTransfer,
    CrystalBlue,
    BottomEvilland,
    Guard3,
    NpcSlowcast,
    Criticalwound,
    Green99_3,
    Green99_5,
    Green99_6,
    Mapsphere,
    PokLove,
    PokWhite,
    PokValen,
    PokBirth,
    PokChristmas,
    MapMagiczone3,
    MapMagiczone4,
    Dust,
    TorchRed,
    TorchGreen,
    MapGhost,
    Glow1,
    Glow2,
    Glow4,
    TorchPurple,
    Cloud7,
    Cloud8,
    Flowerleaf,
    Mapsphere2,
    Glow11,
    Glow12,
    Circlelight,
    Item315,
    Item316,
    Item317,
    Item318,
    StormMin,
    PokJap,
    MapGreenlight,
    MapMagicwall,
    MapGreenlight2,
    Yellowfly1,
    Yellowfly2,
    BottomBlue,
    BottomBlue2,
    Wewish,
    Firepillaron2,
    Forestlight5,
    Soulbreaker3,
    AdoStr,
    IgnStr,
    Chimto2,
    Windcutter,
    Detect2,
    Frostmysty,
    CrimsonStr,
    HellStr,
    SprMash,
    SprSoule,
    DhowlStr,
    Earthwall,
    Soulbreaker4,
    ChainlStr,
    ChookgiFire,
    ChookgiWind,
    ChookgiWater,
    ChookgiGround,
    MagentaTrap,
    CobaltTrap,
    MaizeTrap,
    VerdureTrap,
    NormalTrap,
    Cloaking2,
    AimedStr,
    ArrowstormStr,
    LaulamusStr,
    LauagnusStr,
    MilshieldStr,
    Concentration2,
    Fireball2,
    Bunsinjyutsu2,
    Cleartime,
    Glasswall3,
    Oratio,
    PotionBerserk2,
    Circlepower,
    Rolling1,
    Rolling2,
    Rolling3,
    Rolling4,
    Rolling5,
    Rolling6,
    Rolling7,
    Rolling8,
    Rolling9,
    Rolling10,
    Purplebody,
    Stin6,
    RgCoin4,
    Poisonwav,
    Poisonsmoke,
    Gumgang4,
    Shieldboomerang4,
    Castspin2,
    Vulcanwav,
    Agiup2,
    Detect3,
    Agiup3,
    Detect4,
    Electric3,
    Guard4,
    BottomBarrier,
    BottomStealth,
    Repairtime,
    NcAnal,
    Firethrow,
    Venomimpress,
    Frostmisty,
    Burning,
    Coldthrow,
    Makehallu,
    Hallutime,
    Infraredscan,
    Crashaxe,
    Gthunder,
    Stonering,
    Intimidate2,
    Stasis,
    Redline,
    Frostdiver3,
    BottomBasilica2,
    Recognized,
    Tetra,
    Tetracasting,
    Fireball3,
    Intimidate3,
    Recognized2,
    Cloaking3,
    Intimidate4,
    Stretch,
    Blackbody,
    Enervation,
    Enervation2,
    Enervation3,
    Enervation4,
    Enervation5,
    Enervation6,
    Linelink4,
    RgCoin5,
    WaterfallAni,
    BottomManhole,
    Manhole,
    Makefeint,
    Forestlight6,
    Darkcasting2,
    BottomAni,
    BottomMaelstrom,
    BottomBloodylust,
    BeginspellN1,
    BeginspellN2,
    HealN,
    ChookgiN,
    Joblvup50_2,
    Chemical2dash2,
    Chemical2dash3,
    Rollingcast,
    WaterBelow,
    WaterFade,
    BeginspellN3,
    BeginspellN4,
    BeginspellN5,
    BeginspellN6,
    BeginspellN7,
    BeginspellN8,
    WaterSmoke,
    Dance1,
    Dance2,
    Linkparticle,
    Soullight2,
    SprParticle,
    SprParticle2,
    SprPlant,
    ChemicalV,
    Shootparticle,
    BotReverb,
    RainParticle,
    ChemicalV2,
    Secra,
    BotReverb2,
    Circlepower2,
    Secra2,
    ChemicalV3,
    Enervation7,
    Circlepower3,
    SprPlant2,
    Circlepower4,
    SprPlant3,
    RgCoin6,
    SprPlant4,
    Circlepower5,
    SprPlant5,
    Circlepower6,
    SprPlant6,
    Circlepower7,
    SprPlant7,
    Circlepower8,
    SprPlant8,
    Heartasura,
    Beginspell150,
    Level99_150,
    Primecharge,
    Glasswall4,
    GradiusLaser,
    Bash3d6,
    Gumgang5,
    Hitline8,
    Electric4,
    Teihit1t,
    Spinmove,
    Fireball4,
    Tripleattack4,
    Chemical3s,
    Groundshake,
    Dq9Charge,
    Dq9Charge2,
    Dq9Charge3,
    Dq9Charge4,
    Blueline,
    Selfscroll,
    SprLightprint,
    PngTest,
    BeginspellYb,
    Chemical2dash4,
    Groundshake2,
    Pressure2,
    RgCoin7,
    Primecharge2,
    Primecharge3,
    Primecharge4,
    Greencasting,
    Wallofthorn,
    Fireball5,
    Throwitem11,
    SprPlant9,
    Demonicfire,
    Demonicfire2,
    Demonicfire3,
    Hellsplant,
    Firewall2,
    Vacuum,
    SprPlant10,
    SprLightprint2,
    Poisonsmoke2,
    Makehallu2,
    Shockwave2,
    SprPlant11,
    Coldthrow2,
    Demonicfire4,
    Pressure3,
    Linkparticle2,
    Soullight3,
    Chareffect,
    Gumgang6,
    Fireball6,
    Gumgang7,
    Gumgang8,
    Gumgang9,
    BottomDe2,
    Coldstatus,
    SprLightprint3,
    Waterball3,
    HealN2,
    RainParticle2,
    Cloud9,
    Yellowfly3,
    ElGust,
    ElBlast,
    ElAquaplay,
    ElUpheaval,
    ElWildStorm,
    ElChillyAir,
    ElCursedSoil,
    ElCooler,
    ElTropic,
    ElPyrotechnic,
    ElPetrology,
    ElHeater,
    PoisonMist,
    EraserCutter,
    SilentBreeze,
    MagmaFlow,
    Graybody,
    LavaSlide,
    SonicClaw,
    TinderBreaker,
    MidnightFrenzy,
    Macro,
    ChemicalAllrange,
    TetraFire,
    TetraWater,
    TetraWind,
    TetraGround,
    Emitter,
    VolcanicAsh,
    Level99Orb1,
    Level99Orb2,
    Level150,
    Level150Sub,
    Throwitem4_1,
    ThrowHappokunai,
    ThrowMultipleCoin,
    ThrowBakuretsu,
    RotateHuumaranka,
    RotateBg,
    RotateLineGray,
    _2011rwc,
    _2011rwc2,
    Kaihou,
    GroundExplosion,
    KgKagehumi,
    KoZenkaiWater,
    KoZenkaiLand,
    KoZenkaiFire,
    KoZenkaiWind,
    KoJyumonjikiri,
    KoSetsudan,
    RedCross,
    KoIzayoi,
    RotateLineBlue,
    KgKyomu,
    KoHuumaranka,
    Bluelightbody,
    Kagemusya,
    ObGensou,
    No100Firecracker,
    KoMakibishi,
    Kaihou1,
    Akaitsuki,
    Zangetsu,
    Gensou,
    HatEffect,
    Cherryblossom,
    EventCloud,
    RunMakeOk,
    RunMakeFailure,
    MiresultMakeOk,
    MiresultMakeFail,
    AllRayOfProtection,
    Venomfog,
    Duststorm,
    Level160,
    Level160Sub,
    Mapchain,
    MagicFloor,
    Icemine,
    Flamecorss,
    Icemine1,
    DanceBladeAtk,
    Darkpiercing,
    Invincibleoff2,
    Maxpain,
    Deathsummon,
    Moonstar,
    Strangelights,
    SuperStar,
    Yellobody,
    Colorpaper2,
    EvilsPaw,
    GcDarkcrow,
    RkDragonbreathWater,
    AllFullThrottle,
    SrFlashcombo,
    RkLuxanima,
    Cloud10,
    SoElementalShield,
    AbOffertorium,
    WlTelekinesisIntense,
    GnIllusiondoping,
    NcMagmaEruption,
    LgKingsGrace,
    Blooddrain2,
    NpcWideweb,
    NpcBurnt,
    NpcChill,
    RaUnlimit,
    AbOffertoriumRing,
    ScEscape,
    WmFriggSong,
    Flicker,
    CMaker,
    HammerOfGod,
    MassSpiral,
    FireRain,
    Whitebody,
    BanishingBuster,
    Slugshot,
    DTail,
    BindTrap1,
    BindTrap2,
    BindTrap3,
    Jumpbody1,
    AnimatedEmitter,
    RlExplosion,
    CMaker1,
    QdShot,
    PAlter,
    SStorm,
    MusicHat,
    CloudKill,
    Escape,
    XenoSlasher,
    Flowersmoke,
    Fstone,
    Qscaraba,
    Ljosalfar,
    Happinessstar,
    PowerOfGaia,
    MapleFalls,
    MarkingUseChangemonster,
    MagicalFeather,
    MermaidLonging,
    GiftOfSnow,
    AchComplete,
    TimeAccessory,
    Spritemable,
    Tunaparty,
    Freshshrimp,
    #[numeric_value(1123)]
    SuGrooming,
    SuChattering,
    #[numeric_value(1133)]
    Firedance,
    RichsCoinA,
    #[numeric_value(1137)]
    EChain,
    HeatBarrel,
    HMine,
    FallenAngel,
    #[numeric_value(1149)]
    ImmuneProperty,
    MoveCoordinate,
    #[numeric_value(1197)]
    LightsphereSun,
    LightsphereMoon,
    LightsphereStar,
    #[numeric_value(1202)]
    Novaexplosing,
    StarEmperor,
    SmaBlack,
    #[numeric_value(1208)]
    EnergydrainBlack,
    BlinkBody,
    #[numeric_value(1218)]
    Solarburst,
    SjDocument,
    FallingStar,
    #[numeric_value(1223)]
    Stormkick8,
    #[numeric_value(1229)]
    NewmoonKick,
    FullmoonKick,
    BookOfDimension,
    #[numeric_value(1233)]
    CurseExplosion,
    SoulReaper,
    #[numeric_value(1242)]
    SoulExplosion,
    Max,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01F3)]
pub struct DisplaySpecialEffectPacket {
    pub entity_id: EntityId,
    pub effect_id: EffectId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x043D)]
pub struct DisplaySkillCooldownPacket {
    pub skill_id: SkillId,
    pub until: ClientTick,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SkillCooldownInformation {
    pub skill_id: SkillId,
    pub total_ms: u32,
    pub remaining_ms: u32,
}

/// Cooldowns restored when entering a map (`ZC_SKILL_POSTDELAY_LIST`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0985)]
#[variable_length]
pub struct SkillCooldownListPacket {
    #[repeating_remaining]
    pub cooldowns: Vec<SkillCooldownInformation>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01DE)]
pub struct DisplaySkillEffectAndDamagePacket {
    pub skill_id: SkillId,
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub start_time: ClientTick,
    pub soruce_delay: u32,
    pub destination_delay: u32,
    pub damage: u32,
    pub level: SkillLevel,
    pub div: u16,
    pub skill_type: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum HealType {
    #[numeric_value(5)]
    Health,
    #[numeric_value(7)]
    SpellPoints,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A27)]
pub struct DisplayPlayerHealEffect {
    pub heal_type: HealType,
    pub heal_amount: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09CB)]
pub struct DisplaySkillEffectNoDamagePacket {
    pub skill_id: SkillId,
    pub heal_amount: u32,
    pub destination_entity_id: EntityId,
    pub source_entity_id: EntityId,
    pub result: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0983)]
pub struct StatusChangePacket {
    pub index: u16,
    pub entity_id: EntityId,
    pub state: u8,
    pub duration_in_milliseconds: u32,
    pub remaining_in_milliseconds: u32,
    pub value: [u32; 3],
}

/// Status update without the `Total` duration field (`ZC_STATUS_CHANGE2`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x043F)]
pub struct StatusChange2Packet {
    pub index: u16,
    pub entity_id: EntityId,
    pub state: u8,
    pub remaining_in_milliseconds: u32,
    pub value: [u32; 3],
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A3B)]
#[variable_length]
pub struct EquipmentEffectPacket {
    pub entity_id: EntityId,
    pub status: u8,
    #[repeating_remaining]
    pub effects: Vec<u16>,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct PersonalInformationDetail {
    pub information_type: u8,
    pub exp: i32,
    pub death: i32,
    pub drop: i32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x097B)]
#[variable_length]
pub struct PersonalInformationPacket {
    pub total_exp: i32,
    pub total_death: i32,
    pub total_drop: i32,
    #[repeating_remaining]
    pub details: Vec<PersonalInformationDetail>,
}

#[derive(Debug, Clone, Default, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B1B)]
pub struct NotifyActorInitPacket {}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ObjectiveDetails1 {
    pub hunt_identification: u32,
    pub objective_type: u32,
    pub mob_id: u32,
    pub minimum_level: u16,
    pub maximum_level: u16,
    pub mob_count: u16,
    #[length(24)]
    pub mob_name: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct QuestAddObjectiveDetails4 {
    pub hunt_identification: u32,
    pub hunt_identification2: u32,
    pub objective_type: u32,
    pub mob_id: u32,
    pub minimum_level: u16,
    pub maximum_level: u16,
    pub mob_count: u16,
    #[length(24)]
    pub mob_name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09F9)]
pub struct QuestNotificationPacket1 {
    pub quest_id: u32,
    pub active: u8,
    pub start_time: u32,
    pub expire_time: u32,
    pub objective_count: u16,
    /// For some reason this packet always has space for three objective
    /// details, even if none are sent.
    pub objective_details: [ObjectiveDetails1; 3],
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B0C)]
pub struct QuestNotificationPacket4 {
    pub quest_id: u32,
    pub active: u8,
    pub start_time: u32,
    pub expire_time: u32,
    pub objective_count: u16,
    /// Hercules sends a fixed-size packet with room for three objectives.
    pub objective_details: [QuestAddObjectiveDetails4; 3],
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct HuntingObjective {
    pub quest_id: u32,
    pub mob_id: u32,
    pub total_count: u16,
    pub current_count: u16,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct HuntingObjective4 {
    pub quest_id: u32,
    pub hunt_identification: u32,
    pub hunt_identification2: u32,
    pub total_count: u16,
    pub current_count: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08FE)]
#[variable_length]
pub struct HuntingQuestNotificationPacket {
    #[repeating_remaining]
    pub objective_details: Vec<HuntingObjective>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09FA)]
#[variable_length]
pub struct HuntingQuestUpdateObjectivePacket {
    pub objective_count: u16,
    #[repeating_remaining]
    pub objective_details: Vec<HuntingObjective>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AFE)]
#[variable_length]
pub struct HuntingQuestUpdateObjectivePacket4 {
    pub objective_count: u16,
    #[repeating_remaining]
    pub objective_details: Vec<HuntingObjective4>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02B4)]
pub struct QuestRemovedPacket {
    pub quest_id: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct QuestDetails {
    pub hunt_identification: u32,
    pub objective_type: u32,
    pub mob_id: u32,
    pub minimum_level: u16,
    pub maximum_level: u16,
    pub kill_count: u16,
    pub total_count: u16,
    #[length(24)]
    pub mob_name: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct QuestDetails4 {
    pub hunt_identification: u32,
    pub hunt_identification2: u32,
    pub objective_type: u32,
    pub mob_id: u32,
    pub minimum_level: u16,
    pub maximum_level: u16,
    pub kill_count: u16,
    pub total_count: u16,
    #[length(24)]
    pub mob_name: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Quest {
    pub quest_id: u32,
    pub active: u8,
    pub remaining_time: u32, // TODO: double check these
    pub expire_time: u32,    // TODO: double check these
    #[new_derive]
    pub objective_count: u16,
    #[repeating(objective_count)]
    pub objective_details: Vec<QuestDetails>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Quest4 {
    pub quest_id: u32,
    pub active: u8,
    pub remaining_time: u32,
    pub expire_time: u32,
    #[new_derive]
    pub objective_count: u16,
    #[repeating(objective_count)]
    pub objective_details: Vec<QuestDetails4>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09F8)]
#[variable_length]
pub struct QuestListPacket {
    #[new_derive]
    pub quest_count: u32,
    #[repeating(quest_count)]
    pub quests: Vec<Quest>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AFF)]
#[variable_length]
pub struct QuestListPacket4 {
    #[new_derive]
    pub quest_count: u32,
    #[repeating(quest_count)]
    pub quests: Vec<Quest4>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum VisualEffect {
    BaseLevelUp,
    JobLevelUp,
    RefineFailure,
    RefineSuccess,
    GameOver,
    PharmacySuccess,
    PharmacyFailure,
    BaseLevelUpSuperNovice,
    JobLevelUpSuperNovice,
    BaseLevelUpTaekwon,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x019B)]
pub struct VisualEffectPacket {
    pub entity_id: EntityId,
    pub effect: VisualEffect,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum ExperienceType {
    #[numeric_value(1)]
    BaseExperience,
    JobExperience,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum ExperienceSource {
    Regular,
    Quest,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ACC)]
pub struct DisplayGainedExperiencePacket {
    pub account_id: AccountId,
    pub amount: u64,
    pub experience_type: ExperienceType,
    pub experience_source: ExperienceSource,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum ImageLocation {
    BottomLeft,
    BottomMiddle,
    BottomRight,
    MiddleFloating,
    MiddleColorless,
    #[numeric_value(255)]
    ClearAll,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01B3)]
pub struct DisplayImagePacket {
    #[length(64)]
    pub image_name: String,
    pub location: ImageLocation,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0229)]
pub struct StateChangePacket {
    pub entity_id: EntityId,
    pub body_state: u16,
    pub health_state: u16,
    /// Hercules sends `sc->option` here verbatim (`clif_changeoption`).
    /// Interpret with [`EntityOption`].
    pub effect_state: u32,
    pub is_pk_mode_on: u8,
}

/// `ZC_DISPEL` (0x01B9): the server cancelled an actor's active cast.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01B9)]
pub struct SkillCastCancelledPacket {
    pub entity_id: EntityId,
}

bitflags::bitflags! {
    /// The persistent option bitfield carried by [`StateChangePacket::effect_state`].
    ///
    /// Values mirror `enum` `OPTION_*` in Hercules `src/common/mmo.h`. Only the flags
    /// the client acts on are modelled; unknown bits are discarded rather than
    /// rejected, so a newer server can add flags without breaking us.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct EntityOption: u32 {
        const SIGHT = 0x0000_0001;
        const HIDE = 0x0000_0002;
        const CLOAK = 0x0000_0004;
        const FALCON = 0x0000_0010;
        const RIDING = 0x0000_0020;
        /// GM invisibility (`@hide`), not the Hiding skill.
        const INVISIBLE = 0x0000_0040;
        const RUWACH = 0x0000_2000;
        const CHASEWALK = 0x0000_4000;
        const FLYING = 0x0000_8000;
    }
}

impl EntityOption {
    /// Build from a raw `sc->option`, ignoring bits we don't model.
    pub fn from_raw(raw: u32) -> Self {
        Self::from_bits_truncate(raw)
    }

    /// Whether the entity is concealed by a *skill* (Hiding / Cloaking / Chase
    /// Walk).
    ///
    /// Mirrors Hercules' `pc_ishiding` mask (`src/map/pc.h`). Deliberately
    /// excludes [`EntityOption::INVISIBLE`], which is GM invisibility — see
    /// [`EntityOption::is_gm_invisible`].
    pub fn is_concealed(self) -> bool {
        self.intersects(Self::HIDE | Self::CLOAK | Self::CHASEWALK)
    }

    /// Whether the entity is GM-invisible (`pc_isinvisible`).
    pub fn is_gm_invisible(self) -> bool {
        self.contains(Self::INVISIBLE)
    }
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x009D)]
pub struct GroundItemAppearPacket {
    pub entity_id: EntityId,
    pub item_id: ItemId,
    pub is_identified: u8,
    pub position: TilePosition,
    pub quantity: u16,
    pub x_offset: u8,
    pub y_offset: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x009E)]
pub struct GroundItemAppear2Packet {
    pub entity_id: EntityId,
    pub item_id: ItemId,
    pub is_identified: u8,
    pub position: TilePosition,
    pub x_offset: u8,
    pub y_offset: u8,
    pub quantity: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x084B)]
pub struct GroundItemAppear3Packet {
    pub entity_id: EntityId,
    pub item_id: ItemId,
    pub item_type: u16,
    pub is_identified: u8,
    pub position: TilePosition,
    pub x_offset: u8,
    pub y_offset: u8,
    pub quantity: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ADD)]
pub struct GroundItemAppear4Packet {
    pub entity_id: EntityId,
    pub item_id: ItemId,
    pub item_type: u16,
    pub is_identified: u8,
    pub position: TilePosition,
    pub x_offset: u8,
    pub y_offset: u8,
    pub quantity: u16,
    pub show_drop_effect: u8,
    pub drop_effect_mode: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00A1)]
pub struct ItemDisappearPacket {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum ItemPickupResult {
    Success,
    Invalid,
    Overweight,
    Unknown0,
    NoSpace,
    MaximumOfItem,
    Unknown1,
    StackLimitation,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B41)]
pub struct ItemPickupPacket {
    pub index: InventoryIndex,
    pub quantity: u16,
    pub item_id: ItemId,
    pub is_identified: u8,
    pub is_broken: u8,
    pub cards: [u32; 4],
    pub equip_position: EquipPosition,
    pub item_type: u8,
    pub result: ItemPickupResult,
    pub hire_expiration_date: u32,
    pub bind_on_equip_type: u16,
    pub option_data: [ItemOptions; 5], // fix count
    pub favorite: u8,
    pub look: u16,
    pub refinement_level: u8,
    pub enchantment_level: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum RemoveItemReason {
    Normal,
    ItemUsedForSkill,
    RefineFailed,
    MaterialChanged,
    MovedToStorage,
    MovedToCart,
    ItemSold,
    ConsumedByFourSpiritAnalysis,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x07FA)]
pub struct RemoveItemFromInventoryPacket {
    pub remove_reason: RemoveItemReason,
    pub index: InventoryIndex,
    pub amount: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01C8)]
pub struct UseItemAckPacket {
    pub index: InventoryIndex,
    pub item_id: ItemId,
    pub entity_id: EntityId,
    pub amount: i16,
    pub result: u8,
}

// TODO: improve names
#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum QuestEffect {
    Quest,
    Quest2,
    Job,
    Job2,
    Event,
    Event2,
    ClickMe,
    DailyQuest,
    Event3,
    JobQuest,
    JumpingPoring,
    #[numeric_value(9999)]
    None,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum QuestColor {
    Yellow,
    Orange,
    Green,
    Purple,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0446)]
pub struct QuestEffectPacket {
    pub entity_id: EntityId,
    pub position: TilePosition,
    pub effect: QuestEffect,
    pub color: QuestColor,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B4)]
#[variable_length]
pub struct NpcDialogPacket {
    pub npc_id: EntityId,
    #[length_remaining]
    pub text: String,
}

#[derive(Debug, Clone, Default, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x007D)]
pub struct MapLoadedPacket {}

#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0187)]
#[ping]
pub struct CharacterServerKeepalivePacket {
    /// rAthena never reads this value, so just set it to 0.
    #[new_value(AccountId(0))]
    pub account_id: AccountId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0090)]
pub struct StartDialogPacket {
    pub npc_id: EntityId,
    #[new_value(1)]
    pub dialog_type: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B9)]
pub struct NextDialogPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0146)]
pub struct CloseDialogPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B8)]
pub struct ChooseDialogOptionPacket {
    pub npc_id: EntityId,
    pub option: i8,
}

/// Server → client: open the NPC numeric input dialog (`ZC_OPEN_EDITDLG`).
/// 0142 <npc id>.L
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0142)]
pub struct NpcOpenNumberInputPacket {
    pub npc_id: EntityId,
}

/// Client → server: numeric value from the NPC input dialog
/// (`CZ_INPUT_EDITDLG`). 0143 <npc id>.L <value>.L
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0143)]
pub struct NpcNumberInputPacket {
    pub npc_id: EntityId,
    pub value: i32,
}

/// Server → client: open the NPC string input dialog (`ZC_OPEN_EDITDLGSTR`).
/// 01d4 <npc id>.L
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01D4)]
pub struct NpcOpenStringInputPacket {
    pub npc_id: EntityId,
}

/// Client → server: string value from the NPC input dialog
/// (`CZ_INPUT_EDITDLGSTR`). 01d5 <packet len>.W <npc id>.L <string>.?B
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01D5)]
#[variable_length]
pub struct NpcStringInputPacket {
    pub npc_id: EntityId,
    #[length_remaining_off_by_one]
    pub text: String,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
    pub struct EquipPosition: u32 {
        const NONE = 0;
        const HEAD_LOWER = 1;
        const HEAD_MIDDLE = 512;
        const HEAD_TOP = 256;
        const RIGHT_HAND = 2;
        const LEFT_HAND = 32;
        const ARMOR = 16;
        const SHOES = 64;
        const GARMENT = 4;
        const LEFT_ACCESSORY = 8;
        const RIGTH_ACCESSORY = 128;
        const COSTUME_HEAD_TOP = 1024;
        const COSTUME_HEAD_MIDDLE = 2048;
        const COSTUME_HEAD_LOWER = 4196;
        const COSTUME_GARMENT = 8192;
        const AMMO = 32768;
        const SHADOW_ARMOR = 65536;
        const SHADOW_WEAPON = 131072;
        const SHADOW_SHIELD = 262144;
        const SHADOW_SHOES = 524288;
        const SHADOW_RIGHT_ACCESSORY = 1048576;
        const SHADOW_LEFT_ACCESSORY = 2097152;
        const LEFT_RIGHT_ACCESSORY = 136;
        const LEFT_RIGHT_HAND = 34;
        const SHADOW_LEFT_RIGHT_ACCESSORY = 3145728;
    }
}

impl FixedByteSize for EquipPosition {
    fn size_in_bytes() -> usize {
        <<Self as bitflags::Flags>::Bits as FixedByteSize>::size_in_bytes()
    }
}

impl FromBytes for EquipPosition {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        <Self as bitflags::Flags>::Bits::from_bytes(byte_reader).map(|raw| Self::from_bits(raw).expect("Invalid equip position"))
    }
}

impl ToBytes for EquipPosition {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        self.bits().to_bytes(byte_writer)
    }
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0998)]
pub struct RequestEquipItemPacket {
    pub inventory_index: InventoryIndex,
    pub equip_position: EquipPosition,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum RequestEquipItemStatus {
    Success,
    Failed,
    FailedDueToLevelRequirement,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0999)]
pub struct RequestEquipItemStatusPacket {
    pub inventory_index: InventoryIndex,
    pub equipped_position: EquipPosition,
    pub view_id: u16,
    pub result: RequestEquipItemStatus,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x013C)]
pub struct EquipAmmunitionPacket {
    pub inventory_index: InventoryIndex,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum AmmunitionActionType {
    EquipProperAmmunitionFirst,
    WeightLimitExceeded1,
    WeightLimitExceeded2,
    Equipped,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x013B)]
pub struct AmmunitionActionPacket {
    pub action_type: AmmunitionActionType,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00AB)]
pub struct RequestUnequipItemPacket {
    pub inventory_index: InventoryIndex,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum RequestUnequipItemStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x099A)]
pub struct RequestUnequipItemStatusPacket {
    pub inventory_index: InventoryIndex,
    pub equipped_position: EquipPosition,
    pub result: RequestUnequipItemStatus,
}

// --- Use item / identify / trade / storage (PACKETVER 20220406 / Hercules) ---

/// Use an inventory item (`CZ_USE_ITEM2` 0x0439).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0439)]
pub struct UseItemPacket {
    pub inventory_index: InventoryIndex,
    pub account_id: AccountId,
}

/// List of inventory indices that can be identified (`ZC_ITEMIDENTIFY_LIST`
/// 0x0177).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0177)]
#[variable_length]
pub struct ItemIdentifyListPacket {
    #[repeating_remaining]
    pub indices: Vec<InventoryIndex>,
}

/// Player selected an item to identify, or cancelled (`CZ_REQ_ITEMIDENTIFY`
/// 0x0178). Use `index == -1` to cancel the dialog.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0178)]
pub struct RequestItemIdentifyPacket {
    pub index: i16,
}

/// Result of an identify request (`ZC_ACK_ITEMIDENTIFY` 0x0179).
/// `result == 0` means success.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0179)]
pub struct ItemIdentifyResultPacket {
    pub inventory_index: InventoryIndex,
    pub result: u8,
}

/// One-click identify with a magnifier (`CZ_REQ_ONECLICK_ITEMIDENTIFY` 0x0A35).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A35)]
pub struct OneClickItemIdentifyPacket {
    pub inventory_index: InventoryIndex,
}

/// Incoming trade request (`ZC_REQ_EXCHANGE_ITEM2` 0x01F4).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01F4)]
pub struct TradeRequestPacket {
    #[length(24)]
    pub name: String,
    pub character_id: CharacterId,
    pub base_level: u16,
}

/// Request to trade with another player (`CZ_REQ_EXCHANGE_ITEM` 0x00E4).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00E4)]
pub struct RequestTradePacket {
    pub account_id: AccountId,
}

/// Accept (3) or reject (4) a trade request (`CZ_ACK_EXCHANGE_ITEM` 0x00E6).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00E6)]
pub struct TradeAckPacket {
    pub result: u8,
}

/// Trade request reply / trade window open (`ZC_ACK_EXCHANGE_ITEM2` 0x01F5).
/// result: 0 too far, 1 no char, 2 fail, 3 accept, 4 cancel, 5 busy.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01F5)]
pub struct TradeStartPacket {
    pub result: u8,
    pub character_id: CharacterId,
    pub base_level: u16,
}

/// Add item or zeny to the trade (`CZ_ADD_EXCHANGE_ITEM` 0x00E8).
/// `inventory_index == 0` means zeny; amount is the zeny value.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00E8)]
pub struct TradeAddItemPacket {
    pub inventory_index: InventoryIndex,
    pub amount: u32,
}

/// Other player's item added to the trade (`ZC_ADD_EXCHANGE_ITEM` 0x0A96).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A96)]
pub struct TradeAddItemNotifyPacket {
    pub item_id: ItemId,
    pub item_type: u8,
    pub amount: u32,
    pub identified: u8,
    pub damaged: u8,
    pub refine: u8,
    pub slot: [u32; 4],
    pub option_data: [ItemOptions; 5],
    pub location: u32,
    pub look: u16,
}

/// Result of adding our item to the trade (`ZC_ACK_ADD_EXCHANGE_ITEM` 0x00EA).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00EA)]
pub struct TradeAddItemResultPacket {
    pub inventory_index: InventoryIndex,
    pub result: u8,
}

/// Lock / conclude one side of the trade (`CZ_CONCLUDE_EXCHANGE_ITEM` 0x00EB).
#[derive(Debug, Clone, Default, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00EB)]
pub struct TradeOkPacket {}

/// One side locked their trade offer (`ZC_CONCLUDE_EXCHANGE_ITEM` 0x00EC).
/// `who`: 0 = self, 1 = other.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00EC)]
pub struct TradeLockPacket {
    pub who: u8,
}

/// Cancel trade (`CZ_CANCEL_EXCHANGE_ITEM` 0x00ED).
#[derive(Debug, Clone, Default, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00ED)]
pub struct TradeCancelPacket {}

/// Trade cancelled (`ZC_CANCEL_EXCHANGE_ITEM` 0x00EE).
#[derive(Debug, Clone, Default, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00EE)]
pub struct TradeCancelledPacket {}

/// Commit trade (`CZ_EXEC_EXCHANGE_ITEM` 0x00EF).
#[derive(Debug, Clone, Default, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00EF)]
pub struct TradeCommitPacket {}

/// Trade finished (`ZC_EXEC_EXCHANGE_ITEM` 0x00F0). `result`: 0 success, 1
/// failure.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F0)]
pub struct TradeCompletedPacket {
    pub result: u8,
}

/// Move inventory item into storage (`CZ_MOVE_ITEM_FROM_BODY_TO_STORE2`
/// 0x0364).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0364)]
pub struct MoveItemToStoragePacket {
    pub inventory_index: InventoryIndex,
    pub amount: u32,
}

/// Move storage item into inventory (`CZ_MOVE_ITEM_FROM_STORE_TO_BODY2`
/// 0x0365).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0365)]
pub struct MoveItemFromStoragePacket {
    pub storage_index: StorageIndex,
    pub amount: u32,
}

/// Close storage (`CZ_CLOSE_STORE` 0x00F7).
#[derive(Debug, Clone, Default, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0193)]
pub struct CloseStoragePacket {}

/// Storage capacity (`ZC_NOTIFY_STOREITEM_COUNTINFO` 0x00F2).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F2)]
pub struct StorageAmountPacket {
    pub amount: u16,
    pub max_amount: u16,
}

/// Item added to storage (`ZC_ADD_ITEM_TO_STORE`).
///
/// PACKETVER main ≥ 20200916 uses header **0x0B44** with slot/options before
/// refine/grade (see Hercules `PACKET_ZC_ADD_ITEM_TO_STORE`). Older clients
/// used 0x0A0A with refine before slot — wrong for our 20220406 stack.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B44)]
pub struct StorageItemAddedPacket {
    pub index: StorageIndex,
    pub amount: u32,
    pub item_id: ItemId,
    pub item_type: u8,
    pub identified: u8,
    pub damaged: u8,
    pub slot: [u32; 4],
    pub option_data: [ItemOptions; 5],
    pub refine: u8,
    pub grade: u8,
}

/// Item removed from storage (`ZC_DELETE_ITEM_FROM_STORE` 0x00F6).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F6)]
pub struct StorageItemRemovedPacket {
    pub index: StorageIndex,
    pub amount: u32,
}

/// Storage closed (`ZC_CLOSE_STORE` 0x00F8).
#[derive(Debug, Clone, Default, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F8)]
pub struct StorageClosedPacket {}

/// Inventory list type on unified item-list packets (`ZC_INVENTORY_START`
/// invType).
pub mod inventory_type {
    pub const INVENTORY: u8 = 0;
    pub const CART: u8 = 1;
    pub const STORAGE: u8 = 2;
    pub const GUILD_STORAGE: u8 = 3;
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum RestartType {
    Respawn,
    Disconnect,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B1)]
pub struct ParameterChangePacket {
    pub variable_id: u16,
    pub value: u32,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B2)]
pub struct RestartPacket {
    pub restart_type: RestartType,
}

// TODO: check that this can be only 1 and 0, if not ByteConvertable
// should be implemented manually
#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum RestartResponseStatus {
    Nothing,
    Ok,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B3)]
pub struct RestartResponsePacket {
    pub result: RestartResponseStatus,
}

// TODO: check that this can be only 1 and 0, if not Named, ByteConvertable
// should be implemented manually
#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum DisconnectResponseStatus {
    Ok,
    Wait10Seconds,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x018B)]
pub struct DisconnectResponsePacket {
    pub result: DisconnectResponseStatus,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0438)]
pub struct UseSkillAtIdPacket {
    pub skill_level: SkillLevel,
    pub skill_id: SkillId,
    pub target_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AF4)]
pub struct UseSkillOnGroundPacket {
    pub skill_level: SkillLevel,
    pub skill_id: SkillId,
    pub target_position: TilePosition,
    #[new_default]
    pub unused: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B10)]
pub struct StartUseSkillPacket {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub target_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B11)]
pub struct EndUseSkillPacket {
    pub skill_id: SkillId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x07FB)]
pub struct UseSkillSuccessPacket {
    pub source_entity: EntityId,
    pub destination_entity: EntityId,
    pub position: TilePosition,
    pub skill_id: SkillId,
    pub element: u32,
    pub delay_time: u32,
    pub disposable: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B1A)]
pub struct UseSkillAckPacket {
    pub source_entity: EntityId,
    pub destination_entity: EntityId,
    pub position: TilePosition,
    pub skill_id: SkillId,
    pub element: u32,
    pub delay_time: u32,
    pub disposable: u8,
    pub attack_motion_time: u32,
}

/// Abort the caster's own in-progress cast (`CZ_CANCEL_CAST` 0x0F00).
///
/// **This packet does not exist in official Ragnarok** — it is a Korangar fork
/// addition, matched by a companion Hercules delta (`clif_parse_CancelCast`, see
/// korangar `CLAUDE.md` §3b). Official RO offers no player-initiated cast cancel
/// at all, and forbids moving while casting, so there was nothing to reuse.
///
/// `0x0F00` is deliberate: it is exactly Hercules' `MAX_PACKET_DB` ceiling, so it
/// is in bounds for `packets->db[]` while sitting far above every real packet id.
/// A client packet with no length entry makes Hercules **disconnect the session**,
/// so the length lives in the non-generated `common/packets_len.h`.
#[derive(Debug, Clone, Default, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0F00)]
pub struct CancelCastPacket {}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0110)]
pub struct ToUseSkillSuccessPacket {
    pub skill_id: SkillId,
    pub btype: i32,
    pub item_id: ItemId,
    pub flag: u8,
    pub cause: u8,
}

/// `ZC_NOTIFY_MAPINFO` (0x0189) — an area restriction refused the action.
///
/// Hercules sends this *instead of* `clif->skill_fail` for map-zone rejections
/// (`clif_skill_mapinfomessage`, `clif.c:6213`; the comment there says as much:
/// "This skill uses this msg instead of skill fails"). It is the only feedback
/// the player gets when a skill is listed in the map zone's `disabled_skills`,
/// which on a non-PvP map covers `DC_UGLYDANCE`, `BD_ETERNALCHAOS`,
/// `BD_ROKISWEIL`, `CG_HERMODE`, `BA_DISSONANCE` and `DC_DONTFORGETME`.
///
/// Because it had no packet here, `register_length_fallbacks` consumed it and
/// the refusal was completely silent: the cast simply did nothing, with no
/// message and no cast bar. Found by the Dancer/Gypsy skill sweeps once they
/// could run at all (2026-07-29 shuffle pass).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0189)]
pub struct NotifyMapInfoPacket {
    /// 0 = cannot teleport here, 1 = save point cannot be memorized,
    /// 2 = skill unusable here, 3 = item unusable here.
    pub info_type: u16,
}

#[derive(Debug, Clone, Copy, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum UnitId {
    #[numeric_value(0x7E)]
    Safetywall,
    Firewall,
    WarpWaiting,
    WarpActive,
    Benedictio,
    Sanctuary,
    Magnus,
    Pneuma,
    Dummyskill,
    FirepillarWaiting,
    FirepillarActive,
    HiddenTrap,
    Trap,
    HiddenWarpNpc,
    UsedTraps,
    Icewall,
    Quagmire,
    Blastmine,
    Skidtrap,
    Anklesnare,
    Venomdust,
    Landmine,
    Shockwave,
    Sandman,
    Flasher,
    Freezingtrap,
    Claymoretrap,
    Talkiebox,
    Volcano,
    Deluge,
    Violentgale,
    Landprotector,
    Lullaby,
    Richmankim,
    Eternalchaos,
    Drumbattlefield,
    Ringnibelungen,
    Rokisweil,
    Intoabyss,
    Siegfried,
    Dissonance,
    Whistle,
    Assassincross,
    Poembragi,
    Appleidun,
    Uglydance,
    Humming,
    Dontforgetme,
    Fortunekiss,
    Serviceforyou,
    Graffiti,
    Demonstration,
    Callfamily,
    Gospel,
    Basilica,
    Moonlit,
    Fogwall,
    Spiderweb,
    Gravitation,
    Hermode,
    Kaensin,
    Suiton,
    Tatamigaeshi,
    Kaen,
    GrounddriftWind,
    GrounddriftDark,
    GrounddriftPoison,
    GrounddriftWater,
    GrounddriftFire,
    Deathwave,
    Waterattack,
    Windattack,
    Earthquake,
    Evilland,
    DarkRunner,
    DarkTransfer,
    Epiclesis,
    Earthstrain,
    Manhole,
    Dimensiondoor,
    Chaospanic,
    Maelstrom,
    Bloodylust,
    Feintbomb,
    Magentatrap,
    Cobalttrap,
    Maizetrap,
    Verduretrap,
    Firingtrap,
    Iceboundtrap,
    Electricshocker,
    Clusterbomb,
    Reverberation,
    SevereRainstorm,
    Firewalk,
    Electricwalk,
    Netherworld,
    PsychicWave,
    CloudKill,
    Poisonsmoke,
    Neutralbarrier,
    Stealthfield,
    Warmer,
    ThornsTrap,
    Wallofthorn,
    DemonicFire,
    FireExpansionSmokePowder,
    FireExpansionTearGas,
    HellsPlant,
    VacuumExtreme,
    Banding,
    FireMantle,
    WaterBarrier,
    Zephyr,
    PowerOfGaia,
    FireInsignia,
    WaterInsignia,
    WindInsignia,
    EarthInsignia,
    PoisonMist,
    LavaSlide,
    VolcanicAsh,
    ZenkaiWater,
    ZenkaiLand,
    ZenkaiFire,
    ZenkaiWind,
    Makibishi,
    Venomfog,
    Icemine,
    Flamecross,
    Hellburning,
    MagmaEruption,
    KingsGrace,
    GlitteringGreed,
    BTrap,
    FireRain,
    Catnippowder,
    Nyanggrass,
    Creatingstar,
    Dummy0,
    RainOfCrystal,
    MysteryIllusion,
    #[numeric_value(269)]
    StrantumTremor,
    ViolentQuake,
    AllBloom,
    TornadoStorm,
    FloralFlareRoad,
    AstralStrike,
    CrossRain,
    PneumaticusProcella,
    AbyssSquare,
    AcidifiedZoneWater,
    AcidifiedZoneGround,
    AcidifiedZoneWind,
    AcidifiedZoneFire,
    LightningLand,
    VenomSwamp,
    Conflagration,
    CaneOfEvilEye,
    TwinklingGalaxy,
    StarCannon,
    GrenadesDropping,
    #[numeric_value(290)]
    Fuumashouaku,
    MissionBombard,
    TotemOfTutelary,
    HyunRoksBreeze,
    Shinkirou, // mirage
    JackFrostNova,
    GroundGravitation,
    #[numeric_value(298)]
    Kunaiwaikyoku,
    #[numeric_value(20852)]
    Deepblindtrap,
    Solidtrap,
    Swifttrap,
    Flametrap,
    #[numeric_value(0xC1)]
    GdLeadership,
    #[numeric_value(0xC2)]
    GdGlorywounds,
    #[numeric_value(0xC3)]
    GdSoulcold,
    #[numeric_value(0xC4)]
    GdHawkeyes,
    #[numeric_value(0x190)]
    Max,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09CA)]
pub struct NotifySkillUnitPacket {
    pub lenght: u16,
    pub entity_id: EntityId,
    pub creator_id: EntityId,
    pub position: TilePosition,
    pub unit_id: UnitId,
    pub range: u8,
    pub visible: u8,
    pub skill_level: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0117)]
pub struct NotifyGroundSkillPacket {
    pub skill_id: SkillId,
    pub entity_id: EntityId,
    pub level: SkillLevel,
    pub position: TilePosition,
    pub start_time: ClientTick,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0120)]
pub struct SkillUnitDisappearPacket {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Friend {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0202)]
pub struct AddFriendPacket {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0203)]
pub struct RemoveFriendPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x020A)]
pub struct NotifyFriendRemovedPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0201)]
#[variable_length]
pub struct FriendListPacket {
    #[repeating_remaining]
    pub friend_list: Vec<Friend>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum OnlineState {
    Online,
    Offline,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0206)]
pub struct FriendOnlineStatusPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub state: OnlineState,
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0207)]
pub struct FriendRequestPacket {
    pub requestee: Friend,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum FriendRequestResponse {
    Reject,
    Accept,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0208)]
pub struct FriendRequestResponsePacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub response: FriendRequestResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum FriendRequestResult {
    Accepted,
    Rejected,
    OwnFriendListFull,
    OtherFriendListFull,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0209)]
pub struct FriendRequestResultPacket {
    pub result: FriendRequestResult,
    pub friend: Friend,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C6)]
pub struct PartyInvitePacket {
    pub party_id: PartyId,
    #[length(24)]
    pub party_name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00FA)]
pub struct CreatePartyResultPacket {
    pub result: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01E8)]
pub struct CreatePartyPacket {
    #[length(24)]
    pub party_name: String,
    pub share_pickup: u8,
    pub share_loot: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C4)]
pub struct PartyInviteRequestPacket {
    #[length(24)]
    pub character_name: String,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C7)]
pub struct PartyInviteResponsePacket {
    pub party_id: PartyId,
    pub accepted: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0100)]
pub struct LeavePartyPacket {}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C5)]
pub struct PartyInviteResultPacket {
    #[length(24)]
    pub character_name: String,
    pub result: u32,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct PartyMember {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    #[length(24)]
    pub player_name: String,
    #[length(16)]
    pub map_name: String,
    pub leader: u8,
    pub offline: u8,
    pub job_id: JobId,
    pub base_level: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AE5)]
#[variable_length]
pub struct PartyListPacket {
    #[length(24)]
    pub party_name: String,
    #[repeating_remaining]
    pub members: Vec<PartyMember>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AE4)]
pub struct PartyMemberInfoPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub leader: u32,
    pub job_id: JobId,
    pub base_level: u16,
    pub position: TilePosition,
    pub offline: u8,
    #[length(24)]
    pub party_name: String,
    #[length(24)]
    pub player_name: String,
    #[length(16)]
    pub map_name: String,
    pub share_pickup: u8,
    pub share_loot: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0107)]
pub struct PartyMemberPositionPacket {
    pub account_id: AccountId,
    pub position: TilePosition,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x080E)]
pub struct PartyMemberHealthPacket {
    pub account_id: AccountId,
    pub health_points: u32,
    pub maximum_health_points: u32,
}

/// Party member's HP **and SP** (`ZC_NOTIFY_HP_TO_GROUPM`, wide form).
///
/// Official main-branch servers send the narrow 14-byte [`PartyMemberHealthPacket`]
/// (0x080E) and never report a party member's SP at all; only the Zero branch
/// sends this 22-byte form. Our Hercules delta selects it on every branch so the
/// client can draw an SP bar over party members — see `KORANGAR_PARTY_SP_TO_GROUPM`
/// in `src/map/packets_struct.h` and korangar `CLAUDE.md` §3b.
///
/// Both packets stay registered: 0x0BAB is what our server now sends, and 0x080E
/// remains correct against a stock server.
/// Who sent a party invite (`ZC_PARTY_INVITE_SENDER`, **fork packet 0x0EFF**).
///
/// **This packet does not exist in official Ragnarok.** `ZC_PARTY_JOIN_REQ`
/// carries only the party id and party name, so an official client can say "you
/// are invited to join <party>" but never "<player> invites you". Our Hercules
/// delta sends this immediately *before* the invite and the client pairs the
/// two by party id.
///
/// Sent alongside rather than by widening 0x00FE so the official packet keeps
/// its official shape, and so the feature **degrades gracefully**: without this
/// packet the invite still works and simply names the party.
///
/// 0x0EFF sits above the highest official packet (0x0BC0) and below
/// `MAX_PACKET_DB` (0x0F00, taken by [`CancelCastPacket`]). Its length lives in
/// Hercules' hand-maintained `common/packets_len.h`.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0EFF)]
pub struct PartyInviteSenderPacket {
    pub party_id: PartyId,
    #[length(24)]
    pub character_name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0BAB)]
pub struct PartyMemberVitalsPacket {
    pub account_id: AccountId,
    pub health_points: u32,
    pub maximum_health_points: u32,
    pub spell_points: u32,
    pub maximum_spell_points: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ABD)]
pub struct PartyMemberJobAndLevelPacket {
    pub account_id: AccountId,
    pub job_id: JobId,
    pub base_level: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0105)]
pub struct PartyMemberRemovedPacket {
    pub account_id: AccountId,
    #[length(24)]
    pub character_name: String,
    pub result: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0108)]
#[variable_length]
pub struct PartyChatMessagePacket {
    #[length_remaining_off_by_one]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0109)]
#[variable_length]
pub struct NotifyPartyChatMessagePacket {
    pub account_id: AccountId,
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0096)]
#[variable_length]
pub struct WhisperSendPacket {
    #[length(24)]
    pub target_name: String,
    #[length_remaining_off_by_one]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09DE)]
#[variable_length]
pub struct WhisperMessagePacket {
    pub sender_character_id: CharacterId,
    #[length(24)]
    pub sender_name: String,
    pub is_admin: u8,
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0098)]
pub struct WhisperResultPacket {
    pub result: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09DF)]
pub struct WhisperResult2Packet {
    pub result: u8,
    pub character_id: CharacterId,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ReputationEntry {
    pub reputation_type: u64,
    pub points: i64,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B8D)]
#[variable_length]
pub struct ReputationPacket {
    pub success: u8,
    #[repeating_remaining]
    pub entries: Vec<ReputationEntry>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Aliance {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Antagonist {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x098A)]
#[variable_length]
pub struct ClanInfoPacket {
    pub clan_id: u32,
    #[length(24)]
    pub clan_name: String,
    #[length(24)]
    pub clan_master: String,
    #[length(16)]
    pub clan_map: String,
    #[new_derive]
    pub aliance_count: u8,
    #[new_derive]
    pub antagonist_count: u8,
    #[repeating(aliance_count)]
    pub aliances: Vec<Aliance>,
    #[repeating(antagonist_count)]
    pub antagonists: Vec<Antagonist>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0988)]
pub struct ClanOnlineCountPacket {
    pub online_members: u16,
    pub maximum_members: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0192)]
pub struct ChangeMapCellPacket {
    pub position: TilePosition,
    pub cell_type: u16,
    #[length(16)]
    pub map_name: String,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct MarketItemInformation {
    pub name_id: u32,
    pub item_type: u8,
    pub price: Price,
    pub quantity: u32,
    pub weight: u16,
    pub location: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B7A)]
#[variable_length]
pub struct OpenMarketPacket {
    #[repeating_remaining]
    pub items: Vec<MarketItemInformation>,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ShopItemInformation {
    pub item_id: ItemId,
    pub price: Price,
    pub discount_price: Price,
    pub item_type: u8,
    pub view_sprite: u16,
    pub location: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B77)]
#[variable_length]
pub struct ShopItemListPacket {
    #[repeating_remaining]
    pub items: Vec<ShopItemInformation>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C4)]
pub struct BuyOrSellPacket {
    pub shop_id: ShopId,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum BuyOrSellOption {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C5)]
pub struct SelectBuyOrSellPacket {
    pub shop_id: ShopId,
    pub option: BuyOrSellOption,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u8)]
pub enum BuyItemResult {
    #[numeric_value(0)]
    Successful,
    #[numeric_value(1)]
    NotEoughZeny,
    #[numeric_value(2)]
    WeightLimitExceeded,
    #[numeric_value(3)]
    TooManyItems,
    #[numeric_value(9)]
    TooManyOfThisItem,
    #[numeric_value(10)]
    PropsOpenAir,
    #[numeric_value(11)]
    ExchangeFailed,
    #[numeric_value(12)]
    ExchangeWellDone,
    #[numeric_value(13)]
    ItemSoldOut,
    #[numeric_value(14)]
    NotEnoughGoods,
}

/// One line of `CZ_PC_PURCHASE_ITEMLIST` (0x00C8) for PACKETVER ≥ 20181121.
///
/// Wire layout: `amount` (u16) + `itemId` (u32). Older clients used u16 item
/// ids.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct BuyItemInformation {
    pub amount: u16,
    pub item_id: ItemId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C8)]
#[variable_length]
pub struct BuyItemsPacket {
    #[repeating_remaining]
    pub items: Vec<BuyItemInformation>,
}

/// `ZC_PC_PURCHASE_RESULT` (0x00CA) — NPC shop purchase outcome (not market
/// 0x0B4E).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00CA)]
pub struct BuyItemsResultPacket {
    pub result: BuyItemResult,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct BuyShopItemInformation {
    pub item_id: ItemId,
    pub amount: u32,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09D6)]
#[variable_length]
pub struct BuyShopItemsPacket {
    pub items: Vec<BuyShopItemInformation>,
}

#[derive(Debug, Clone, Copy, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum BuyShopItemsResult {
    #[numeric_value(0)]
    Success,
    #[numeric_value(0xFFFF)]
    Error,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct BoughtShopItemInformation {
    pub item_id: ItemId,
    pub amount: u16,
    pub price: Price,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B4E)]
#[variable_length]
pub struct BuyShopItemsResultPacket {
    pub result: BuyShopItemsResult,
    #[repeating_remaining]
    pub purchased_items: Vec<BoughtShopItemInformation>,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09D4)]
pub struct CloseShopPacket {}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SellItemInformation {
    pub inventory_index: InventoryIndex,
    pub price: Price,
    pub overcharge_price: Price,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C7)]
#[variable_length]
pub struct SellListPacket {
    #[repeating_remaining]
    pub items: Vec<SellItemInformation>,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SoldItemInformation {
    pub inventory_index: InventoryIndex,
    pub amount: u16,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C9)]
#[variable_length]
pub struct SellItemsPacket {
    #[repeating_remaining]
    pub items: Vec<SoldItemInformation>,
}

#[derive(Debug, Clone, Copy, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum SellItemsResult {
    Success,
    Error,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00CB)]
pub struct SellItemsResultPacket {
    pub result: SellItemsResult,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0112)]
pub struct LevelUpSkillPacket {
    pub skill_id: SkillId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x010E)]
pub struct UpdateSkillPacket {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub spell_point_cost: u16,
    pub attack_range: AttackRange,
    // TODO: bool,
    pub upgradable: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0441)]
pub struct RemoveSkillPacket {
    pub skill_id: SkillId,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet_bytes(packet: impl PacketExt) -> Vec<u8> {
        let mut byte_writer = ByteWriter::new();
        let written = packet.packet_to_bytes(&mut byte_writer).unwrap();

        assert_eq!(written, byte_writer.len());

        byte_writer.into_inner()
    }

    fn read_packet<T: PacketExt>(bytes: &[u8]) -> T {
        let mut byte_reader = ByteReader::without_metadata(bytes);
        T::packet_from_bytes(&mut byte_reader).unwrap()
    }

    fn fixed_string(value: &str, length: usize) -> Vec<u8> {
        let mut bytes = value.as_bytes().to_vec();
        bytes.resize(length, 0);
        bytes
    }

    #[test]
    fn character_information_sizes_match_hercules_layouts() {
        // Verified against `char_mmo_char_tobuf` in Hercules src/char/char.c:
        // 155 bytes at PACKETVER 20190605 (32-bit hp, 16-bit sp),
        // 175 bytes at PACKETVER >= 20201007 (64-bit hp and sp).
        assert_eq!(<CharacterInformationLegacy as FixedByteSize>::size_in_bytes(), 155);
        assert_eq!(<CharacterInformation as FixedByteSize>::size_in_bytes(), 175);
    }

    #[test]
    fn cancel_cast_packet_is_a_bare_two_byte_header() {
        // Must be exactly 2 bytes (header only) to match the `packetLen(0x0f00, 2)`
        // entry the companion Hercules delta adds in `common/packets_len.h`. A
        // mismatch does not warn — Hercules reads the wrong number of bytes and
        // desyncs, or `clif_parse` disconnects the session outright.
        assert_eq!(packet_bytes(CancelCastPacket::default()), [0x00, 0x0F]);

        // 0x0F00 is Hercules' MAX_PACKET_DB ceiling: any higher and both
        // `packets_addLen` and `packetdb_addpacket` reject the registration, so the
        // server would drop every client that sent it.
        assert!(CancelCastPacket::HEADER.0 <= 0x0F00);
    }

    #[test]
    fn map_server_login_packet_matches_20220406_layout() {
        // 23-byte CZ_ENTER with login_id2 — requires the server to be built
        // with --enable-packetver=20220406 (a default Hercules build at
        // 20190605 expects 19 bytes and desyncs on this packet).
        let packet = MapServerLoginPacket::new(
            AccountId(2_000_004),
            CharacterId(150_004),
            0x287C_D2C1,
            0x1122_3344,
            ClientTick(100),
            Sex::Male,
        );

        assert_eq!(packet_bytes(packet), [
            0x36, 0x04, 0x84, 0x84, 0x1E, 0x00, 0xF4, 0x49, 0x02, 0x00, 0xC1, 0xD2, 0x7C, 0x28, 0x44, 0x33, 0x22, 0x11, 0x64, 0x00, 0x00,
            0x00, 0x01,
        ]);
    }

    #[test]
    fn request_player_move_packet_matches_20220406_opcode() {
        assert_eq!(
            packet_bytes(RequestPlayerMovePacket::new(WorldPosition::new(100, 200, Direction::North))),
            [0x5F, 0x03, 0x19, 0x0C, 0x84]
        );
    }

    #[test]
    fn request_action_packet_matches_20220406_opcode() {
        assert_eq!(packet_bytes(RequestActionPacket::new(EntityId(0x0102_0304), Action::Attack)), [
            0x37, 0x04, 0x04, 0x03, 0x02, 0x01, 0x00
        ]);
    }

    #[test]
    fn item_pickup_request_packet_matches_20220406_opcode() {
        assert_eq!(packet_bytes(ItemPickupRequestPacket::new(EntityId(0x0102_0304))), [
            0x62, 0x03, 0x04, 0x03, 0x02, 0x01
        ]);
    }

    #[test]
    fn drop_item_packet_matches_20220406_layout() {
        // Header 0x0363, inventory index 0 serializes as 2, amount 5.
        assert_eq!(packet_bytes(DropItemPacket::new(InventoryIndex(0), 5)), [
            0x63, 0x03, 0x02, 0x00, 0x05, 0x00
        ]);
    }

    #[test]
    fn request_details_packet_matches_20220406_opcode() {
        assert_eq!(packet_bytes(RequestDetailsPacket::new(EntityId(0x0102_0304))), [
            0x68, 0x03, 0x04, 0x03, 0x02, 0x01
        ]);
    }

    #[test]
    fn request_server_tick_packet_matches_20220406_opcode() {
        assert_eq!(packet_bytes(RequestServerTickPacket::new(ClientTick(100))), [
            0x60, 0x03, 0x64, 0x00, 0x00, 0x00
        ]);
    }

    #[test]
    fn open_ui_packet_matches_20220406_layout() {
        let packet = read_packet::<OpenUiPacket>(&[0xE2, 0x0A, 0x05, 0x00, 0x00, 0x00, 0x00]);

        assert_eq!(packet.ui_type, 5);
        assert_eq!(packet.data, 0);
    }

    #[test]
    fn message_table_color_packet_matches_20220406_layout() {
        let packet = read_packet::<MessageTableColorPacket>(&[0xCD, 0x09, 0x92, 0x0D, 0xFF, 0x00, 0x00, 0x00]);

        assert_eq!(packet.message_id, 0x0D92);
        assert_eq!(packet.message_color, 0x0000_00FF);
    }

    #[test]
    fn change_direction_packet_matches_20220406_layout() {
        let packet = read_packet::<ChangeDirectionPacket>(&[0x9C, 0x00, 0x6D, 0xC4, 0x8E, 0x06, 0x00, 0x00, 0x02]);

        assert_eq!(packet.entity_id, EntityId(0x068E_C46D));
        assert_eq!(packet.head_direction, 0);
        assert_eq!(packet.direction, 2);
    }

    #[test]
    fn party_member_info_packet_matches_20220406_layout() {
        let mut bytes = vec![0xE4, 0x0A];
        bytes.extend([0x04, 0x03, 0x02, 0x01]); // AID
        bytes.extend([0x08, 0x07, 0x06, 0x05]); // GID
        bytes.extend([0x00, 0x00, 0x00, 0x00]); // leader
        bytes.extend([0xA1, 0x0F]); // class
        bytes.extend([0x63, 0x00]); // base level
        bytes.extend([0x78, 0x00]); // x
        bytes.extend([0x8C, 0x00]); // y
        bytes.push(0x00); // online
        bytes.extend(fixed_string("Seal Party", 24));
        bytes.extend(fixed_string("Alice", 24));
        bytes.extend(fixed_string("prontera", 16));
        bytes.push(0x01); // share pickup
        bytes.push(0x00); // share loot

        assert_eq!(bytes.len(), 89);

        let packet = read_packet::<PartyMemberInfoPacket>(&bytes);

        assert_eq!(packet.account_id, AccountId(0x0102_0304));
        assert_eq!(packet.character_id, CharacterId(0x0506_0708));
        assert_eq!(packet.leader, 0);
        assert_eq!(packet.job_id, JobId(4001));
        assert_eq!(packet.base_level, 99);
        assert_eq!(packet.position, TilePosition { x: 120, y: 140 });
        assert_eq!(packet.party_name, "Seal Party");
        assert_eq!(packet.player_name, "Alice");
        assert_eq!(packet.map_name, "prontera");
        assert_eq!(packet.share_pickup, 1);
        assert_eq!(packet.share_loot, 0);
    }

    #[test]
    fn party_list_packet_consumes_modern_member_rows() {
        let mut bytes = vec![0xE5, 0x0A];
        bytes.extend([0x88, 0x00]); // 28-byte header + 2 * 54-byte members.
        bytes.extend(fixed_string("Seal Party", 24));

        bytes.extend([0x04, 0x03, 0x02, 0x01]);
        bytes.extend([0x08, 0x07, 0x06, 0x05]);
        bytes.extend(fixed_string("Alice", 24));
        bytes.extend(fixed_string("prontera", 16));
        bytes.push(0x00);
        bytes.push(0x00);
        bytes.extend([0xA1, 0x0F]);
        bytes.extend([0x63, 0x00]);

        bytes.extend([0x14, 0x13, 0x12, 0x11]);
        bytes.extend([0x18, 0x17, 0x16, 0x15]);
        bytes.extend(fixed_string("Bob", 24));
        bytes.extend(fixed_string("izlude", 16));
        bytes.push(0x01);
        bytes.push(0x00);
        bytes.extend([0x02, 0x00]);
        bytes.extend([0x2D, 0x00]);

        assert_eq!(bytes.len(), 136);

        let packet = read_packet::<PartyListPacket>(&bytes);

        assert_eq!(packet.party_name, "Seal Party");
        assert_eq!(packet.members.len(), 2);
        assert_eq!(packet.members[0].account_id, AccountId(0x0102_0304));
        assert_eq!(packet.members[0].character_id, CharacterId(0x0506_0708));
        assert_eq!(packet.members[0].player_name, "Alice");
        assert_eq!(packet.members[0].map_name, "prontera");
        assert_eq!(packet.members[0].leader, 0);
        assert_eq!(packet.members[0].job_id, JobId(4001));
        assert_eq!(packet.members[0].base_level, 99);
        assert_eq!(packet.members[1].account_id, AccountId(0x1112_1314));
        assert_eq!(packet.members[1].character_id, CharacterId(0x1516_1718));
        assert_eq!(packet.members[1].player_name, "Bob");
        assert_eq!(packet.members[1].map_name, "izlude");
        assert_eq!(packet.members[1].leader, 1);
        assert_eq!(packet.members[1].job_id, JobId(2));
        assert_eq!(packet.members[1].base_level, 45);
    }

    #[test]
    fn party_update_packets_match_20220406_layouts() {
        let position = read_packet::<PartyMemberPositionPacket>(&[0x07, 0x01, 0x04, 0x03, 0x02, 0x01, 0x78, 0x00, 0x8C, 0x00]);
        assert_eq!(position.account_id, AccountId(0x0102_0304));
        assert_eq!(position.position, TilePosition { x: 120, y: 140 });

        let health =
            read_packet::<PartyMemberHealthPacket>(&[0x0E, 0x08, 0x04, 0x03, 0x02, 0x01, 0xE8, 0x03, 0x00, 0x00, 0xD0, 0x07, 0x00, 0x00]);
        assert_eq!(health.account_id, AccountId(0x0102_0304));
        assert_eq!(health.health_points, 1000);
        assert_eq!(health.maximum_health_points, 2000);

        let job = read_packet::<PartyMemberJobAndLevelPacket>(&[0xBD, 0x0A, 0x04, 0x03, 0x02, 0x01, 0xA1, 0x0F, 0x63, 0x00]);
        assert_eq!(job.account_id, AccountId(0x0102_0304));
        assert_eq!(job.job_id, JobId(4001));
        assert_eq!(job.base_level, 99);
    }

    #[test]
    fn party_chat_and_whisper_packets_match_20220406_layouts() {
        let party_chat = read_packet::<NotifyPartyChatMessagePacket>(&[
            0x09, 0x01, 0x15, 0x00, 0x04, 0x03, 0x02, 0x01, b'A', b'l', b'i', b'c', b'e', b' ', b':', b' ', b'h', b'e', b'l', b'l', b'o',
        ]);
        assert_eq!(party_chat.account_id, AccountId(0x0102_0304));
        assert_eq!(party_chat.message, "Alice : hello");

        let mut whisper_bytes = vec![0xDE, 0x09, 0x27, 0x00];
        whisper_bytes.extend([0x08, 0x07, 0x06, 0x05]);
        whisper_bytes.extend(fixed_string("Alice", 24));
        whisper_bytes.push(0x00);
        whisper_bytes.extend(b"secret");
        let whisper = read_packet::<WhisperMessagePacket>(&whisper_bytes);
        assert_eq!(whisper.sender_character_id, CharacterId(0x0506_0708));
        assert_eq!(whisper.sender_name, "Alice");
        assert_eq!(whisper.is_admin, 0);
        assert_eq!(whisper.message, "secret");

        let whisper_result = read_packet::<WhisperResult2Packet>(&[0xDF, 0x09, 0x00, 0x08, 0x07, 0x06, 0x05]);
        assert_eq!(whisper_result.result, 0);
        assert_eq!(whisper_result.character_id, CharacterId(0x0506_0708));

        assert_eq!(packet_bytes(WhisperSendPacket::new("Bob".to_owned(), "hello".to_owned())), [
            0x96, 0x00, 0x22, 0x00, b'B', b'o', b'b', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, b'h', b'e', b'l', b'l', b'o', 0x00,
        ]);
    }

    #[test]
    fn outgoing_party_management_packets_match_20220406_layouts() {
        assert_eq!(packet_bytes(CreatePartyPacket::new("Seal Party".to_owned(), 0, 0)), [
            0xE8, 0x01, b'S', b'e', b'a', b'l', b' ', b'P', b'a', b'r', b't', b'y', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        assert_eq!(packet_bytes(PartyInviteRequestPacket::new("Bob".to_owned())), [
            0xC4, 0x02, b'B', b'o', b'b', 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        assert_eq!(packet_bytes(PartyInviteResponsePacket::new(PartyId(0x0102_0304), 1)), [
            0xC7, 0x02, 0x04, 0x03, 0x02, 0x01, 0x01,
        ]);
        assert_eq!(packet_bytes(LeavePartyPacket::new()), [0x00, 0x01]);
        assert_eq!(packet_bytes(SetPartyInvitationStatePacket::new(1)), [0xC8, 0x02, 0x01]);
    }

    #[test]
    fn equipment_effect_packet_consumes_variable_length_header() {
        let packet = read_packet::<EquipmentEffectPacket>(&[0x3B, 0x0A, 0x09, 0x00, 0x84, 0x84, 0x1E, 0x00, 0x01]);

        assert_eq!(packet.entity_id, EntityId(0x001E_8484));
        assert_eq!(packet.status, 1);
        assert!(packet.effects.is_empty());
    }

    #[test]
    fn personal_information_packet_consumes_modifier_rows() {
        let packet = read_packet::<PersonalInformationPacket>(&[
            0x7B, 0x09, 0x44, 0x00, 0xA0, 0x86, 0x01, 0x00, 0xE8, 0x03, 0x00, 0x00, 0xA0, 0x86, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xA0, 0x86, 0x01, 0x00, 0xE8, 0x03, 0x00,
            0x00, 0xA0, 0x86, 0x01, 0x00,
        ]);

        assert_eq!(packet.total_exp, 100_000);
        assert_eq!(packet.total_death, 1_000);
        assert_eq!(packet.total_drop, 100_000);
        assert_eq!(packet.details.len(), 4);
        assert_eq!(packet.details[3].information_type, 2);
    }

    #[test]
    fn quest_notification_packet4_matches_20220406_layout() {
        // 0x0B0C is questAddType / ZC_ADD_QUEST_EX on 20220406: a fixed 155-byte
        // frame carrying a header plus room for MAX_QUEST_OBJECTIVES (3) objectives.
        let mut bytes = vec![0x0C, 0x0B];
        bytes.extend([0x64, 0x00, 0x00, 0x00]); // quest_id = 100
        bytes.push(0x01); // active
        bytes.extend([0x00, 0x00, 0x00, 0x00]); // start_time
        bytes.extend([0x00, 0x00, 0x00, 0x00]); // expire_time
        bytes.extend([0x01, 0x00]); // objective_count = 1

        // Objective 0: Poring hunt.
        bytes.extend([0x64, 0x00, 0x00, 0x00]); // hunt_identification = 100
        bytes.extend([0x00, 0x00, 0x00, 0x00]); // hunt_identification2 = 0
        bytes.extend([0x00, 0x00, 0x00, 0x00]); // objective_type = 0
        bytes.extend([0xEA, 0x03, 0x00, 0x00]); // mob_id = 1002
        bytes.extend([0x00, 0x00]); // minimum_level
        bytes.extend([0x00, 0x00]); // maximum_level
        bytes.extend([0x05, 0x00]); // mob_count = 5
        let mut mob_name = b"Poring".to_vec();
        mob_name.resize(24, 0x00);
        bytes.extend(mob_name);

        // Objectives 1 and 2 are zero-filled but still present (fixed-size frame).
        bytes.extend([0x00; 46 * 2]);

        assert_eq!(bytes.len(), 155);

        let packet = read_packet::<QuestNotificationPacket4>(&bytes);

        assert_eq!(packet.quest_id, 100);
        assert_eq!(packet.active, 1);
        assert_eq!(packet.objective_count, 1);
        assert_eq!(packet.objective_details[0].hunt_identification, 100);
        assert_eq!(packet.objective_details[0].mob_id, 1002);
        assert_eq!(packet.objective_details[0].mob_count, 5);
        assert_eq!(packet.objective_details[0].mob_name, "Poring");
    }

    #[test]
    fn quest_list_packet4_matches_20220406_live_capture() {
        // Mirrors the 2026-07-07 live Hercules capture of 0x0AFF (ZC_ALL_QUEST_LIST4):
        // a 23-byte frame with quest_count = 1 and a single quest carrying zero
        // objectives. Header(2) + length(2) + count(4) + one 15-byte quest = 23.
        let mut bytes = vec![0xFF, 0x0A]; // header
        bytes.extend([0x17, 0x00]); // packet length = 23 (consumed by #[variable_length])
        bytes.extend([0x01, 0x00, 0x00, 0x00]); // quest_count = 1

        // Quest 0 with no objectives.
        bytes.extend([0x64, 0x00, 0x00, 0x00]); // quest_id = 100
        bytes.push(0x01); // active
        bytes.extend([0x00, 0x00, 0x00, 0x00]); // remaining_time
        bytes.extend([0x00, 0x00, 0x00, 0x00]); // expire_time
        bytes.extend([0x00, 0x00]); // objective_count = 0

        assert_eq!(bytes.len(), 23);

        let packet = read_packet::<QuestListPacket4>(&bytes);

        assert_eq!(packet.quest_count, 1);
        assert_eq!(packet.quests.len(), 1);
        assert_eq!(packet.quests[0].quest_id, 100);
        assert_eq!(packet.quests[0].active, 1);
        assert_eq!(packet.quests[0].objective_count, 0);
        assert!(packet.quests[0].objective_details.is_empty());
    }

    #[test]
    fn quest_removed_packet_matches_20220406_layout() {
        // 0x02B4 ZC_DEL_QUEST: header + quest_id. Drives NetworkEvent::QuestRemoved
        // (exercised live by the headless dm-quest-lifecycle scenario).
        let bytes = [0xB4, 0x02, 0x21, 0x4E, 0x00, 0x00]; // quest_id = 20001
        let packet = read_packet::<QuestRemovedPacket>(&bytes);
        assert_eq!(packet.quest_id, 20001);
    }

    #[test]
    fn notify_actor_init_packet_matches_20220406_layout() {
        read_packet::<NotifyActorInitPacket>(&[0x1B, 0x0B]);
    }

    #[test]
    fn use_skill_at_id_packet_matches_20220406_opcode() {
        assert_eq!(
            packet_bytes(UseSkillAtIdPacket::new(SkillLevel(5), SkillId(0x1234), EntityId(0x0102_0304))),
            [0x38, 0x04, 0x05, 0x00, 0x34, 0x12, 0x04, 0x03, 0x02, 0x01]
        );
    }

    #[test]
    fn use_skill_ack_packet_matches_20220406_layout() {
        let packet = read_packet::<UseSkillAckPacket>(&[
            0x1A, 0x0B, 0x6D, 0xC4, 0x8E, 0x06, 0x84, 0x84, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0xB8, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        assert_eq!(packet.source_entity, EntityId(0x068E_C46D));
        assert_eq!(packet.destination_entity, EntityId(0x001E_8484));
        assert_eq!(packet.position, TilePosition { x: 0, y: 0 });
        assert_eq!(packet.skill_id, SkillId(0x00B8));
        assert_eq!(packet.element, 1);
        assert_eq!(packet.delay_time, 0);
        assert_eq!(packet.disposable, 0);
        assert_eq!(packet.attack_motion_time, 0);
    }

    /// `ZC_NOTIFY_HP_TO_GROUPM` in the wide 22-byte form our Hercules delta
    /// selects (`KORANGAR_PARTY_SP_TO_GROUPM`). The length must stay 22 to match
    /// `packetLen(0x0bab, 22)` in Hercules' own main table and the generated
    /// `lengths_20220406.rs` — a mismatch would desync the read buffer rather
    /// than fail loudly.
    #[test]
    fn party_member_vitals_packet_matches_wide_layout() {
        let bytes = [
            0xAB, 0x0B, // header
            0x81, 0x84, 0x1E, 0x00, // account id 2000001
            0x2C, 0x01, 0x00, 0x00, // hp 300
            0xE8, 0x03, 0x00, 0x00, // max hp 1000
            0x2D, 0x00, 0x00, 0x00, // sp 45
            0xC8, 0x00, 0x00, 0x00, // max sp 200
        ];
        assert_eq!(bytes.len(), 22);

        let packet = read_packet::<PartyMemberVitalsPacket>(&bytes);

        assert_eq!(packet.account_id, AccountId(2000001));
        assert_eq!(packet.health_points, 300);
        assert_eq!(packet.maximum_health_points, 1000);
        assert_eq!(packet.spell_points, 45);
        assert_eq!(packet.maximum_spell_points, 200);
    }

    /// The fork invite-sender packet must stay 30 bytes to match
    /// `packetLen(0x0eff, 30)` in Hercules' hand-maintained `packets_len.h`.
    #[test]
    fn party_invite_sender_packet_matches_fork_layout() {
        let mut bytes = vec![0xFF, 0x0E, 0x07, 0x00, 0x00, 0x00];
        let mut name = b"test".to_vec();
        name.resize(24, 0);
        bytes.extend_from_slice(&name);
        assert_eq!(bytes.len(), 30);

        let packet = read_packet::<PartyInviteSenderPacket>(&bytes);

        assert_eq!(packet.party_id, PartyId(7));
        assert_eq!(packet.character_name, "test");
    }

    #[test]
    fn npc_number_input_packets_match_hercules_layout() {
        // ZC_OPEN_EDITDLG 0x0142: header + npc_id
        assert_eq!(packet_bytes(NpcOpenNumberInputPacket::new(EntityId(0x0102_0304))), [
            0x42, 0x01, 0x04, 0x03, 0x02, 0x01
        ]);

        // CZ_INPUT_EDITDLG 0x0143: header + npc_id + value
        assert_eq!(packet_bytes(NpcNumberInputPacket::new(EntityId(0x0102_0304), 42)), [
            0x43, 0x01, 0x04, 0x03, 0x02, 0x01, 0x2A, 0x00, 0x00, 0x00
        ]);
    }

    #[test]
    fn npc_string_input_packets_match_hercules_layout() {
        // ZC_OPEN_EDITDLGSTR 0x01D4: header + npc_id
        assert_eq!(packet_bytes(NpcOpenStringInputPacket::new(EntityId(0x0102_0304))), [
            0xD4, 0x01, 0x04, 0x03, 0x02, 0x01
        ]);

        // CZ_INPUT_EDITDLGSTR 0x01D5: header + length + npc_id + null-terminated text
        // "Hi" = 2 chars + null → payload after header: len(2) + id(4) + text(3) = 9,
        // total 11
        let bytes = packet_bytes(NpcStringInputPacket::new(EntityId(0x0102_0304), "Hi".to_owned()));
        assert_eq!(&bytes[0..2], &[0xD5, 0x01]);
        let len = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
        assert_eq!(len, bytes.len());
        assert_eq!(&bytes[4..8], &[0x04, 0x03, 0x02, 0x01]);
        assert_eq!(&bytes[8..], b"Hi\0");
    }

    #[test]
    fn friend_list_packet_matches_20220406_layout() {
        // Friend list packet 0x0201 is variable length.
        // It carries a header, a length field (2 bytes), and repeating Friend rows.
        let mut bytes = vec![0x01, 0x02]; // header 0x0201
        bytes.extend([0x44, 0x00]); // total length: 4 (header+len) + 2 * 32 (Friend rows) = 68 bytes = 0x0044

        // Friend 1
        bytes.extend([0x04, 0x03, 0x02, 0x01]); // AID
        bytes.extend([0x08, 0x07, 0x06, 0x05]); // GID
        bytes.extend(fixed_string("FriendOne", 24)); // name

        // Friend 2
        bytes.extend([0x14, 0x13, 0x12, 0x11]); // AID
        bytes.extend([0x18, 0x17, 0x16, 0x15]); // GID
        bytes.extend(fixed_string("FriendTwo", 24)); // name

        assert_eq!(bytes.len(), 68);

        let packet = read_packet::<FriendListPacket>(&bytes);
        assert_eq!(packet.friend_list.len(), 2);

        assert_eq!(packet.friend_list[0].account_id, AccountId(0x0102_0304));
        assert_eq!(packet.friend_list[0].character_id, CharacterId(0x0506_0708));
        assert_eq!(packet.friend_list[0].name, "FriendOne");

        assert_eq!(packet.friend_list[1].account_id, AccountId(0x1112_1314));
        assert_eq!(packet.friend_list[1].character_id, CharacterId(0x1516_1718));
        assert_eq!(packet.friend_list[1].name, "FriendTwo");
    }

    #[test]
    fn add_friend_packet_matches_20220406_layout() {
        // CZ_ADD_FRIEND 0x0202: header + name (24 bytes)
        assert_eq!(
            packet_bytes(AddFriendPacket { name: "Alice".to_owned() }),
            [&[0x02, 0x02][..], &fixed_string("Alice", 24)[..]].concat()
        );
    }

    #[test]
    fn remove_friend_packet_matches_20220406_layout() {
        // CZ_REMOVE_FRIEND 0x0203: header + account_id (4 bytes) + character_id (4
        // bytes)
        assert_eq!(
            packet_bytes(RemoveFriendPacket {
                account_id: AccountId(0x0102_0304),
                character_id: CharacterId(0x0506_0708),
            }),
            [0x03, 0x02, 0x04, 0x03, 0x02, 0x01, 0x08, 0x07, 0x06, 0x05]
        );
    }

    #[test]
    fn headless_fallback_packets_match_hercules_20220406_layouts() {
        let slide = read_packet::<EntitySlidePacket>(&[0xFF, 0x01, 0x04, 0x03, 0x02, 0x01, 120, 0, 140, 0]);
        assert_eq!(slide.entity_id, EntityId(0x0102_0304));
        assert_eq!(slide.position, TilePosition { x: 120, y: 140 });

        let mut monster = vec![0x8C, 0x01];
        monster.extend([0xEA, 0x03, 10, 0, 1, 0]); // Poring, level 10, medium
        monster.extend(500u32.to_le_bytes());
        monster.extend([20, 0, 3, 0, 15, 0, 2, 0]);
        monster.extend([100, 100, 100, 100, 100, 100, 100, 100, 100]);
        let monster = read_packet::<MonsterInformationPacket>(&monster);
        assert_eq!(monster.job_id, JobId(1002));
        assert_eq!(monster.health_points, 500);

        let mut warps = vec![0xBE, 0x0A, 0x16, 0x00, 0x1A, 0x00];
        warps.extend(fixed_string("payon.gat", 16));
        let warps = read_packet::<WarpListPacket>(&warps);
        assert_eq!(warps.skill_id, SkillId(26));
        assert_eq!(warps.destinations[0].map_name, fixed_string("payon.gat", 16).as_slice());

        let cooldowns = read_packet::<SkillCooldownListPacket>(&[0x85, 0x09, 0x0E, 0x00, 89, 0, 0xE8, 0x03, 0, 0, 0xF4, 0x01, 0, 0]);
        assert_eq!(cooldowns.cooldowns[0].skill_id, SkillId(89));
        assert_eq!(cooldowns.cooldowns[0].remaining_ms, 500);

        let status = read_packet::<StatusChange2Packet>(&[
            0x3F, 0x04, 10, 0, 0x04, 0x03, 0x02, 0x01, 1, 0xE8, 0x03, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0,
        ]);
        assert_eq!(status.entity_id, EntityId(0x0102_0304));
        assert_eq!(status.remaining_in_milliseconds, 1000);

        let mut weapons = vec![0x21, 0x02, 0x1B, 0x00];
        weapons.extend([2, 0]); // server inventory index
        weapons.extend(1101u32.to_le_bytes());
        weapons.push(7);
        for card in [4001u32, 4002, 4003, 4004] {
            weapons.extend(card.to_le_bytes());
        }
        let weapons = read_packet::<RefinableWeaponListPacket>(&weapons);
        assert_eq!(weapons.weapons[0].item_id, ItemId(1101));
        assert_eq!(weapons.weapons[0].refinement_level, 7);
    }

    #[test]
    fn warp_and_weapon_refine_selection_packets_match_hercules_layouts() {
        assert_eq!(
            packet_bytes(SelectWarpDestinationPacket::new(SkillId(26), "payon.gat".to_owned())),
            [&[0x1B, 0x01, 0x1A, 0x00][..], &fixed_string("payon.gat", 16)[..]].concat()
        );
        assert_eq!(packet_bytes(RequestWeaponRefinePacket::new(7)), [
            0x22, 0x02, 0x07, 0x00, 0x00, 0x00
        ]);

        let result = read_packet::<WeaponRefineResultPacket>(&[0x23, 0x02, 0, 0, 0, 0, 0x4D, 0x04, 0, 0]);
        assert_eq!(result.result, 0);
        assert_eq!(result.item_id, ItemId(1101));
    }

    #[test]
    fn repair_weapon_packets_match_hercules_20220406_layouts() {
        let mut list = vec![0x65, 0x0B, 0x1C, 0x00, 7, 0];
        list.extend(1101u32.to_le_bytes());
        for card in [4001u32, 4002, 4003, 4004] {
            list.extend(card.to_le_bytes());
        }
        list.extend([3, 2]);

        let list = read_packet::<RepairableItemListPacket>(&list);
        let item = list.items[0].clone();
        assert_eq!(item.inventory_index, RawIndex(7));
        assert_eq!(item.item_id, ItemId(1101));
        assert_eq!(item.cards, [ItemId(4001), ItemId(4002), ItemId(4003), ItemId(4004)]);
        assert_eq!(item.refinement_level, 3);
        assert_eq!(item.grade, 2);

        assert_eq!(packet_bytes(RequestItemRepairPacket::new(item)), [
            0x66, 0x0B, 7, 0, 0x4D, 0x04, 0, 0, 0xA1, 0x0F, 0, 0, 0xA2, 0x0F, 0, 0, 0xA3, 0x0F, 0, 0, 0xA4, 0x0F, 0, 0, 3, 2,
        ]);

        let result = read_packet::<ItemRepairResultPacket>(&[0xFE, 0x01, 9, 0, 0]);
        assert_eq!(result.inventory_index, InventoryIndex(7));
        assert_eq!(result.result, 0);
    }
}

#[cfg(test)]
mod shop_item_list_tests {
    use ragnarok_bytes::{ByteReader, FixedByteSize};

    use super::*;

    #[test]
    fn shop_item_information_size_is_19() {
        assert_eq!(ShopItemInformation::size_in_bytes(), 19);
    }

    // Regression tests for M1-007: `StateChangePacket` was `register_noop`, so hide
    // / cloak never reached the UI. Values mirror Hercules `src/common/mmo.h`
    // `OPTION_*` and the `pc_ishiding` / `pc_isinvisible` masks in
    // `src/map/pc.h`.

    // M1-012: statuses start on 0x0983 but *end* on 0x0196
    // (`clif_status_change_end` → `status_change_endType`). 0x0196 was noop, so the
    // server could never clear a buff and cancelling one early left its timer on
    // screen.
    #[test]
    fn status_change_sequence_0x196_matches_hercules_end_layout() {
        // Hercules `packet_status_change_end`: PacketType.W index.W AID.L state.B = 9
        // bytes.
        let bytes = [
            0x96, 0x01, // header 0x0196
            0x04, 0x00, // index 4 (SI_HIDING)
            0x80, 0x84, 0x1E, 0x00, // AID 2000000
            0x00, // state 0 = ended
        ];
        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = StatusChangeSequencePacket::packet_from_bytes(&mut reader).expect("parse");

        assert_eq!(packet.index, 4);
        assert_eq!(packet.id, 2000000);
        assert_eq!(packet.state, 0, "state 0 is what marks this an end");
    }

    #[test]
    fn entity_option_values_match_hercules() {
        assert_eq!(EntityOption::HIDE.bits(), 0x0000_0002);
        assert_eq!(EntityOption::CLOAK.bits(), 0x0000_0004);
        assert_eq!(EntityOption::INVISIBLE.bits(), 0x0000_0040);
        assert_eq!(EntityOption::CHASEWALK.bits(), 0x0000_4000);
    }

    #[test]
    fn entity_option_is_concealed_matches_pc_ishiding() {
        // pc_ishiding = option & (OPTION_HIDE|OPTION_CLOAK|OPTION_CHASEWALK)
        assert!(EntityOption::from_raw(0x02).is_concealed(), "Hiding");
        assert!(EntityOption::from_raw(0x04).is_concealed(), "Cloaking");
        assert!(EntityOption::from_raw(0x4000).is_concealed(), "Chase Walk");
        assert!(!EntityOption::from_raw(0).is_concealed(), "no options");
        // GM invisibility is NOT part of the pc_ishiding mask.
        assert!(!EntityOption::from_raw(0x40).is_concealed(), "GM invisible");
        assert!(EntityOption::from_raw(0x40).is_gm_invisible());
    }

    #[test]
    fn entity_option_ignores_unmodelled_bits() {
        // A newer server may set flags we don't model (e.g. OPTION_XMAS 0x10000);
        // truncating must not panic and must not disturb the flags we do model.
        let option = EntityOption::from_raw(0x0001_0002);
        assert!(option.is_concealed(), "HIDE survives alongside unknown bits");
        assert!(!option.contains(EntityOption::CLOAK));
    }

    #[test]
    fn parse_state_change_0x229_carries_all_atomic_actor_fields() {
        // header 0x0229, AID 2000000, bodyState 1 (stone), healthState 4,
        // effectState OPTION_HIDE, isPKModeON 1.
        let bytes = [
            0x29, 0x02, // header
            0x80, 0x84, 0x1E, 0x00, // entity id 2000000
            0x01, 0x00, // body_state
            0x04, 0x00, // health_state
            0x02, 0x00, 0x00, 0x00, // effect_state = OPTION_HIDE
            0x01, // is_pk_mode_on
        ];
        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = StateChangePacket::packet_from_bytes(&mut reader).expect("parse");
        assert_eq!(packet.entity_id.0, 2000000);
        assert_eq!(packet.body_state, 1);
        assert_eq!(packet.health_state, 4);
        assert_eq!(packet.effect_state, 0x02);
        assert_eq!(packet.is_pk_mode_on, 1);
        assert!(EntityOption::from_raw(packet.effect_state).is_concealed());
    }

    #[test]
    fn parse_skill_cast_cancelled_0x1b9_carries_actor_id() {
        let bytes = [
            0xB9, 0x01, // header 0x01B9 (ZC_DISPEL)
            0x80, 0x84, 0x1E, 0x00, // entity id 2000000
        ];
        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = SkillCastCancelledPacket::packet_from_bytes(&mut reader).expect("parse");
        assert_eq!(packet.entity_id.0, 2000000);
    }

    #[test]
    fn parse_shop_item_list_0xb77_one_item() {
        // header 0x0b77, len 23 (4 + 19), one red potion style entry
        let bytes = [
            0x77, 0x0B, 0x17, 0x00, 0xF5, 0x01, 0x00, 0x00, // item 501
            0x32, 0x00, 0x00, 0x00, // price 50
            0x32, 0x00, 0x00, 0x00, // discount 50
            0x00, // type
            0x00, 0x00, // view
            0x00, 0x00, 0x00, 0x00, // location
        ];
        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = ShopItemListPacket::packet_from_bytes(&mut reader).expect("parse");
        assert_eq!(packet.items.len(), 1);
        assert_eq!(packet.items[0].item_id.0, 501);
        assert_eq!(packet.items[0].price.0, 50);
    }
}
