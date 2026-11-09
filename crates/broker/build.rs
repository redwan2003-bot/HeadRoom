fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("msvc") {
        let manifest_path = std::path::Path::new("../../assets/broker.manifest")
            .canonicalize()
            .expect("broker manifest file must exist");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
            manifest_path.display()
        );
    }
    println!("cargo:rerun-if-changed=../../assets/broker.manifest");
}
