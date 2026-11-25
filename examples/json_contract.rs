fn main() {
    let json = r#"{"name":"rust-misc","enabled":true,"routes":["/","/dashboard"]}"#;

    let document =
        rs_kdl::Document::from_json_str(json).expect("json should convert to KDL contract");
    println!("KDL contract:\n{document}");

    let rendered = document
        .to_json_string()
        .expect("contract should render back to json");
    println!("JSON again:\n{rendered}");
}
