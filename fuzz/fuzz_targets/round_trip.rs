#![no_main]
use libfuzzer_sys::fuzz_target;
use rs_kdl::{Document, Reader};

fuzz_target!(|data: &[u8]| {
    let Ok(document) = Reader::new(data).collect::<Result<Document, _>>() else {
        return;
    };
    let rendered = document.to_string();
    let reparsed = Reader::new(rendered.as_bytes())
        .collect::<Result<Document, _>>()
        .expect("writer produced invalid KDL");
    assert_eq!(rendered, reparsed.to_string());
});
