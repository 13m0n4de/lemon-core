# LemonCore

A Simple RISC-V OS Kernel, Reference From [rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3).

🚧 Working In Progress

## Features & TODOs

- [x] Architecture: RISC-V 64
- [x] Platform: QEMU
- [x] Colorful logging
- [x] FIFO scheduler
- [x] SV39 3-level page table
- [x] Easy File System
- [x] Multi-thread
- [x] Synchronization
- [x] VirtIO blk/input/gpu drivers
- [ ] RR/MLFQ/CFS scheduler
- [ ] VirtIO net drivers
- [ ] Test framework [#2](https://github.com/13m0n4de/lemon-core/issues/2)
- [ ] A detailed documentation or step-by-step tutorial

## Difference With rCore-Tutorial-v3

- Fully documented, with `#[deny(missing_docs)]`.
- Stricter code style enforced with `#[deny(clippy::all, clippy::pedantic)]`.
- Replaces `Makefile` and `build.rs` with [Just](https://github.com/casey/just/) for build automation.
- Implementation of multi-level directories, file deletion, and file metadata.
- Data structure design enhancements, such as:
    - `MapArea`'s `data_frames` are designed to be placed directly in the `PageTable`.
    - `BlockCacheManager` uses `Vec` instead of `VecDeque`.
    - ...
- Module naming and organization changes, such as:
    - `os` module has been renamed to [kernel](./kernel/).
    - `easy-fs-fuse` has been renamed to [easy-fs-tool](./easy-fs-tool/).
    - `TaskControlBlock` is located in `tcb.rs` instead of `task.rs` to avoid using `#[allow(clippy::module_inception)]`.
    - `ProcessControlBlock` is placed in `pcb.rs` rather than `process.rs`.
    - User library modules are named consistently with kernel modules.
    - User programs and test cases are separated from `user_lib`.
- No need to manually append `\0` to strings.
- Adoption of newer crates and RustSBI versions.
- Use of [clap](https://docs.rs/clap/latest/clap/) for command-line argument parsing in [easy-fs-tool](./easy-fs-tool/).
- More comprehensive shell application featuring:
    - Command input and output redirection.
    - Prompt displaying the current path.
    - Entering the directory name allows for direct navigation to the directory.
    - ...
- [More command-line applications](./apps/src/bin/).

## Project Structure

```
.
├── apps                # User Applications
├── bootloader          # RustSBI
├── easy-fs-tool        # Command-line tool to create EFS image
├── easy-fs             # Easy File System
├── kernel              # OS Kernel
│   ├── assets          # Static data, images, fonts, or other binary assets
│   └── src             # Source code of the kernel
│       ├── boards      # Board Support Packages (BSPs) for different hardware platforms
│       ├── drivers     # Device drivers
│       ├── fs          # File System management
│       ├── mm          # Memory Management
│       ├── sync        # Synchronization primitives
│       ├── syscall     # System Calls
│       ├── task        # Task Management
│       └── trap        # Trap handling
├── tests               # Test cases
├── user                # User library
└── ...
```

## Build & Run

### Install Rust

[Rust](https://www.rust-lang.org/tools/install) is a prerequisite for this project. Install it by following the official guide.
This will also install `cargo`, Rust's package manager, which is used for dependency management and project building.

### Install Just

[Just](https://github.com/casey/just) is a handy command runner that simplifies the execution of project-specific commands.
Install it by following the instructions on its GitHub page. This tool is used for setting up the environment, building, and
running the project with predefined commands.

### Setup Environment

Run the following command to set up the required environment for the project. This command adds the necessary Rust targets,
installs essential Rust tools, and sets up other required components.

```
just env
```

### Install QEMU

QEMU is a generic and open source machine emulator and virtualizer. Install QEMU to emulate the hardware environment for this project.
The installation instructions can vary depending on your operating system. Please refer to the QEMU Documentation for detailed installation instructions.

### Quick Run

```
just run
```

If you need to enable GPU support for the project, you can run:

```
just run on
```
