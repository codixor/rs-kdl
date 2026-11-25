use rs_kdl::dom::Number;
use rs_kdl::{
    Document, Entry, InteropDocument, InteropError, InteropField, InteropMapEntry, InteropNode,
    InteropRoot, InteropTemporal, InteropTemporalKind, InteropTypedValue, InteropValue, Node,
    Value,
};

#[test]
fn interop_round_trip_preserves_node_structure() {
    let document = Document::from(vec![Node {
        r#type: Some("config".into()),
        name: "app".into(),
        entries: vec![
            Entry {
                name: None,
                r#type: Some("label".into()),
                value: Value::String("demo".into()),
            },
            Entry {
                name: Some("enabled".into()),
                r#type: None,
                value: Value::Bool(true),
            },
            Entry {
                name: Some("retries".into()),
                r#type: Some("count".into()),
                value: Value::Number(Number::from(3_u8)),
            },
        ],
        children: Some(Document::from(vec![Node {
            r#type: None,
            name: "route".into(),
            entries: vec![Entry::new_value("/dashboard".into())],
            children: None,
        }])),
    }]);

    let interop = document.to_interop();
    let reparsed = Document::from_interop(&interop).expect("interop round trip should succeed");

    assert_eq!(reparsed, document);
}

#[test]
fn interop_value_root_is_not_valid_kdl_document() {
    let interop = InteropDocument::value(InteropValue::String("hello".into()));

    let err = Document::from_interop(&interop).expect_err("value-root should fail for KDL");

    assert_eq!(err, InteropError::ExpectedNodeRoot);
}

#[test]
fn interop_composite_values_are_rejected_for_kdl_entries() {
    let interop = InteropDocument {
        root: InteropRoot::Nodes(vec![InteropNode {
            name: "app".into(),
            type_hint: None,
            fields: vec![
                InteropField::Value(InteropTypedValue {
                    type_hint: None,
                    value: InteropValue::Sequence(vec![InteropValue::Bool(true)]),
                }),
                InteropField::Property(rs_kdl::InteropProperty {
                    name: "meta".into(),
                    typed: InteropTypedValue {
                        type_hint: None,
                        value: InteropValue::Map(vec![InteropMapEntry {
                            name: "dark".into(),
                            value: InteropValue::Bool(true),
                        }]),
                    },
                }),
            ],
            children: Vec::new(),
        }]),
    };

    let err =
        Document::from_interop(&interop).expect_err("composite values are not scalar KDL entries");

    assert_eq!(err, InteropError::CompositeValueUnsupported);
}

#[test]
fn interop_temporal_values_round_trip_through_kdl_type_hints() {
    let interop = InteropDocument::nodes(vec![InteropNode {
        name: "build".into(),
        type_hint: None,
        fields: vec![InteropField::Property(rs_kdl::InteropProperty {
            name: "published".into(),
            typed: InteropTypedValue::new(InteropValue::Temporal(InteropTemporal::new(
                InteropTemporalKind::OffsetDateTime,
                "2026-03-31T08:15:00Z",
            ))),
        })],
        children: Vec::new(),
    }]);

    let document = Document::from_interop(&interop).expect("temporal interop should convert");
    let reparsed = document.to_interop();

    assert_eq!(reparsed, interop);
}
