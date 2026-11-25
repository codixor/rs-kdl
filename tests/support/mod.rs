#![allow(dead_code, reason = "shared integration-test support")]

use rs_kdl::{Document, Event, Reader, Validator, Writer};

pub const WRITER_INPUT: &str = include_str!("../golden/writer_input.kdl");
pub const WRITER_SURFACE: &str = include_str!("../golden/writer_surface.kdl");
pub const NORMALIZE_INPUT: &str = include_str!("../golden/normalize_input.kdl");
pub const NORMALIZE_SURFACE: &str = include_str!("../golden/normalize_surface.kdl");
pub const STRINGS_INPUT: &str = include_str!("../golden/strings_input.kdl");
pub const STRINGS_SURFACE: &str = include_str!("../golden/strings_surface.kdl");
pub const NUMBERS_INPUT: &str = include_str!("../golden/numbers_input.kdl");
pub const NUMBERS_SURFACE: &str = include_str!("../golden/numbers_surface.kdl");
pub const ANNOTATIONS_INPUT: &str = include_str!("../golden/annotations_input.kdl");
pub const ANNOTATIONS_SURFACE: &str = include_str!("../golden/annotations_surface.kdl");
pub const INVALID_BAD_NUMBER_INPUT: &str = include_str!("../golden/invalid_bad_number.kdl");
pub const INVALID_MISSING_CLOSE_PAREN_INPUT: &str =
    include_str!("../golden/invalid_missing_close_paren.kdl");
pub const INVALID_UNCLOSED_CHILDREN_INPUT: &str =
    include_str!("../golden/invalid_unclosed_children.kdl");

pub fn parse_document(source: &str) -> Document {
    Reader::new(source.as_bytes())
        .collect::<Result<Document, _>>()
        .expect("fixture should parse")
}

pub fn parse_events(source: &str) -> Vec<Event> {
    Reader::new(source.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .expect("fixture should parse")
        .into_iter()
        .map(|(event, _)| event)
        .collect()
}

pub fn render_document(document: &Document) -> String {
    let mut output = String::new();
    let mut writer = Writer::new(&mut output);
    for event in document {
        writer.push(&event).expect("writer should succeed");
    }
    output.push('\n');
    output
}

pub fn validate_events(events: &[Event]) {
    let mut validator = Validator::new();
    for event in events {
        validator.push(event).expect("event stream should validate");
    }
    validator
        .done()
        .expect("event stream should terminate cleanly");
}
