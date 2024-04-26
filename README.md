# LemonCore

A Simple RISC-V OS Kernel, Inspired by [rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3).

🚧 Work In Progress

## Features & TODOs

- [x] Architecture: RISC-V 64
- [x] Platform: QEMU, K210
- [x] Colorful logging
- [x] FIFO / RR scheduler
- [x] SV39 3-level page table
- [x] Easy File System
- [x] Multi-thread
- [x] Synchronization
- [x] VirtIO blk/input/gpu drivers
- [x] Test framework (unit test and integration test)
- [ ] MLFQ/CFS scheduler
- [ ] VirtIO net drivers
- [ ] Triple indirect pointer
- [ ] K210 platform support for `ch1` - `ch7`
- [ ] A detailed documentation or step-by-step tutorial

## Difference with rCore-Tutorial-v3

- Stricter code style enforced with `#[deny(clippy::all, clippy::pedantic)]`.
- Replaces `Makefile` and `build.rs` with [Just](https://github.com/casey/just/) for build automation.
- Implementation of multi-level directories, file deletion, and file metadata.
- Test framework (unit test and integration test)
- Data structure design enhancements, such as:
    - `MapArea`'s `data_frames` are designed to be placed directly in the `PageTable`.
    - `BlockCacheManager` uses `Vec` instead of `VecDeque`.
    - ...
- Module naming and organization changes, such as:
    - `os` module has been renamed to [kernel](./kernel/).
    - `TaskControlBlock` is located in `tcb.rs` instead of `task.rs` to avoid using `#[allow(clippy::module_inception)]`.
    - `ProcessControlBlock` is placed in `pcb.rs` rather than `process.rs`.
    - User library modules are named consistently with kernel modules.
    - User programs and test cases are separated from `user_lib`.
    - ...
- No need to manually append `\0` to strings.
- Adoption of newer crates and RustSBI versions.
- Use of [clap](https://docs.rs/clap/latest/clap/) for command-line argument parsing in [easy-fs-fuse](./easy-fs-fuse/).
- More comprehensive shell application featuring:
    - Command input and output redirection.
    - Prompt displaying the current path.
    - Entering the directory name allows for direct navigation to the directory.
    - ...
- [More command-line applications](./apps/src/bin/).
- ...

## Project Structure

```
.
├── apps                # user applications
├── bootloader          # RustSBI
├── easy-fs-fuse        # command-line tool to create EFS image
├── easy-fs-root        # Root directory structure for the EFS image
├── easy-fs             # easy file system
├── kernel              # os kernel
│   ├── assets          # static data, images, fonts, or other binary assets
│   └── src             # source code of the kernel
│       ├── boards      # board support Packages (BSPs) for different hardware platforms
│       ├── drivers     # device drivers
│       ├── fs          # file system management
│       ├── mm          # memory management
│       ├── sync        # synchronization primitives
│       ├── syscall     # system calls
│       ├── task        # task management
│       └── trap        # trap handling
├── tests               # integration tests 
├── user                # user library
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

### Run on QEMU

```
just run
```

If you need to enable GPU support for the project, you can run:

```
just run on
```

### Run on K210

For the latest updates and adaptations for the K210 platform, switch to the [feature/k210](https://github.com/13m0n4de/lemon-core/tree/feature/k210), which is based on the updates from the [ch8](https://github.com/13m0n4de/lemon-core/tree/feature/ch8) branch.

```
git switch feature/k210
just run
```

## Test

### Unit test

```
just unit-tests
```

### Integration Test

```
just integration-tests
```

## References

- [github.com/rcore-os/rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3)
- [github.com/rcore-os/rCore-Tutorial-Book-v3](https://github.com/rcore-os/rCore-Tutorial-Book-v3)
- [Operating Systems: Three Easy Pieces](http://pages.cs.wisc.edu/~remzi/OSTEP/)
- [github.com/TD-Sky/rCore](https://github.com/TD-Sky/rCore)
- [github.com/CelestialMelody/fs-rs](https://github.com/CelestialMelody/fs-rs)
- [github.com/Direktor799/rusted_os](https://github.com/Direktor799/rusted_os)
- [Just's user manual](https://just.systems/man/zh/)
- [Linux kernel system calls for all architectures](https://gpages.juszkiewicz.com.pl/syscalls-table/syscalls.html)
