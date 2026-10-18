#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

mkdir -p contracts

echo "==> 1/3 API shape: rustdoc JSON -> contracts/api.json -> specs/api.cue"
cargo +nightly rustdoc --lib -- -Z unstable-options --output-format json
uv run --no-project scripts/normalize_rustdoc.py target/doc/coinductive_repair.json \
    > contracts/api.json
cue vet -c specs/api.cue contracts/api.json

echo "==> 2/3 Behaviour of add: contracts/observations.json -> specs/behavior.cue"
cargo run --quiet > contracts/observations.json
cue vet -c specs/behavior.cue contracts/observations.json

echo "==> 3/3 Ledger trace: contracts/ledger.json -> specs/ledger.cue"
cargo run --quiet --example ledger > contracts/ledger.json
cue vet -c specs/ledger.cue contracts/ledger.json
uv run --no-project scripts/check_ledger.py

echo "All checks passed: the API and observed behaviour match the CUE specs."
