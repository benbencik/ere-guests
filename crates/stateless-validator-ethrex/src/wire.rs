//! EIP-8025 framing helpers for the ethrex guest path.

use alloc::vec::Vec;

use libssz::SszEncode;
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

/// Encodes a canonical EEST `statelessInputBytes` payload paired with the
/// rkyv-encoded ethrex `ChainConfig` as
/// `[ssz_len: u32 LE][ssz_bytes][cfg_len: u32 LE][rkyv_chain_config_bytes]`.
pub fn encode_eip8025_canonical(ssz_input: &[u8], rkyv_chain_config_bytes: &[u8]) -> Vec<u8> {
    let ssz_len = u32::try_from(ssz_input.len()).expect("SSZ input length exceeds u32");
    let cfg_len =
        u32::try_from(rkyv_chain_config_bytes.len()).expect("chain config length exceeds u32");

    let mut out = Vec::with_capacity(4 + ssz_input.len() + 4 + rkyv_chain_config_bytes.len());
    out.extend_from_slice(&ssz_len.to_le_bytes());
    out.extend_from_slice(ssz_input);
    out.extend_from_slice(&cfg_len.to_le_bytes());
    out.extend_from_slice(rkyv_chain_config_bytes);
    out
}
