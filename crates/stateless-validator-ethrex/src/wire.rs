//! EIP-8025 framing helpers for the ethrex guest path.

use alloc::vec::Vec;
use ethrex_common::types::block_execution_witness::ExecutionWitness;
use libssz::SszEncode;
use stateless_validator_common::new_payload_request::NewPayloadRequestElectraFulu;

/// Encodes an Electra/Fulu new payload request and opaque witness bytes as
/// `[ssz_len: u32 LE][ssz_bytes][witness_ssz_bytes]`.
pub fn encode_eip8025(
    npr: &NewPayloadRequestElectraFulu,
    execution_witness: &ExecutionWitness,
) -> Result<Vec<u8>, ethrex_common::types::block_execution_witness::ExecutionWitnessSszError> {
    let ssz_bytes = npr.to_ssz();
    let ssz_len = u32::try_from(ssz_bytes.len()).expect("SSZ payload length exceeds u32");
    let witness_ssz_bytes = execution_witness.to_ssz_bytes()?;

    let mut out = Vec::with_capacity(4 + ssz_bytes.len() + witness_ssz_bytes.len());
    out.extend_from_slice(&ssz_len.to_le_bytes());
    out.extend_from_slice(&ssz_bytes);
    out.extend_from_slice(&witness_ssz_bytes);
    Ok(out)
}
