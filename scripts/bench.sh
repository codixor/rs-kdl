#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"

cargo bench -p rs-kdl@0.1.0 --bench kdl_core --manifest-path "$repo_root/Cargo.toml" -- --sample-size 20
