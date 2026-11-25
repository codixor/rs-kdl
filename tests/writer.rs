mod support;

use rs_kdl::{Document, Reader, Writer};

#[test]
fn writer_matches_golden_surface() {
    let document = support::parse_document(support::WRITER_INPUT);
    assert_eq!(support::render_document(&document), support::WRITER_SURFACE);
}

#[test]
fn display_matches_writer_output() {
    let document = support::parse_document(support::WRITER_INPUT);
    assert_eq!(format!("{document}\n"), support::WRITER_SURFACE);
}

#[test]
fn writer_supports_custom_indent() {
    let source: &[u8] = br"root { child { leaf } }";
    let document = Reader::new(source)
        .collect::<Result<Document, _>>()
        .expect("valid KDL document");

    let mut output = String::new();
    let mut writer = Writer::new(&mut output);
    writer.set_indent("  ");
    for event in &document {
        writer.push(&event).expect("writer should succeed");
    }

    assert_eq!(output, "root {\n  child {\n    leaf\n  }\n}");
}
