# Debug level
export LOG := "DEBUG"

# Target architecture
target := "riscv64gc-unknown-none-elf"

# Directories for kernel, user applications
kernel_dir := "os"
user_dir := "user"

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

# Build the kernel
build-kernel:
    cd {{kernel_dir}} && just build

# Build App and Kernel
build: build-apps build-kernel

# Run the kernel in QEMU
run:
    cd {{kernel_dir}} && just run

# Debug the kernel in QEMU using tmux
debug:
    cd {{kernel_dir}} && just debug

# Start a GDB server for debugging
gdbserver:
    cd {{kernel_dir}} && just gdbserver

# Connect to the GDB server
gdbclient:
    cd {{kernel_dir}} && just gdbserver

# Clean build artifacts
clean:
    cd {{user_dir}} && just clean
    cd {{kernel_dir}} && just clean
