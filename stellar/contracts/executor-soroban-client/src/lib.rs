//! Wormhole Executor contract interface for Stellar/Soroban.
//!
//! This crate provides the public API for interacting with the Wormhole
//! Executor contract implemented by `wormhole-executors`.

#![no_std]

pub mod constants;
pub mod error;
pub mod types;

pub use constants::*;
pub use error::ExecutorError;
pub use types::*;

use soroban_sdk::{Address, Bytes, BytesN, Env, String, contractclient};

/// Public interface for the Wormhole Executor contract.
///
/// The Executor is a prepaid cross-chain delivery payment rail. A `payer`
/// submits a [`SignedQuote`] alongside a delivery request, the contract
/// validates the quote, transfers the agreed `amount` of native token from
/// the payer to the quote's `payee`, and emits an event consumed by off-chain
/// relayers that fulfill the delivery on the destination chain.
///
/// # Quote authentication is NOT performed on-chain
///
/// Despite its name, [`SignedQuote`] is not verified by the Executor. Neither
/// the [`SignedQuote::prefix`] domain tag nor the `quoter`'s signature over
/// the quote are checked on chain. Quote authentication is a caller-side
/// responsibility, performed off-chain before the transaction is submitted.
#[contractclient(name = "ExecutorClient")]
pub trait ExecutorInterface {
    /// Returns the Wormhole chain id configured at construction.
    fn chain_id(env: Env) -> u32;

    /// Returns the version string of the Executor implementation.
    fn executor_version(env: Env) -> String;

    /// Records a prepaid cross-chain delivery request.
    ///
    /// Validates the [`SignedQuote`], requires the payer's authorization,
    /// transfers `amount` native tokens from `payer` to
    /// `signed_quote.payee`, and emits a `RequestForExecution` event consumed
    /// by off-chain relayers.
    #[allow(clippy::too_many_arguments)]
    fn request_execution(
        env: Env,
        dst_chain: u32,
        dst_addr_wa32: BytesN<32>,
        refund: Address,
        payer: Address,
        amount: i128,
        signed_quote: SignedQuote,
        request: Bytes,
        relay_instructions: Bytes,
    ) -> Result<(), ExecutorError>;
}
