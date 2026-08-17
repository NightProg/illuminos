use std::{env::consts::ARCH, fs};

use ovmf_prebuilt::{Prebuilt, Source};

pub const ILLUMINOS_IMAGE_NAME: &str = "illuminos.iso";

fn run_target(target: &str, flags: &str) {
    match target {
        "all" => {
            run_target("iso", flags);
            run_target("ovmf", flags);
            run_target("qemu", flags);
        }
        "iso" => {
            fs::remove_file(ILLUMINOS_IMAGE_NAME);
            fs::remove_dir_all("iso_root");
            fs::create_dir_all("iso_root/boot/limine").unwrap();
            fs::copy(env!("ILLUMINOS_KERNEL_PATH"), "iso_root/boot/kernel").unwrap();
            fs::copy("limine.conf", "iso_root/boot/limine/limine.conf").unwrap();
            fs::create_dir_all("iso_root/EFI/BOOT").unwrap();
            fs::copy("limine-binary/BOOTX64.EFI", "iso_root/EFI/BOOT/BOOTX64.EFI").unwrap();
            fs::copy(
                "limine-binary/limine-uefi-cd.bin",
                "iso_root/boot/limine/limine-uefi-cd.bin",
            )
            .unwrap();
            fs::copy(
                "limine-binary/limine-bios-cd.bin",
                "iso_root/boot/limine/limine-bios-cd.bin",
            )
            .unwrap();
            fs::copy(
                "limine-binary/limine-bios.sys",
                "iso_root/boot/limine/limine-bios.sys",
            )
            .unwrap();
            let mut xorriso_cmd = std::process::Command::new("xorriso");
            xorriso_cmd.args([
                "-as",
                "mkisofs",
                "-b",
                "boot/limine/limine-bios-cd.bin",
                "-no-emul-boot",
                "-boot-load-size",
                "4",
                "-boot-info-table",
                "--efi-boot",
                "boot/limine/limine-uefi-cd.bin",
                "-efi-boot-part",
                "--efi-boot-image",
                "--protective-msdos-label",
                "iso_root",
                "-o",
                ILLUMINOS_IMAGE_NAME,
            ]);

            let status = xorriso_cmd.status().unwrap();
            if !status.success() {
                panic!("xorriso failed with status: {}", status);
            }
            let mut limine_cmd = std::process::Command::new("./limine-binary/limine");
            limine_cmd.args(["bios-install", ILLUMINOS_IMAGE_NAME]);
            let status = limine_cmd.status().unwrap();
            if !status.success() {
                panic!("limine failed with status: {}", status);
            }
        }
        "ovmf" => {
            let prebuilt =
                Prebuilt::fetch(Source::LATEST, "target/ovmf").expect("failed to update prebuilt");
            let ovmf_code =
                prebuilt.get_file(ovmf_prebuilt::Arch::X64, ovmf_prebuilt::FileType::Code);
            let ovmf_vars =
                prebuilt.get_file(ovmf_prebuilt::Arch::X64, ovmf_prebuilt::FileType::Vars);
            println!("OVMF code: {}", ovmf_code.display());
            println!("OVMF vars: {}", ovmf_vars.display());
        }
        "qemu" => {
            if !fs::exists(ILLUMINOS_IMAGE_NAME).unwrap() {
                run_target("iso", flags);
            }
            let is_ovmf_available = {
                let code_available = fs::metadata("target/ovmf/x64/code.fd").is_ok();
                let vars_available = fs::metadata("target/ovmf/x64/vars.fd").is_ok();
                code_available && vars_available
            };
            if !is_ovmf_available {
                run_target("ovmf", flags);
            }
            let mut cmd = std::process::Command::new("qemu-system-x86_64");
            cmd.arg("-drive")
                .arg("if=pflash,format=raw,file=target/ovmf/x64/code.fd,readonly=on");
            cmd.arg("-drive")
                .arg("if=pflash,format=raw,file=target/ovmf/x64/vars.fd,readonly=on");

            cmd.arg("-cdrom").arg(ILLUMINOS_IMAGE_NAME);
            cmd.args([
                /*"-M",
                "q35",*/
                "-serial",
                "stdio",
                "-enable-kvm",
                "-m",
                "1G",
                "-vga",
                "std",
                "-d",
                "cpu_reset,int",
                "-D",
                "qemu.log",
                "-display",
                "gtk",
            ]);

            cmd.args([
                "-device",
                "ide-hd,drive=disk,bus=ide.0",
                "-drive",
                "if=none,format=raw,file=disk.img,id=disk",
            ]);

            if flags.contains("debug") {
                cmd.args(["-s", "-S"]);
            }

            let mut child = cmd.spawn().unwrap();
            child.wait().unwrap();
        }
        t => {
            eprintln!("Unknown target: {}", t);
            std::process::exit(1);
        }
    }
}

fn main() {
    let kernel_path = env!("ILLUMINOS_KERNEL_PATH");

    let prebuilt =
        Prebuilt::fetch(Source::LATEST, "target/ovmf").expect("failed to update prebuilt");
    let ovmf_code = prebuilt.get_file(ovmf_prebuilt::Arch::X64, ovmf_prebuilt::FileType::Code);
    let ovmf_vars = prebuilt.get_file(ovmf_prebuilt::Arch::X64, ovmf_prebuilt::FileType::Vars);

    println!("{}", kernel_path);

    let args = std::env::args().collect::<Vec<String>>();

    let target = args.get(1).map(|s| s.as_str()).unwrap_or("all");

    let flags = args.get(2).map(|s| s.as_str()).unwrap_or("");

    run_target(target, flags);
    /*
    let mut cmd = std::process::Command::new("qemu-system-x86_64");
    cmd.arg("-drive").arg(format!(
        "if=pflash,format=raw,file={},readonly=on",
        ovmf_code.display()
    ));
    cmd.arg("-drive").arg(format!(
        "if=pflash,format=raw,file={},readonly=on",
        ovmf_vars.display()
    ));

    if uefi {
        println!("[INFO] fetch ovmf code");

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
    cmd.arg("-vga").arg("std");
    cmd.arg("-d").arg("cpu_reset,int").arg("-D").arg("qemu.log");
    cmd.arg("-display").arg("gtk");

    println!("[INFO] Execute QEMU");
    println!("{:?}", cmd);
    let mut child = cmd.spawn().unwrap();
    child.wait().unwrap();
    */
}
