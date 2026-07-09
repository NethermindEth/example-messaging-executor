use super::*;
use soroban_sdk::{
    Address, Bytes, BytesN, Env, Event, String, contract, contractimpl, contracttype,
    testutils::{Address as _, Events, Ledger},
};

#[contracttype]
#[derive(Clone)]
enum MockTokenStorage {
    Balance(Address),
}

#[contract]
struct MockNativeToken;

#[contractimpl]
impl MockNativeToken {
    pub fn set_balance(env: Env, id: Address, amount: i128) {
        env.storage()
            .persistent()
            .set(&MockTokenStorage::Balance(id), &amount);
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&MockTokenStorage::Balance(id))
            .unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        let from_balance = Self::balance(env.clone(), from.clone());
        assert!(amount >= 0, "negative transfer");
        assert!(from_balance >= amount, "insufficient");

        let to_balance = Self::balance(env.clone(), to.clone());
        Self::set_balance(env.clone(), from, from_balance - amount);
        Self::set_balance(env, to, to_balance + amount);
    }
}

fn install_native_token_mock(env: &Env) -> MockNativeTokenClient<'_> {
    let native = Address::generate(env);
    env.register_at(&native, MockNativeToken, ());
    MockNativeTokenClient::new(env, &native)
}

fn register_executor<'a>(
    env: &'a Env,
    chain_id: u16,
    native_token: &Address,
) -> ExecutorClient<'a> {
    let exec_addr = env.register(Executor, (&u32::from(chain_id), native_token));
    ExecutorClient::new(env, &exec_addr)
}

/// Assembles raw quote bytes: prefix(4) quoter(20) payee(32) src(2 be)
/// dst(2 be) expiry(8 be), followed by an opaque `tail` (EQ01 body + signature)
/// that the contract must preserve verbatim.
fn build_quote(
    env: &Env,
    quoter: &[u8; 20],
    payee32: &[u8; 32],
    src: u16,
    dst: u16,
    expiry: u64,
    tail: &[u8],
) -> Bytes {
    let mut header = [0u8; 68];
    header[0..4].copy_from_slice(b"EQ01");
    header[4..24].copy_from_slice(quoter);
    header[24..56].copy_from_slice(payee32);
    header[56..58].copy_from_slice(&src.to_be_bytes());
    header[58..60].copy_from_slice(&dst.to_be_bytes());
    header[60..68].copy_from_slice(&expiry.to_be_bytes());

    let mut bytes = Bytes::from_slice(env, &header);
    bytes.append(&Bytes::from_slice(env, tail));
    bytes
}

#[test]
fn init_roundtrip_and_version() {
    let env = Env::default();

    let client = register_executor(&env, 1234, &Address::generate(&env));

    assert_eq!(client.chain_id(), 1234);
    assert_eq!(
        client.executor_version(),
        String::from_str(&env, EXECUTOR_VERSION)
    );
}

#[test]
fn happy_path() {
    let env = Env::default();
    env.mock_all_auths();
    let native = install_native_token_mock(&env);

    let src_chain = 1234u16;
    let dst_chain = 4321u16;
    let payer = Address::generate(&env);
    let payee = Address::generate(&env);
    let refund = Address::generate(&env);
    let dst_addr = BytesN::<32>::from_array(&env, &[9u8; 32]);
    let amount = 250i128;

    native.set_balance(&payer, &1_000);

    let client = register_executor(&env, src_chain, &native.address);
    let quote = build_quote(
        &env,
        &[7u8; 20],
        &[0u8; 32],
        src_chain,
        dst_chain,
        env.ledger().timestamp() + 600,
        &[],
    );
    let request = Bytes::from_slice(&env, b"any-request-bytes");
    let relay_instructions = Bytes::from_slice(&env, &[0xCA, 0xFE]);

    client.request_execution(
        &u32::from(dst_chain),
        &dst_addr,
        &refund,
        &payer,
        &payee,
        &amount,
        &quote,
        &request,
        &relay_instructions,
    );

    let expected = RequestForExecution {
        quoter_address: BytesN::from_array(&env, &[7u8; 20]),
        amt_paid: amount,
        dst_chain: u32::from(dst_chain),
        dst_addr,
        refund_addr: refund,
        signed_quote: quote,
        request,
        relay_instructions,
    };

    assert_eq!(env.events().all(), [expected.to_xdr(&env, &client.address)]);
    assert_eq!(native.balance(&payer), 750);
    assert_eq!(native.balance(&payee), amount);
}

/// A full 165-byte EQ01 quote (68 header + 32 body + 65 signature) must be
/// emitted byte-for-byte, proving the body and signature survive for off-chain
/// verification, and the 20-byte quoter must be lifted out of the header.
#[test]
fn event_preserves_full_quote_bytes() {
    let env = Env::default();
    env.mock_all_auths();
    let native = install_native_token_mock(&env);

    let src_chain = 61u16;
    let dst_chain = 2u16;
    let payer = Address::generate(&env);
    let payee = Address::generate(&env);
    native.set_balance(&payer, &10);

    let quoter = [0xABu8; 20];
    let tail: [u8; 97] = core::array::from_fn(|i| i as u8); // 32-byte body + 65-byte signature
    let quote = build_quote(
        &env,
        &quoter,
        &[0xCD; 32],
        src_chain,
        dst_chain,
        env.ledger().timestamp() + 1,
        &tail,
    );
    assert_eq!(quote.len(), 165);

    let client = register_executor(&env, src_chain, &native.address);
    let dst_addr = BytesN::<32>::from_array(&env, &[1u8; 32]);
    let refund = Address::generate(&env);

    client.request_execution(
        &u32::from(dst_chain),
        &dst_addr,
        &refund,
        &payer,
        &payee,
        &5,
        &quote,
        &Bytes::new(&env),
        &Bytes::new(&env),
    );

    let expected = RequestForExecution {
        quoter_address: BytesN::from_array(&env, &quoter),
        amt_paid: 5,
        dst_chain: u32::from(dst_chain),
        dst_addr,
        refund_addr: refund,
        signed_quote: quote,
        request: Bytes::new(&env),
        relay_instructions: Bytes::new(&env),
    };
    assert_eq!(env.events().all(), [expected.to_xdr(&env, &client.address)]);
}

#[test]
fn accepts_empty_request_and_relay_instructions() {
    let env = Env::default();
    env.mock_all_auths();
    let native = install_native_token_mock(&env);

    let src_chain = 10u16;
    let dst_chain = 20u16;
    let payer = Address::generate(&env);
    let payee = Address::generate(&env);

    let client = register_executor(&env, src_chain, &native.address);
    let quote = build_quote(
        &env,
        &[0u8; 20],
        &[0u8; 32],
        src_chain,
        dst_chain,
        env.ledger().timestamp() + 60,
        &[],
    );

    let res = client.try_request_execution(
        &u32::from(dst_chain),
        &BytesN::<32>::from_array(&env, &[1u8; 32]),
        &Address::generate(&env),
        &payer,
        &payee,
        &0,
        &quote,
        &Bytes::new(&env),
        &Bytes::new(&env),
    );

    assert_eq!(res, Ok(Ok(())));
    assert_eq!(native.balance(&payee), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")] // InvalidQuote
fn rejects_quote_shorter_than_header() {
    let env = Env::default();
    env.mock_all_auths();
    let native = install_native_token_mock(&env);

    let client = register_executor(&env, 1, &native.address);

    client.request_execution(
        &2,
        &BytesN::<32>::from_array(&env, &[0u8; 32]),
        &Address::generate(&env),
        &Address::generate(&env),
        &Address::generate(&env),
        &1,
        &Bytes::from_slice(&env, &[0u8; 67]),
        &Bytes::new(&env),
        &Bytes::new(&env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")] // QuoteExpired (boundary: strict >)
fn rejects_expired_quote() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000);
    let native = install_native_token_mock(&env);

    let src_chain = 111u16;
    let dst_chain = 222u16;
    let client = register_executor(&env, src_chain, &native.address);
    let quote = build_quote(
        &env,
        &[0u8; 20],
        &[0u8; 32],
        src_chain,
        dst_chain,
        1000,
        &[],
    );

    client.request_execution(
        &u32::from(dst_chain),
        &BytesN::<32>::from_array(&env, &[3u8; 32]),
        &Address::generate(&env),
        &Address::generate(&env),
        &Address::generate(&env),
        &1,
        &quote,
        &Bytes::new(&env),
        &Bytes::new(&env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")] // QuoteSrcChainMismatch
fn rejects_src_chain_mismatch() {
    let env = Env::default();
    env.mock_all_auths();
    let native = install_native_token_mock(&env);

    let dst_chain = 88u16;
    let client = register_executor(&env, 9999, &native.address);
    let quote = build_quote(
        &env,
        &[0u8; 20],
        &[0u8; 32],
        77,
        dst_chain,
        env.ledger().timestamp() + 600,
        &[],
    );

    client.request_execution(
        &u32::from(dst_chain),
        &BytesN::<32>::from_array(&env, &[4u8; 32]),
        &Address::generate(&env),
        &Address::generate(&env),
        &Address::generate(&env),
        &1,
        &quote,
        &Bytes::new(&env),
        &Bytes::new(&env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")] // QuoteDstChainMismatch
fn rejects_dst_chain_mismatch() {
    let env = Env::default();
    env.mock_all_auths();
    let native = install_native_token_mock(&env);

    let src_chain = 55u16;
    let client = register_executor(&env, src_chain, &native.address);
    let quote = build_quote(
        &env,
        &[0u8; 20],
        &[0u8; 32],
        src_chain,
        66,
        env.ledger().timestamp() + 600,
        &[],
    );

    client.request_execution(
        &99,
        &BytesN::<32>::from_array(&env, &[5u8; 32]),
        &Address::generate(&env),
        &Address::generate(&env),
        &Address::generate(&env),
        &1,
        &quote,
        &Bytes::new(&env),
        &Bytes::new(&env),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")] // InvalidAmount
fn rejects_negative_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let native = install_native_token_mock(&env);

    let src_chain = 20u16;
    let dst_chain = 30u16;
    let client = register_executor(&env, src_chain, &native.address);
    let quote = build_quote(
        &env,
        &[0u8; 20],
        &[0u8; 32],
        src_chain,
        dst_chain,
        env.ledger().timestamp() + 600,
        &[],
    );

    client.request_execution(
        &u32::from(dst_chain),
        &BytesN::<32>::from_array(&env, &[9u8; 32]),
        &Address::generate(&env),
        &Address::generate(&env),
        &Address::generate(&env),
        &-1,
        &quote,
        &Bytes::new(&env),
        &Bytes::new(&env),
    );
}
