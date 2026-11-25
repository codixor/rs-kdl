# rs-kdl

`rs-kdl` is the workspace’s main pure-Rust KDL parser and writer crate.

The crate direction is:

- keep the crate library-first
- keep the parser/writer focused on KDL itself
- modernize errors with `thiserror`
- move proof tests to crate-root `tests/`
- add golden fixtures and a benchmark matrix
- use performance helpers only after measurement

## Current status

The crate is on a clean finished core now:

- workspace-aligned manifest
- `thiserror` public error types
- crate-root integration tests
- split-out writer, round-trip, validator, and normalization tests
- golden parser/writer fixtures, including strings, numbers, type hints, and invalid syntax inputs
- public crate-root re-exports for common types
- neutral interop model plus KDL conversion hooks
- examples for parse, stream, write, and validate flows
- broader Criterion benchmark matrix
- maintained-alternative comparison baseline against official `kdl`
- bench runner and bench README
- strict validation script

The crate-level decisions are also settled:

- keep `std` as the default feature surface for now
- do not add a separate explicit `alloc` feature until it unlocks a real use case
- keep `LexerError`, `ReaderError`, and `ValidatorError` public
- keep `ssb2` private as an internal implementation detail
- keep performance helper crates opt-in only after measurement

## Current starting point

The current code is clearly based on the `just-kdl` parser/writer line:

- streaming parser
- event reader/writer
- small dependency surface
- KDL v2-oriented performance-first design

That is a good fit for a pure-Rust fast KDL crate.

## Ecosystem position

There are two important KDL directions in Rust today:

- `just-kdl`
  - performance-oriented
  - small streaming parser/writer
  - good fit for machine-owned parse/write and conversion work
- official `kdl`
  - formatting-preserving
  - better for editing human-maintained KDL files
  - heavier and intentionally document-oriented

So the current local direction is the right one if the goal is:

- fast KDL parsing
- fast KDL writing
- lower allocation pressure
- interop and conversion pipelines

## Important interop rule

KDL itself should not become the universal semantic pivot for JSON, TOML, and YAML by going through KDL text.

That would be too lossy and format-shaped.

Instead, the better architecture is:

- parse each format into a neutral document/value model
- convert between models there
- let `kdl` be the KDL adapter in that pipeline

The crate now ships that neutral starting point directly:

- `InteropDocument`
- `InteropRoot`
- `InteropNode`
- `InteropField`
- `InteropValue`

KDL converts to and from that model with:

- `Document::to_interop()`
- `Document::from_interop(...)`

With the JSON adapter enabled on the default feature set, the crate also ships
explicit JSON, TOML, and YAML-on-KDL contracts:

- `InteropDocument::from_json_str(...)`
- `InteropDocument::from_json_item(...)`
- `InteropDocument::to_json_string()`
- `InteropValue::from_json_item(...)`
- `Document::from_json_item(...)`
- `Document::from_json_str(...)`
- `Document::to_json_string()`
- `InteropDocument::from_toml_str(...)`
- `InteropDocument::from_toml_document(...)`
- `InteropDocument::from_toml_table(...)`
- `InteropDocument::to_toml_string()`
- `InteropDocument::to_toml_table(...)`
- `InteropValue::from_toml_item(...)`
- `Document::from_toml_document(...)`
- `Document::from_toml_table(...)`
- `Document::from_toml_str(...)`
- `Document::to_toml_string()`
- `InteropDocument::from_yaml_str(...)`
- `InteropDocument::from_yaml_value(...)`
- `InteropDocument::to_yaml_string()`
- `InteropValue::from_yaml_value(...)`
- `Document::from_yaml_value(...)`
- `Document::from_yaml_str(...)`
- `Document::to_yaml_string()`

That contract is generated KDL, not generic KDL semantics. It exists so we can
benchmark and validate JSON <-> KDL conversion through the neutral model
without forcing arbitrary formats through raw KDL text.

The measured ownership decision is now clear:

- the direct shared-model path is the preferred JSON bridge
- the generated JSON-on-KDL contract stays as an additive KDL adapter when a
  real `Document` is required
- the direct shared-model path is also the preferred TOML bridge
- the generated TOML-on-KDL contract stays as an additive KDL adapter when a
  real `Document` is required

Current TOML interop baseline:

- `kdl/interop/toml_text_to_interop`: about `2.58-2.69 µs`
- `kdl/interop/toml_table_to_interop`: about `1.24-1.32 µs`
- `kdl/interop/toml_to_kdl_contract`: about `17.76-19.41 µs`
- `kdl/interop/toml_interop_to_toml`: about `2.74-2.85 µs`
- `kdl/interop/toml_direct_round_trip`: about `5.16-5.75 µs`
- `kdl/interop/toml_contract_round_trip`: about `26.62-29.75 µs`

That gap is large enough to keep the same rule we adopted for JSON and YAML:
direct shared-model interop is the default semantic bridge, and the generated
TOML-on-KDL contract is the additive path when a real KDL `Document` is the
thing the caller actually needs.
- the direct shared-model path is also the preferred YAML bridge
- the generated YAML-on-KDL contract stays as an additive KDL adapter when a
  real `Document` is required

So this crate should prepare for future interop, but not by forcing every format through raw KDL syntax.

## Current scope

This crate should own:

- KDL lexer/parser
- KDL reader/event stream
- KDL DOM/document model
- KDL writer
- KDL validation
- KDL-focused interop adapters later

This crate should not own:

- JSON parsing
- TOML parsing
- YAML parsing
- generic config merging policy
- serde derive frameworks

## Validation shape

- crate-root `tests/`
- golden parser/writer fixtures
- Criterion benchmark matrix
- strict Clippy
- examples and validation scripts

## Current examples

- `cargo run -p rs-kdl@0.1.0 --example parse_document`
- `cargo run -p rs-kdl@0.1.0 --example stream_events`
- `cargo run -p rs-kdl@0.1.0 --example write_document`
- `cargo run -p rs-kdl@0.1.0 --example validate_stream`
- `cargo run -p rs-kdl@0.1.0 --example json_contract`
- `cargo run -p rs-kdl@0.1.0 --example toml_contract`
- `cargo run -p rs-kdl@0.1.0 --example yaml_contract`

## Interop example

```rust
use rs_kdl::{Document, InteropDocument, Reader};

let source: &[u8] = br#"app "demo" enabled=#true"#;
let document = Reader::new(source)
    .collect::<Result<Document, _>>()
    .expect("valid KDL");

let interop = document.to_interop();
let round_trip = Document::from_interop(&interop).expect("interop stays KDL-compatible");

assert_eq!(round_trip, document);
```

JSON contract example:

```rust
use rs_json::parse as parse_json;
use rs_kdl::{Document, InteropDocument};

let json = r#"{"name":"rust-misc","enabled":true}"#;
let item = parse_json(json).expect("json parse");
let interop = InteropDocument::from_json_item(&item).expect("shared-model bridge");
let document = Document::from_json_item(&item).expect("json contract");
let rendered = document.to_json_string().expect("json render");

let reparsed = InteropDocument::from_json_str(&rendered).expect("json parse");
assert_eq!(interop, reparsed);
assert!(matches!(reparsed.root, rs_kdl::InteropRoot::Value(_)));
```

YAML contract example:

```rust
use rs_kdl::{Document, InteropDocument};

let yaml = "meta:\n  name: rust-misc\n  enabled: true\nroutes:\n  - /\n  - /dashboard\n";
let interop = InteropDocument::from_yaml_str(yaml).expect("yaml parse");
let document = Document::from_yaml_str(yaml).expect("yaml contract");
let rendered = document.to_yaml_string().expect("yaml render");

let reparsed = InteropDocument::from_yaml_str(&rendered).expect("yaml parse");
assert_eq!(interop, reparsed);
assert!(matches!(reparsed.root, rs_kdl::InteropRoot::Value(_)));
```

TOML contract example:

```rust
use rs_kdl::{Document, InteropDocument, InteropTemporalKind, InteropValue};

let toml = "title = \"rust-misc\"\npublished = 2026-03-31T08:15:00Z\n";
let interop = InteropDocument::from_toml_str(toml).expect("toml parse");
let document = Document::from_toml_str(toml).expect("toml contract");
let rendered = document.to_toml_string().expect("toml render");

let reparsed = InteropDocument::from_toml_str(&rendered).expect("toml parse");
assert_eq!(interop, reparsed);
assert!(matches!(reparsed.root, rs_kdl::InteropRoot::Value(_)));

let rs_kdl::InteropRoot::Value(InteropValue::Map(entries)) = reparsed.root else {
    unreachable!();
};
assert_eq!(
    entries.iter().find(|entry| entry.name.as_str() == "published").unwrap().value,
    InteropValue::Temporal(rs_kdl::InteropTemporal::new(
        InteropTemporalKind::OffsetDateTime,
        "2026-03-31T08:15:00Z",
    ))
);
```

Current YAML bridge boundaries:

- the neutral YAML bridge is value-root only
- map keys must be strings
- aliases and explicit tagged values are rejected instead of being flattened away
- emitted YAML text preserves numeric lexemes from the interop model so `1.0`
  does not collapse into `1` on the round trip

Current TOML bridge boundaries:

- the neutral TOML bridge is value-root and map-root only for text rendering
- TOML temporals stay explicit as `InteropValue::Temporal(...)`
- TOML text rendering rejects nulls and scalar-root documents
- the release-mode contract path is covered by a regression test that keeps
  numeric lexemes stable through `Document::from_toml_str(...).to_toml_string()`

## Validation

- `bash ./kdl/scripts/validate.sh`

## Benchmarks

- `bash ./kdl/scripts/bench.sh`

The execution roadmap lives in [PLAN.md](./PLAN.md).
