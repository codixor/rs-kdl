use rs_kdl::{Category, Document, FormatError, Limits, LocatedError};

fn parse(s: &[u8], limits: Limits) -> Result<Document, LocatedError> {
    Document::parse_with_limits(s, limits)
}

fn at(s: &[u8], limits: Limits, byte: usize) {
    let check = |e: LocatedError| {
        assert_eq!(e.category(), Category::Limit, "{e:?}");
        assert_eq!(e.position().unwrap().offset, Some(byte as u64), "{e:?}");
    };
    check(parse(s, limits).unwrap_err());
    #[cfg(feature = "std")]
    check(
        rs_kdl::Reader::with_limits(rs_kdl::ReadInput::new(s), limits)
            .collect::<Result<Document, _>>()
            .unwrap_err()
            .into(),
    );
}

#[test]
fn encoded_token_bodies_exclude_delimiters() {
    for (source, body, start) in [
        ("node", 4, 0),
        ("n 12345", 5, 2),
        ("n \"a\\tb\"", 4, 3),
        ("n \"é\"", 2, 3),
        ("/*abc*/n", 3, 2),
        ("n //abc", 3, 4),
        ("n ##\"a\"#x\"##", 4, 5),
        ("n \"\"\"\nx\n\"\"\"", 3, 5),
        ("n #true", 5, 2),
    ] {
        let mut limits = Limits {
            max_token_bytes: body,
            ..Limits::default()
        };
        parse(source.as_bytes(), limits).unwrap_or_else(|e| panic!("{source}: {e}"));
        limits.max_token_bytes -= 1;
        at(source.as_bytes(), limits, start + body - 1);
    }
}

#[test]
fn exactly_at_each_bound_and_first_excess() {
    let mut limits = Limits {
        max_document_bytes: 2,
        ..Limits::default()
    };
    parse(b"ab", limits).unwrap();
    at(b"abc", limits, 2);
    limits = Limits {
        max_depth: 1,
        ..Limits::default()
    };
    parse(b"n { x; }", limits).unwrap();
    at(b"n { x { y; }; }", limits, 6);
    parse(b"/*a*/n", limits).unwrap();
    at(b"/*a/*b*/*/n", limits, 3);
    limits = Limits {
        max_expansion: 1,
        ..Limits::default()
    };
    parse(b"n /- 1", limits).unwrap();
    at(b"n /- 1 /- 2", limits, 7);
    limits.max_expansion = 0;
    at(b"/- n", limits, 0);
}

#[test]
fn zero_limits_and_known_positions() {
    let limits = Limits {
        max_depth: 0,
        max_document_bytes: 0,
        max_token_bytes: 0,
        max_expansion: 0,
    };
    parse(b"", limits).unwrap();
    at(b"x", limits, 0);
    let e = parse(b"node {", Limits::default()).unwrap_err();
    assert_eq!(e.category(), Category::Eof);
    assert_eq!(e.position().unwrap().offset, Some(6));
}

#[test]
fn recovery_keeps_the_budget_of_the_failed_token() {
    use rs_kdl::{Lexer, Token};
    let limits = Limits {
        max_token_bytes: 4,
        ..Limits::default()
    };
    for src in [
        b"\"a\\?long\"".as_slice(),
        b"/*a\xfflong*/",
        b"//a\xfflong\n",
    ] {
        let mut lexer = Lexer::with_limits(src, limits);
        let mut errors = Vec::new();
        for _ in 0..12 {
            match lexer.next_token(false).0 {
                Ok(Token::Eof) => break,
                Err(e) => errors.push(e.category()),
                Ok(_) => {}
            }
        }
        assert_eq!(errors, [Category::Syntax, Category::Limit]);
    }
}

#[cfg(all(
    feature = "json-interop",
    feature = "yaml-interop",
    feature = "toml-interop"
))]
#[test]
fn adapters_preserve_a_shared_limit_and_its_position() {
    use rs_kdl::InteropDocument;
    let limits = Limits {
        max_token_bytes: 2,
        ..Limits::default()
    };
    fn check(e: &dyn FormatError, expected: u64) {
        assert_eq!(e.category(), Category::Limit);
        assert_eq!(e.position().unwrap().offset, Some(expected));
        assert!(!format!("{e} {e:?}").contains("purple"));
    }
    check(
        &InteropDocument::from_json_str_with_limits("\"purple\"", limits).unwrap_err(),
        3,
    );
    check(
        &Document::from_json_str_with_limits("\"purple\"", limits).unwrap_err(),
        3,
    );
    check(
        &InteropDocument::from_yaml_str_with_limits("purple", limits).unwrap_err(),
        2,
    );
    check(
        &Document::from_yaml_str_with_limits("purple", limits).unwrap_err(),
        2,
    );
    check(
        &InteropDocument::from_toml_str_with_limits("purple=1", limits).unwrap_err(),
        2,
    );
    check(
        &Document::from_toml_str_with_limits("purple=1", limits).unwrap_err(),
        2,
    );
}

#[test]
fn unicode_indentation_is_consumed_as_complete_characters() {
    let src = "n \"\"\"\n  \u{2002}first\n  \u{2003}second\n  \"\"\"";
    for skip in [false, true] {
        let mut lexer = rs_kdl::Lexer::new(src.as_bytes());
        loop {
            if lexer.next_token(skip).0.unwrap() == rs_kdl::Token::Eof {
                break;
            }
        }
    }
}

#[test]
fn event_reader_can_use_a_higher_depth_than_an_owned_tree() {
    let limits = Limits {
        max_depth: 129,
        ..Limits::default()
    };
    let source = "n{".repeat(129) + &"}".repeat(129);
    let e = Document::parse_with_limits(source.as_bytes(), limits).unwrap_err();
    assert_eq!(e.category(), Category::Limit);
    assert!(rs_kdl::Reader::with_limits(source.as_bytes(), limits).all(|item| item.is_ok()));
}
