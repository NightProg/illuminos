use std::path::Path;

use bootloader::UefiBoot;

fn main() {
    let boot_info = UefiBoot::new(
        Path::new("target/x86_64-unknown-none/debug/illuminos"),
    );

    boot_info.create_disk_image(Path::new("illuminos-uefi.img")).unwrap();
}
