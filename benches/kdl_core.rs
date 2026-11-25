use criterion::{Criterion, criterion_group, criterion_main};
use official_kdl::KdlDocument as OfficialDocument;
use rs_json::legacy::parse as parse_json;
use rs_kdl::{Document, Event, Lexer, Reader, Token, Validator, Writer};
use rs_toml::{Arena as TomlArena, parse as parse_toml};
use rs_yaml::safe_load as parse_yaml;
use std::hint::black_box;

const SAMPLE: &[u8] = br#"
app "rust-misc" version=1 {
    theme "dark"
    locale "en-US"
    route "/dashboard"
    widget kind="chart" title="Revenue"
}
"#;

const SAMPLE_KEYWORDS: &[u8] = br"
node #true #false #null #inf #-inf #nan
node #true #false #null #inf #-inf #nan
node #true #false #null #inf #-inf #nan
";

const SAMPLE_UTF8: &str = r#"
node "مرحبا" city="دبي" emoji="🙂" accent="café" greek="κόσμος"
node "مرحبا" city="دبي" emoji="🙂" accent="café" greek="κόσμος"
node "مرحبا" city="دبي" emoji="🙂" accent="café" greek="κόσμος"
"#;

const SAMPLE_NESTED: &[u8] = br#"
root {
    a { b { c { d { e { f { g { value "x" } } } } } } }
    a { b { c { d { e { f { g { value "x" } } } } } } }
    a { b { c { d { e { f { g { value "x" } } } } } } }
}
"#;

const SAMPLE_JSON: &str = r#"{
  "meta":{"name":"rust-misc","enabled":true,"retries":3},
  "routes":["/","/dashboard","/settings"],
  "nothing":null
}"#;

const SAMPLE_YAML: &str = r"meta:
  name: rust-misc
  enabled: true
  retries: 3
routes:
  - /
  - /dashboard
  - /settings
nothing: null
ratio: 1.0
";

const SAMPLE_TOML: &str = r#"
title = "rust-misc"
enabled = true
retries = 3
ratio = 1.0
published = 2026-03-31T08:15:00Z
local_date = 2026-03-31
local_time = 08:15:00
routes = ["/", "/dashboard", "/settings"]
"#;

fn lex_token_count(input: &[u8]) -> usize {
    let mut lexer = Lexer::new(input);
    let mut count = 0;
    loop {
        match lexer.next_token(false).0.expect("sample should lex") {
            Token::Eof => break count,
            _ => count += 1,
        }
    }
}

fn read_events(input: &[u8]) -> Vec<Event> {
    Reader::new(input)
        .collect::<Result<Vec<_>, _>>()
        .expect("sample should parse")
        .into_iter()
        .map(|(event, _)| event)
        .collect()
}

fn parse_document(input: &[u8]) -> Document {
    Reader::new(input)
        .collect::<Result<Document, _>>()
        .expect("sample document should parse")
}

fn render_document(document: &Document) -> String {
    let mut output = String::new();
    let mut writer = Writer::new(&mut output);
    for event in document {
        writer.push(&event).expect("writer should succeed");
    }
    output
}

fn parse_json_contract_document(source: &str) -> Document {
    Document::from_json_str(source).expect("json sample should convert to KDL contract")
}

fn parse_json_interop_document(source: &str) -> rs_kdl::InteropDocument {
    rs_kdl::InteropDocument::from_json_str(source).expect("json sample should convert to interop")
}

fn render_json_from_contract(document: &Document) -> String {
    document
        .to_json_string()
        .expect("KDL contract document should render back to JSON")
}

fn render_json_from_interop(document: &rs_kdl::InteropDocument) -> String {
    document
        .to_json_string()
        .expect("interop document should render back to JSON")
}

fn parse_yaml_contract_document(source: &str) -> Document {
    Document::from_yaml_str(source).expect("yaml sample should convert to KDL contract")
}

fn parse_yaml_interop_document(source: &str) -> rs_kdl::InteropDocument {
    rs_kdl::InteropDocument::from_yaml_str(source).expect("yaml sample should convert to interop")
}

fn render_yaml_from_contract(document: &Document) -> String {
    document
        .to_yaml_string()
        .expect("KDL contract document should render back to YAML")
}

fn render_yaml_from_interop(document: &rs_kdl::InteropDocument) -> String {
    document
        .to_yaml_string()
        .expect("interop document should render back to YAML")
}

fn parse_toml_contract_document(source: &str) -> Document {
    Document::from_toml_str(source).expect("toml sample should convert to KDL contract")
}

fn parse_toml_interop_document(source: &str) -> rs_kdl::InteropDocument {
    rs_kdl::InteropDocument::from_toml_str(source).expect("toml sample should convert to interop")
}

fn render_toml_from_contract(document: &Document) -> String {
    document
        .to_toml_string()
        .expect("KDL contract document should render back to TOML")
}

fn render_toml_from_interop(document: &rs_kdl::InteropDocument) -> String {
    document
        .to_toml_string()
        .expect("interop document should render back to TOML")
}

fn parse_official_document(input: &[u8]) -> OfficialDocument {
    let text = std::str::from_utf8(input).expect("benchmark sample should be utf-8");
    text.parse::<OfficialDocument>()
        .expect("official sample document should parse")
}

fn render_official_document(document: &OfficialDocument) -> String {
    document.to_string()
}

fn validate_events(events: &[Event]) {
    let mut validator = Validator::new();
    for event in events {
        validator.push(event).expect("event stream should validate");
    }
    validator
        .done()
        .expect("event stream should terminate cleanly");
}

fn collect_document(events: &[Event]) -> Document {
    events.iter().cloned().collect()
}

fn bench_lexer(c: &mut Criterion) {
    c.bench_function("kdl/lexer_tokens", |b| {
        b.iter(|| black_box(lex_token_count(black_box(SAMPLE))));
    });
}

fn bench_reader(c: &mut Criterion) {
    c.bench_function("kdl/reader_events", |b| {
        b.iter(|| black_box(read_events(black_box(SAMPLE))));
    });
}

fn bench_reader_nested(c: &mut Criterion) {
    c.bench_function("kdl/reader_events_nested", |b| {
        b.iter(|| black_box(read_events(black_box(SAMPLE_NESTED))));
    });
}

fn bench_lexer_keywords(c: &mut Criterion) {
    c.bench_function("kdl/lexer_tokens_keywords", |b| {
        b.iter(|| black_box(lex_token_count(black_box(SAMPLE_KEYWORDS))));
    });
}

fn bench_lexer_utf8(c: &mut Criterion) {
    c.bench_function("kdl/lexer_tokens_utf8", |b| {
        b.iter(|| black_box(lex_token_count(black_box(SAMPLE_UTF8.as_bytes()))));
    });
}

fn bench_parse_document(c: &mut Criterion) {
    c.bench_function("kdl/parse_document", |b| {
        b.iter(|| black_box(parse_document(black_box(SAMPLE))));
    });
}

fn bench_parse_document_nested(c: &mut Criterion) {
    c.bench_function("kdl/parse_document_nested", |b| {
        b.iter(|| black_box(parse_document(black_box(SAMPLE_NESTED))));
    });
}

fn bench_parse_document_keywords(c: &mut Criterion) {
    c.bench_function("kdl/parse_document_keywords", |b| {
        b.iter(|| black_box(parse_document(black_box(SAMPLE_KEYWORDS))));
    });
}

fn bench_parse_document_utf8(c: &mut Criterion) {
    c.bench_function("kdl/parse_document_utf8", |b| {
        b.iter(|| black_box(parse_document(black_box(SAMPLE_UTF8.as_bytes()))));
    });
}

fn bench_write_document(c: &mut Criterion) {
    let document = parse_document(SAMPLE);

    c.bench_function("kdl/write_document", |b| {
        b.iter(|| black_box(render_document(black_box(&document))));
    });
}

fn bench_validator(c: &mut Criterion) {
    let events = read_events(SAMPLE);

    c.bench_function("kdl/validator", |b| {
        b.iter(|| validate_events(black_box(&events)));
    });
}

fn bench_document_from_events(c: &mut Criterion) {
    let events = read_events(SAMPLE);

    c.bench_function("kdl/document_from_events", |b| {
        b.iter(|| black_box(collect_document(black_box(&events))));
    });
}

fn bench_document_from_events_nested(c: &mut Criterion) {
    let events = read_events(SAMPLE_NESTED);

    c.bench_function("kdl/document_from_events_nested", |b| {
        b.iter(|| black_box(collect_document(black_box(&events))));
    });
}

fn bench_round_trip(c: &mut Criterion) {
    c.bench_function("kdl/round_trip_parse_write", |b| {
        b.iter(|| {
            let document = parse_document(black_box(SAMPLE));
            black_box(render_document(black_box(&document)));
        });
    });
}

fn bench_compare_official_parse_document(c: &mut Criterion) {
    c.bench_function("kdl/compare/official_parse_document", |b| {
        b.iter(|| black_box(parse_official_document(black_box(SAMPLE))));
    });
}

fn bench_compare_official_write_document(c: &mut Criterion) {
    let document = parse_official_document(SAMPLE);

    c.bench_function("kdl/compare/official_write_document", |b| {
        b.iter(|| black_box(render_official_document(black_box(&document))));
    });
}

fn bench_compare_official_round_trip(c: &mut Criterion) {
    c.bench_function("kdl/compare/official_round_trip_parse_write", |b| {
        b.iter(|| {
            let document = parse_official_document(black_box(SAMPLE));
            black_box(render_official_document(black_box(&document)));
        });
    });
}

fn bench_json_to_kdl_contract(c: &mut Criterion) {
    c.bench_function("kdl/interop/json_to_kdl_contract", |b| {
        b.iter(|| black_box(parse_json_contract_document(black_box(SAMPLE_JSON))));
    });
}

fn bench_json_text_to_interop(c: &mut Criterion) {
    c.bench_function("kdl/interop/json_text_to_interop", |b| {
        b.iter(|| black_box(parse_json_interop_document(black_box(SAMPLE_JSON))));
    });
}

fn bench_json_dom_to_interop(c: &mut Criterion) {
    let json = parse_json(SAMPLE_JSON).expect("json sample should parse");

    c.bench_function("kdl/interop/json_dom_to_interop", |b| {
        b.iter(|| black_box(rs_kdl::InteropDocument::from_json_item(black_box(&json)).unwrap()));
    });
}

fn bench_json_dom_to_kdl_contract(c: &mut Criterion) {
    let json = parse_json(SAMPLE_JSON).expect("json sample should parse");

    c.bench_function("kdl/interop/json_dom_to_kdl_contract", |b| {
        b.iter(|| black_box(Document::from_json_item(black_box(&json)).unwrap()));
    });
}

fn bench_kdl_contract_to_json(c: &mut Criterion) {
    let document = parse_json_contract_document(SAMPLE_JSON);

    c.bench_function("kdl/interop/kdl_contract_to_json", |b| {
        b.iter(|| black_box(render_json_from_contract(black_box(&document))));
    });
}

fn bench_json_interop_to_json(c: &mut Criterion) {
    let interop = parse_json_interop_document(SAMPLE_JSON);

    c.bench_function("kdl/interop/json_interop_to_json", |b| {
        b.iter(|| black_box(render_json_from_interop(black_box(&interop))));
    });
}

fn bench_json_direct_round_trip(c: &mut Criterion) {
    c.bench_function("kdl/interop/json_direct_round_trip", |b| {
        b.iter(|| {
            let interop = parse_json_interop_document(black_box(SAMPLE_JSON));
            black_box(render_json_from_interop(black_box(&interop)));
        });
    });
}

fn bench_json_contract_round_trip(c: &mut Criterion) {
    c.bench_function("kdl/interop/json_contract_round_trip", |b| {
        b.iter(|| {
            let document = parse_json_contract_document(black_box(SAMPLE_JSON));
            black_box(render_json_from_contract(black_box(&document)));
        });
    });
}

fn bench_yaml_text_to_interop(c: &mut Criterion) {
    c.bench_function("kdl/interop/yaml_text_to_interop", |b| {
        b.iter(|| black_box(parse_yaml_interop_document(black_box(SAMPLE_YAML))));
    });
}

fn bench_yaml_value_to_interop(c: &mut Criterion) {
    let yaml = parse_yaml(SAMPLE_YAML).expect("yaml sample should parse");

    c.bench_function("kdl/interop/yaml_value_to_interop", |b| {
        b.iter(|| black_box(rs_kdl::InteropDocument::from_yaml_value(black_box(&yaml)).unwrap()));
    });
}

fn bench_yaml_to_kdl_contract(c: &mut Criterion) {
    c.bench_function("kdl/interop/yaml_to_kdl_contract", |b| {
        b.iter(|| black_box(parse_yaml_contract_document(black_box(SAMPLE_YAML))));
    });
}

fn bench_yaml_kdl_contract_to_text(c: &mut Criterion) {
    let document = parse_yaml_contract_document(SAMPLE_YAML);

    c.bench_function("kdl/interop/kdl_contract_to_yaml", |b| {
        b.iter(|| black_box(render_yaml_from_contract(black_box(&document))));
    });
}

fn bench_yaml_interop_to_text(c: &mut Criterion) {
    let interop = parse_yaml_interop_document(SAMPLE_YAML);

    c.bench_function("kdl/interop/yaml_interop_to_yaml", |b| {
        b.iter(|| black_box(render_yaml_from_interop(black_box(&interop))));
    });
}

fn bench_yaml_direct_round_trip(c: &mut Criterion) {
    c.bench_function("kdl/interop/yaml_direct_round_trip", |b| {
        b.iter(|| {
            let interop = parse_yaml_interop_document(black_box(SAMPLE_YAML));
            black_box(render_yaml_from_interop(black_box(&interop)));
        });
    });
}

fn bench_yaml_contract_round_trip(c: &mut Criterion) {
    c.bench_function("kdl/interop/yaml_contract_round_trip", |b| {
        b.iter(|| {
            let document = parse_yaml_contract_document(black_box(SAMPLE_YAML));
            black_box(render_yaml_from_contract(black_box(&document)));
        });
    });
}

fn bench_toml_text_to_interop(c: &mut Criterion) {
    c.bench_function("kdl/interop/toml_text_to_interop", |b| {
        b.iter(|| black_box(parse_toml_interop_document(black_box(SAMPLE_TOML))));
    });
}

fn bench_toml_table_to_interop(c: &mut Criterion) {
    let arena = TomlArena::new();
    let document = parse_toml(SAMPLE_TOML, &arena).expect("toml sample should parse");

    c.bench_function("kdl/interop/toml_table_to_interop", |b| {
        b.iter(|| {
            black_box(rs_kdl::InteropDocument::from_toml_table(black_box(
                document.table(),
            )))
        });
    });
}

fn bench_toml_to_kdl_contract(c: &mut Criterion) {
    c.bench_function("kdl/interop/toml_to_kdl_contract", |b| {
        b.iter(|| black_box(parse_toml_contract_document(black_box(SAMPLE_TOML))));
    });
}

fn bench_toml_kdl_contract_to_text(c: &mut Criterion) {
    let document = parse_toml_contract_document(SAMPLE_TOML);

    c.bench_function("kdl/interop/kdl_contract_to_toml", |b| {
        b.iter(|| black_box(render_toml_from_contract(black_box(&document))));
    });
}

fn bench_toml_interop_to_text(c: &mut Criterion) {
    let interop = parse_toml_interop_document(SAMPLE_TOML);

    c.bench_function("kdl/interop/toml_interop_to_toml", |b| {
        b.iter(|| black_box(render_toml_from_interop(black_box(&interop))));
    });
}

fn bench_toml_direct_round_trip(c: &mut Criterion) {
    c.bench_function("kdl/interop/toml_direct_round_trip", |b| {
        b.iter(|| {
            let interop = parse_toml_interop_document(black_box(SAMPLE_TOML));
            black_box(render_toml_from_interop(black_box(&interop)));
        });
    });
}

fn bench_toml_contract_round_trip(c: &mut Criterion) {
    c.bench_function("kdl/interop/toml_contract_round_trip", |b| {
        b.iter(|| {
            let document = parse_toml_contract_document(black_box(SAMPLE_TOML));
            black_box(render_toml_from_contract(black_box(&document)));
        });
    });
}

criterion_group!(
    benches,
    bench_lexer,
    bench_lexer_keywords,
    bench_lexer_utf8,
    bench_reader,
    bench_reader_nested,
    bench_parse_document,
    bench_parse_document_nested,
    bench_parse_document_keywords,
    bench_parse_document_utf8,
    bench_write_document,
    bench_validator,
    bench_document_from_events,
    bench_document_from_events_nested,
    bench_round_trip,
    bench_json_text_to_interop,
    bench_json_dom_to_interop,
    bench_json_to_kdl_contract,
    bench_json_dom_to_kdl_contract,
    bench_json_interop_to_json,
    bench_kdl_contract_to_json,
    bench_json_direct_round_trip,
    bench_json_contract_round_trip,
    bench_yaml_text_to_interop,
    bench_yaml_value_to_interop,
    bench_yaml_to_kdl_contract,
    bench_yaml_kdl_contract_to_text,
    bench_yaml_interop_to_text,
    bench_yaml_direct_round_trip,
    bench_yaml_contract_round_trip,
    bench_toml_text_to_interop,
    bench_toml_table_to_interop,
    bench_toml_to_kdl_contract,
    bench_toml_kdl_contract_to_text,
    bench_toml_interop_to_text,
    bench_toml_direct_round_trip,
    bench_toml_contract_round_trip,
    bench_compare_official_parse_document,
    bench_compare_official_write_document,
    bench_compare_official_round_trip
);
criterion_main!(benches);
