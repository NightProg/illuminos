set -xe
cargo bootimage
qemu-system-x86_64 -drive format=raw,file=target/x86_64-illuminos/debug/bootimage-illuminos.bin
