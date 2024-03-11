# Debug level
export LOG := "DEBUG"

# Target architecture
target := "riscv64gc-unknown-none-elf"
mode := "release"

# Board and bootloader
board := "qemu"
sbi := "rustsbi"
bootloader := "bootloader" / sbi + "-" + board + ".bin"

# Directories for kernel, user applications, and EasyFS-CLI
kernel_dir := "os"
user_dir := "user"
efs_cli_dir := "easy-fs-fuse"

# Kernel binary and entry point 
kernel_entry_pa := "0x80200000"
kernel_elf := kernel_dir / "target" / target / mode / "os"
kernel_bin := kernel_elf + ".bin"

# User applications and filesystem image paths
app_dir := user_dir / "src/bin"
app_target_dir := user_dir / "target" / target / mode
fs_img := app_target_dir / "fs.img"

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
    cd {{user_dir}} && just build

# Build the filesystem image
build-efs: 
    cd {{efs_cli_dir}} && just run ../{{app_dir}} ../{{app_target_dir}}

# Build the kernel
build-kernel:
    cd {{kernel_dir}} && just build {{board}}

# Build App, EFS and Kernel
build: build-apps build-efs build-kernel

# Run the kernel in QEMU
run: build
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
    cd {{user_dir}} && just clean
    cd {{efs_cli_dir}} && just clean
    cd {{kernel_dir}} && just clean

clippy:
    cd {{user_dir}} && just clippy
    cd {{efs_cli_dir}} && just clippy
    cd {{kernel_dir}} && just clippy
