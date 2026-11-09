use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter, Result as FmtResult};

use crate::error::IpcError;
use crate::io::Read;
use crate::vlq::{vlq_decode, vlq_encode};

/// Maximum accepted payload size (2 MiB), excluding the packet header.
pub const MAX_PAYLOAD_SIZE: usize = 2 * 1024 * 1024;

fn read_payload(reader: &mut impl Read, length: u64) -> Result<Vec<u8>, IpcError> {
    if length > MAX_PAYLOAD_SIZE as u64 {
        return Err(IpcError::PayloadTooLarge);
    }
    let length = length as usize;
    let mut payload = Vec::new();
    payload
        .try_reserve_exact(length)
        .map_err(|_| IpcError::AllocationFailed)?;
    payload.resize(length, 0);
    reader
        .read_exact(&mut payload)
        .map_err(|_| IpcError::ReadExactError)?;
    Ok(payload)
}

/// The `Packet` trait defines the interface for handling packets in an IPC context.
/// Types implementing this trait can be used to represent and manipulate packets.
///
/// # Required Methods
///
/// * `version` - This method returns the version of the packet.
/// * `payload` - This method returns a reference to the payload of the packet.
/// * `read_from` - This method reads a packet from a reader and returns an instance of the implementing type.
/// * `serialize` - This method serializes the packet into a vector of bytes.
pub trait Packet {
    fn version(&self) -> u8;
    fn payload(&self) -> &[u8];
    fn read_from<R: Read>(reader: &mut R) -> Result<Self, IpcError>
    where
        Self: Sized;
    fn serialize(&self) -> Vec<u8>;
}

pub struct RequestPacket {
    version: u8,
    method_id: u64,
    payload: Vec<u8>,
}

impl Debug for RequestPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "RequestPacket, {} bytes payload: {}",
            self.payload.len(),
            String::from_utf8_lossy(&self.payload)
        )
    }
}

impl Packet for RequestPacket {
    fn version(&self) -> u8 {
        self.version
    }
    fn payload(&self) -> &[u8] {
        &self.payload
    }
    fn read_from<R: Read>(reader: &mut R) -> Result<Self, IpcError> {
        let version = read_next_vlq(reader)? as u8;
        let method_id = read_next_vlq(reader)?;
        let payload_length = read_next_vlq(reader)?;
        let payload = read_payload(reader, payload_length)?;
        Ok(RequestPacket {
            version,
            method_id,
            payload,
        })
    }
    fn serialize(&self) -> Vec<u8> {
        let mut buf = vec![];
        buf.extend_from_slice(&vlq_encode(self.version as u64));
        buf.extend_from_slice(&vlq_encode(self.method_id));
        buf.extend_from_slice(&vlq_encode(self.payload.len() as u64));
        buf.extend_from_slice(&self.payload);
        buf
    }
}

impl RequestPacket {
    pub fn new(payload: Vec<u8>) -> Self {
        Self {
            version: 0,
            method_id: 0,
            payload,
        }
    }
    pub fn method_id(&self) -> u64 {
        self.method_id
    }
}

pub struct ResponsePacket {
    version: u8,
    error_code: u64,
    payload: Vec<u8>,
}

impl Debug for ResponsePacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "ResponsePacket, error_code: {}, {} bytes payload: {}",
            self.error_code,
            self.payload.len(),
            String::from_utf8_lossy(&self.payload)
        )
    }
}

impl Packet for ResponsePacket {
    fn version(&self) -> u8 {
        self.version
    }
    fn payload(&self) -> &[u8] {
        &self.payload
    }
    fn read_from<R: Read>(reader: &mut R) -> Result<Self, IpcError> {
        let version = read_next_vlq(reader)? as u8;
        let error_code = read_next_vlq(reader)?;
        let payload_length = read_next_vlq(reader)?;
        let payload = read_payload(reader, payload_length)?;
        Ok(ResponsePacket {
            version,
            error_code,
            payload,
        })
    }
    fn serialize(&self) -> Vec<u8> {
        let mut buf = vec![];
        buf.extend_from_slice(&vlq_encode(self.version as u64));
        buf.extend_from_slice(&vlq_encode(self.error_code));
        buf.extend_from_slice(&vlq_encode(self.payload.len() as u64));
        buf.extend_from_slice(&self.payload);
        buf
    }
}

impl ResponsePacket {
    pub fn new(error_code: u64, payload: Vec<u8>) -> Self {
        Self {
            version: 0,
            error_code,
            payload,
        }
    }
    pub fn error_code(&self) -> u64 {
        self.error_code
    }
}

pub fn read_next_vlq(reader: &mut impl Read) -> Result<u64, IpcError> {
    let mut buf = [0u8; 10];
    for i in 0..buf.len() {
        let n = reader
            .read(&mut buf[i..i + 1])
            .map_err(|_| IpcError::ReadVlqError)?;
        if n == 0 {
            return Err(IpcError::IncompleteVlqSeq);
        }
        if buf[i] & 0x80 == 0 {
            return vlq_decode(&buf[..i + 1]);
        }
    }
    Err(IpcError::DecodeVlqOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_packets_are_rejected_before_reading_the_payload() {
        for length in [MAX_PAYLOAD_SIZE as u64 + 1, u64::MAX] {
            let mut bytes = vec![0, 0];
            bytes.extend_from_slice(&vlq_encode(length));
            bytes.push(42);
            let mut reader = bytes.as_slice();
            assert!(matches!(
                RequestPacket::read_from(&mut reader),
                Err(IpcError::PayloadTooLarge)
            ));
            assert_eq!(reader, &[42]);
            let mut reader = bytes.as_slice();
            assert!(matches!(
                ResponsePacket::read_from(&mut reader),
                Err(IpcError::PayloadTooLarge)
            ));
            assert_eq!(reader, &[42]);
        }
    }

    #[test]
    fn payload_limit_is_inclusive_and_truncation_is_an_error() {
        let payload = vec![42; MAX_PAYLOAD_SIZE];
        let packet = RequestPacket::new(payload.clone()).serialize();
        assert_eq!(
            RequestPacket::read_from(&mut packet.as_slice())
                .unwrap()
                .payload(),
            payload
        );
        let packet = ResponsePacket::new(0, payload.clone()).serialize();
        assert_eq!(
            ResponsePacket::read_from(&mut packet.as_slice())
                .unwrap()
                .payload(),
            payload
        );
        assert!(matches!(
            RequestPacket::read_from(&mut &[0, 0, 2, 42][..]),
            Err(IpcError::ReadExactError)
        ));
    }

    #[test]
    fn test_read_next_vlq_stops_after_ten_continuation_bytes() {
        struct ContinuationReader(usize);

        impl Read for ContinuationReader {
            fn read(&mut self, buf: &mut [u8]) -> Result<usize, crate::io::Error> {
                assert!(self.0 < 10, "must not read an eleventh byte");
                self.0 += 1;
                buf[0] = 0x80;
                Ok(1)
            }
        }

        let mut reader = ContinuationReader(0);
        assert!(matches!(
            read_next_vlq(&mut reader),
            Err(IpcError::DecodeVlqOverflow)
        ));
        assert_eq!(reader.0, 10);
    }

    #[test]
    fn test_read_next_vlq_tenth_byte_boundaries() {
        for terminal in 0..=u8::MAX {
            let mut bytes = [0x80; 11];
            bytes[9] = terminal;
            bytes[10] = 42;
            let mut reader = bytes.as_slice();
            let result = read_next_vlq(&mut reader);
            match terminal {
                0 => assert_eq!(result.unwrap(), 0),
                1 => assert_eq!(result.unwrap(), 1u64 << 63),
                _ => assert!(matches!(result, Err(IpcError::DecodeVlqOverflow))),
            }
            assert_eq!(reader, &[42]);
        }
    }

    #[test]
    fn test_read_next_vlq_valid_values() {
        for value in [0, 127, 128, 16384, u64::MAX / 2, 1u64 << 63, u64::MAX] {
            let mut bytes = vlq_encode(value);
            bytes.push(42);
            let mut reader = bytes.as_slice();
            assert_eq!(read_next_vlq(&mut reader).unwrap(), value);
            assert_eq!(reader, &[42]);
        }
    }

    #[test]
    fn test_read_next_vlq_incomplete_sequence() {
        let bytes = [0x80; 9];
        for len in 0..=bytes.len() {
            let mut reader = &bytes[..len];
            assert!(matches!(
                read_next_vlq(&mut reader),
                Err(IpcError::IncompleteVlqSeq)
            ));
        }
    }
}
