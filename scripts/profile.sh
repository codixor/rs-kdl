#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ $# -ne 1 ]]; then
  echo "usage: $0 '<criterion bench regex>'" >&2
  echo "example: $0 '^kdl/parse_document_nested$'" >&2
  exit 1
fi

bench_regex="$1"
bench_name="kdl_core"
bench_binary="$(find "$repo_root/target/release/deps" -maxdepth 1 -type f -name "${bench_name}-*" -perm -111 | sort | tail -n 1)"
sample_output="/tmp/${bench_name}.sample.txt"

if [[ -z "${bench_binary:-}" ]]; then
  echo "missing bench binary, run cargo bench once first" >&2
  exit 1
fi

if ! command -v sample >/dev/null 2>&1; then
  echo "macOS sample tool is required for this profiling flow" >&2
  exit 1
fi

bench_process_name="$(basename "$bench_binary")"
rm -f "$sample_output"

sample "$bench_process_name" 5 1 -wait -mayDie -file "$sample_output" >/dev/null 2>&1 &
sample_pid=$!
trap 'kill "$sample_pid" 2>/dev/null || true' EXIT

cargo bench -p rs-kdl@0.1.0 --bench "$bench_name" --manifest-path "$repo_root/Cargo.toml" -- "$bench_regex" --sample-size 100 --measurement-time 15

wait "$sample_pid" || true
trap - EXIT

echo "profile written to $sample_output"
