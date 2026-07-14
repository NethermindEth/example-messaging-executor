#!/usr/bin/env bash
#
# Builds the optimized Executor WASM and deploys it to a Stellar network.
#
# Usage: NETWORK=testnet SOURCE=my-identity ./sh/deployExecutor.sh
#
# The constructor needs the network's native XLM Stellar Asset Contract, whose
# id is derived from the network passphrase and so differs per network. It is
# chosen automatically below, or set NATIVE_TOKEN to override.

set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:-default}"
CHAIN_ID="${CHAIN_ID:-61}"

if [ -z "${NATIVE_TOKEN:-}" ]; then
  case "$NETWORK" in
    mainnet | public) NATIVE_TOKEN=CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA ;;
    testnet) NATIVE_TOKEN=CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC ;;
    *)
      echo "Unknown NETWORK '$NETWORK'; set NATIVE_TOKEN explicitly." >&2
      exit 1
      ;;
  esac
fi

cd "$(dirname "$0")/.."

stellar contract build --optimize

stellar contract deploy \
  --wasm target/wasm32v1-none/release/wormhole_executors.wasm \
  --source "$SOURCE" \
  --network "$NETWORK" \
  -- \
  --chain_id "$CHAIN_ID" \
  --native_token "$NATIVE_TOKEN"
