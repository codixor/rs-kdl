#![cfg(feature = "yaml-interop")]

use rs_kdl::{Document, InteropDocument, InteropRoot, InteropValue, Node, YamlInteropError};
use rs_yaml::safe_load as parse_yaml;

const YAML_SAMPLE: &str = r"meta:
  name: rust-misc
  enabled: true
  retries: 3
routes:
  - /
  - /dashboard
nothing: null
ratio: 1.0
";

#[test]
fn yaml_text_round_trip_through_direct_interop() {
    let interop = InteropDocument::from_yaml_str(YAML_SAMPLE).expect("yaml should parse");
    let rendered = interop
        .to_yaml_string()
        .expect("interop value-root should render");
    let reparsed = InteropDocument::from_yaml_str(&rendered).expect("rendered yaml should parse");

    assert_eq!(reparsed, interop);
}

#[test]
fn yaml_value_bridge_matches_text_bridge() {
    let value = parse_yaml(YAML_SAMPLE).expect("yaml sample should parse");
    let from_value = InteropDocument::from_yaml_value(&value).expect("value bridge should work");
    let from_text = InteropDocument::from_yaml_str(YAML_SAMPLE).expect("text bridge should work");

    assert_eq!(from_value, from_text);
}

#[test]
fn yaml_text_round_trip_through_kdl_contract_document() {
    let document = Document::from_yaml_str(YAML_SAMPLE).expect("yaml should convert to KDL");
    let rendered = document
        .to_yaml_string()
        .expect("contract document should render back to yaml");

    let baseline = InteropDocument::from_yaml_str(YAML_SAMPLE).expect("baseline should parse");
    let round_trip =
        InteropDocument::from_yaml_str(&rendered).expect("round-tripped yaml should parse");

    assert_eq!(round_trip, baseline);
}

#[test]
fn scalar_yaml_root_round_trip_through_kdl_contract() {
    let document = Document::from_yaml_str("1.0\n").expect("float root should convert");
    let rendered = document
        .to_yaml_string()
        .expect("contract document should render float root");

    let round_trip =
        InteropDocument::from_yaml_str(&rendered).expect("rendered float root should parse");
    assert_eq!(
        round_trip.root,
        InteropRoot::Value(InteropValue::Number(rs_kdl::InteropNumber::new("1.0")))
    );
}

#[test]
fn non_string_yaml_map_keys_are_rejected() {
    let error = InteropDocument::from_yaml_str("1: value\n").expect_err("integer keys should fail");

    assert!(matches!(
        error,
        YamlInteropError::UnsupportedMapKey("integer scalar")
    ));
}

#[test]
fn tagged_yaml_values_are_rejected_by_the_neutral_bridge() {
    let error = InteropDocument::from_yaml_str("tagged: !custom value\n")
        .expect_err("tagged yaml values should fail");

    assert!(matches!(
        error,
        YamlInteropError::UnsupportedYamlValue("tagged value")
    ));
}

#[test]
fn invalid_yaml_contract_document_is_rejected() {
    let invalid = Document::from(vec![Node::new("yaml-entry")]);

    let error = invalid
        .to_yaml_string()
        .expect_err("invalid contract document should fail");

    assert!(matches!(
        error,
        YamlInteropError::InvalidContract("unexpected node name in YAML contract document")
    ));
}
