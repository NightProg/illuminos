fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let target = std::env::var("TARGET").unwrap();
    let profile = std::env::var("PROFILE").unwrap();
    let name = std::env::var("CARGO_PKG_NAME").unwrap();

    let exe = format!("{manifest_dir}/target/{target}/{profile}/{name}");

    println!("cargo:rustc-env=KERNEL_PATH={exe}");
    println!("cargo:rustc-link-arg=-Tlinker.ld");
    println!("cargo:rerun-if-changed=linker.ld");
}
