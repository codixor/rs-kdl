mod support;

#[test]
fn annotation_fixture_matches_writer_surface() {
    let document = support::parse_document(support::ANNOTATIONS_INPUT);
    assert_eq!(
        support::render_document(&document),
        support::ANNOTATIONS_SURFACE
    );
}

#[test]
fn annotation_fixture_round_trips_with_type_hints_intact() {
    let document = support::parse_document(support::ANNOTATIONS_INPUT);
    let rendered = support::render_document(&document);
    let reparsed = support::parse_document(&rendered);
    assert_eq!(document, reparsed);

    let events = support::parse_events(support::ANNOTATIONS_INPUT);
    support::validate_events(&events);
}
