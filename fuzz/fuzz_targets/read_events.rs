#![no_main]
use libfuzzer_sys::fuzz_target;
use rs_kdl::{Reader, Validator};

fuzz_target!(|data: &[u8]| {
    let mut validator = Validator::new();
    for item in Reader::new(data) {
        let Ok((event, _)) = item else { return };
        validator
            .push(&event)
            .expect("reader produced invalid events");
    }
    validator.done().expect("reader left an open event");
});
