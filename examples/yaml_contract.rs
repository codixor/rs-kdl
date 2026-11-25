#[cfg(feature = "yaml-interop")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = r"meta:
  name: rust-misc
  enabled: true
routes:
  - /
  - /dashboard
";

    let document = rs_kdl::Document::from_yaml_str(yaml)?;
    println!("contract nodes: {}", document.nodes.len());
    println!("{}", document.to_yaml_string()?);

    Ok(())
}

#[cfg(not(feature = "yaml-interop"))]
fn main() {
    eprintln!("enable the `yaml-interop` feature to run this example");
}
