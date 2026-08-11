//! Phase 10: protocol coverage ledger.
//!
//! A [`PacketCallback`] that counts every packet by header, records packets
//! consumed by the length-fallback (server sends them, client models nothing),
//! and captures deserialization failures. One instance is shared across all
//! scenario sessions so the report covers the whole run.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ragnarok_packets::Packet;
use ragnarok_packets::handler::PacketCallback;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct UnknownPacketSummary {
    pub header: u16,
    pub count: u64,
    pub sample_hex: String,
}

#[derive(Debug, Serialize)]
pub struct PacketCoverageSummary {
    pub distinct_incoming_handled: usize,
    pub distinct_outgoing: usize,
    pub unknown: Vec<UnknownPacketSummary>,
    pub deserialization_failures: Vec<String>,
}

#[derive(Default)]
pub struct LedgerInner {
    /// header -> (count, rust type name) for packets with dedicated handlers.
    pub incoming: HashMap<u16, (u64, &'static str)>,
    pub outgoing: HashMap<u16, (u64, &'static str)>,
    /// header -> count for packets consumed by the length-fallback (or true
    /// unknowns). The modeling backlog, ranked by count.
    pub unknown: HashMap<u16, u64>,
    /// First few raw samples per unknown header, for later inspection.
    pub unknown_samples: HashMap<u16, Vec<u8>>,
    /// Deserialization failures: (bytes, error). Any entry is a P0 finding.
    pub failed: Vec<(Vec<u8>, String)>,
}

#[derive(Clone, Default)]
pub struct Ledger {
    inner: Arc<Mutex<LedgerInner>>,
}

impl Ledger {
    pub fn summary(&self) -> PacketCoverageSummary {
        let inner = self.inner.lock().unwrap();
        let mut unknown: Vec<_> = inner
            .unknown
            .iter()
            .map(|(&header, &count)| UnknownPacketSummary {
                header,
                count,
                sample_hex: inner
                    .unknown_samples
                    .get(&header)
                    .map(|bytes| {
                        bytes
                            .iter()
                            .take(24)
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default(),
            })
            .collect();
        unknown.sort_by(|left, right| right.count.cmp(&left.count).then(left.header.cmp(&right.header)));

        PacketCoverageSummary {
            distinct_incoming_handled: inner.incoming.len(),
            distinct_outgoing: inner.outgoing.len(),
            unknown,
            deserialization_failures: inner.failed.iter().map(|(_, error)| error.clone()).collect(),
        }
    }

    pub fn report(&self) -> String {
        let inner = self.inner.lock().unwrap();
        let mut out = String::new();

        let mut unknown: Vec<_> = inner.unknown.iter().collect();
        unknown.sort_by(|a, b| b.1.cmp(a.1));

        out.push_str(&format!(
            "\nPacket coverage: {} distinct incoming handled, {} distinct outgoing, {} distinct unknown/fallback, {} failed\n",
            inner.incoming.len(),
            inner.outgoing.len(),
            unknown.len(),
            inner.failed.len(),
        ));

        if !unknown.is_empty() {
            out.push_str("\nUnmodeled packets seen (modeling backlog, by frequency):\n");
            for (header, count) in unknown {
                let sample = inner
                    .unknown_samples
                    .get(header)
                    .map(|bytes| bytes.iter().take(24).map(|byte| format!("{byte:02x} ")).collect::<String>())
                    .unwrap_or_default();
                out.push_str(&format!("  0x{header:04X}  x{count:<6} sample: {sample}\n"));
            }
        }

        if !inner.failed.is_empty() {
            out.push_str("\n!! DESERIALIZATION FAILURES (P0 — document in headless_findings.md):\n");
            for (bytes, error) in &inner.failed {
                let hex: String = bytes.iter().take(32).map(|byte| format!("{byte:02x} ")).collect();
                out.push_str(&format!("  error: {error}\n  bytes: {hex}\n"));
            }
        }

        out
    }

    pub fn failed_count(&self) -> usize {
        self.inner.lock().unwrap().failed.len()
    }

    /// Unknown/fallback headers not present in the reviewed baseline.
    ///
    /// Counts are returned as well as headers so the gate explains its failure
    /// without requiring somebody to find and inspect the JSON artifact first.
    pub fn unexpected_unknown(&self, reviewed_headers: &[u16]) -> Vec<(u16, u64)> {
        let inner = self.inner.lock().unwrap();
        let mut unexpected: Vec<_> = inner
            .unknown
            .iter()
            .filter(|(header, _)| !reviewed_headers.contains(header))
            .map(|(&header, &count)| (header, count))
            .collect();
        unexpected.sort_unstable_by_key(|(header, _)| *header);
        unexpected
    }
}

impl PacketCallback for Ledger {
    fn incoming_packet<P>(&self, _packet: &P)
    where
        P: Packet,
    {
        let mut inner = self.inner.lock().unwrap();
        let entry = inner.incoming.entry(P::HEADER.0).or_insert((0, std::any::type_name::<P>()));
        entry.0 += 1;
    }

    fn outgoing_packet<P>(&self, _packet: &P)
    where
        P: Packet,
    {
        let mut inner = self.inner.lock().unwrap();
        let entry = inner.outgoing.entry(P::HEADER.0).or_insert((0, std::any::type_name::<P>()));
        entry.0 += 1;
    }

    fn unknown_packet(&self, bytes: Vec<u8>) {
        let mut inner = self.inner.lock().unwrap();
        // The first two bytes are the header (little endian) — guaranteed for
        // fallback-consumed packets and for the unregistered-header path.
        let header = match bytes.get(0..2) {
            Some(&[low, high]) => u16::from_le_bytes([low, high]),
            _ => 0,
        };
        *inner.unknown.entry(header).or_insert(0) += 1;
        inner.unknown_samples.entry(header).or_insert(bytes);
    }

    fn failed_packet(&self, bytes: Vec<u8>, error: Box<ragnarok_bytes::ConversionError>) {
        let mut inner = self.inner.lock().unwrap();
        inner.failed.push((bytes, format!("{error:?}")));
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::handler::PacketCallback;

    use super::Ledger;

    #[test]
    fn only_new_unknown_headers_fail_the_reviewed_baseline() {
        let ledger = Ledger::default();
        ledger.unknown_packet(vec![0x78, 0x00, 0xAA]);
        ledger.unknown_packet(vec![0x34, 0x12, 0xBB]);
        ledger.unknown_packet(vec![0x34, 0x12, 0xCC]);

        assert_eq!(ledger.unexpected_unknown(&[0x0078]), vec![(0x1234, 2)]);
    }
}
