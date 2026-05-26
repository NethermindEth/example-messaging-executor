# Wormhole Executor on Stellar

This directory contains the Stellar/Soroban implementation of the Wormhole
Executor contract.

The Executor is a small on-chain payment rail for cross-chain delivery
requests. A payer submits an off-chain quote and request payload, the contract
checks the quote chain ids and expiry, transfers native XLM to the quote payee,
and emits a `RequestForExecution` event for off-chain relayers.

## Repository Structure

```text
contracts/
├── wormhole-executors/          # Executor contract implementation
└── executor-soroban-client/     # Public interface crate (ExecutorInterface, SignedQuote, errors)
```

## Build

```bash
cd stellar/contracts/wormhole-executors
stellar contract build --optimize
```

## Test

```bash
cd stellar
cargo test -p wormhole-executors
```

## Deploy

Build the optimized WASM and deploy it with the Stellar CLI, passing the
Wormhole chain id for Stellar (`61`) to the constructor.

```bash
cd stellar/contracts/wormhole-executors
stellar contract build --optimize

stellar contract deploy \
  --wasm ../../target/wasm32v1-none/release/wormhole_executors.wasm \
  --source-account <identity> \
  --network <network> \
  -- \
  --chain_id 61
```

## License

Apache-2.0. See `LICENSE` at the repository root.
