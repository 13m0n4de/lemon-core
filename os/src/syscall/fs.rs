//! File and filesystem-related syscalls

use core::ptr::slice_from_raw_parts;

use easy_fs::DIRENT_SIZE;

use crate::fs::{find_inode, get_full_path, make_pipe, open_file, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_mut_ref, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_getcwd(buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();

    let mut user_buffer = UserBuffer::new(translated_byte_buffer(token, buf, len));
    let cwd = task_inner.cwd.as_bytes();

    if cwd.len() > len {
        return -1;
    }

    user_buffer
        .iter_mut()
        .zip(cwd)
        .for_each(|(p, &c)| unsafe { *p = c });

    cwd.len() as isize
}

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

pub fn sys_dup2(old_fd: usize, new_fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    let fd_table = &mut task_inner.fd_table;
    if old_fd >= fd_table.len() {
        return -1;
    }
    if new_fd >= fd_table.len() {
        fd_table.resize(new_fd + 1, None);
    }
    fd_table[new_fd] = fd_table[old_fd].clone();

    0
}

pub fn sys_chdir(path: *const u8) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    let path = translated_str(token, path);
    let path = get_full_path(&task_inner.cwd, &path);

    if let Some(inode) = find_inode(&path) {
        if inode.is_dir() {
            task_inner.cwd = path;
            0
        } else {
            -2 // not dir
        }
    } else {
        -1 // no such file
    }
}

pub fn sys_mkdir(path: *const u8) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();

    let path = translated_str(token, path);
    let path = get_full_path(&task_inner.cwd, &path);

    let (parent_path, target) = path
        .rsplit_once('/')
        .expect("Invalid path: the path must contain a '/'.");
    match find_inode(parent_path) {
        Some(parent_inode) => match parent_inode.create_dir(target) {
            Some(_cur_inode) => 0,
            None => -2,
        },
        None => -1,
    }
}

const AT_REMOVEDIR: u32 = 1;

pub fn sys_unlink(path: *const u8, flags: u32) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();

    let path = translated_str(token, path);
    let path = get_full_path(&task_inner.cwd, &path);

    let (parent_path, target) = path
        .rsplit_once('/')
        .expect("Invalid path: the path must contain a '/'.");
    match find_inode(parent_path) {
        Some(parent_inode) => match parent_inode.find(target) {
            Some(inode) => {
                let remove_dir = flags & AT_REMOVEDIR == AT_REMOVEDIR;
                if !remove_dir && !inode.is_dir() {
                    inode.clear();
                    parent_inode.delete(target);
                    return 0;
                }
                if remove_dir && inode.is_dir() {
                    if inode.file_size() as usize == DIRENT_SIZE * 2 {
                        inode.clear();
                        parent_inode.delete(target);
                        return 0;
                    } else {
                        return -3; // not empty
                    }
                }
                -2 // type not matched
            }
            None => -1,
        },
        None => -1,
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    let path = translated_str(token, path);
    let path = get_full_path(&task_inner.cwd, &path);

    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let fd = task_inner.alloc_fd();
        task_inner.fd_table[fd] = Some(inode);
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

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();

    if fd >= task_inner.fd_table.len() {
        return -1;
    }

    if let Some(file) = &task_inner.fd_table[fd] {
        let file = file.clone();
        if !file.is_readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(task_inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();

    if fd >= task_inner.fd_table.len() {
        return -1;
    }

    if let Some(file) = &task_inner.fd_table[fd] {
        if !file.is_writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(task_inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_fstat(fd: usize, stat: *mut u8) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();

    let mut user_buffer = UserBuffer::new(translated_byte_buffer(
        token,
        stat,
        core::mem::size_of::<Stat>(),
    ));

    let fd_table = &task_inner.fd_table;
    if fd >= fd_table.len() || fd_table[fd].is_none() {
        return -1;
    }
    let file = fd_table[fd].clone().unwrap();
    let stat = Stat::from(file);
    let stat_slice =
        slice_from_raw_parts(&stat as *const _ as *const u8, core::mem::size_of::<Stat>());

    for (i, p) in user_buffer.iter_mut().enumerate() {
        unsafe {
            *p = (*stat_slice)[i];
        }
    }
    0
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    let (pipe_read, pipe_write) = make_pipe();

    let read_fd = task_inner.alloc_fd();
    task_inner.fd_table[read_fd] = Some(pipe_read);

    let write_fd = task_inner.alloc_fd();
    task_inner.fd_table[write_fd] = Some(pipe_write);

    *translated_mut_ref(token, pipe) = read_fd;
    *translated_mut_ref(token, unsafe { pipe.add(1) }) = write_fd;

    0
}
