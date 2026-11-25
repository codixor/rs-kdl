# rs-kdl Plan

Goal: turn `rs-kdl` into the main pure-Rust KDL parser/writer crate in this workspace, modernize its error model, align its structure with the cleaner crate pattern used elsewhere, and build a measured parser/writer benchmark matrix before adopting any extra performance tooling.

## Direction

- [x] Keep the crate library-first.
- [x] Keep the public surface centered on:
  - [x] reader / event stream
  - [x] DOM/document model
  - [x] writer
  - [x] validator
- [x] Replace `displaydoc` + handwritten `Error` impls with `thiserror`.
- [x] Move tests to crate-root `tests/`.
- [x] Add golden fixtures for parser/writer round-trips and compliance.
- [x] Add a benchmark matrix with reproducible entry points.
- [x] Use performance helper crates only after measurement.

## Ecosystem decision

- [x] Keep this local parser/writer line if the goal stays:
  - [x] pure Rust
  - [x] small dependency footprint
  - [x] fast parse/write
  - [x] no formatting-preserving edit model
- [x] Treat `just-kdl` as the current best donor/reference for pure-Rust performance-first KDL.
- [x] Treat the official `kdl` crate as the better reference for:
  - [x] formatting preservation
  - [x] document editing
  - [x] human-maintained file round-tripping
- [x] Do not replace this crate with official `kdl` unless we decide editing fidelity matters more than parser/writer throughput.

## Interop rule

- [x] Do not use raw KDL text as the universal pivot for JSON/TOML/YAML conversion.
- [x] Plan future interop through a neutral data/document model instead.
- [x] Let this crate become the KDL adapter in that interop layer, not the universal semantic hub by itself.
- [x] Keep future interop modules additive:
  - [x] `interop/json`
  - [x] `interop/toml`
  - [x] `interop/yaml`

## Stage 1: Workspace And Manifest Cleanup

- [x] Align `Cargo.toml` to workspace metadata:
  - [x] edition
  - [x] rust-version
  - [x] license
  - [x] version
  - [x] repository
  - [x] description
  - [x] keywords/categories
- [x] Add the crate to the root workspace when the manifest is aligned.
- [x] Replace standalone dependency versions with workspace dependencies where appropriate.
- [x] Add `README.md`.
- [x] Add `PLAN.md`.
- [x] Decide whether to keep `std` as the only default feature or introduce `alloc`/`std` split explicitly.

## Stage 2: Error Model With thiserror

- [x] Replace `displaydoc`-based error enums in:
  - [x] [lexer.rs](./src/lexer.rs)
  - [x] [reader.rs](./src/reader.rs)
  - [x] [validator.rs](./src/validator.rs)
- [x] Use `thiserror` for the public error types.
- [x] Keep spans and error context intact.
- [x] Decide which errors remain public and which stay internal.
- [x] Keep the no-std/alloc story intact if we preserve it.

## Stage 3: API And Module Surface Cleanup

- [x] Audit the public modules:
  - [x] [dom.rs](./src/dom.rs)
  - [x] [lexer.rs](./src/lexer.rs)
  - [x] [reader.rs](./src/reader.rs)
  - [x] [validator.rs](./src/validator.rs)
  - [x] [writer.rs](./src/writer.rs)
  - [x] [ssb2.rs](./src/ssb2.rs)
- [x] Keep the main public API small and obvious.
- [x] Decide whether `ssb2` should remain public, private, or renamed.
- [x] Audit naming and docs so the crate consistently presents itself as `kdl`, not `just-kdl`.
- [x] Add a crate-root re-export story that makes the common types easy to discover.

## Stage 4: Tests To Crate-Root

- [x] Move [src/tests.rs](./src/tests.rs) into crate-root `tests/`.
- [x] Split tests into focused files:
  - [x] parser compliance
  - [x] reader event stream
  - [x] writer output
  - [x] round-trip
  - [x] validator behavior
  - [x] DOM normalization
- [x] Keep local fixture helpers out of `lib.rs`.

## Stage 5: Golden Matrix

- [x] Add golden fixtures for:
  - [x] normalized output
  - [x] writer round-trip output
  - [x] compact vs formatted cases
  - [x] edge-case strings
  - [x] numbers
  - [x] annotations
  - [x] child blocks
  - [x] invalid syntax cases
- [x] Add explicit parser/writer round-trip tests:
  - [x] bytes -> reader -> DOM -> writer -> reader
  - [x] events -> writer -> reader -> events

## Stage 6: Benchmark Matrix

- [x] Replace ad hoc benchmark stories with a clean Criterion bench surface.
- [x] Add benchmarks for:
  - [x] lexer throughput
  - [x] reader event throughput
  - [x] DOM collection
  - [x] writer throughput
  - [x] validator overhead
  - [x] round-trip parse+write
- [x] Compare against maintained alternatives where useful:
  - [x] official `kdl`
  - [x] local crate baseline
- [x] Document the benchmark workflow with a bench README and runner script.
- [x] Record the current maintained-alternative baseline carefully:
  - [x] local `parse_document`: about `122-138 µs`
  - [x] official `kdl` `parse_document`: about `4.52-5.25 ms`
  - [x] local `round_trip_parse_write`: about `211-280 µs`
  - [x] official `kdl` `round_trip_parse_write`: about `6.43-8.17 ms`
  - [x] note that write-only comparison is not strictly apples-to-apples because official `kdl` preserves source formatting/comment structure while this crate emits canonical writer output

## Stage 7: Performance Pass

- [x] Profile before changing hot paths.
- [x] Run the first measured lexer hot-path experiment.
- [x] Run the second measured DOM-builder experiment.
- [x] Apply performance helpers best-effort only on critical paths.
- [x] Reviewed candidate measured upgrades:
  - [x] `smallvec`: tested on the `reader.rs` child-block stack; nested reader and parse lanes regressed, reverted
  - [x] smarter event buffering / stack storage: bit-stack and builder-capacity experiments both rejected
  - [x] `memchr`: deferred for now because the current profiler points first at reader control flow and lexer token dispatch, not byte-scan helpers
  - [x] `arrayvec`: deferred for now because the current hot paths did not expose a clear fixed-capacity win over the simpler stable code
  - [x] `compact_str`: deferred for now because the profiler points first at tokenization/control flow, not stored DOM string ownership
- [x] Keep any change that gives a validated `>=40%` win on a real hot path.
- [x] Revert complexity that does not produce a meaningful measured win.
- [x] Reject the first `lexer.rs` `peek_table` single-max-peek rewrite for now:
  - [x] A/B benchmarked on the warmed system
  - [x] did not produce a trustworthy parser-path `>=40%` win
  - [x] reverted to the stable path instead of keeping speculative complexity
- [x] Reject the `reader.rs` `Reader::peek` closure-removal simplification for now:
  - [x] focused `reader_events` / `parse_document` A/B run attempted
  - [x] one run looked promising, but the confidence rerun was not repeatable
  - [x] did not clear the crate bar for a trustworthy `>=40%` validated win
  - [x] reverted to the stable path instead of keeping an uncertain optimization
- [x] Add tighter lexer microbench lanes before the next hot-path rewrite:
  - [x] added `kdl/lexer_tokens_keywords`
  - [x] added `kdl/lexer_tokens_utf8`
  - [x] added `kdl/parse_document_keywords`
  - [x] added `kdl/parse_document_utf8`
- [x] Reject the `lexer.rs` `utf8_len` `leading_ones` alternative for now:
  - [x] A/B benchmarked against the new UTF-8-heavy lexer and parse lanes
  - [x] did not produce a meaningful or trustworthy win
  - [x] reverted to the stable nibble-LUT path
- [x] Reject the first wider `#...` keyword-dispatch rewrite for now:
  - [x] widened the dispatch experiment from 3-byte to 4-byte prefixes
  - [x] keyword-heavy lexer lane showed no trustworthy win
  - [x] UTF-8-heavy lexer lane regressed
  - [x] reverted to the stable keyword path instead of keeping churn
- [x] Reject the first specialized `peek_table` helper rewrite for now:
  - [x] replaced common closure call sites with explicit helper methods (`utf8`, `utf8+slash`, `utf8+quote+cr`, initial token peek)
  - [x] benchmarked against `lexer_tokens`, `reader_events`, `lexer_tokens_utf8`, and `parse_document_utf8`
  - [x] generic lexer, reader, and UTF-8-heavy lanes all regressed
  - [x] reverted to the stable closure-based path instead of keeping a broad slowdown
- [x] Reject the first `StringOutput` dynamic-dispatch rewrite for now:
  - [x] made the sink object-safe and routed the hot ident/single-line-string path through `&mut dyn StringOutput`
  - [x] benchmarked against `lexer_tokens`, `lexer_tokens_utf8`, `parse_document_utf8`, and `reader_events`
  - [x] UTF-8-heavy lexer improved, but did not clear the crate adoption bar
  - [x] parser and reader lanes did not show a trustworthy end-to-end win
  - [x] reverted to the simpler monomorphic path instead of carrying extra indirection
- [x] Add nested child-block reader lanes before touching `reader.rs` representation:
  - [x] added `kdl/reader_events_nested`
  - [x] added `kdl/parse_document_nested`
- [x] Reject the first `reader.rs` compact bit-stack replacement for now:
  - [x] replaced `Vec<bool>` child-block tracking with an inline-bit-plus-`Vec<u64>` stack
  - [x] benchmarked against `reader_events`, `parse_document`, `reader_events_nested`, and `parse_document_nested`
  - [x] nested reader lane showed no meaningful win
  - [x] generic parse and nested parse lanes regressed
  - [x] reverted to `Vec<bool>` instead of keeping a slower representation
- [x] Measure the `dom/iter.rs` `extend(option)` vs manual `if let Some(...)` builder path:
  - [x] added a dedicated `kdl/document_from_events` microbench
  - [x] baseline: about `41.9-53.5 µs`
  - [x] manual branch: about `39.8-54.8 µs`
  - [x] no statistically significant change
  - [x] kept the manual branch because it is clearer and not measurably worse
- [x] Add a real profiler workflow before more Stage 7 churn:
  - [x] added `scripts/profile.sh` for focused macOS `sample` captures against Criterion lanes
  - [x] verified it on `^kdl/parse_document_nested$`
  - [x] latest sample still points first at `Lexer::next_token_value`, `Reader::next_inner_event`, and `Lexer::ident_inner`
  - [x] `Vec::from_iter` / document collection is visible, but secondary to reader/lexer work
- [x] Reject the first document-builder preallocation experiment for now:
  - [x] added `kdl/document_from_events_nested` to isolate nested builder cost
  - [x] tried eager child-document `Vec::with_capacity(1)` and top-level `size_hint` preallocation in `dom/iter.rs`
  - [x] `document_from_events` showed no significant change
  - [x] `document_from_events_nested` established a baseline around `7.12-8.09 µs`, but no trustworthy win from the preallocation rewrite
  - [x] focused rerun also showed unrelated layout-side regression on `reader_events_nested`
  - [x] reverted to the simpler stable builder path instead of keeping speculative allocation tuning
- [x] Reject the first `smallvec` helper-crate experiment for now:
  - [x] tested `SmallVec<[bool; 8]>` for `reader.rs` child-block tracking
  - [x] `reader_events_nested` regressed by about `22%` on the median in the focused rerun
  - [x] `parse_document_nested` regressed by about `31%` on the median in the focused rerun
  - [x] reverted to `Vec<bool>` instead of keeping a slower helper-crate rewrite
- [x] Close Stage 7 on the stable path for now:
  - [x] the profiler workflow is committed and repeatable
  - [x] no measured helper-crate or hot-path rewrite cleared the crate adoption bar
  - [x] the current code stays simpler, benchmarked, and honestly documented

## Source TODO Integration

Inventory from `src/` is now explicitly tracked here instead of being left as scattered comments. The TODOs cluster into API/model work, reader architecture, and lexer/input hot paths.

### DOM, Writer, And Model Shape

- [x] [dom.rs](./src/dom.rs): keep unified ordered entries for now; do not split args and props unless we can preserve `Events -> Document -> Events` parity and show a real benefit.
- [x] [dom.rs](./src/dom.rs): keep the current marker-string duplicate-property normalization path for now; only replace it if a clearer explicit marking/removal strategy also improves readability or correctness.
- [x] [writer.rs](./src/writer.rs): do not add a separate minified writer mode for now; canonical writer output stays the only built-in writer mode.
- [x] [dom/number.rs](./src/dom/number.rs): keep the current `Base` representation for now; no benchmark or profiler result justifies changing it.
- [x] [ssb2.rs](./src/ssb2.rs): keep the current ASCII write path for now; only revisit `ascii::Char` once it is stable and measurably useful.
- [x] [dom/iter.rs](./src/dom/iter.rs): compared `nodes.extend(builder.push(event))` against a manual `Option` branch; no significant perf change, manual branch kept for clarity.

### Reader Architecture

- [x] [reader.rs](./src/reader.rs): keep the current lexer/reader split; do not add hinted lexing or a partial re-merge without a fresh profiler signal.
- [x] [reader.rs](./src/reader.rs): do not add a generic token-source abstraction until a second real token producer exists.
- [x] [reader.rs](./src/reader.rs): keep the current `peek` / `advance` control flow for now; prior reader-side micro-optimizations did not justify the churn.
- [x] [reader.rs](./src/reader.rs): replace the `Vec<bool>` child-block tracking with a more compact representation only if benchmarks show a win; the first inline-bit stack attempt regressed and was reverted, so `Vec<bool>` stays for now.

### Lexer, Input, And Hot-Path Cleanup

- [x] [lexer.rs](./src/lexer.rs): defer a `BufRead`-aware `ReadInput` path; the current fixed small buffer stays until a real std-I/O workload justifies the extra surface area.
- [x] [lexer.rs](./src/lexer.rs): keep the current intentionally narrow invalid UTF-8 / surrogate classification policy for now.
- [x] [lexer.rs](./src/lexer.rs): benchmark `utf8_len` alternatives such as `leading_ones`; the first `leading_ones` pass did not beat the current nibble LUT, so the LUT stays for now.
- [x] [lexer.rs](./src/lexer.rs): benchmark `StringOutput` monomorphization against dynamic dispatch; the first dynamic-dispatch pass improved one lexer-heavy lane but did not earn adoption, so the monomorphic path stays for now.
- [x] [lexer.rs](./src/lexer.rs): reject a full UTF-8/state-machine lexer rewrite for now; the benchmark matrix and profiler do not justify that churn.
- [x] [lexer.rs](./src/lexer.rs): keep irrecoverable I/O handling where it is for now; no measured win justified moving it back out of `peek`.
- [x] [lexer.rs](./src/lexer.rs): close the current `peek_table` / keyword-dispatch experiment line on the stable path; multiple measured rewrites regressed and further work now needs a new profiler reason.
- [x] [lexer.rs](./src/lexer.rs): defer vectorized checks in raw and multiline string scanning until profiling identifies those loops as primary hot spots.
- [x] [lexer.rs](./src/lexer.rs): treat the remaining style TODOs as ordinary maintenance, not open plan work.

## Stage 8: Examples And Validation Scripts

- [x] Add examples for:
  - [x] parse document
  - [x] stream reader events
  - [x] write a document
  - [x] JSON contract
  - [x] TOML contract
  - [x] validate an event stream
- [x] Add a validation script.
- [x] Run:
  - [x] `cargo test`
  - [x] `cargo test --examples`
  - [x] `cargo clippy --all-targets --all-features -D warnings`
  - [x] `cargo doc --no-deps`
  - [x] `cargo check --benches`

## Stage 9: Interop Contract Batch

- [x] Keep JSON interop additive on top of the neutral model.
- [x] Add JSON text -> neutral interop conversion.
- [x] Add neutral interop -> JSON text conversion.
- [x] Add explicit JSON-on-KDL contract conversion:
  - [x] `Document::from_json_str(...)`
  - [x] `Document::to_json_interop()`
  - [x] `Document::to_json_string()`
- [x] Add proof tests for both directions:
  - [x] JSON text -> interop -> JSON text
  - [x] JSON text -> KDL contract -> JSON text
  - [x] scalar JSON root through the KDL contract
  - [x] invalid contract rejection
- [x] Add initial contract bench lanes:
  - [x] `kdl/interop/json_text_to_interop`
  - [x] `kdl/interop/json_dom_to_interop`
  - [x] `kdl/interop/json_to_kdl_contract`
  - [x] `kdl/interop/json_dom_to_kdl_contract`
  - [x] `kdl/interop/json_interop_to_json`
  - [x] `kdl/interop/kdl_contract_to_json`
- [x] Record the measured JSON interop baseline:
  - [x] `json_text_to_interop`: about `1.55-1.60 µs`
  - [x] `json_dom_to_interop`: about `843-852 ns`
  - [x] `json_to_kdl_contract`: about `8.79-8.96 µs`
  - [x] `json_dom_to_kdl_contract`: about `8.15-8.28 µs`
  - [x] `json_interop_to_json`: about `841-859 ns`
  - [x] `kdl_contract_to_json`: about `4.81-4.85 µs`
  - [x] `json_direct_round_trip`: about `2.49-2.69 µs`
  - [x] `json_contract_round_trip`: about `13.89-14.02 µs`
- [x] Adopt the direct shared-model JSON bridge as the long-term performance owner:
  - [x] keep `InteropDocument::from_json_item(...)` and `InteropValue::from_json_item(...)` public
  - [x] keep `Document::from_json_item(...)` as the explicit KDL-contract adapter for callers that actually need a KDL document
  - [x] keep the generated JSON-on-KDL contract additive, benchmarked, and available, but not the preferred semantic bridge when the caller only needs interop
- [x] Keep YAML interop additive on top of the neutral model.
- [x] Add YAML text -> neutral interop conversion.
- [x] Add YAML value -> neutral interop conversion.
- [x] Add neutral interop -> YAML text conversion.
- [x] Add explicit YAML-on-KDL contract conversion:
  - [x] `Document::from_yaml_value(...)`
  - [x] `Document::from_yaml_str(...)`
  - [x] `Document::to_yaml_interop()`
  - [x] `Document::to_yaml_string()`
- [x] Add proof tests for both directions:
  - [x] YAML text -> interop -> YAML text
  - [x] YAML value -> interop
  - [x] YAML text -> KDL contract -> YAML text
  - [x] scalar YAML root through the KDL contract
  - [x] invalid contract rejection
  - [x] non-string map keys are rejected
  - [x] tagged values are rejected by the neutral bridge
- [x] Add initial YAML contract bench lanes:
  - [x] `kdl/interop/yaml_text_to_interop`
  - [x] `kdl/interop/yaml_value_to_interop`
  - [x] `kdl/interop/yaml_to_kdl_contract`
  - [x] `kdl/interop/yaml_interop_to_yaml`
  - [x] `kdl/interop/yaml_direct_round_trip`
  - [x] `kdl/interop/yaml_contract_round_trip`
- [x] Record the measured YAML interop baseline:
  - [x] `yaml_text_to_interop`: about `21.8-23.6 µs`
  - [x] `yaml_value_to_interop`: about `1.79-1.93 µs`
  - [x] `yaml_to_kdl_contract`: about `29.8-31.6 µs`
  - [x] `yaml_interop_to_yaml`: about `9.53-10.63 µs`
  - [x] `yaml_direct_round_trip`: about `30.4-33.6 µs`
  - [x] `yaml_contract_round_trip`: about `45.9-49.9 µs`
- [x] Adopt the direct shared-model YAML bridge as the long-term performance owner:
  - [x] keep `InteropDocument::from_yaml_value(...)` and `InteropValue::from_yaml_value(...)` public
  - [x] keep `Document::from_yaml_value(...)` and `Document::from_yaml_str(...)` as the explicit KDL-contract adapters for callers that actually need a KDL document
  - [x] keep the generated YAML-on-KDL contract additive, benchmarked, and available, but not the preferred semantic bridge when the caller only needs interop
  - [x] preserve interop numeric lexemes when rendering YAML text so the bridge does not collapse `1.0` into `1`
- [x] Keep TOML interop additive on top of the neutral model.
- [x] Add TOML document/table/text -> neutral interop conversion.
- [x] Add neutral interop -> TOML table/text conversion.
- [x] Add explicit TOML-on-KDL contract conversion:
  - [x] `Document::from_toml_document(...)`
  - [x] `Document::from_toml_table(...)`
  - [x] `Document::from_toml_str(...)`
  - [x] `Document::to_toml_interop()`
  - [x] `Document::to_toml_string()`
- [x] Add proof tests for both directions:
  - [x] TOML text -> interop -> TOML text
  - [x] TOML document/table -> interop
  - [x] TOML text -> KDL contract -> TOML text
  - [x] temporal values stay explicit in the neutral model
  - [x] scalar-root render rejection
  - [x] null-value render rejection
  - [x] invalid contract rejection
- [x] Add TOML contract bench lanes:
  - [x] `kdl/interop/toml_text_to_interop`
  - [x] `kdl/interop/toml_table_to_interop`
  - [x] `kdl/interop/toml_to_kdl_contract`
  - [x] `kdl/interop/kdl_contract_to_toml`
  - [x] `kdl/interop/toml_interop_to_toml`
  - [x] `kdl/interop/toml_direct_round_trip`
  - [x] `kdl/interop/toml_contract_round_trip`
- [x] Keep TOML temporal values explicit in the neutral model with:
  - [x] `InteropValue::Temporal(...)`
  - [x] `InteropTemporal`
  - [x] `InteropTemporalKind`
- [x] Add a release-mode TOML contract regression test for numeric lexeme
  stability.
- [x] Record the measured TOML interop baseline:
  - [x] `toml_text_to_interop`: about `2.58-2.69 µs`
  - [x] `toml_table_to_interop`: about `1.24-1.32 µs`
  - [x] `toml_to_kdl_contract`: about `17.76-19.41 µs`
  - [x] `toml_interop_to_toml`: about `2.74-2.85 µs`
  - [x] `toml_direct_round_trip`: about `5.16-5.75 µs`
  - [x] `toml_contract_round_trip`: about `26.62-29.75 µs`
- [x] Adopt the direct shared-model TOML bridge as the long-term performance owner:
  - [x] keep `InteropDocument::from_toml_document(...)`, `InteropDocument::from_toml_table(...)`, and `InteropValue::from_toml_item(...)` public
  - [x] keep `Document::from_toml_document(...)`, `Document::from_toml_table(...)`, and `Document::from_toml_str(...)` as the explicit KDL-contract adapters for callers that actually need a KDL document
  - [x] keep the generated TOML-on-KDL contract additive, benchmarked, and available, but not the preferred semantic bridge when the caller only needs interop

## Success Criteria

- [x] `rs-kdl` is the clear main pure-Rust KDL parser/writer crate in the workspace.
- [x] Public errors use `thiserror`.
- [x] Tests live under crate-root `tests/`.
- [x] Golden fixtures protect parser/writer behavior.
- [x] Benchmark matrix exists and is reproducible.
- [x] Performance helpers are only adopted when measurement justifies them.
- [x] The crate remains the performance-first KDL lane, while future JSON/TOML/YAML interop goes through a neutral model instead of raw KDL text.
