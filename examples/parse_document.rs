use rs_kdl::{Document, Reader};

fn main() {
    let source: &[u8] = br#"
app "demo" enabled=#true {
    route "/"
}
"#;

    let document = Reader::new(source)
        .collect::<Result<Document, _>>()
        .expect("valid KDL document");

    println!("{document}");
}
