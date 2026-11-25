use rs_kdl::{Document, Reader, Writer};

fn main() {
    let source: &[u8] = br"root { child answer=42 }";
    let document = Reader::new(source)
        .collect::<Result<Document, _>>()
        .expect("valid KDL document");

    let mut output = String::new();
    let mut writer = Writer::new(&mut output);
    for event in &document {
        writer.push(&event).expect("write succeeds");
    }

    println!("{output}");
}
