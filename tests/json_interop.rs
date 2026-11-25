use rs_json::parse as parse_json;
use rs_kdl::{Document, InteropDocument, InteropValue, JsonInteropError, Node};

#[test]
fn json_writer_validates_the_entire_numeric_spelling() {
    use rs_kdl::{Category, FormatError, InteropNumber};
    for text in ["inf", "-nan", "01", "1e", "1 ", " 1", "2,3"] {
        let doc = InteropDocument::value(InteropValue::Number(InteropNumber::new(text)));
        assert_eq!(doc.to_json_string().unwrap_err().category(), Category::Data);
    }
    let doc = InteropDocument::value(InteropValue::Number(InteropNumber::new("-1.20e+99")));
    assert_eq!(doc.to_json_string().unwrap(), "-1.20e+99");
    #[cfg(feature = "toml-interop")]
    assert_eq!(
        InteropDocument::from_toml_str("rate = inf")
            .unwrap()
            .to_json_string()
            .unwrap_err()
            .category(),
        Category::Data
    );
}

#[test]
fn generated_kdl_observes_its_own_depth_limit() {
    use rs_kdl::{Category, FormatError, Limits};
    let object = |n: usize| format!("{}0{}", "{\"k\":".repeat(n), "}".repeat(n));
    for (max_depth, levels) in [(4, 2), (128, 64)] {
        let limits = Limits {
            max_depth,
            ..Limits::default()
        };
        let doc = Document::from_json_str_with_limits(&object(levels), limits).unwrap();
        Document::parse_with_limits(doc.to_string().as_bytes(), limits).unwrap();
        let over = object(levels + 1);
        assert!(InteropDocument::from_json_str_with_limits(&over, limits).is_ok());
        assert_eq!(
            Document::from_json_str_with_limits(&over, limits)
                .unwrap_err()
                .category(),
            Category::Limit
        );
    }
}

const JSON_SAMPLE: &str = r#"{
  "meta": {
    "name": "rust-misc",
    "emoji": "\uD83D\uDE42",
    "enabled": true,
    "retries": 3
  },
  "routes": ["/", "/dashboard"],
  "nothing": null
}"#;

#[test]
fn json_text_round_trip_through_interop_contract() {
    let interop = InteropDocument::from_json_str(JSON_SAMPLE).expect("json should parse");
    let rendered = interop
        .to_json_string()
        .expect("interop value-root should render");
    let reparsed = InteropDocument::from_json_str(&rendered).expect("rendered json should parse");

    assert_eq!(reparsed, interop);
}

#[test]
fn json_dom_bridge_matches_json_text_bridge() {
    let json = parse_json(JSON_SAMPLE).expect("json sample should parse");
    let from_dom = InteropDocument::from_json_item(json.root()).expect("dom bridge should succeed");
    let from_text = InteropDocument::from_json_str(JSON_SAMPLE).expect("text bridge should parse");

    assert_eq!(from_dom, from_text);
}

#[test]
fn json_dom_to_kdl_contract_matches_text_bridge() {
    let json = parse_json(JSON_SAMPLE).expect("json sample should parse");
    let from_dom =
        Document::from_json_item(json.root()).expect("dom contract bridge should succeed");
    let from_text =
        Document::from_json_str(JSON_SAMPLE).expect("text contract bridge should parse");

    assert_eq!(from_dom, from_text);
}

#[test]
fn json_text_round_trip_through_kdl_contract_document() {
    let document = Document::from_json_str(JSON_SAMPLE).expect("json should convert to KDL");
    let rendered = document
        .to_json_string()
        .expect("contract document should convert back to json");

    let baseline = InteropDocument::from_json_str(JSON_SAMPLE).expect("baseline should parse");
    let round_trip =
        InteropDocument::from_json_str(&rendered).expect("round-tripped json should parse");

    assert_eq!(round_trip, baseline);
}

#[test]
fn scalar_json_root_round_trip_through_kdl_contract() {
    let document =
        Document::from_json_str(r#""hello\nworld""#).expect("string root should convert");
    let rendered = document
        .to_json_string()
        .expect("contract document should render string root");

    let round_trip =
        InteropDocument::from_json_str(&rendered).expect("rendered string root should parse");
    assert_eq!(
        round_trip.root,
        rs_kdl::InteropRoot::Value(InteropValue::String("hello\nworld".into()))
    );
}

#[test]
fn invalid_contract_document_is_rejected() {
    let invalid = Document::from(vec![Node::new("json-entry")]);

    let error = invalid
        .to_json_string()
        .expect_err("invalid contract document should fail");

    assert_eq!(
        error,
        JsonInteropError::InvalidContract("unexpected node name in JSON contract document")
    );
}

#[test]
fn native_json_values_decode_escaped_keys_and_keep_number_lexemes() {
    let source = r#"{"\u006b":"\uD83D\uDE42","n":1.250e+02}"#;
    let parsed = parse_json(source).unwrap();
    let model = InteropDocument::from_json_item(parsed.root()).unwrap();
    assert_eq!(
        model.to_json_string().unwrap(),
        r#"{"k":"🙂","n":1.250e+02}"#
    );
}
