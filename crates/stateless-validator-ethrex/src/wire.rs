//! EIP-8025 framing helpers for the ethrex guest path.

use alloc::vec::Vec;
use core::fmt;

use libssz::{SszDecode, SszEncode};
use stateless_validator_common::new_payload_request::NewPayloadRequestElectraFulu;

/// Encodes an Electra/Fulu new payload request and opaque witness bytes as
/// `[ssz_len: u32 LE][ssz_bytes][rkyv_bytes]`.
pub fn encode_eip8025(npr: &NewPayloadRequestElectraFulu, rkyv_witness_bytes: &[u8]) -> Vec<u8> {
    let ssz_bytes = npr.to_ssz();
    let ssz_len = u32::try_from(ssz_bytes.len()).expect("SSZ payload length exceeds u32");

    let mut out = Vec::with_capacity(4 + ssz_bytes.len() + rkyv_witness_bytes.len());
    out.extend_from_slice(&ssz_len.to_le_bytes());
    out.extend_from_slice(&ssz_bytes);
    out.extend_from_slice(rkyv_witness_bytes);
    out
}

/// Decodes `[ssz_len: u32 LE][ssz_bytes][rkyv_bytes]` into an Electra/Fulu new
/// payload request and the remaining witness bytes.
pub fn decode_eip8025(bytes: &[u8]) -> Result<(NewPayloadRequestElectraFulu, &[u8]), WireError> {
    if bytes.len() < 4 {
        return Err(WireError::TooShort);
    }

    let ssz_len = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    if bytes.len() < 4 + ssz_len {
        return Err(WireError::TooShort);
    }

    let ssz_bytes = &bytes[4..4 + ssz_len];
    let rkyv_bytes = &bytes[4 + ssz_len..];
    let npr = NewPayloadRequestElectraFulu::from_ssz_bytes(ssz_bytes).map_err(WireError::Ssz)?;

    Ok((npr, rkyv_bytes))
}

/// Errors returned while decoding the local EIP-8025 framing.
#[derive(Debug)]
pub enum WireError {
    /// The framing header or SSZ segment is truncated.
    TooShort,
    /// The SSZ payload did not decode as `NewPayloadRequestElectraFulu`.
    Ssz(libssz::DecodeError),
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => write!(f, "input too short"),
            Self::Ssz(err) => write!(f, "SSZ decode error: {err:?}"),
        }
    }
}

impl core::error::Error for WireError {}

#[cfg(test)]
mod tests {
    use libssz::SszEncode;
    use stateless_validator_common::new_payload_request::{
        ExecutionPayloadV3, NewPayloadRequestElectraFulu,
    };

    use super::{decode_eip8025, encode_eip8025, WireError};

    fn sample_request() -> NewPayloadRequestElectraFulu {
        NewPayloadRequestElectraFulu {
            execution_payload: ExecutionPayloadV3 {
                parent_hash: [1; 32],
                fee_recipient: [2; 20],
                state_root: [3; 32],
                receipts_root: [4; 32],
                logs_bloom: [5; 256],
                prev_randao: [6; 32],
                block_number: 7,
                gas_limit: 8,
                gas_used: 9,
                timestamp: 10,
                extra_data: Default::default(),
                base_fee_per_gas: [11; 32],
                block_hash: [12; 32],
                transactions: Default::default(),
                withdrawals: Default::default(),
                blob_gas_used: 13,
                excess_blob_gas: 14,
            },
            versioned_hashes: Default::default(),
            parent_beacon_block_root: [15; 32],
            execution_requests: Default::default(),
        }
    }

    #[test]
    fn roundtrips_wire_format() {
        let request = sample_request();
        let witness = [0xaa, 0xbb, 0xcc, 0xdd];

        let encoded = encode_eip8025(&request, &witness);
        let (decoded_request, decoded_witness) = decode_eip8025(&encoded).unwrap();

        assert_eq!(decoded_request.to_ssz(), request.to_ssz());
        assert_eq!(decoded_witness, witness);
    }

    #[test]
    fn rejects_truncated_wire_header() {
        assert!(matches!(decode_eip8025(&[]), Err(WireError::TooShort)));
        assert!(matches!(
            decode_eip8025(&[0, 1, 2]),
            Err(WireError::TooShort)
        ));
    }
}
