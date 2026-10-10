fn main() {
    println!("cargo:rerun-if-changed=../../scripts/print-macos.swift");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let output =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("compose-print");
        let status = std::process::Command::new("swiftc")
            .args(["-O", "../../scripts/print-macos.swift", "-o"])
            .arg(output)
            .status()
            .expect("Swift compiler is required to build the macOS native print adapter");
        assert!(status.success(), "Could not build the macOS print adapter");
    }
}
