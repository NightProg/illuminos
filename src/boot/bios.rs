use std::path::Path;

use bootloader::BiosBoot;

fn main() {
    let boot_info = BiosBoot::new(
        Path::new("target/x86_64-unknown-none/debug/illuminos"),
    );

    boot_info.create_disk_image(Path::new("illuminos-bios.img")).unwrap();
}
