//! Wormhole Executor contract interface for Stellar/Soroban.
//!
//! This crate provides the public API for interacting with the Wormhole
//! Executor contract implemented by `wormhole-executors`.

#![no_std]

pub mod error;

pub use error::ExecutorError;

use soroban_sdk::{Address, Bytes, BytesN, Env, String, contractclient};

/// Public interface for the Wormhole Executor contract.
///
/// The Executor is a stateless, permissionless cross-chain delivery payment
/// rail. A `payer` submits an off-chain-signed quote (as opaque bytes)
/// alongside a delivery request, the contract validates the quote header,
/// transfers the agreed `amount` of native token from the payer to the
/// `payee`, and emits an event carrying the full quote verbatim for off-chain
/// relayers that fulfill the delivery on the destination chain.
///
/// # Quote authentication and payee binding are OFF-CHAIN
///
/// The contract parses only the 68-byte quote header (chain ids and expiry).
/// It does not verify the quote's signature, nor does it bind `payee` to the
/// payee encoded in the quote header — both are the relayer's off-chain
/// responsibility, which the verbatim-emitted quote enables.
#[contractclient(name = "ExecutorClient")]
pub trait ExecutorInterface {
    /// Returns the Wormhole chain id configured at construction.
    ///
    /// Wormhole chain ids are 16-bit; `u32` is used here only because Soroban's
    /// ABI has no 16-bit value type. The value is always in `0..=u16::MAX`.
    fn chain_id(env: Env) -> u32;

    /// Returns the version string of the Executor implementation.
    fn executor_version(env: Env) -> String;

    /// Records a prepaid cross-chain delivery request.
    ///
    /// Parses the quote header from `signed_quote_bytes`, requires the payer's
    /// authorization, transfers `amount` native tokens from `payer` to
    /// `payee`, and emits a `RequestForExecution` event carrying the full
    /// quote bytes for off-chain relayers.
    #[allow(clippy::too_many_arguments)]
    fn request_execution(
        env: Env,
        dst_chain: u32,
        dst_addr: BytesN<32>,
        refund: Address,
        payer: Address,
        payee: Address,
        amount: i128,
        signed_quote_bytes: Bytes,
        request: Bytes,
        relay_instructions: Bytes,
    ) -> Result<(), ExecutorError>;
}
