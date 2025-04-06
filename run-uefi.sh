set -xe
cargo build --target x86_64-unknown-none
cargo run --bin illuminos-uefi --features illuminos-boot-features

qemu-system-x86_64 -enable-kvm \
  -drive if=pflash,format=raw,readonly=on,file=uefi/OVMF_CODE_4M.fd \
  -drive if=pflash,format=raw,readonly=on,file=uefi/OVMF_VARS_4M.fd \
  -drive format=raw,file=illuminos-uefi.img \
  -serial mon:stdio \
  -device ide-hd,drive=disk -drive if=none,file=disk.img,format=raw,id=disk
