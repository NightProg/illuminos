use std::process::Command;

fn main() {
    let kernel_exe = std::env::var("CARGO_BIN_FILE_ILLUMINOS_KERNEL").unwrap();
    
    println!("cargo:rustc-env=ILLUMINOS_KERNEL_PATH={}", kernel_exe);
}
