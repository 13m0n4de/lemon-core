//! File and filesystem-related syscalls

use crate::fs::{make_pipe, open_file, OpenFlags};
use crate::mm::{translated_byte_buffer, translated_mut_ref, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    match inner.fd_table.get(fd) {
        Some(Some(file)) => {
            let new_file = file.clone();
            let new_fd = inner.alloc_fd();
            inner.fd_table[new_fd] = Some(new_file);
            new_fd as isize
        }
        _ => -1,
    }
}

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.inner_exclusive_access();

    if fd >= inner.fd_table.len() {
        return -1;
    }

    if let Some(file) = &inner.fd_table[fd] {
        if !file.is_writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.inner_exclusive_access();

    if fd >= inner.fd_table.len() {
        return -1;
    }

    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.is_readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);

    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    if fd >= inner.fd_table.len() {
        return -1;
    }

    match inner.fd_table[fd].take() {
        Some(_) => 0,
        None => -1,
    }
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let mut inner = task.inner_exclusive_access();
    let (pipe_read, pipe_write) = make_pipe();
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write);
    *translated_mut_ref(token, pipe) = read_fd;
    *translated_mut_ref(token, unsafe { pipe.add(1) }) = write_fd;
    0
}
