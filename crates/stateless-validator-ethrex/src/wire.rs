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
