mod support;

use rs_kdl::{LexerError, Reader, ReaderError};

fn first_error(source: &str) -> ReaderError {
    Reader::new(source.as_bytes())
        .find_map(Result::err)
        .map(|(error, _span)| error)
        .expect("fixture should fail to parse")
}

#[test]
fn invalid_number_fixture_reports_lexer_error() {
    let error = first_error(support::INVALID_BAD_NUMBER_INPUT);
    assert!(matches!(
        error,
        ReaderError::Lexer(LexerError::InvalidNumber)
    ));
}

#[test]
fn invalid_type_fixture_reports_missing_close_paren() {
    let error = first_error(support::INVALID_MISSING_CLOSE_PAREN_INPUT);

    assert!(matches!(error, ReaderError::ExpectedCloseParen));
}

#[test]
fn invalid_unclosed_children_fixture_reports_reader_error() {
    let error = first_error(support::INVALID_UNCLOSED_CHILDREN_INPUT);
    assert!(matches!(error, ReaderError::UnclosedOpen));
}

fn assert_redacted(error: &dyn std::error::Error, marker: &str) {
    let mut current = Some(error);
    while let Some(item) = current {
        assert!(!item.to_string().contains(marker));
        assert!(!format!("{item:?}").contains(marker));
        current = item.source();
    }
}

#[test]
fn unexpected_token_values_are_discarded() {
    for (source, marker) in [
        ("node prop=(tag private-fixture", "private-fixture"),
        ("node prop=(tag 987654321", "987654321"),
        ("node prop=(tag #true", "true"),
    ] {
        assert_redacted(&first_error(source), marker);
    }
}

#[cfg(feature = "std")]
#[test]
fn io_errors_preserve_the_kind_and_discard_messages() {
    use rs_kdl::ReadInput;
    use std::io::{self, ErrorKind, Read};
    struct Unavailable;
    impl Read for Unavailable {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "private-fixture",
            ))
        }
    }

    let (error, _) = Reader::new(ReadInput::new(Unavailable))
        .next()
        .unwrap()
        .unwrap_err();
    assert_redacted(&error, "private-fixture");
    assert!(matches!(
        error,
        ReaderError::Lexer(LexerError::Io(ErrorKind::PermissionDenied))
    ));
}

#[cfg(feature = "toml-interop")]
#[test]
fn toml_bridge_drops_error_key_paths() {
    let error = rs_kdl::InteropDocument::from_toml_str("private_fixture = 1\nprivate_fixture = 2")
        .unwrap_err();
    assert_redacted(&error, "private_fixture");
}

#[cfg(feature = "yaml-interop")]
#[test]
fn yaml_bridge_drops_number_literals_and_backend_messages() {
    let model = rs_kdl::InteropDocument::value(rs_kdl::InteropValue::Number(
        rs_kdl::InteropNumber::new("private-fixture"),
    ));
    assert_redacted(&model.to_yaml_value().unwrap_err(), "private-fixture");
    let error = rs_kdl::YamlInteropError::from(rs_yaml::EmitError::Emit("private-fixture"));
    assert_redacted(&error, "private-fixture");
}
