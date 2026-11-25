use rs_kdl::{Reader, Validator};

fn main() {
    let mut validator = Validator::new();
    let source: &[u8] = br#"job "build" retries=3"#;

    for event in Reader::new(source) {
        let (event, _) = event.expect("valid KDL event");
        validator.push(&event).expect("valid event stream");
    }

    validator.done().expect("closed document");
    println!("event stream is valid");
}
