use bootloader::BootConfig;
use bootloader::UefiBoot;
use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let use_bios = env::var("ILLUMINOS_USE_BIOS_BOOT").is_ok();
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let uefi_path = out_dir.join("illuminos_uefi.img");
    let bios_path = out_dir.join("illuminos_bios.img");
    let kernel = env::var_os("CARGO_BIN_FILE_ILLUMINOS_KERNEL")
        .expect(concat!(
            "The os var CARGO_BIN_FILE_ILLUMINOS_KERNEL should be set by cargo build",
            " and should point to the path of the kernel binary",
            "otherwise try to setup yourself",
        ))
        .into_string()
        .unwrap();

    let kernel = Path::new(&kernel);

    if use_bios {
        panic!("Using BIOS boot is not supported now");
    } else {
        println!("Using UEFI boot");
        let uefi_boot = UefiBoot::new(kernel);
        uefi_boot.create_disk_image(uefi_path.as_path()).unwrap();
    }

    println!("cargo:rustc-env=UEFI_PATH={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());
    println!("cargo:rerun-if-changed={}", kernel.display());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rustc-env=ILLUMINOS_KERNEL_PATH={}", kernel.display());

    if use_bios {
        println!("cargo:rustc-env=ILLUMINOS_USE_BIOS_BOOT=1");
    }
}
