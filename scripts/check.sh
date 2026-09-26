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
echo "API et observations conformes aux contrats CUE."
