#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"

cargo test -p rs-kdl@0.1.0 --manifest-path "$repo_root/Cargo.toml"
cargo test -p rs-kdl@0.1.0 --examples --manifest-path "$repo_root/Cargo.toml"
cargo clippy -p rs-kdl@0.1.0 --all-targets --all-features --manifest-path "$repo_root/Cargo.toml" -- -D warnings
cargo doc -p rs-kdl@0.1.0 --no-deps --manifest-path "$repo_root/Cargo.toml"
cargo check -p rs-kdl@0.1.0 --benches --manifest-path "$repo_root/Cargo.toml"
