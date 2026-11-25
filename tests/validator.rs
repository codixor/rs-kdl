mod support;

use rs_kdl::{Entry, Event, Validator, ValidatorError, Value};

#[test]
fn validator_accepts_reader_output() {
    let events = support::parse_events(support::WRITER_INPUT);
    support::validate_events(&events);
}

#[test]
fn validator_rejects_entry_before_node() {
    let mut validator = Validator::new();
    let error = validator
        .push(&Event::Entry(Entry::new_value(Value::String(
            "private-fixture".into(),
        ))))
        .expect_err("entry before node must be invalid");

    assert!(!format!("{error} {error:?}").contains("private-fixture"));
    match error {
        ValidatorError::ExpectedEvent(expected) => {
            assert_eq!(expected, "Node or End");
        }
        other => panic!("unexpected validator error: {other:?}"),
    }
}

#[test]
fn validator_rejects_extra_end() {
    let mut validator = Validator::new();
    let error = validator
        .push(&Event::End)
        .expect_err("extra end must be invalid");

    assert!(matches!(error, ValidatorError::TooManyEnd));
}

#[test]
fn validator_reports_unclosed_document() {
    let mut validator = Validator::new();
    validator
        .push(&Event::Node {
            r#type: None,
            name: "root".into(),
        })
        .expect("node start is valid");

    assert!(matches!(validator.done(), Err(ValidatorError::Unclosed)));
}
