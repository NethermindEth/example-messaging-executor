//! Error types for the Wormhole Executor contract.

use soroban_sdk::contracterror;

/// Errors that can occur during Wormhole Executor contract operations.
///
/// Returned by `request_execution` on the Executor contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExecutorError {
    /// The quote header's `expiryTime` is less than or equal to the current
    /// ledger timestamp at the time of the call.
    QuoteExpired = 11,
    /// The quote header's `srcChain` does not match the Wormhole chain id
    /// configured at construction.
    QuoteSrcChainMismatch = 12,
    /// The quote header's `dstChain` does not match the `dst_chain` argument
    /// passed to `request_execution`.
    QuoteDstChainMismatch = 13,
    /// The `amount` argument passed to `request_execution` is negative.
    InvalidAmount = 14,
    /// The signed quote is shorter than the 68-byte header.
    InvalidQuote = 15,
    /// The `payee` argument's on-chain identity does not match the 32-byte
    /// payee the quoter signed at `quote[24..56]`.
    QuotePayeeMismatch = 16,
}
