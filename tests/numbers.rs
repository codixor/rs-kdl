mod support;

#[test]
fn number_fixture_matches_writer_surface() {
    let document = support::parse_document(support::NUMBERS_INPUT);
    assert_eq!(
        support::render_document(&document),
        support::NUMBERS_SURFACE
    );
}

#[test]
fn number_fixture_round_trips_through_document_and_events() {
    let document = support::parse_document(support::NUMBERS_INPUT);
    let rendered = support::render_document(&document);
    let reparsed = support::parse_document(&rendered);
    assert_eq!(document, reparsed);

    let events = support::parse_events(support::NUMBERS_INPUT);
    support::validate_events(&events);
}
