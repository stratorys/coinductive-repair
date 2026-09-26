#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

mkdir -p target/contracts
cargo +nightly rustdoc --lib -- -Z unstable-options --output-format json
uv run --no-project scripts/normalize_rustdoc.py target/doc/coinductive_repair.json \
    > target/contracts/api.json
cue vet -c specs/api.cue target/contracts/api.json

cargo run --quiet > target/contracts/observations.json
cue vet -c specs/behavior.cue target/contracts/observations.json
cargo run --quiet --example account > target/contracts/account.json
cue vet -c specs/account.cue target/contracts/account.json
uv run --no-project scripts/check_account.py
echo "API et observations conformes aux contrats CUE."
