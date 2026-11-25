# rs-kdl

A Rust implementation of KDL v2 with a streaming event reader, a document tree, a validator and a writer. Use it to parse, transform and write KDL configuration files.

```sh
cargo add rs-kdl@0.1.0
```

```rust
use rs_kdl::{Document, Reader};

let source: &[u8] = br#"service "api" port=8080"#;
let document = Reader::new(source).collect::<Result<Document, _>>().expect("valid KDL");
assert_eq!(document.nodes.len(), 1);
```

## Format adapters

The default features enable `std`, `json-interop`, `toml-interop`, and `yaml-interop`. Each adapter can be enabled separately. They use the crates.io releases of `rs-json`, `rs-toml`, and `rs-yaml` respectively. The JSON adapter uses the `rs-json` 0.3 compatibility API for borrowed `JsonItem` values. Use `InteropDocument` to convert values between formats, or `Document` when the output should be KDL. See the [examples](https://github.com/codixor/rs-kdl/tree/main/examples) for each adapter.

The YAML adapter accepts string map keys and rejects aliases and explicit tags. TOML text output requires a table root and rejects null values.

## Examples and conversion behavior

- `cargo run --example parse_document`
- `cargo run --example stream_events`
- `cargo run --example write_document`
- `cargo run --example validate_stream`
- `cargo run --example json_contract`
- `cargo run --example toml_contract`
- `cargo run --example yaml_contract`

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
use rs_json::legacy::parse as parse_json;
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

## Development

The minimum and pinned stable Rust version is 1.100.0. Run `bash scripts/validate.sh` for strict Clippy, tests and documentation, `cargo deny --locked --all-features check` for dependencies, and `bash scripts/bench.sh` for the Criterion suite, including comparisons with the `kdl` crate. CI runs both fuzz targets with AddressSanitizer on a pinned nightly. [Benchmarks](https://github.com/codixor/rs-kdl/tree/main/benches)

Originally developed in [codixor/rust-misc](https://github.com/codixor/rust-misc/tree/ae68abb54c655a8fcc009123fbc31821b82435e4/rs-kdl), building on [just-kdl](https://github.com/1e1001/rsutil/tree/main/just-kdl). The library is distributed under MIT or Apache-2.0. The generated compliance test data comes from the [KDL specification test suite](https://github.com/kdl-org/kdl/tree/main/tests/test_cases). See [the fixture license](https://github.com/codixor/rs-kdl/blob/main/LICENSE-KDL-TESTS).
