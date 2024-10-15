use alloc::vec::Vec;

use crate::error::IpcError;
/// Encodes an integer using VLQ (Variable-Length Quantity) encoding.
pub fn vlq_encode(mut value: u64) -> Vec<u8> {
    let mut buffer = Vec::new();
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buffer.push(byte);
        if value == 0 {
            break;
        }
    }
    buffer
}

/// Decodes a VLQ (Variable-Length Quantity) encoded byte slice into an integer.
pub fn vlq_decode(bytes: &[u8]) -> Result<u64, IpcError> {
    let mut value = 0u64;
    let mut shift = 0;
    for &byte in bytes {
        value |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        shift += 7;
        if shift >= 64 {
            return Err(IpcError::DecodeVlqOverflow);
        }
    }
    Err(IpcError::IncompleteVlqSeq)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn test_vlq_encode() {
        assert_eq!(vlq_encode(0), vec![0]);
        assert_eq!(vlq_encode(127), vec![127]);
        assert_eq!(vlq_encode(128), vec![128, 1]);
        assert_eq!(vlq_encode(16384), vec![128, 128, 1]);
        assert_eq!(
            vlq_encode(u64::MAX),
            vec![255, 255, 255, 255, 255, 255, 255, 255, 255, 1]
        );
    }

    #[test]
    fn test_vlq_decode() {
        assert_eq!(vlq_decode(&[0]).unwrap(), 0);
        assert_eq!(vlq_decode(&[127]).unwrap(), 127);
        assert_eq!(vlq_decode(&[128, 1]).unwrap(), 128);
        assert_eq!(vlq_decode(&[128, 128, 1]).unwrap(), 16384);
        assert_eq!(
            vlq_decode(&[255, 255, 255, 255, 255, 255, 255, 255, 255, 1]).unwrap(),
            u64::MAX
        );
    }

    #[test]
    fn test_vlq_encode_decode_roundtrip() {
        let test_values = vec![0, 1, 127, 128, 16383, 16384, u64::MAX / 2, u64::MAX];
        for value in test_values {
            let encoded = vlq_encode(value);
            let decoded = vlq_decode(&encoded).unwrap();
            assert_eq!(decoded, value, "Roundtrip failed for value: {}", value);
        }
    }

    #[test]
    fn test_vlq_decode_errors() {
        assert!(matches!(
            vlq_decode(&[128, 128, 128, 128, 128, 128, 128, 128, 128, 128]),
            Err(IpcError::DecodeVlqOverflow)
        ));
        assert!(matches!(
            vlq_decode(&[128]),
            Err(IpcError::IncompleteVlqSeq)
        ));
    }
}
