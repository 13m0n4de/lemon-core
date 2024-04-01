# LemonCore

A Simple RISC-V OS Kernel, Reference From [rCore-Tutorial-Book-v3](https://github.com/rcore-os/rCore-Tutorial-v3).

🚧 Working In Progress

## Features && TODOs

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
- [ ] Test framework #2

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
- Use of [Clap](https://docs.rs/clap/latest/clap/) for command-line argument parsing in [easy-fs-tool](./easy-fs-tool/).
- More comprehensive shell application featuring:
    - Command input and output redirection.
    - Prompt displaying the current path.
    - Entering the directory name allows for direct navigation to the directory.
    - ...
- [More command-line applications](./apps/src/bin/).
