#[cfg(feature = "toml-interop")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let toml = r#"
title = "rust-misc"
enabled = true
published = 2026-03-31T08:15:00Z
"#;

    let document = rs_kdl::Document::from_toml_str(toml)?;
    println!("contract nodes: {}", document.nodes.len());
    println!("{}", document.to_toml_string()?);

    Ok(())
}

#[cfg(not(feature = "toml-interop"))]
fn main() {
    eprintln!("enable the `toml-interop` feature to run this example");
}
