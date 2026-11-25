mod support;

#[test]
fn string_fixture_matches_writer_surface() {
    let document = support::parse_document(support::STRINGS_INPUT);
    assert_eq!(
        support::render_document(&document),
        support::STRINGS_SURFACE
    );
}

#[test]
fn string_fixture_round_trips_through_document_and_events() {
    let document = support::parse_document(support::STRINGS_INPUT);
    let rendered = support::render_document(&document);
    let reparsed = support::parse_document(&rendered);
    assert_eq!(document, reparsed);

    let events = support::parse_events(support::STRINGS_INPUT);
    support::validate_events(&events);
}
