#!/bin/bash
# Localnet check of the escrow program: builds the SBF program, starts solana-test-validator
# with it, then runs the adapter scenarios against it. Run inside WSL (Ubuntu), see chain/README.md.
set -u
export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"
REPO="${SIL_REPO:-$(cd "$(dirname "$0")/../.." && pwd)}"
KEYS="${SIL_KEYS:-$HOME/sil-localnet}"
RPC_PORT="${SIL_RPC_PORT:-8899}"
mkdir -p "$KEYS"

(cd "$REPO/chain/program" && cargo build-sbf --manifest-path Cargo.toml) || exit 1
PROGRAM_SO="$REPO/chain/program/target/deploy/sil_escrow_program.so"

if [ ! -f "$KEYS/program.json" ]; then
  solana-keygen new --no-bip39-passphrase --silent -o "$KEYS/program.json" >/dev/null
fi
PROGRAM_ID=$(solana-keygen pubkey "$KEYS/program.json")
echo "program id: $PROGRAM_ID"

pkill -f solana-test-validator 2>/dev/null
sleep 2
nohup solana-test-validator --reset --quiet --ledger /tmp/sil-ledger --rpc-port "$RPC_PORT" \
  --bpf-program "$PROGRAM_ID" "$PROGRAM_SO" > /tmp/sil-validator.log 2>&1 &

for i in $(seq 1 120); do
  if solana cluster-version -u "http://127.0.0.1:$RPC_PORT" >/dev/null 2>&1; then
    echo "validator ready after ${i}s"
    break
  fi
  sleep 1
done

cd "$REPO/chain/adapter" && \
  SIL_RPC_URL="http://127.0.0.1:$RPC_PORT" SIL_ESCROW_PROGRAM="$PROGRAM_ID" \
  cargo test --tests -- --include-ignored --nocapture --skip devnet

# Moteur de rendu de référence en WebAssembly (isolé par construction, même moteur que Windows).
(cd "$REPO" && cargo build -p sil-wl-render --bin sil-render-reference --target wasm32-wasip1) || exit 1
export SIL_RENDER_ENGINE="$REPO/target/wasm32-wasip1/debug/sil-render-reference.wasm"

# Flux complet : nœuds réels (comité et vérificateur) contre le même validateur.
cd "$REPO/chain/e2e" && \
  SIL_RPC_URL="http://127.0.0.1:$RPC_PORT" SIL_ESCROW_PROGRAM="$PROGRAM_ID" \
  cargo test --tests -- --include-ignored --nocapture --test-threads=1
