//! EIP-8025 framing helpers for the ethrex guest path.
//!
//! The wire format is version-prefixed so a single ethrex guest binary can
//! decode both the legacy Electra/Fulu and the canonical-input
//! payloads. The leading byte is consumed by the dispatcher in
//! `ethrex_guest_program::l1::decode_eip8025`.

use alloc::vec::Vec;

use libssz::SszEncode;
use stateless_validator_common::new_payload_request::NewPayloadRequestElectraFulu;

/// Wire-format version byte for the legacy framing.
pub const EIP8025_VERSION_LEGACY: u8 = 0x00;

/// Wire-format version byte for the canonical-input framing.
pub const EIP8025_VERSION_CANONICAL: u8 = 0x01;

/// Encodes an Electra/Fulu new payload request and opaque witness bytes as
/// `[version=0x00][ssz_len: u32 LE][ssz_bytes][rkyv_bytes]`.
pub fn encode_eip8025(npr: &NewPayloadRequestElectraFulu, rkyv_witness_bytes: &[u8]) -> Vec<u8> {
    let ssz_bytes = npr.to_ssz();
    let ssz_len = u32::try_from(ssz_bytes.len()).expect("SSZ payload length exceeds u32");

    let mut out = Vec::with_capacity(1 + 4 + ssz_bytes.len() + rkyv_witness_bytes.len());
    out.push(EIP8025_VERSION_LEGACY);
    out.extend_from_slice(&ssz_len.to_le_bytes());
    out.extend_from_slice(&ssz_bytes);
    out.extend_from_slice(rkyv_witness_bytes);
    out
}

/// Encodes a canonical EEST `statelessInputBytes` payload paired with the
/// rkyv-encoded ethrex `ChainConfig` as
/// `[version=0x01][ssz_len: u32 LE][ssz_bytes][cfg_len: u32 LE][rkyv_chain_config_bytes]`.
pub fn encode_eip8025_canonical(ssz_input: &[u8], rkyv_chain_config_bytes: &[u8]) -> Vec<u8> {
    let ssz_len = u32::try_from(ssz_input.len()).expect("SSZ input length exceeds u32");
    let cfg_len =
        u32::try_from(rkyv_chain_config_bytes.len()).expect("chain config length exceeds u32");

    let mut out = Vec::with_capacity(1 + 4 + ssz_input.len() + 4 + rkyv_chain_config_bytes.len());
    out.push(EIP8025_VERSION_CANONICAL);
    out.extend_from_slice(&ssz_len.to_le_bytes());
    out.extend_from_slice(ssz_input);
    out.extend_from_slice(&cfg_len.to_le_bytes());
    out.extend_from_slice(rkyv_chain_config_bytes);
    out
}
