mod support;

#[test]
fn nan_round_trips_at_both_float_widths() {
    use rs_kdl::dom::Number;
    assert_eq!(Number::from(f32::NAN), Number::NAN);
    assert_eq!(Number::from(f64::NAN), Number::NAN);
    assert!(f32::try_from(Number::from(f32::NAN)).unwrap().is_nan());
    assert!(f64::try_from(Number::from(f64::NAN)).unwrap().is_nan());
}

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
