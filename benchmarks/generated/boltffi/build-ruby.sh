#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/../../.."
DEMO_DIR="$ROOT_DIR/examples/demo"

export CARGO_TARGET_DIR="$SCRIPT_DIR/target"

cd "$DEMO_DIR"

cargo run \
    --manifest-path "$ROOT_DIR/Cargo.toml" \
    -p boltffi_cli \
    -- \
    --overlay boltffi.benchmark.toml \
    pack ruby \
    --experimental \
    --release \
    --regenerate
