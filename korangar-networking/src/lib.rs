#![cfg_attr(feature = "interface", feature(impl_trait_in_assoc_type))]
#![cfg_attr(feature = "interface", feature(negative_impls))]

mod entity;
mod event;
mod hotkey;
mod items;
mod message;
mod packet_versions;
mod server;

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use event::{
    CharacterServerDisconnectedEvent, DisconnectedEvent, LoginServerDisconnectedEvent, MapServerDisconnectedEvent, NetworkEventList,
};
use ragnarok_bytes::encoding::UTF_8;
use ragnarok_bytes::{ByteReader, ByteWriter, FromBytes};
use ragnarok_packets::handler::{DuplicateHandlerError, HandlerResult, NoPacketCallback, PacketCallback, PacketHandler};
use ragnarok_packets::*;
use server::{ServerConnectCommand, ServerConnection};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

pub use self::entity::EntityData;
pub use self::event::{DisconnectReason, NetworkEvent};
pub use self::hotkey::HotkeyState;
pub use self::items::{InventoryItem, InventoryItemDetails, ItemQuantity, NoMetadata, SellItem, ShopItem};
pub use self::message::MessageColor;
pub use self::packet_versions::SupportedPacketVersion;
pub use self::server::{
    CharacterServerLoginData, LoginServerLoginData, NotConnectedError, UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
};

const fn weapon_refine_wire_index(inventory_index: InventoryIndex) -> u32 {
    inventory_index.0 as u32 + 2
}
use crate::server::NetworkTaskError;

/// Buffer for networking events. This struct exists to reduce heap allocations
/// and is purely an optimization.
pub struct NetworkEventBuffer(Vec<NetworkEvent>);

impl NetworkEventBuffer {
    pub fn drain(&mut self) -> std::vec::Drain<'_, NetworkEvent> {
        self.0.drain(..)
    }
}

/// Simple time synchronization using the Cristian's algorithm.
struct TimeSynchronization {
    request_send: Instant,
    request_received: Instant,
    client_tick: f64,
}

impl TimeSynchronization {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            request_send: now,
            request_received: now,
            client_tick: 100.0,
        }
    }

    /// Returns the client tick that must be used when sending the time
    /// synchronization request immediately after calling this function.
    fn request_client_tick(&mut self) -> u32 {
        let request_send = Instant::now();
        let elapsed = request_send.duration_since(self.request_received).as_secs_f64();
        (self.client_tick + (elapsed * 1000.0)) as u32
    }

    /// Returns the estimated client tick using the Cristian's algorithm.
    fn estimated_client_tick(&mut self, server_tick: u32, request_received: Instant) -> u32 {
        self.request_received = request_received;
        let round_trip_time = self.request_received.duration_since(self.request_send).as_secs_f64();
        let tick_adjustment = (round_trip_time / 2.0) * 1000.0;
        self.client_tick = f64::from(server_tick) + tick_adjustment;
        self.client_tick as u32
    }
}

#[cfg(feature = "debug")]
fn log_packet_bytes(label: &str, bytes: &[u8]) {
    if std::env::var_os("KORANGAR_PACKET_LOG").is_some() {
        let hex: String = bytes.iter().map(|byte| format!("{byte:02x} ")).collect();
        eprintln!("[packet-log] {label} {} bytes: {hex}", bytes.len());
    }
}

pub struct NetworkingSystem<Callback> {
    command_sender: UnboundedSender<ServerConnectCommand>,
    time_synchronization: Arc<Mutex<TimeSynchronization>>,
    login_server_connection: ServerConnection,
    character_server_connection: ServerConnection,
    map_server_connection: ServerConnection,
    packet_callback: Callback,
}

impl NetworkingSystem<NoPacketCallback> {
    pub fn spawn() -> (Self, NetworkEventBuffer) {
        let (command_sender, time_synchronization) = Self::spawn_networking_thread(NoPacketCallback);
        Self::inner_new(command_sender, time_synchronization, NoPacketCallback)
    }
}

impl<Callback> NetworkingSystem<Callback>
where
    Callback: PacketCallback + Send,
{
    fn inner_new(
        command_sender: UnboundedSender<ServerConnectCommand>,
        time_synchronization: Arc<Mutex<TimeSynchronization>>,
        packet_callback: Callback,
    ) -> (Self, NetworkEventBuffer) {
        let networking_system = Self {
            command_sender,
            time_synchronization,
            login_server_connection: ServerConnection::Disconnected,
            character_server_connection: ServerConnection::Disconnected,
            map_server_connection: ServerConnection::Disconnected,
            packet_callback,
        };
        let event_buffer = NetworkEventBuffer(Vec::new());

        (networking_system, event_buffer)
    }

    pub fn spawn_with_callback(packet_callback: Callback) -> (Self, NetworkEventBuffer) {
        let (command_sender, time_synchronization) = Self::spawn_networking_thread(packet_callback.clone());
        Self::inner_new(command_sender, time_synchronization, packet_callback)
    }

    fn spawn_networking_thread(packet_callback: Callback) -> (UnboundedSender<ServerConnectCommand>, Arc<Mutex<TimeSynchronization>>) {
        let (command_sender, mut command_receiver) = tokio::sync::mpsc::unbounded_channel::<ServerConnectCommand>();
        let time_synchronization = Arc::new(Mutex::new(TimeSynchronization::new()));
        let thread_time_synchronization = Arc::clone(&time_synchronization);

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

            let _guard = runtime.enter();
            let local_set = tokio::task::LocalSet::new();

            let mut login_server_task_handle: Option<JoinHandle<Result<(), NetworkTaskError>>> = None;
            let mut character_server_task_handle: Option<JoinHandle<Result<(), NetworkTaskError>>> = None;
            let mut map_server_task_handle: Option<JoinHandle<Result<(), NetworkTaskError>>> = None;

            local_set.block_on(&runtime, async {
                while let Some(command) = command_receiver.recv().await {
                    match command {
                        ServerConnectCommand::Login {
                            address,
                            action_receiver,
                            event_sender,
                            packet_version,
                        } => {
                            if let Some(handle) = login_server_task_handle.take() {
                                // Abort instead of awaiting: a stale task that missed its
                                // shutdown signal would otherwise block this loop forever,
                                // freezing all networking (observed with a lingering login
                                // connection after a rejected login).
                                handle.abort();
                                let _ = handle.await;
                            }

                            let packet_handler = Self::create_login_server_packet_handler(packet_callback.clone(), packet_version).unwrap();
                            let handle = local_set.spawn_local(Self::handle_server_connection(
                                address,
                                action_receiver,
                                event_sender,
                                packet_handler,
                                |_| LoginServerKeepalivePacket::new(),
                                Duration::from_secs(58),
                                false,
                                thread_time_synchronization.clone(),
                            ));

                            login_server_task_handle = Some(handle);
                        }
                        ServerConnectCommand::Character {
                            address,
                            action_receiver,
                            event_sender,
                            packet_version,
                        } => {
                            if let Some(handle) = character_server_task_handle.take() {
                                // See the login handler above: never block on a stale task.
                                handle.abort();
                                let _ = handle.await;
                            }

                            let packet_handler =
                                Self::create_character_server_packet_handler(packet_callback.clone(), packet_version).unwrap();
                            let handle = local_set.spawn_local(Self::handle_server_connection(
                                address,
                                action_receiver,
                                event_sender,
                                packet_handler,
                                |_| CharacterServerKeepalivePacket::new(),
                                Duration::from_secs(10),
                                true,
                                thread_time_synchronization.clone(),
                            ));

                            character_server_task_handle = Some(handle);
                        }
                        ServerConnectCommand::Map {
                            address,
                            action_receiver,
                            event_sender,
                            packet_version,
                        } => {
                            if let Some(handle) = map_server_task_handle.take() {
                                // See the login handler above: never block on a stale task.
                                handle.abort();
                                let _ = handle.await;
                            }

                            let packet_handler = Self::create_map_server_packet_handler(packet_callback.clone(), packet_version).unwrap();
                            let handle = local_set.spawn_local(Self::handle_server_connection(
                                address,
                                action_receiver,
                                event_sender,
                                packet_handler,
                                |time_synchronization| match time_synchronization.lock() {
                                    Ok(mut time_synchronization) => {
                                        let client_tick = time_synchronization.request_client_tick();
                                        RequestServerTickPacket::new(ClientTick(client_tick))
                                    }
                                    Err(_) => RequestServerTickPacket::new(ClientTick(100)),
                                },
                                Duration::from_secs(10),
                                false,
                                thread_time_synchronization.clone(),
                            ));

                            map_server_task_handle = Some(handle);
                        }
                    }
                }
            });
        });

        (command_sender, time_synchronization)
    }

    fn handle_connection<Event>(connection: &mut ServerConnection, event_buffer: &mut NetworkEventBuffer)
    where
        Event: DisconnectedEvent,
    {
        match connection.take() {
            ServerConnection::Connected {
                action_sender,
                mut event_receiver,
                packet_version,
            } => loop {
                match event_receiver.try_recv() {
                    Ok(login_event) => {
                        event_buffer.0.push(login_event);
                    }
                    Err(TryRecvError::Empty) => {
                        *connection = ServerConnection::Connected {
                            action_sender,
                            event_receiver,
                            packet_version,
                        };
                        break;
                    }
                    Err(..) => {
                        event_buffer.0.push(Event::create_event(DisconnectReason::ConnectionError));
                        *connection = ServerConnection::Disconnected;
                        break;
                    }
                }
            },
            ServerConnection::ClosingManually => {
                event_buffer.0.push(Event::create_event(DisconnectReason::ClosedByClient));
                *connection = ServerConnection::Disconnected;
            }
            _ => (),
        };
    }

    pub fn get_events(&mut self, events: &mut NetworkEventBuffer) {
        Self::handle_connection::<LoginServerDisconnectedEvent>(&mut self.login_server_connection, events);
        Self::handle_connection::<CharacterServerDisconnectedEvent>(&mut self.character_server_connection, events);
        Self::handle_connection::<MapServerDisconnectedEvent>(&mut self.map_server_connection, events);
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_server_connection<PingPacket>(
        address: SocketAddr,
        mut action_receiver: UnboundedReceiver<Vec<u8>>,
        event_sender: UnboundedSender<NetworkEvent>,
        mut packet_handler: PacketHandler<NetworkEventList, Callback>,
        ping_factory: impl Fn(&Mutex<TimeSynchronization>) -> PingPacket,
        ping_frequency: Duration,
        // After logging in to the character server, it sends the account id without any packet.
        // Since our packet handler has no way of working with this, we need to add some special
        // logic.
        mut read_account_id: bool,
        time_synchronization: Arc<Mutex<TimeSynchronization>>,
    ) -> Result<(), NetworkTaskError>
    where
        PingPacket: Packet + ClientPacket,
        Callback: PacketCallback,
    {
        let mut stream = TcpStream::connect(address).await.map_err(|_| NetworkTaskError::FailedToConnect)?;
        let mut interval = tokio::time::interval(ping_frequency);
        let mut buffer = [0u8; 8192];
        let mut cut_off_buffer_base = 0;
        let mut events = Vec::new();
        let mut byte_writer = ByteWriter::with_encoding(UTF_8);

        loop {
            tokio::select! {
                // Send a packet to the server.
                action = action_receiver.recv() => {
                    let Some(action) = action else {
                        // Channel was closed by the main thread.
                        break Ok(());
                    };

                    stream.write_all(&action).await.map_err(|_| NetworkTaskError::ConnectionClosed)?;
                }
                // Receive some packets from the server.
                received_bytes = stream.read(&mut buffer[cut_off_buffer_base..]) => {
                    let Ok(received_bytes) = received_bytes else {
                        // Channel was closed by the main thread.
                        break Err(NetworkTaskError::ConnectionClosed);
                    };

                    if received_bytes == 0 {
                        // Receiving Ok(0) means the stream was closed by the server, most
                        // likely because the client sent an incorrect packet.
                        break Err(NetworkTaskError::ConnectionClosed);
                    }

                    let data = &buffer[..cut_off_buffer_base + received_bytes];
                    let mut byte_reader = ByteReader::without_metadata(data);
                    byte_reader.set_encoding(UTF_8);

                    if read_account_id {
                        let account_id = AccountId::from_bytes(&mut byte_reader).unwrap();
                        events.push(NetworkEvent::AccountId { account_id });
                        read_account_id = false;
                    }

                    while !byte_reader.is_empty() {
                        match packet_handler.process_one(&mut byte_reader) {
                            HandlerResult::Ok(packet_events) => events.extend(packet_events.0.into_iter()),
                            HandlerResult::PacketCutOff => {
                                let packet_start = byte_reader.get_offset();
                                let packet_end = cut_off_buffer_base + received_bytes;

                                if packet_start == 0 {
                                    // If the packet_start is 0, that means the packet is allegidly bigger than the MTU of a TCP packet.
                                    // We limit the size of a packet to the MTU, to avoid getting stuck on packets that are parsed incorrectly.
                                    // TODO: Call the packet callback?
                                    cut_off_buffer_base = 0;
                                    break;
                                }

                                buffer.copy_within(packet_start..packet_end, 0);
                                cut_off_buffer_base = packet_end - packet_start;

                                break;
                            },
                            // The packet callback can take care of handling these properly.
                            HandlerResult::UnhandledPacket => {
                                cut_off_buffer_base = 0;
                                break
                            },
                            HandlerResult::InternalError(..) => {
                                cut_off_buffer_base = 0;
                                break
                            },
                        }
                    }

                    for event in events.drain(..) {
                        if let NetworkEvent::UpdateClientTick {client_tick,received_at} = &event && let Ok(mut time_synchronization) = time_synchronization.lock() {
                            time_synchronization.estimated_client_tick(client_tick.0, *received_at);
                        }

                        event_sender.send(event).map_err(|_| NetworkTaskError::ConnectionClosed)?;
                    }
                }
                // Send a keep-alive packet to the server.
                _ = interval.tick() => {
                    ping_factory(&time_synchronization).packet_to_bytes(&mut byte_writer).unwrap();

                    #[cfg(feature = "debug")]
                    log_packet_bytes("keepalive", byte_writer.as_slice());

                    stream.write_all(byte_writer.as_slice()).await.map_err(|_| NetworkTaskError::ConnectionClosed)?;
                    byte_writer.clear();
                }
            }
        }
    }

    pub fn connect_to_login_server(
        &mut self,
        packet_version: SupportedPacketVersion,
        address: SocketAddr,
        username: impl Into<String>,
        password: impl Into<String>,
    ) {
        if !matches!(self.login_server_connection, ServerConnection::Disconnected) {
            return;
        }

        let (action_sender, action_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();

        self.command_sender
            .send(ServerConnectCommand::Login {
                address,
                action_receiver,
                event_sender,
                packet_version,
            })
            .expect("network thread dropped");

        let login_packet = LoginServerLoginPacket::new(username.into(), password.into());

        self.packet_callback.outgoing_packet(&login_packet);

        let mut byte_writer = ByteWriter::with_encoding(UTF_8);
        login_packet.packet_to_bytes(&mut byte_writer).unwrap();
        // If the receiver is already gone the connection attempt failed
        // instantly; the closed event channel will produce a disconnect event.
        let _ = action_sender.send(byte_writer.into_inner());

        self.login_server_connection = ServerConnection::Connected {
            action_sender,
            event_receiver,
            packet_version,
        };
    }

    pub fn connect_to_character_server(
        &mut self,
        packet_version: SupportedPacketVersion,
        login_data: &LoginServerLoginData,
        server: CharacterServerInformation,
    ) {
        if !matches!(self.character_server_connection, ServerConnection::Disconnected) {
            return;
        }

        let (action_sender, action_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let address = SocketAddr::new(IpAddr::V4(server.server_ip.into()), server.server_port);

        self.command_sender
            .send(ServerConnectCommand::Character {
                address,
                action_receiver,
                event_sender,
                packet_version,
            })
            .expect("network thread dropped");

        let login_packet = CharacterServerLoginPacket::new(
            login_data.account_id,
            login_data.login_id1,
            login_data.login_id2,
            login_data.sex,
        );

        self.packet_callback.outgoing_packet(&login_packet);

        let mut byte_writer = ByteWriter::with_encoding(UTF_8);
        login_packet.packet_to_bytes(&mut byte_writer).unwrap();
        // If the receiver is already gone the connection attempt failed
        // instantly; the closed event channel will produce a disconnect event.
        let _ = action_sender.send(byte_writer.into_inner());

        self.character_server_connection = ServerConnection::Connected {
            action_sender,
            event_receiver,
            packet_version,
        };
    }

    pub fn connect_to_map_server(
        &mut self,
        packet_version: SupportedPacketVersion,
        login_server_login_data: &LoginServerLoginData,
        character_server_login_data: CharacterServerLoginData,
    ) {
        if !matches!(self.map_server_connection, ServerConnection::Disconnected) {
            return;
        }

        let (action_sender, action_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let address = SocketAddr::new(character_server_login_data.server_ip, character_server_login_data.server_port);

        self.command_sender
            .send(ServerConnectCommand::Map {
                address,
                action_receiver,
                event_sender,
                packet_version,
            })
            .expect("network thread dropped");

        let login_packet = MapServerLoginPacket::new(
            login_server_login_data.account_id,
            character_server_login_data.character_id,
            login_server_login_data.login_id1,
            login_server_login_data.login_id2,
            // Always passing 100 seems to work fine for now, but it might cause
            // issues when connecting to something other than rAthena.
            ClientTick(100),
            login_server_login_data.sex,
        );

        self.packet_callback.outgoing_packet(&login_packet);

        let mut byte_writer = ByteWriter::with_encoding(UTF_8);
        login_packet.packet_to_bytes(&mut byte_writer).unwrap();
        let login_bytes = byte_writer.into_inner();

        #[cfg(feature = "debug")]
        log_packet_bytes("map-enter", &login_bytes);

        // If the receiver is already gone the connection attempt failed
        // instantly; the closed event channel will produce a disconnect event.
        let _ = action_sender.send(login_bytes);

        self.map_server_connection = ServerConnection::Connected {
            action_sender,
            event_receiver,
            packet_version,
        };
    }

    pub fn disconnect_from_login_server(&mut self) {
        self.login_server_connection = ServerConnection::ClosingManually;
    }

    pub fn disconnect_from_character_server(&mut self) {
        self.character_server_connection = ServerConnection::ClosingManually;
    }

    pub fn disconnect_from_map_server(&mut self) {
        self.map_server_connection = ServerConnection::ClosingManually;
    }

    pub fn is_login_server_connected(&self) -> bool {
        matches!(self.login_server_connection, ServerConnection::Connected { .. })
    }

    pub fn is_character_server_connected(&self) -> bool {
        matches!(self.character_server_connection, ServerConnection::Connected { .. })
    }

    pub fn is_map_server_connected(&self) -> bool {
        matches!(self.map_server_connection, ServerConnection::Connected { .. })
    }

    fn character_server_packet_version(&self) -> Result<SupportedPacketVersion, NotConnectedError> {
        match &self.character_server_connection {
            ServerConnection::Connected { packet_version, .. } => Ok(*packet_version),
            _ => Err(NotConnectedError),
        }
    }

    fn map_server_packet_version(&self) -> Result<SupportedPacketVersion, NotConnectedError> {
        match &self.map_server_connection {
            ServerConnection::Connected { packet_version, .. } => Ok(*packet_version),
            _ => Err(NotConnectedError),
        }
    }

    fn send_character_server_packet(&mut self, packet: impl CharacterServerPacket) -> Result<(), NotConnectedError> {
        match &mut self.character_server_connection {
            ServerConnection::Connected { action_sender, .. } => {
                self.packet_callback.outgoing_packet(&packet);

                // FIX: Don't unwrap.
                let mut byte_writer = ByteWriter::with_encoding(UTF_8);
                packet.packet_to_bytes(&mut byte_writer).unwrap();
                action_sender.send(byte_writer.into_inner()).map_err(|_| NotConnectedError)
            }
            _ => Err(NotConnectedError),
        }
    }

    fn send_map_server_packet(&mut self, packet: impl MapServerPacket) -> Result<(), NotConnectedError> {
        match &mut self.map_server_connection {
            ServerConnection::Connected { action_sender, .. } => {
                self.packet_callback.outgoing_packet(&packet);

                // FIX: Don't unwrap.
                let mut byte_writer = ByteWriter::with_encoding(UTF_8);
                packet.packet_to_bytes(&mut byte_writer).unwrap();
                let bytes = byte_writer.into_inner();

                #[cfg(feature = "debug")]
                log_packet_bytes("map-send", &bytes);

                action_sender.send(bytes).map_err(|_| NotConnectedError)
            }
            _ => Err(NotConnectedError),
        }
    }

    fn create_login_server_packet_handler(
        packet_callback: Callback,
        packet_version: SupportedPacketVersion,
    ) -> Result<PacketHandler<NetworkEventList, Callback>, DuplicateHandlerError> {
        let mut packet_handler = PacketHandler::<NetworkEventList, Callback>::with_callback(packet_callback);

        match packet_version {
            SupportedPacketVersion::_20220406 => packet_versions::version_20220406::register_login_server_packets(&mut packet_handler)?,
        }

        Ok(packet_handler)
    }

    fn create_character_server_packet_handler(
        packet_callback: Callback,
        packet_version: SupportedPacketVersion,
    ) -> Result<PacketHandler<NetworkEventList, Callback>, DuplicateHandlerError> {
        let mut packet_handler = PacketHandler::<NetworkEventList, Callback>::with_callback(packet_callback);

        match packet_version {
            SupportedPacketVersion::_20220406 => packet_versions::version_20220406::register_character_server_packets(&mut packet_handler)?,
        }

        Ok(packet_handler)
    }

    fn create_map_server_packet_handler(
        packet_callback: Callback,
        packet_version: SupportedPacketVersion,
    ) -> Result<PacketHandler<NetworkEventList, Callback>, DuplicateHandlerError> {
        let mut packet_handler = PacketHandler::<NetworkEventList, Callback>::with_callback(packet_callback);

        match packet_version {
            SupportedPacketVersion::_20220406 => packet_versions::version_20220406::register_map_server_packets(&mut packet_handler)?,
        }

        Ok(packet_handler)
    }

    pub fn request_character_list(&mut self) -> Result<(), NotConnectedError> {
        match self.character_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_character_server_packet(RequestCharacterListPacket::default()),
        }
    }

    pub fn select_character(&mut self, character_slot: usize) -> Result<(), NotConnectedError> {
        match self.character_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_character_server_packet(SelectCharacterPacket::new(character_slot as u8)),
        }
    }

    pub fn create_character(&mut self, slot: usize, name: String) -> Result<(), NotConnectedError> {
        let hair_color = 0;
        let hair_style = 0;
        let start_job_id = JobId(0);
        let sex = Sex::Male;

        match self.character_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_character_server_packet(CreateCharacterPacket::new(
                name,
                slot as u8,
                hair_color,
                hair_style,
                start_job_id,
                sex,
            )),
        }
    }

    pub fn delete_character(&mut self, character_id: CharacterId) -> Result<(), NotConnectedError> {
        let email = "a@a.com".to_string();

        match self.character_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_character_server_packet(DeleteCharacterPacket::new(character_id, email)),
        }
    }

    pub fn switch_character_slot(&mut self, origin_slot: usize, destination_slot: usize) -> Result<(), NotConnectedError> {
        match self.character_server_packet_version()? {
            SupportedPacketVersion::_20220406 => {
                self.send_character_server_packet(SwitchCharacterSlotPacket::new(origin_slot as u16, destination_slot as u16))
            }
        }
    }

    pub fn map_loaded(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MapLoadedPacket::default()),
        }
    }

    pub fn request_client_tick(&mut self) -> Result<(), NotConnectedError> {
        let client_tick = self
            .time_synchronization
            .lock()
            .map(|time_synchronization| time_synchronization.client_tick as u32)
            .unwrap_or(100);

        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestServerTickPacket::new(ClientTick(client_tick))),
        }
    }

    pub fn respawn(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RestartPacket::new(RestartType::Respawn)),
        }
    }

    pub fn log_out(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RestartPacket::new(RestartType::Disconnect)),
        }
    }

    pub fn player_move(&mut self, position: WorldPosition) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestPlayerMovePacket::new(position)),
        }
    }

    pub fn warp_to_map(&mut self, map_name: String, position: TilePosition) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestWarpToMapPacket::new(map_name, position)),
        }
    }

    pub fn select_warp_destination(&mut self, skill_id: SkillId, map_name: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(SelectWarpDestinationPacket::new(skill_id, map_name)),
        }
    }

    pub fn cancel_warp_selection(&mut self, skill_id: SkillId) -> Result<(), NotConnectedError> {
        // Warp selection has no server-side modal state to dismiss. Sending an
        // empty map name is not a cancellation on Hercules: Teleport treats it
        // as the random destination and immediately changes maps.
        let _ = skill_id;
        Ok(())
    }

    pub fn request_weapon_refine(&mut self, inventory_index: InventoryIndex) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => {
                self.send_map_server_packet(RequestWeaponRefinePacket::new(weapon_refine_wire_index(inventory_index)))
            }
        }
    }

    pub fn cancel_weapon_refine(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestWeaponRefinePacket::new(0)),
        }
    }

    pub fn request_item_repair(&mut self, item: RepairableItemInformation) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestItemRepairPacket::new(item)),
        }
    }

    pub fn cancel_item_repair(&mut self) -> Result<(), NotConnectedError> {
        self.request_item_repair(RepairableItemInformation {
            inventory_index: RawIndex(u16::MAX),
            item_id: ItemId(0),
            cards: [ItemId(0); 4],
            refinement_level: 0,
            grade: 0,
        })
    }

    pub fn entity_details(&mut self, entity_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestDetailsPacket::new(entity_id)),
        }
    }

    pub fn player_attack(&mut self, entity_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestActionPacket::new(entity_id, Action::Attack)),
        }
    }

    /// Sit down. Target entity id is unused for sit/stand on Hercules; send 0.
    pub fn player_sit(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestActionPacket::new(EntityId(0), Action::SitDown)),
        }
    }

    /// Stand up from a sit. Target entity id is unused for sit/stand on
    /// Hercules.
    pub fn player_stand(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestActionPacket::new(EntityId(0), Action::StandUp)),
        }
    }

    pub fn pick_up_item(&mut self, entity_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(ItemPickupRequestPacket::new(entity_id)),
        }
    }

    pub fn send_chat_message(&mut self, player_name: &str, text: &str) -> Result<(), NotConnectedError> {
        let message = format!("{} : {}", player_name, text);

        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(GlobalMessagePacket::new(message)),
        }
    }

    pub fn send_party_chat_message(&mut self, player_name: &str, text: &str) -> Result<(), NotConnectedError> {
        let message = format!("{} : {}", player_name, text);

        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyChatMessagePacket::new(message)),
        }
    }

    pub fn send_whisper_message(&mut self, target_name: &str, text: &str) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => {
                self.send_map_server_packet(WhisperSendPacket::new(target_name.to_owned(), text.to_owned()))
            }
        }
    }

    pub fn create_party(&mut self, party_name: &str) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CreatePartyPacket::new(party_name.to_owned(), 0, 0)),
        }
    }

    pub fn invite_to_party(&mut self, character_name: &str) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyInviteRequestPacket::new(character_name.to_owned())),
        }
    }

    pub fn accept_party_invite(&mut self, party_id: PartyId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyInviteResponsePacket::new(party_id, 1)),
        }
    }

    pub fn reject_party_invite(&mut self, party_id: PartyId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(PartyInviteResponsePacket::new(party_id, 0)),
        }
    }

    pub fn leave_party(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(LeavePartyPacket::new()),
        }
    }

    pub fn set_party_invitation_block(&mut self, blocked: bool) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(SetPartyInvitationStatePacket::new(blocked as u8)),
        }
    }

    pub fn start_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(StartDialogPacket::new(npc_id)),
        }
    }

    pub fn next_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(NextDialogPacket::new(npc_id)),
        }
    }

    pub fn close_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CloseDialogPacket::new(npc_id)),
        }
    }

    pub fn choose_dialog_option(&mut self, npc_id: EntityId, option: i8) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(ChooseDialogOptionPacket::new(npc_id, option)),
        }
    }

    pub fn submit_dialog_number(&mut self, npc_id: EntityId, value: i32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(NpcNumberInputPacket::new(npc_id, value)),
        }
    }

    pub fn submit_dialog_string(&mut self, npc_id: EntityId, text: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(NpcStringInputPacket::new(npc_id, text)),
        }
    }

    pub fn request_item_equip(&mut self, item_index: InventoryIndex, equip_position: EquipPosition) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestEquipItemPacket::new(item_index, equip_position)),
        }
    }

    pub fn request_item_unequip(&mut self, item_index: InventoryIndex) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestUnequipItemPacket::new(item_index)),
        }
    }

    pub fn use_item(&mut self, inventory_index: InventoryIndex, account_id: AccountId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(UseItemPacket::new(inventory_index, account_id)),
        }
    }

    /// Drop `amount` of an inventory item onto the ground (`CZ_ITEM_THROW2`
    /// 0x0363).
    pub fn drop_item(&mut self, inventory_index: InventoryIndex, amount: u16) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(DropItemPacket::new(inventory_index, amount)),
        }
    }

    pub fn request_item_identify(&mut self, inventory_index: InventoryIndex) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => {
                self.send_map_server_packet(RequestItemIdentifyPacket::new((inventory_index.0 + 2) as i16))
            }
        }
    }

    pub fn cancel_item_identify(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestItemIdentifyPacket::new(-1)),
        }
    }

    pub fn one_click_item_identify(&mut self, inventory_index: InventoryIndex) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(OneClickItemIdentifyPacket::new(inventory_index)),
        }
    }

    pub fn request_trade(&mut self, account_id: AccountId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestTradePacket::new(account_id)),
        }
    }

    pub fn accept_trade(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(TradeAckPacket::new(3)),
        }
    }

    pub fn reject_trade(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(TradeAckPacket::new(4)),
        }
    }

    pub fn trade_add_item(&mut self, inventory_index: InventoryIndex, amount: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(TradeAddItemPacket::new(inventory_index, amount)),
        }
    }

    pub fn trade_add_zeny(&mut self, amount: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(TradeAddItemPacket::new(InventoryIndex(65534), amount)),
        }
    }

    pub fn trade_ok(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(TradeOkPacket::default()),
        }
    }

    pub fn trade_cancel(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(TradeCancelPacket::default()),
        }
    }

    pub fn trade_commit(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(TradeCommitPacket::default()),
        }
    }

    pub fn move_item_to_storage(&mut self, inventory_index: InventoryIndex, amount: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(MoveItemToStoragePacket::new(inventory_index, amount)),
        }
    }

    pub fn move_item_from_storage(&mut self, storage_index: InventoryIndex, amount: u32) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => {
                self.send_map_server_packet(MoveItemFromStoragePacket::new(StorageIndex(storage_index.0), amount))
            }
        }
    }

    pub fn close_storage(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CloseStoragePacket::default()),
        }
    }

    pub fn cast_skill(&mut self, skill_id: SkillId, skill_level: SkillLevel, entity_id: EntityId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(UseSkillAtIdPacket::new(skill_level, skill_id, entity_id)),
        }
    }

    pub fn cast_ground_skill(
        &mut self,
        skill_id: SkillId,
        skill_level: SkillLevel,
        target_position: TilePosition,
    ) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => {
                self.send_map_server_packet(UseSkillOnGroundPacket::new(skill_level, skill_id, target_position))
            }
        }
    }

    pub fn cast_channeling_skill(
        &mut self,
        skill_id: SkillId,
        skill_level: SkillLevel,
        entity_id: EntityId,
    ) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(StartUseSkillPacket::new(skill_id, skill_level, entity_id)),
        }
    }

    pub fn stop_channeling_skill(&mut self, skill_id: SkillId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(EndUseSkillPacket::new(skill_id)),
        }
    }

    pub fn add_friend(&mut self, name: String) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(AddFriendPacket::new(name)),
        }
    }

    pub fn request_emotion(&mut self, emotion: u8) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestEmotionPacket::new(emotion)),
        }
    }

    pub fn remove_friend(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RemoveFriendPacket::new(account_id, character_id)),
        }
    }

    pub fn reject_friend_request(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(FriendRequestResponsePacket::new(
                account_id,
                character_id,
                FriendRequestResponse::Reject,
            )),
        }
    }

    pub fn accept_friend_request(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(FriendRequestResponsePacket::new(
                account_id,
                character_id,
                FriendRequestResponse::Accept,
            )),
        }
    }

    pub fn set_hotkey_data(&mut self, tab: HotbarTab, index: HotbarSlot, hotkey_data: HotkeyData) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(SetHotkeyData2Packet::new(tab, index, hotkey_data)),
        }
    }

    pub fn select_buy_or_sell(&mut self, shop_id: ShopId, buy_or_sell: BuyOrSellOption) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(SelectBuyOrSellPacket::new(shop_id, buy_or_sell)),
        }
    }

    pub fn purchase_items(&mut self, items: Vec<ShopItem<u32>>) -> Result<(), NotConnectedError> {
        // Regular NPC shops use CZ_PC_PURCHASE_ITEMLIST (0x00C8), not the NPC market
        // packet 0x09D6 (that is only for `callshop` market UIs).
        let item_information = items
            .into_iter()
            .map(|item| BuyItemInformation {
                amount: item.metadata.min(u16::MAX as u32) as u16,
                item_id: item.item_id,
            })
            .collect();

        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(BuyItemsPacket::new(item_information)),
        }
    }

    pub fn close_shop(&mut self) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(CloseShopPacket::new()),
        }
    }

    pub fn sell_items(&mut self, items: Vec<SoldItemInformation>) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(SellItemsPacket { items }),
        }
    }

    pub fn request_stat_up(&mut self, stat_type: StatUpType) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(RequestStatUpPacket::new(stat_type)),
        }
    }

    pub fn level_up_skill(&mut self, skill_id: SkillId) -> Result<(), NotConnectedError> {
        match self.map_server_packet_version()? {
            SupportedPacketVersion::_20220406 => self.send_map_server_packet(LevelUpSkillPacket::new(skill_id)),
        }
    }
}

#[cfg(test)]
mod packet_handlers {
    use ragnarok_packets::handler::NoPacketCallback;

    use crate::{NetworkingSystem, SupportedPacketVersion};

    #[test]
    fn weapon_refine_restores_server_inventory_offset() {
        assert_eq!(crate::weapon_refine_wire_index(ragnarok_packets::InventoryIndex(0)), 2);
        assert_eq!(crate::weapon_refine_wire_index(ragnarok_packets::InventoryIndex(17)), 19);
    }

    #[test]
    fn login_server() {
        let result = NetworkingSystem::create_login_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406);
        assert!(result.is_ok());
    }

    #[test]
    fn character_server() {
        let result = NetworkingSystem::create_character_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406);
        assert!(result.is_ok());
    }

    #[test]
    fn map_server() {
        let result = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406);
        assert!(result.is_ok());
    }

    /// Hercules' clif_sitting/clif_standing (0x008A `ZC_NOTIFY_ACT`) carry the
    /// acting entity in the SOURCE field with the destination zeroed. The
    /// sit/stand events must be keyed off the source or every sit/stand is
    /// attributed to entity 0. (Found by the headless tester; HF-001.)
    #[test]
    fn sit_ack_uses_source_entity() {
        use ragnarok_bytes::ByteReader;
        use ragnarok_packets::handler::HandlerResult;

        use crate::NetworkEvent;

        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // header 0x008A | source 0x001E8480 (2000000) | destination 0 |
        // tick 0 | attack_duration 0 | damage_delay 0 | damage 0 | hits 0 |
        // damage_type 2 (SitDown) | damage2 0
        let mut bytes = vec![0x8A, 0x00];
        bytes.extend_from_slice(&2000000u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 20]);
        bytes.push(2);
        bytes.extend_from_slice(&[0; 2]);

        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("sit packet did not parse");
        };

        assert!(
            matches!(
                events.0.as_slice(),
                [NetworkEvent::PlayerSitDown { entity_id }] if entity_id.0 == 2000000
            ),
            "expected PlayerSitDown for entity 2000000, got {:?}",
            events.0
        );
    }

    #[test]
    fn cast_cancel_packet_targets_the_named_actor() {
        use ragnarok_bytes::ByteReader;
        use ragnarok_packets::handler::HandlerResult;

        use crate::NetworkEvent;

        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();
        let bytes = [
            0xB9, 0x01, // ZC_DISPEL
            0x80, 0x84, 0x1E, 0x00, // actor 2000000
        ];
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("cast-cancel packet did not parse");
        };

        assert!(matches!(
            events.0.as_slice(),
            [NetworkEvent::SkillCastCancelled {
                source_entity_id: Some(entity_id),
            }] if entity_id.0 == 2000000
        ));
    }

    #[test]
    fn state_change_preserves_all_four_atomic_actor_fields() {
        use ragnarok_bytes::ByteReader;
        use ragnarok_packets::handler::HandlerResult;

        use crate::NetworkEvent;

        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();
        let bytes = [
            0x29, 0x02, // ZC_STATE_CHANGE
            0x80, 0x84, 0x1E, 0x00, // actor 2000000
            0x01, 0x00, // body_state: stone
            0x04, 0x00, // health_state
            0x02, 0x00, 0x00, 0x00, // option: hide
            0x01, // isPKModeON
        ];
        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("state-change packet did not parse");
        };

        assert!(matches!(
            events.0.as_slice(),
            [NetworkEvent::StateChange {
                entity_id,
                option: 2,
                body_state: 1,
                health_state: 4,
                is_pk_mode_on: true,
            }] if entity_id.0 == 2000000
        ));
    }

    /// Golden fixture for the animation-fidelity plan (phase A): the basic
    /// damage packet's native timing fields must survive into `DamageEffect`
    /// unmodified — `sMotion` and `dMotion` are separate clocks, and the
    /// packet tick is retained for occurrence correlation.
    #[test]
    fn basic_damage_0x08c8_keeps_smotion_and_dmotion_separate() {
        use ragnarok_bytes::ByteReader;
        use ragnarok_packets::handler::HandlerResult;

        use crate::NetworkEvent;

        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // header 0x08C8 | source 2000000 | destination 110000001 | tick 5000 |
        // sMotion 576 | dMotion 288 | damage 120 | special 0 | hits 1 |
        // type 0 (Damage) | damage2 0
        let mut bytes = vec![0xC8, 0x08];
        bytes.extend_from_slice(&2000000u32.to_le_bytes());
        bytes.extend_from_slice(&110000001u32.to_le_bytes());
        bytes.extend_from_slice(&5000u32.to_le_bytes());
        bytes.extend_from_slice(&576u32.to_le_bytes());
        bytes.extend_from_slice(&288u32.to_le_bytes());
        bytes.extend_from_slice(&120u32.to_le_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&0u32.to_le_bytes());

        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("basic damage packet did not parse");
        };

        assert!(
            matches!(
                events.0.as_slice(),
                [NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    skill_id: None,
                    packet_tick,
                    damage_amount: Some(120),
                    hit_count: 1,
                    attack_duration: 576,
                    damage_delay: 288,
                    is_critical: false,
                }] if source_entity_id.0 == 2000000
                    && destination_entity_id.0 == 110000001
                    && packet_tick.0 == 5000
            ),
            "unexpected events: {:?}",
            events.0
        );
    }

    /// A zero-damage basic hit of type Damage is a miss: `damage_amount` must
    /// be `None` (Miss particle), while the timing fields still schedule the
    /// impact boundary. Critical type must set `is_critical`.
    #[test]
    fn basic_damage_0x08c8_miss_and_critical_variants() {
        use ragnarok_bytes::ByteReader;
        use ragnarok_packets::handler::HandlerResult;

        use crate::NetworkEvent;

        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let mut build = |damage: u32, damage_type: u8| {
            let mut bytes = vec![0xC8, 0x08];
            bytes.extend_from_slice(&2000000u32.to_le_bytes());
            bytes.extend_from_slice(&110000001u32.to_le_bytes());
            bytes.extend_from_slice(&5000u32.to_le_bytes());
            bytes.extend_from_slice(&500u32.to_le_bytes());
            bytes.extend_from_slice(&144u32.to_le_bytes());
            bytes.extend_from_slice(&damage.to_le_bytes());
            bytes.push(0);
            bytes.extend_from_slice(&1u16.to_le_bytes());
            bytes.push(damage_type);
            bytes.extend_from_slice(&0u32.to_le_bytes());
            bytes
        };

        let miss = build(0, 0);
        let mut reader = ByteReader::without_metadata(&miss);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("miss packet did not parse");
        };
        assert!(
            matches!(events.0.as_slice(), [NetworkEvent::DamageEffect {
                damage_amount: None,
                damage_delay: 144,
                is_critical: false,
                ..
            }]),
            "unexpected miss events: {:?}",
            events.0
        );

        // damage_type 10 = CriticalHit.
        let critical = build(333, 10);
        let mut reader = ByteReader::without_metadata(&critical);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("critical packet did not parse");
        };
        assert!(
            matches!(events.0.as_slice(), [NetworkEvent::DamageEffect {
                damage_amount: Some(333),
                is_critical: true,
                ..
            }]),
            "unexpected critical events: {:?}",
            events.0
        );
    }

    /// Assassin dual-wield / Double Attack normals arrive as `MultiHitDamage`
    /// (type 8) with a second damage value in `damage_amount_2`. Regression for
    /// the auto-attack stall: these must still yield a `DamageEffect` (summing
    /// both hits) so the player's own damage-ack keeps the attack loop alive.
    /// The critical multi-hit type (13) must set `is_critical`.
    #[test]
    fn dual_wield_multi_hit_damage_0x08c8_surfaces_damage_effect() {
        use ragnarok_bytes::ByteReader;
        use ragnarok_packets::handler::HandlerResult;

        use crate::NetworkEvent;

        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        let build = |damage: u32, damage2: u32, hits: u16, damage_type: u8| {
            let mut bytes = vec![0xC8, 0x08];
            bytes.extend_from_slice(&2000000u32.to_le_bytes());
            bytes.extend_from_slice(&110000001u32.to_le_bytes());
            bytes.extend_from_slice(&5000u32.to_le_bytes());
            bytes.extend_from_slice(&500u32.to_le_bytes());
            bytes.extend_from_slice(&144u32.to_le_bytes());
            bytes.extend_from_slice(&damage.to_le_bytes());
            bytes.push(0);
            bytes.extend_from_slice(&hits.to_le_bytes());
            bytes.push(damage_type);
            bytes.extend_from_slice(&damage2.to_le_bytes());
            bytes
        };

        // type 8 = MultiHitDamage (dual wield): 100 + 90 = 190, two hits.
        let dual = build(100, 90, 2, 8);
        let mut reader = ByteReader::without_metadata(&dual);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("dual-wield damage packet did not parse");
        };
        assert!(
            matches!(events.0.as_slice(), [NetworkEvent::DamageEffect {
                damage_amount: Some(190),
                hit_count: 2,
                is_critical: false,
                ..
            }]),
            "dual-wield multi-hit should surface a DamageEffect: {:?}",
            events.0
        );

        // type 13 = CriticalMultiHit.
        let crit = build(150, 140, 2, 13);
        let mut reader = ByteReader::without_metadata(&crit);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("critical multi-hit packet did not parse");
        };
        assert!(
            matches!(events.0.as_slice(), [NetworkEvent::DamageEffect {
                damage_amount: Some(290),
                hit_count: 2,
                is_critical: true,
                ..
            }]),
            "critical multi-hit should surface a critical DamageEffect: {:?}",
            events.0
        );
    }

    /// Golden fixture for `ZC_NOTIFY_SKILL2` (0x01DE): `attack_duration` must
    /// be the *source* delay alone (the old `max(sdelay, ddelay)` bug), the
    /// destination delay must arrive as `damage_delay`, the volley `div`
    /// becomes `hit_count`, and skill_type 8 is the multi-hit critical family.
    #[test]
    fn skill_damage_0x01de_routes_native_timing_fields() {
        use ragnarok_bytes::ByteReader;
        use ragnarok_packets::handler::HandlerResult;

        use crate::NetworkEvent;

        let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

        // header 0x01DE | skill 56 (Pierce) | source 2000000 |
        // destination 110000001 | tick 7777 | sdelay 672 | ddelay 480 |
        // damage 900 | level 10 | div 3 | skill_type 6
        let mut bytes = vec![0xDE, 0x01];
        bytes.extend_from_slice(&56u16.to_le_bytes());
        bytes.extend_from_slice(&2000000u32.to_le_bytes());
        bytes.extend_from_slice(&110000001u32.to_le_bytes());
        bytes.extend_from_slice(&7777u32.to_le_bytes());
        bytes.extend_from_slice(&672u32.to_le_bytes());
        bytes.extend_from_slice(&480u32.to_le_bytes());
        bytes.extend_from_slice(&900u32.to_le_bytes());
        bytes.extend_from_slice(&10u16.to_le_bytes());
        bytes.extend_from_slice(&3u16.to_le_bytes());
        bytes.push(6);

        let mut reader = ByteReader::without_metadata(&bytes);
        let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
            panic!("skill damage packet did not parse");
        };

        assert!(
            matches!(
                events.0.as_slice(),
                [NetworkEvent::DamageEffect {
                    source_entity_id,
                    skill_id: Some(skill_id),
                    packet_tick,
                    damage_amount: Some(900),
                    hit_count: 3,
                    attack_duration: 672,
                    damage_delay: 480,
                    is_critical: false,
                    ..
                }] if source_entity_id.0 == 2000000 && skill_id.0 == 56 && packet_tick.0 == 7777
            ),
            "unexpected events: {:?}",
            events.0
        );
    }
}
