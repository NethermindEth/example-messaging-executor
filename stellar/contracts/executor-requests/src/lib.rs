//! Encoders for Wormhole Executor delivery-request payloads.
//!
//! Each function returns the canonical big-endian byte layout that an
//! integrator passes as the `request` argument to `request_execution`. Pure
//! and `no_std`; wrap the result in `soroban_sdk::Bytes` at the call site.

#![no_std]

extern crate alloc;
use alloc::vec::Vec;

// Request type prefixes.
const REQ_VAA_V1: &[u8; 4] = b"ERV1";
const REQ_NTT_V1: &[u8; 4] = b"ERN1";
const REQ_CCTP_V1: &[u8; 4] = b"ERC1";
const REQ_CCTP_V2: &[u8; 4] = b"ERC2";

/// Encodes a version 1 VAA request payload.
pub fn make_vaa_v1_request(chain: u16, address: [u8; 32], sequence: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 2 + 32 + 8);
    out.extend_from_slice(REQ_VAA_V1);
    out.extend_from_slice(&chain.to_be_bytes());
    out.extend_from_slice(&address);
    out.extend_from_slice(&sequence.to_be_bytes());
    out
}

/// Encodes a version 1 NTT request payload.
pub fn make_ntt_v1_request(
    source_chain: u16,
    source_manager: [u8; 32],
    message_id: [u8; 32],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 2 + 32 + 32);
    out.extend_from_slice(REQ_NTT_V1);
    out.extend_from_slice(&source_chain.to_be_bytes());
    out.extend_from_slice(&source_manager);
    out.extend_from_slice(&message_id);
    out
}

/// Encodes a version 1 CCTP request payload.
pub fn make_cctp_v1_request(source_domain: u32, nonce: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 4 + 8);
    out.extend_from_slice(REQ_CCTP_V1);
    out.extend_from_slice(&source_domain.to_be_bytes());
    out.extend_from_slice(&nonce.to_be_bytes());
    out
}

/// Encodes a version 2 CCTP request payload. The trailing byte flags that the
/// Executor auto-detects the event off-chain.
pub fn make_cctp_v2_request() -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + 1);
    out.extend_from_slice(REQ_CCTP_V2);
    out.push(1); // auto discovery
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDR: [u8; 32] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xd4, 0xa6, 0xa7,
        0x2a, 0x02, 0x55, 0x99, 0xfd, 0x73, 0x57, 0xc0, 0xf1, 0x57, 0xc7, 0x18, 0xd0, 0xf5, 0xe3,
        0x8c, 0x76,
    ];

    #[test]
    fn vaa_v1() {
        assert_eq!(
            make_vaa_v1_request(10002, ADDR, 29),
            [
                0x45, 0x52, 0x56, 0x31, 0x27, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0xd4, 0xa6, 0xa7, 0x2a, 0x02, 0x55, 0x99, 0xfd, 0x73, 0x57,
                0xc0, 0xf1, 0x57, 0xc7, 0x18, 0xd0, 0xf5, 0xe3, 0x8c, 0x76, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x1d
            ]
        );
    }

    #[test]
    fn ntt_v1() {
        let mut message_id = [0u8; 32];
        message_id[24..].copy_from_slice(&29u64.to_be_bytes());
        assert_eq!(
            make_ntt_v1_request(10002, ADDR, message_id),
            [
                0x45, 0x52, 0x4E, 0x31, 0x27, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0xd4, 0xa6, 0xa7, 0x2a, 0x02, 0x55, 0x99, 0xfd, 0x73, 0x57,
                0xc0, 0xf1, 0x57, 0xc7, 0x18, 0xd0, 0xf5, 0xe3, 0x8c, 0x76, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1d
            ]
        );
    }

    #[test]
    fn cctp_v1() {
        assert_eq!(
            make_cctp_v1_request(6, 6344),
            [
                0x45, 0x52, 0x43, 0x31, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x18, 0xc8
            ]
        );
    }

    #[test]
    fn cctp_v2() {
        assert_eq!(make_cctp_v2_request(), [0x45, 0x52, 0x43, 0x32, 0x01]);
    }
}
