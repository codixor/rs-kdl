# rs-kdl Benchmarks

The committed bench entry point is:

- `cargo bench -p rs-kdl --bench kdl_core -- --sample-size 20`

Focused macOS profiler entry point:

- `bash scripts/profile.sh '^kdl/parse_document_nested$'`

The bench includes both:

- local parser/writer throughput baselines
- maintained-alternative comparison points against official `kdl`
- JSON interop contract baselines:
  - `kdl/interop/json_text_to_interop`
  - `kdl/interop/json_dom_to_interop`
  - `kdl/interop/json_to_kdl_contract`
  - `kdl/interop/json_dom_to_kdl_contract`
  - `kdl/interop/json_interop_to_json`
  - `kdl/interop/kdl_contract_to_json`
  - `kdl/interop/json_direct_round_trip`
  - `kdl/interop/json_contract_round_trip`
- YAML interop contract baselines:
  - `kdl/interop/yaml_text_to_interop`
  - `kdl/interop/yaml_value_to_interop`
  - `kdl/interop/yaml_to_kdl_contract`
  - `kdl/interop/yaml_interop_to_yaml`
  - `kdl/interop/yaml_direct_round_trip`
  - `kdl/interop/yaml_contract_round_trip`
- TOML interop contract baselines:
  - `kdl/interop/toml_text_to_interop`
  - `kdl/interop/toml_table_to_interop`
  - `kdl/interop/toml_to_kdl_contract`
  - `kdl/interop/kdl_contract_to_toml`
  - `kdl/interop/toml_interop_to_toml`
  - `kdl/interop/toml_direct_round_trip`
  - `kdl/interop/toml_contract_round_trip`

The current preferred ownership decision is:

- direct shared-model JSON interop first
- generated JSON-on-KDL contract second, only when a real KDL `Document` is
  the required output
- direct shared-model YAML interop first
- generated YAML-on-KDL contract second, only when a real KDL `Document` is
  the required output
- TOML currently follows the same intended rule:
  - direct shared-model TOML interop first
  - generated TOML-on-KDL contract second when a real KDL `Document` is the
    required output

Current TOML interop baseline:

- `kdl/interop/toml_text_to_interop`: about `2.58-2.69 µs`
- `kdl/interop/toml_table_to_interop`: about `1.24-1.32 µs`
- `kdl/interop/toml_to_kdl_contract`: about `17.76-19.41 µs`
- `kdl/interop/toml_interop_to_toml`: about `2.74-2.85 µs`
- `kdl/interop/toml_direct_round_trip`: about `5.16-5.75 µs`
- `kdl/interop/toml_contract_round_trip`: about `26.62-29.75 µs`

Criterion output is written under the workspace `target/criterion/` directory.
Profiler samples are written to `/tmp/kdl_core.sample.txt`.
