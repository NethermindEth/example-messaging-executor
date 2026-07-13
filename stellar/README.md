# Wormhole Executor on Stellar

The Stellar/Soroban implementation of the Wormhole Executor: a stateless,
permissionless on-chain payment rail for cross-chain delivery requests.

A payer submits an off-chain-signed quote (as opaque bytes) plus a request
payload. The contract:

1. parses the 68-byte quote header and checks its chain ids and expiry,
2. requires the payer's authorization and transfers the native token to the
   `payee`,
3. emits a `RequestForExecution` event carrying the full quote verbatim.

Off-chain relayers consume the event and fulfil the delivery on the destination
chain. The contract does **not** verify the quote's signature or bind the payee
— those are off-chain responsibilities (see below).

## Crates

```text
contracts/
├── wormhole-executors/          # the Executor contract
├── executor-soroban-client/     # public interface + error codes
└── executor-requests/           # request-payload encoders
```

| Crate                     | Role                                                                                        |
| ------------------------- | ------------------------------------------------------------------------------------------- |
| `wormhole-executors`      | The contract: validates the quote header, transfers payment, emits `RequestForExecution`.   |
| `executor-soroban-client` | `ExecutorInterface` (the callable surface) and `ExecutorError` codes, for callers/clients.  |
| `executor-requests`       | Pure `no_std` encoders for the `request` payload (VAA v1, NTT v1, CCTP v1/v2).               |

Per-crate API detail lives in the crate rustdoc: `cargo doc -p <crate> --open`.

## Signed-quote wire format

The quote is an opaque byte string. Only the 68-byte header is parsed on-chain;
everything after it is preserved and emitted verbatim so relayers can verify the
body and signature off-chain. All integers are big-endian.

| offset | size | field         | on-chain use                          |
| ------ | ---- | ------------- | ------------------------------------- |
| 0      | 4    | prefix        | e.g. `EQ01` — skipped, not validated  |
| 4      | 20   | quoterAddress | EVM address — emitted in the event    |
| 24     | 32   | payeeAddress  | informational (see below)             |
| 56     | 2    | srcChain      | must equal this deployment's chain id |
| 58     | 2    | dstChain      | must equal the `dst_chain` argument   |
| 60     | 8    | expiryTime    | unix seconds — must be `> now`        |
| 68     | 32   | body          | opaque (EQ01: 4×u64 pricing)          |
| 100    | 65   | signature     | opaque                                |

A quote shorter than 68 bytes is rejected (`InvalidQuote`); everything past the
header is ignored on-chain.

### Payee binding and signature are off-chain

The contract transfers to the `payee` argument and emits the quote's 32-byte
`payeeAddress` verbatim, but it does **not** bind the two, nor verify the
quote's signature — a Soroban `Address` does not expose its raw bytes in
deployed wasm. Both checks are the relayer's off-chain responsibility, which the
verbatim-emitted quote enables.

> Chain ids are 16-bit on the wire but `u32` at the contract ABI, because
> Soroban has no 16-bit value type.

## Build & test

```bash
cd stellar
stellar contract build --optimize   # -> target/wasm32v1-none/release/wormhole_executors.wasm
cargo test --workspace
```

## Integration example

Build a request payload with the `executor-requests` encoders and submit it with
the signed-quote bytes:

```rust
use executor_requests::make_vaa_v1_request;
use soroban_sdk::Bytes;

// emitter_chain: u16, emitter_address: [u8; 32], sequence: u64
let request = Bytes::from_slice(&env, &make_vaa_v1_request(emitter_chain, emitter_address, sequence));

client.request_execution(
    &dst_chain,                 // u32 (Wormhole chain id)
    &dst_addr,                  // BytesN<32>
    &refund,                    // Address
    &payer,                     // Address (authorizes the transfer)
    &payee,                     // Address credited with `amount`
    &amount,                    // i128
    &signed_quote_bytes,        // Bytes (header + body + signature)
    &request,                   // Bytes
    &Bytes::new(&env),          // relay_instructions
);
```

## Deploy

The deploy script selects the network's native XLM Stellar Asset Contract
automatically and passes Stellar's Wormhole chain id (`61`):

```bash
cd stellar
NETWORK=testnet SOURCE=<identity> ./sh/deployExecutor.sh
```

Or invoke the CLI directly (the native SAC differs per network):

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/wormhole_executors.wasm \
  --source-account <identity> \
  --network <network> \
  -- \
  --chain_id 61 \
  --native_token <native SAC for the network>
```

## License

Apache-2.0. See `LICENSE` at the repository root.
