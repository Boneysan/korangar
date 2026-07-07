use std::collections::HashMap;

use ragnarok_bytes::{ByteReader, ConversionError, ConversionResult, FromBytes};

use crate::PacketHeader;

/// Possible results of [`PacketHandler::process_one`].
pub enum HandlerResult<Output> {
    /// Packet was successfully processed and produced some output.
    Ok(Output),
    /// No packet handler was registered for the incoming packet.
    UnhandledPacket,
    /// Packet was most likely cut-off.
    PacketCutOff,
    /// An error occurred inside the packet handler.
    InternalError(Box<ConversionError>),
}

/// Error when trying to register two separate handlers for the same packet.
#[derive(Debug, Clone)]
pub struct DuplicateHandlerError {
    /// Header of the packet.
    pub packet_header: PacketHeader,
}

/// Trait for monitoring the incoming and outgoing packets.
pub trait PacketCallback: Clone + 'static {
    /// Called by the [`PacketHandler`] when a packet is received.
    fn incoming_packet<Packet>(&self, packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
        let _ = packet;
    }

    /// Called by when a packet is sent.
    fn outgoing_packet<Packet>(&self, packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
        let _ = packet;
    }

    /// Called by the [`PacketHandler`] when a packet arrives that doesn't have
    /// a handler registered.
    fn unknown_packet(&self, bytes: Vec<u8>) {
        let _ = bytes;
    }

    /// Called by the [`PacketHandler`] when a packet handler returned an error.
    fn failed_packet(&self, bytes: Vec<u8>, error: Box<ConversionError>) {
        let _ = (bytes, error);
    }
}

#[derive(Debug, Default, Clone)]
pub struct NoPacketCallback;

impl PacketCallback for NoPacketCallback {}

pub type HandlerFunction<Output> = Box<dyn Fn(&mut ByteReader) -> ConversionResult<Output>>;

/// A struct to help with reading packets from a [`ByteReader`] and
/// converting them to some common event type.
///
/// It allows passing a packet callback to monitor incoming packets.
pub struct PacketHandler<Output, Callback> {
    handlers: HashMap<PacketHeader, HandlerFunction<Output>>,
    packet_callback: Callback,
}

impl<Output, Callback> Default for PacketHandler<Output, Callback>
where
    Callback: Default,
{
    fn default() -> Self {
        Self {
            handlers: Default::default(),
            packet_callback: Default::default(),
        }
    }
}

impl<Output, Callback> PacketHandler<Output, Callback>
where
    Output: Default,
    Callback: PacketCallback,
{
    /// Create a new packet handler with a callback.
    pub fn with_callback(packet_callback: Callback) -> Self {
        Self {
            handlers: Default::default(),
            packet_callback,
        }
    }

    /// Register a new packet handler.
    pub fn register<Packet, Return>(&mut self, handler: impl Fn(Packet) -> Return + 'static) -> Result<(), DuplicateHandlerError>
    where
        Packet: ragnarok_packets::Packet,
        Return: Into<Output>,
    {
        let packet_callback = self.packet_callback.clone();
        let old_handler = self.handlers.insert(
            Packet::HEADER,
            Box::new(move |byte_reader| {
                let packet = Packet::payload_from_bytes(byte_reader)?;

                packet_callback.incoming_packet(&packet);

                Ok(handler(packet).into())
            }),
        );

        match old_handler.is_some() {
            true => Err(DuplicateHandlerError {
                packet_header: Packet::HEADER,
            }),
            false => Ok(()),
        }
    }

    /// Register a noop packet handler.
    pub fn register_noop<Packet>(&mut self) -> Result<(), DuplicateHandlerError>
    where
        Packet: ragnarok_packets::Packet,
    {
        let packet_callback = self.packet_callback.clone();
        let old_handler = self.handlers.insert(
            Packet::HEADER,
            Box::new(move |byte_reader| {
                let packet = Packet::payload_from_bytes(byte_reader)?;

                packet_callback.incoming_packet(&packet);

                Ok(Output::default())
            }),
        );

        match old_handler.is_some() {
            true => Err(DuplicateHandlerError {
                packet_header: Packet::HEADER,
            }),
            false => Ok(()),
        }
    }

    /// Register opaque fallback handlers for every packet whose on-wire length
    /// is known but that has no dedicated handler yet.
    ///
    /// Korangar frames packets by deserialization, so an unregistered header
    /// desyncs the read buffer and drops everything after it. Each entry in
    /// `lengths` is `(header, length)`, where `length` is the total packet size
    /// including the 2-byte header, or a negative value for variable-length
    /// packets (whose real size is read from the 2-byte length field that
    /// follows the header).
    ///
    /// For any header not already registered, this installs a handler that
    /// consumes exactly the right number of bytes and still reports the packet
    /// through [`PacketCallback::unknown_packet`] — so it keeps surfacing for
    /// auditing while the rest of the buffer stays correctly framed. Call this
    /// last, after every dedicated handler is registered, so it never shadows
    /// them.
    pub fn register_length_fallbacks(&mut self, lengths: &[(u16, i32)]) {
        for &(header, length) in lengths {
            let header = PacketHeader(header);

            // Never override a dedicated handler.
            if self.handlers.contains_key(&header) {
                continue;
            }

            let packet_callback = self.packet_callback.clone();
            self.handlers.insert(
                header,
                Box::new(move |byte_reader| {
                    // `process_one` has already consumed the 2-byte header.
                    let payload_length = if length < 0 {
                        // Variable-length: the total size lives in the next u16.
                        let total = u16::from_bytes(byte_reader)? as usize;
                        // Subtract the header (2) and the length field (2).
                        total.saturating_sub(4)
                    } else {
                        (length as usize).saturating_sub(2)
                    };

                    let bytes = byte_reader.slice::<()>(payload_length)?.to_vec();
                    packet_callback.unknown_packet(bytes);

                    Ok(Output::default())
                }),
            );
        }
    }

    /// Take a single packet from the byte stream.
    pub fn process_one(&mut self, byte_reader: &mut ByteReader) -> HandlerResult<Output> {
        let save_point = byte_reader.create_save_point();

        let Ok(header) = PacketHeader::from_bytes(byte_reader) else {
            // Packet is cut-off at the header.
            byte_reader.restore_save_point(save_point);
            return HandlerResult::PacketCutOff;
        };

        let Some(handler) = self.handlers.get(&header) else {
            byte_reader.restore_save_point(save_point);

            self.packet_callback.unknown_packet(byte_reader.remaining_bytes());

            return HandlerResult::UnhandledPacket;
        };

        match handler(byte_reader) {
            Ok(output) => HandlerResult::Ok(output),
            // Cut-off packet (probably).
            Err(error) if error.is_byte_reader_too_short() => {
                byte_reader.restore_save_point(save_point);
                HandlerResult::PacketCutOff
            }
            Err(error) => {
                byte_reader.restore_save_point(save_point);

                self.packet_callback.failed_packet(byte_reader.remaining_bytes(), error.clone());

                HandlerResult::InternalError(error)
            }
        }
    }
}

#[cfg(test)]
mod length_fallback_tests {
    use ragnarok_bytes::ByteReader;

    use super::{HandlerResult, NoPacketCallback, PacketHandler};

    fn handler_with(lengths: &[(u16, i32)]) -> PacketHandler<(), NoPacketCallback> {
        let mut handler = PacketHandler::<(), NoPacketCallback>::default();
        handler.register_length_fallbacks(lengths);
        handler
    }

    #[test]
    fn fixed_length_fallback_consumes_exact_payload() {
        let mut handler = handler_with(&[(0x1234, 5)]);

        // header 0x1234 + 3 payload bytes = 5 total, then the start of a second packet.
        let bytes = [0x34, 0x12, 0xAA, 0xBB, 0xCC, 0x78, 0x56];
        let mut reader = ByteReader::without_metadata(&bytes);

        assert!(matches!(handler.process_one(&mut reader), HandlerResult::Ok(())));
        // Consumed exactly the 5-byte packet, leaving the next one framed.
        assert_eq!(reader.get_offset(), 5);
    }

    #[test]
    fn variable_length_fallback_reads_length_field() {
        let mut handler = handler_with(&[(0x1234, -1)]);

        // header + length(0x0007) + 3 payload bytes = 7 total.
        let bytes = [0x34, 0x12, 0x07, 0x00, 0xAA, 0xBB, 0xCC];
        let mut reader = ByteReader::without_metadata(&bytes);

        assert!(matches!(handler.process_one(&mut reader), HandlerResult::Ok(())));
        assert_eq!(reader.get_offset(), 7);
    }

    #[test]
    fn cut_off_fallback_packet_is_reported_as_cutoff() {
        let mut handler = handler_with(&[(0x1234, 8)]);

        // Only 4 of the 8 bytes have arrived.
        let bytes = [0x34, 0x12, 0xAA, 0xBB];
        let mut reader = ByteReader::without_metadata(&bytes);

        assert!(matches!(handler.process_one(&mut reader), HandlerResult::PacketCutOff));
        // The reader was rewound so the partial packet is retried on the next read.
        assert_eq!(reader.get_offset(), 0);
    }

    #[test]
    fn first_registration_wins_and_is_not_overridden() {
        let mut handler = PacketHandler::<(), NoPacketCallback>::default();
        // Register a fixed 5-byte fallback, then try to shadow it with a variable one.
        handler.register_length_fallbacks(&[(0x1234, 5)]);
        handler.register_length_fallbacks(&[(0x1234, -1)]);

        let bytes = [0x34, 0x12, 0xAA, 0xBB, 0xCC];
        let mut reader = ByteReader::without_metadata(&bytes);

        // If the variable handler had won it would read 0xBBAA as the length and
        // fail; the fixed 5-byte handler consuming everything proves it stuck.
        assert!(matches!(handler.process_one(&mut reader), HandlerResult::Ok(())));
        assert_eq!(reader.get_offset(), 5);
    }
}
