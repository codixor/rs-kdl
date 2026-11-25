mod support;

use rs_kdl::{LexerError, Reader, ReaderError, Token};

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

    match error {
        ReaderError::ExpectedCloseParen(Token::String(text)) => {
            assert_eq!(text, "value");
        }
        other => panic!("unexpected parse error: {other:?}"),
    }
}

#[test]
fn invalid_unclosed_children_fixture_reports_reader_error() {
    let error = first_error(support::INVALID_UNCLOSED_CHILDREN_INPUT);
    assert!(matches!(error, ReaderError::UnclosedOpen));
}
