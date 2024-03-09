# Log level
export LOG := "DEBUG"

# Target architecture
target := "riscv64gc-unknown-none-elf"

# Directories for kernel
kernel_dir := "os"

# List available recipes
default:
    just --list

# Environment setup
env:
    rustup target add {{target}}
    cargo install cargo-binutils
    rustup component add rust-src
    rustup component add llvm-tools-preview

# Build the kernel
build-kernel:
    cd {{kernel_dir}} && just build

# Build the kernel
build: build-kernel

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
    cd {{kernel_dir}} && just clean
