# Debug level
export LOG := "DEBUG"

# Target architecture
target := "riscv64gc-unknown-none-elf"
mode := "release"

# Board and bootloader
board := "qemu"
sbi := "rustsbi"
bootloader := "bootloader" / sbi + "-" + board + ".bin"

# Directories for user applications, EasyFS-CLI, kernel, test cases and user lib
apps_dir := "apps"
efs_dir := "easy-fs"
efs_root_dir := "easy-fs-root"
efs_tool_dir := "easy-fs-tool"
kernel_dir := "kernel"
tests_dir := "tests"
user_dir := "user"

# Kernel binary and entry point 
kernel_entry_pa := "0x80200000"
kernel_elf := kernel_dir / "target" / target / mode / "kernel"
kernel_bin := kernel_elf + ".bin"

# User applications
apps_source_dir := apps_dir / "src/bin"
apps_target_dir := apps_dir / "target" / target / mode

# Test cases
tests_source_dir := tests_dir / "src/bin"
tests_target_dir := tests_dir / "target" / target / mode

# File system image
fs_img := efs_tool_dir / "target" / mode / "fs.img"

# Tools for handling object files
objdump := "rust-objdump --arch-name=riscv64"
objcopy := "rust-objcopy --binary-architecture=riscv64"

# QEMU
machine_option := "-machine virt"
bootloader_option := "-bios " + bootloader
serial_option := "-serial stdio"
loader_option := "-device loader,file=" + kernel_bin + ",addr=" + kernel_entry_pa
drive_option := "-drive file=" + fs_img + ",if=none,format=raw,id=x0"
blk_device_option := "-device virtio-blk-device,drive=x0"
gpu_device_option := "-device virtio-gpu-device"
keyboard_device_option := "-device virtio-keyboard-device"
mouse_device_option := "-device virtio-mouse-device"

qemu_args := machine_option+ " " + \
			 bootloader_option + " " + \
			 serial_option + " " + \
			 loader_option + " " + \
			 drive_option + " " + \
			 blk_device_option + " " + \
			 gpu_device_option + " " + \
             keyboard_device_option + " " + \
             mouse_device_option


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
    mkdir -p {{efs_root_dir}}/bin/
    for app in `find {{apps_source_dir}} -name "*.rs"`; do \
        app_name=`basename $app .rs`; \
        cp "{{apps_target_dir}}/$app_name" {{efs_root_dir}}/bin/; \
    done
    cd {{efs_tool_dir}} && just run ../{{efs_root_dir}}/ ../{{efs_tool_dir}}/target/{{mode}}/

# Build the kernel
build-kernel:
    cd {{kernel_dir}} && just build {{board}}

# Build the test cases
build-tests:
    cd {{tests_dir}} && just build

# Build App, EFS and Kernel
build: build-apps build-efs build-kernel

# Run the kernel in QEMU
run gpu="off": build
	qemu-system-riscv64 {{qemu_args}} -display {{ if gpu == "on" { "sdl" } else { "none" } }}

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
    cd {{efs_tool_dir}} && just clean
    rm {{efs_root_dir}}/* -rf
    cd {{kernel_dir}} && just clean
    cd {{tests_dir}} && just clean
    cd {{user_dir}} && just clean

# Checks packages to catch common mistakes and improve code.
clippy:
    cd {{apps_dir}} && just clippy 
    cd {{efs_dir}} && just clippy
    cd {{efs_tool_dir}} && just clippy
    cd {{kernel_dir}} && just clippy
    cd {{tests_dir}} && just clippy
    cd {{user_dir}} && just clippy
