mod support;

#[cfg(feature = "std")]
#[test]
fn normalize_matches_golden_surface() {
    let mut document = support::parse_document(support::NORMALIZE_INPUT);
    document.normalize();

    assert_eq!(
        support::render_document(&document),
        support::NORMALIZE_SURFACE
    );
}
