//! Public type definitions for the Wormhole Executor contract.

use soroban_sdk::{Address, BytesN, contracttype};

/// Off-chain quote submitted alongside a delivery request to the Wormhole
/// Executor contract.
///
/// A `SignedQuote` binds a `quoter` and a `payee` to a (`src_chain`,
/// `dst_chain`, `expiry`) tuple so the payer knows who will relay the request,
/// who must be paid, and until when the quote is valid.
///
/// # Authentication is NOT performed on-chain
///
/// Neither the [`prefix`](Self::prefix) domain tag nor the `quoter`'s
/// signature over the quote are verified by the Executor contract. The
/// on-chain Executor treats the struct as untrusted data: only `src_chain`,
/// `dst_chain`, and `expiry` are checked, and the payer's authorization is
/// required for the token transfer.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SignedQuote {
    /// 4-byte domain separator from the off-chain signing scheme, such as
    /// `b"EQ01"`.
    pub prefix: BytesN<4>,
    /// Address of the party that issued and off-chain-signed this quote.
    pub quoter: Address,
    /// Address credited with the native token transfer when
    /// `request_execution` succeeds.
    pub payee: Address,
    /// Wormhole chain id the quote was issued for as the source chain.
    pub src_chain: u32,
    /// Wormhole chain id of the intended destination chain.
    pub dst_chain: u32,
    /// Unix timestamp in seconds after which the quote is no longer valid.
    pub expiry: u64,
}
