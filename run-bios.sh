set -xe
cargo build --target x86_64-unknown-none
cargo run --bin illuminos-bios --features illuminos-boot-features

qemu-system-x86_64 -enable-kvm \
    -drive format=raw,file=illuminos-bios.img \
      -serial mon:stdio \
        -device ide-hd,drive=disk -drive if=none,file=disk.img,format=raw,id=disk
