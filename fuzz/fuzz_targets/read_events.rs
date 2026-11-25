#![no_main]
use libfuzzer_sys::fuzz_target;
use rs_kdl::{FormatError, Limits, LocatedError, ReadInput, Reader, Validator};

fuzz_target!(|data: &[u8]| {
    if let [depth, token, expansion, document, input @ ..] = data {
        let limits = Limits {
            max_depth: u32::from(*depth),
            max_token_bytes: usize::from(*token),
            max_expansion: u64::from(*expansion),
            max_document_bytes: u64::from(*document),
        };
        let outcome = |error: LocatedError| (error.category(), error.position());
        let slice = Reader::with_limits(input, limits)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| outcome(e.into()));
        let stream = Reader::with_limits(ReadInput::new(input), limits)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| outcome(e.into()));
        assert_eq!(slice, stream);
    }
    let mut validator = Validator::new();
    for item in Reader::new(data) {
        let Ok((event, _)) = item else { return };
        validator
            .push(&event)
            .expect("reader produced invalid events");
    }
    validator.done().expect("reader left an open event");
});
