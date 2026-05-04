use std::env::consts::ARCH;

use ovmf_prebuilt::{Prebuilt, Source};

fn main() {
    let kernel_path = env!("ILLUMINOS_KERNEL_PATH");

    println!("{}", kernel_path);
    let args = std::env::args().collect::<Vec<String>>();

    let debug = args.len() > 1 && args[1] == "debug";
    let uefi_path = env!("UEFI_PATH");
    let bios_path = env!("BIOS_PATH");
    println!("UEFI_PATH: {}", uefi_path);

    let uefi = std::env::var("ILLUMINOS_USE_BIOS_BOOT").is_err();

    let mut cmd = std::process::Command::new("qemu-system-x86_64");
    if uefi {
        println!("[INFO] fetch ovmf code");
        let prebuilt =
            Prebuilt::fetch(Source::LATEST, "target/ovmf").expect("failed to update prebuilt");
        let ovmf_code = prebuilt.get_file(ovmf_prebuilt::Arch::X64, ovmf_prebuilt::FileType::Code);
        let ovmf_vars = prebuilt.get_file(ovmf_prebuilt::Arch::X64, ovmf_prebuilt::FileType::Vars);
        cmd.arg("-drive").arg(format!(
            "if=pflash,format=raw,file={},readonly=on",
            ovmf_code.display()
        ));
        cmd.arg("-drive").arg(format!(
            "if=pflash,format=raw,file={},readonly=on",
            ovmf_vars.display()
        ));
        cmd.arg("-drive")
            .arg(format!("format=raw,file={}", uefi_path));
    } else {
        cmd.arg("-drive")
            .arg(format!("format=raw,file={bios_path}"));
    }
    cmd.arg("-serial").arg("stdio").args([
        "-device",
        "ide-hd,drive=disk",
        "-drive",
        "if=none,format=raw,file=disk.img,id=disk",
    ]);

    cmd.arg("-enable-kvm");
    cmd.arg("-m").arg("1G");

    if debug {
        cmd.arg("-s").arg("-S");
    }

    cmd.arg("-vga").arg("std");
    cmd.arg("-d").arg("cpu_reset,int").arg("-D").arg("qemu.log");
    cmd.arg("-display").arg("gtk");

    println!("[INFO] Execute QEMU");
    println!("{:?}", cmd);
    let mut child = cmd.spawn().unwrap();
    child.wait().unwrap();
}
