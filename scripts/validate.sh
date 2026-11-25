#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
for features in --no-default-features --all-features; do
  cargo clippy --all-targets --locked "$features" -- -D warnings
  cargo test --locked "$features"
done
for adapter in json-interop toml-interop yaml-interop; do
  cargo test --locked --no-default-features --features "$adapter" --test "${adapter/-/_}"
done
RUSTDOCFLAGS="-D warnings" cargo doc --locked --all-features --no-deps
