mod support;

use rs_kdl::Document;

#[test]
fn document_round_trip_preserves_structure() {
    let document = support::parse_document(support::WRITER_INPUT);
    let rendered = support::render_document(&document);
    let reparsed = support::parse_document(&rendered);

    assert_eq!(document, reparsed);
}

#[test]
fn event_round_trip_preserves_event_stream() {
    let original_events = support::parse_events(support::WRITER_INPUT);
    support::validate_events(&original_events);

    let document = original_events.clone().into_iter().collect::<Document>();
    let rendered = support::render_document(&document);
    let reparsed_events = support::parse_events(&rendered);

    assert_eq!(original_events, reparsed_events);
}
