//! Wormhole Executor contract implementation for Stellar/Soroban.
//!
//! The Executor is a stateless, permissionless payment rail: a `payer` prepays
//! a relayer to execute a cross-chain delivery request on a destination chain.
//! The relaying happens off-chain; this contract only records the payment and
//! emits the event that off-chain relayers consume.
//!
//! # On-chain responsibilities
//!
//! On a call to [`ExecutorInterface::request_execution`] it:
//!
//! 1. Accepts the signed quote as opaque bytes plus a delivery request from the
//!    payer, and parses only the 68-byte quote header by byte offset.
//! 2. Validates basic sanity: `amount >= 0`, the quote is at least a full
//!    header, the header's `srcChain` matches the chain id configured at
//!    construction, the header's `dstChain` matches the `dst_chain` argument,
//!    and the quote has not expired relative to the current ledger timestamp.
//! 3. Requires the `payer`'s authorization, then transfers `amount` of the
//!    native token (via the Stellar Asset Contract configured at construction)
//!    from `payer` to `payee`.
//! 4. Emits a [`RequestForExecution`] event carrying the full quote verbatim so
//!    off-chain relayers can verify it and fulfil the delivery.
//!
//! # Quote authentication and payee binding are OFF-CHAIN
//!
//! The contract does not verify the quote's signature, nor does it bind the
//! `payee` argument to the 32-byte payee inside the quote header: a Soroban
//! `Address` does not expose its raw bytes in deployed wasm. Both the signature
//! and the `payee`/`quote[24..56]` binding are the relayer's off-chain
//! responsibility, which the verbatim-emitted quote enables.
//!
//! # Architecture
//!
//! - [`Executor`] - Main contract struct implementing [`ExecutorInterface`]
//! - [`ExecutorInterface`] - Public interface (re-exported from
//!   `executor-soroban-client`)
//! - [`RequestForExecution`] - Event emitted on successful requests
//! - [`ExecutorError`] - Error codes returned by `request_execution`

#![no_std]

use executor_soroban_client::{ExecutorError, ExecutorInterface};
use soroban_sdk::{
    Address, Bytes, BytesN, Env, String, contract, contractevent, contractimpl, contracttype, token,
};

#[cfg(test)]
mod tests;

const EXECUTOR_VERSION: &str = "Executor-0.0.1";

/// Minimum quote length: the header that is parsed on-chain. Everything past
/// it (EQ01 body and signature) is opaque and emitted verbatim.
const QUOTE_HEADER_LEN: u32 = 68;

/// Instance storage keys for the [`Executor`] contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Wormhole chain id of this deployment, set once by
    /// [`Executor::__constructor`] and read on every call to
    /// [`ExecutorInterface::request_execution`] to validate the quote header's
    /// `srcChain`.
    ChainId,
    /// Native-token Stellar Asset Contract address, set once by
    /// [`Executor::__constructor`] and used to transfer payment from `payer`
    /// to `payee`.
    NativeToken,
}

/// Event emitted when a payer successfully requests a cross-chain delivery.
///
/// Published by [`ExecutorInterface::request_execution`] after the native
/// token transfer has completed. Off-chain relayers subscribe to this event to
/// learn that they have been paid and must fulfil the associated delivery on
/// the destination chain.
///
/// The event is published with topics `["Executor", "RequestForExecution"]`.
#[contractevent(topics = ["Executor", "RequestForExecution"])]
#[derive(Clone)]
pub struct RequestForExecution {
    /// 20-byte EVM identity of the quoter, read from the quote header.
    pub quoter_address: BytesN<20>,
    /// Amount of native token (stroops) paid by the payer to `payee`.
    pub amt_paid: i128,
    /// Wormhole chain id of the destination chain (16-bit value in a `u32`;
    /// Soroban's ABI has no 16-bit type).
    pub dst_chain: u32,
    /// 32-byte destination address in Wormhole's left-zero-padded encoding.
    pub dst_addr: BytesN<32>,
    /// Address the off-chain relayer should refund on failure. Pass-through.
    pub refund_addr: Address,
    /// The full signed quote submitted by the payer (header, body, and
    /// signature), recorded verbatim for off-chain verification.
    pub signed_quote: Bytes,
    /// Opaque delivery request payload, defined by the off-chain protocol.
    pub request: Bytes,
    /// Opaque relaying instructions, defined by the off-chain protocol.
    pub relay_instructions: Bytes,
}

fn read_u16_be(b: &Bytes, at: u32) -> u16 {
    let mut buf = [0u8; 2];
    b.slice(at..at + 2).copy_into_slice(&mut buf);
    u16::from_be_bytes(buf)
}

fn read_u64_be(b: &Bytes, at: u32) -> u64 {
    let mut buf = [0u8; 8];
    b.slice(at..at + 8).copy_into_slice(&mut buf);
    u64::from_be_bytes(buf)
}

fn read_quoter(env: &Env, b: &Bytes) -> BytesN<20> {
    let mut buf = [0u8; 20];
    b.slice(4..24).copy_into_slice(&mut buf);
    BytesN::from_array(env, &buf)
}

/// Wormhole Executor contract for Stellar/Soroban.
///
/// Implements [`ExecutorInterface`]. See the crate-level documentation for the
/// Executor's role as a prepaid cross-chain delivery payment rail and for the
/// important note that quote authentication and payee binding are **not**
/// performed on chain.
#[contract]
pub struct Executor;

#[contractimpl]
impl Executor {
    /// Constructor called atomically during contract deployment.
    ///
    /// Stores the Wormhole `chain_id` and native-token address of this
    /// deployment in instance storage. `chain_id` is read on every call to
    /// [`ExecutorInterface::request_execution`] to validate the quote header's
    /// `srcChain`; `native_token` is the Stellar Asset Contract used to
    /// transfer payment.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - Wormhole chain id this Executor instance runs on (16-bit
    ///   value passed as `u32`; Soroban's ABI has no 16-bit type).
    /// * `native_token` - Stellar Asset Contract address for the native token.
    pub fn __constructor(env: Env, chain_id: u32, native_token: Address) {
        env.storage().instance().set(&DataKey::ChainId, &chain_id);
        env.storage()
            .instance()
            .set(&DataKey::NativeToken, &native_token);
    }
}

#[contractimpl]
impl ExecutorInterface for Executor {
    fn chain_id(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::ChainId).unwrap()
    }

    fn executor_version(env: Env) -> String {
        String::from_str(&env, EXECUTOR_VERSION)
    }

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
    ) -> Result<(), ExecutorError> {
        if amount < 0 {
            return Err(ExecutorError::InvalidAmount);
        }
        if signed_quote_bytes.len() < QUOTE_HEADER_LEN {
            return Err(ExecutorError::InvalidQuote);
        }
        // Widen the 16-bit wire fields to u32 for comparison; never truncate
        // the u32 args down to u16, which would alias out-of-range ids.
        if u32::from(read_u16_be(&signed_quote_bytes, 56)) != Self::chain_id(env.clone()) {
            return Err(ExecutorError::QuoteSrcChainMismatch);
        }
        if u32::from(read_u16_be(&signed_quote_bytes, 58)) != dst_chain {
            return Err(ExecutorError::QuoteDstChainMismatch);
        }
        if read_u64_be(&signed_quote_bytes, 60) <= env.ledger().timestamp() {
            return Err(ExecutorError::QuoteExpired);
        }

        payer.require_auth();

        let native_token: Address = env.storage().instance().get(&DataKey::NativeToken).unwrap();
        token::TokenClient::new(&env, &native_token).transfer(&payer, &payee, &amount);

        RequestForExecution {
            quoter_address: read_quoter(&env, &signed_quote_bytes),
            amt_paid: amount,
            dst_chain,
            dst_addr,
            refund_addr: refund,
            signed_quote: signed_quote_bytes,
            request,
            relay_instructions,
        }
        .publish(&env);

        Ok(())
    }
}
