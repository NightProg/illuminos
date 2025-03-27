set -xe
cargo bootimage

dd if=/dev/zero of=disk.img bs=1M count=10
dd if=target/x86_64-illuminos/debug/bootimage-illuminos.bin of=disk.img conv=notrunc
qemu-system-x86_64 -drive format=raw,file=disk.img,id=ahci0 -device ahci,id=ahci0 -smp 4 -display sdl,gl=on  -device virtio-vga-gl
