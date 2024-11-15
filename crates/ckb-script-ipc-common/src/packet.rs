use alloc::vec;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter, Result as FmtResult};
use hex;

use crate::error::IpcError;
use crate::vlq::{vlq_decode, vlq_encode};
use ckb_rust_std::io::Read;

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
        write!(f, "RequestPacket, payload: {}", hex::encode(&self.payload))
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
        let mut payload = vec![0u8; payload_length as usize];
        reader
            .read_exact(&mut payload[..])
            .map_err(|_| IpcError::ReadExactError)?;
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
            "ResponsePacket, error_code: {}, payload: {}",
            self.error_code,
            hex::encode(&self.payload)
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
        let mut payload = vec![0u8; payload_length as usize];
        reader
            .read_exact(&mut payload[..])
            .map_err(|_| IpcError::ReadExactError)?;
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
    let mut peek = [0u8; 1];
    let mut buf = vec![];
    loop {
        let n = reader.read(&mut peek).map_err(|_| IpcError::ReadVlqError)?;
        if n == 0 {
            break;
        }
        buf.push(peek[0]);
        if peek[0] & 0x80 == 0 {
            break;
        }
    }
    vlq_decode(&buf)
}
