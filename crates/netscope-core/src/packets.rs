//! Recorded packet bytes and checked layer metadata, separate from HAR requests.
use std::{net::SocketAddr, ops::Range};

/// One classic PCAP recording in original record order.
#[derive(Debug)]
pub struct PacketCapture {
    /// File's link-layer identifier.
    pub link_type: u32,
    /// Maximum stored bytes per record reported by the file.
    pub snaplen: u32,
    /// Whether timestamp fractions were recorded in nanoseconds.
    pub nanoseconds: bool,
    /// Packet records; their vector positions are stable identifiers.
    pub packets: Vec<Packet>,
    /// File-level notes, such as records that were not read.
    pub notes: Vec<&'static str>,
}
/// One stored record; layer interpretation never changes these bytes.
#[derive(Debug)]
pub struct Packet {
    /// Recorded whole seconds, with no invented clock accuracy.
    pub seconds: u32,
    /// Recorded fraction converted exactly to nanoseconds.
    pub nanos: u32,
    /// Reported original packet size, possibly larger than retained bytes.
    pub original_len: u32,
    /// Stored packet bytes, excluding the PCAP record header.
    pub bytes: Vec<u8>,
    /// Checked layer descriptions and unavailable/unsupported explanations.
    pub layers: Vec<String>,
    /// TCP metadata when the supported headers are available.
    pub tcp: Option<TcpSegment>,
    /// UDP metadata when the supported headers are available.
    pub udp: Option<UdpDatagram>,
}
/// Checked TCP header fields. Checksums and receiver acceptance are not verified.
#[derive(Debug)]
pub struct TcpSegment {
    /// Source address and port.
    pub source: SocketAddr,
    /// Destination address and port.
    pub destination: SocketAddr,
    /// Recorded sequence number before any SYN adjustment.
    pub sequence: u32,
    /// Recorded acknowledgement number; meaningful only with ACK.
    pub acknowledgement: u32,
    /// Eight recorded control bits, including SYN/ACK/FIN/RST.
    pub flags: u8,
    /// Recorded advertised window, without negotiated scaling.
    pub window: u16,
    /// Retained payload's byte range in its packet.
    pub payload: Range<usize>,
    /// Payload length declared by the enclosing supported IP packet.
    pub declared_payload: usize,
}
/// Checked UDP header fields; a datagram is not a TCP byte stream.
#[derive(Debug)]
pub struct UdpDatagram {
    /// Source address and port.
    pub source: SocketAddr,
    /// Destination address and port.
    pub destination: SocketAddr,
    /// Retained UDP data range within the packet.
    pub payload: Range<usize>,
    /// Whether all bytes declared by UDP are retained.
    pub complete: bool,
}
