#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

mkdir -p contracts
cargo +nightly rustdoc --lib -- -Z unstable-options --output-format json
uv run --no-project scripts/normalize_rustdoc.py target/doc/coinductive_repair.json \
    > contracts/api.json
cue vet -c specs/api.cue contracts/api.json

cargo run --quiet > contracts/observations.json
cue vet -c specs/behavior.cue contracts/observations.json
cargo run --quiet --example account > contracts/account.json
cue vet -c specs/account.cue contracts/account.json
uv run --no-project scripts/check_account.py
echo "API et observations conformes aux contrats CUE."
