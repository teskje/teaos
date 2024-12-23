#! /bin/bash

qemu="$(dirname "$0")"

mkdir -p "$qemu/esp/efi/boot"
cp "$1" "$qemu/esp/efi/boot/bootaa64.efi"

cd "$qemu"
qemu-system-aarch64 \
    -machine virt \
    -cpu neoverse-n1 \
    -m 512M \
    -drive if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-aarch64-code.fd \
    -drive format=raw,file=fat:rw:esp \
    -nographic
