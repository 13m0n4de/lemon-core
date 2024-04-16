# Debug level
export LOG := "DEBUG"

# Target architecture
target := "riscv64gc-unknown-none-elf"
mode := "release"

# Board and bootloader
board := "k210"
sbi := "rustsbi"
bootloader := "bootloader" / sbi + "-" + board + ".bin"

# Directories for user applications, EasyFS-CLI, kernel, test cases and user lib
apps_dir := "apps"
efs_dir := "easy-fs"
efs_root_dir := "easy-fs-root"
efs_fuse_dir := "easy-fs-fuse"
kernel_dir := "kernel"
tests_dir := "tests"
user_dir := "user"

# Kernel binary and entry point 
kernel_entry_pa := "0x80200000"
kernel_elf := kernel_dir / "target" / target / mode / "kernel"
kernel_bin := kernel_elf + ".bin"

# User applications
apps_source_dir := apps_dir / "src/bin"
apps_target_dir := "apps" / "target" / target / mode

# Test cases
tests_source_dir := tests_dir / "src/bin"
tests_target_dir := tests_dir / "target" / target / mode

# File system image
fs_img := apps_target_dir / "fs.img"

# Tools for handling object files
objdump := "rust-objdump --arch-name=riscv64"
objcopy := "rust-objcopy --binary-architecture=riscv64"

# QEMU
bootloader_option := "-bios " + bootloader
display_option := "-nographic"
machine_option := "-machine virt"
loader_option := "-device loader,file=" + kernel_bin + ",addr=" + kernel_entry_pa
drive_option := "-drive file=" + fs_img + ",if=none,format=raw,id=x0"
device_option := "-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0"
qemu_args := machine_option + " " + display_option + " " + bootloader_option + " " + loader_option + " " + drive_option + " " + device_option

# K210
k210_serialport := "/dev/ttyUSB0"
k210_bootloader_size := "131072"

# SDCard
sdcard := "/dev/sda"


# List available recipes
default:
    just --list

# Environment setup
env:
    rustup target add {{target}}
    cargo install cargo-binutils
    rustup component add rust-src
    rustup component add llvm-tools-preview

# Build the user applications
build-apps:
    cd {{apps_dir}} && just build

# Build the filesystem image
build-efs: 
    cd {{efs_fuse_dir}} && just run ../{{apps_source_dir}} ../{{apps_target_dir}}

# Build the kernel
build-kernel:
    cd {{kernel_dir}} && just build {{board}}

# Build the test cases
build-tests:
    cd {{tests_dir}} && just build

# Build App, EFS and Kernel
build: build-apps build-efs build-kernel

# Build the filesystem image and write it to the sdcard
sdcard: build-efs
	@echo "Are you sure write to {{sdcard}} ? [y/N] " && read ans && [ ${ans:-N} = y ]
	sudo dd if=/dev/zero of={{sdcard}} bs=1048576 count=32
	sudo dd if={{fs_img}} of={{sdcard}}


# Run the kernel
run: build
    #!/usr/bin/env bash
    if [[ "{{board}}" == "qemu" ]]; then
        qemu-system-riscv64 {{qemu_args}};
    elif [[ "{{board}}" == "k210" ]]; then
        cp {{bootloader}} {{bootloader}}.copy;
        dd if={{kernel_bin}} of={{bootloader}}.copy bs={{k210_bootloader_size}} seek=1;
        mv {{bootloader}}.copy {{kernel_bin}};
        sudo kflash -p {{k210_serialport}} -b 1500000 {{kernel_bin}};
        # sudo python3 -m serial.tools.miniterm --eol LF --dtr 0 --rts 0 --filter direct {{k210_serialport}} 115200
        # sudo picocom {{k210_serialport}} --baud 115200 --flow n --lower-dtr --lower-rts --imap lfcrlf;
        sudo tio -b 115200 --script "toggle(DTR); toggle(RTS);" -m INLCRNL {{k210_serialport}}
    fi

run-with-tests: build-tests build-efs build-kernel
    # todo!
    qemu-system-riscv64 {{qemu_args}}

# Debug the kernel in QEMU using tmux
debug: build
    tmux new-session -d "qemu-system-riscv64 {{qemu_args}} -s -S"
    tmux split-window -h "gdb-multiarch -ex 'file {{kernel_elf}}' -ex 'target remote localhost:1234'"
    tmux -2 attach-session -d

# Start a GDB server for debugging
gdbserver: build
    qemu-system-riscv64 {{qemu_args}} -s -S

# Connect to the GDB server
gdbclient:
    gdb-multiarch -ex "file {{kernel_elf}}" -ex "target remote localhost:1234"

# Clean build artifacts
clean:
    cd {{apps_dir}} && just clean
    cd {{efs_fuse_dir}} && just clean
    cd {{kernel_dir}} && just clean
    cd {{tests_dir}} && just clean
    cd {{user_dir}} && just clean

# Checks packages to catch common mistakes and improve code.
clippy:
    cd {{apps_dir}} && just clippy 
    cd {{efs_fuse_dir}} && just clippy
    cd {{kernel_dir}} && just clippy
    cd {{tests_dir}} && just clippy
    cd {{user_dir}} && just clippy
