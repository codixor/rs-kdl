use rs_kdl::Reader;

fn main() {
    let source: &[u8] = br#"service "api" port=8080"#;

    for event in Reader::new(source) {
        let (event, span) = event.expect("valid KDL event");
        println!("{span:?}: {event:?}");
    }
}
