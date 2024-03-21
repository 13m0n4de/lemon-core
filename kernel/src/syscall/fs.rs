//! File System System Calls

use core::ptr::slice_from_raw_parts;

use easy_fs::DIRENT_SIZE;

use crate::fs::{find_inode, get_full_path, make_pipe, open_file, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_mut_ref, translated_str, UserBuffer};
use crate::task::{current_process, current_user_token};

/// Retrieves the current working directory of the calling process.
///
/// This function copies the current working directory into a user-provided buffer, up to the specified `len`.
///
/// # Arguments
///
/// - `buf`: A pointer to the buffer where the current working directory should be copied.
/// - `len`: The maximum number of bytes to copy into the buffer.
///
/// # Returns
///
/// - The length of the directory path if successful.
/// - `-1` if the buffer is too small.
pub fn sys_getcwd(buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    let mut user_buffer = UserBuffer::new(translated_byte_buffer(token, buf, len));
    let cwd = process_inner.cwd.as_bytes();

    if cwd.len() > len {
        return -1;
    }

    user_buffer
        .iter_mut()
        .zip(cwd)
        .for_each(|(p, &c)| unsafe { *p = c });

    cwd.len() as isize
}

/// Duplicates an open file descriptor.
///
/// Returns a new file descriptor that refers to the same file as the original file descriptor `fd`.
///
/// # Arguments
///
/// - `fd`: The file descriptor to duplicate.
///
/// # Returns
///
/// - The new file descriptor if successful.
/// - `-1` if the original file descriptor is invalid.
pub fn sys_dup(fd: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    match process_inner.fd_table.get(fd) {
        Some(Some(file)) => {
            let new_file = file.clone();
            let new_fd = process_inner.alloc_fd();
            process_inner.fd_table[new_fd] = Some(new_file);
            new_fd as isize
        }
        _ => -1,
    }
}

/// Duplicates an open file descriptor to a specified file descriptor number.
///
/// If the target file descriptor `new_fd` is already open, it is silently closed before being reused.
///
/// # Arguments
///
/// - `old_fd`: The original file descriptor to duplicate.
/// - `new_fd`: The file descriptor number to duplicate to.
///
/// # Returns
///
/// - `0` if successful.
/// - `-1` if either `old_fd` or `new_fd` is invalid.
pub fn sys_dup2(old_fd: usize, new_fd: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let fd_table = &mut process_inner.fd_table;
    if old_fd >= fd_table.len() {
        return -1;
    }
    if new_fd >= fd_table.len() {
        fd_table.resize(new_fd + 1, None);
    }
    fd_table[new_fd] = fd_table[old_fd].clone();

    0
}

/// Changes the current working directory of the calling process.
///
/// # Arguments
///
/// - `path`: A pointer to the null-terminated string representing the path to the new directory.
///
/// # Returns
///
/// - `0` if successful.
/// - `-1` if no such file.
/// - `-2` if is not a directory.
pub fn sys_chdir(path: *const u8) -> isize {
    let token = current_user_token();
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    let path = translated_str(token, path);
    let path = get_full_path(&process_inner.cwd, &path);

    drop(process_inner);

    if let Some(inode) = find_inode(&path) {
        if inode.is_dir() {
            let mut process_inner = process.inner_exclusive_access();
            process_inner.cwd = path;
            0
        } else {
            -2 // not dir
        }
    } else {
        -1 // no such file
    }
}

/// Creates a new directory at the specified path.
///
/// # Arguments
///
/// - `path`: A pointer to the path where the directory will be created.
///
/// # Returns
///
/// - `0` on successful creation.
/// - `-1` if the parent directory does not exist or cannot be accessed.
/// - `-2` if the directory cannot be created (e.g., due to permissions or if the directory already exists).
pub fn sys_mkdir(path: *const u8) -> isize {
    let token = current_user_token();
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    let path = translated_str(token, path);
    let path = get_full_path(&process_inner.cwd, &path);

    drop(process_inner);

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

/// Deletes a file or directory specified by path, with behavior modified by flags.
///
/// # Arguments
///
/// - `path`: A pointer to the path of the file or directory to delete.
/// - `flags`: Modification flags (e.g., `AT_REMOVEDIR` to specify directory removal).
///
/// # Returns
///
/// - `0` on successful deletion,
/// - `-1` if the path does not exist.
/// - `-2` if the type does not match (e.g., trying to delete a directory without `AT_REMOVEDIR`).
/// - `-3` if the directory is not empty.
pub fn sys_unlink(path: *const u8, flags: u32) -> isize {
    let token = current_user_token();
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    let path = translated_str(token, path);
    let path = get_full_path(&process_inner.cwd, &path);

    drop(process_inner);

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

/// Opens or creates a file or directory with specified flags.
///
/// # Arguments
///
/// - `path`: A pointer to the path of the file or directory.
/// - `flags`: Operation flags.
///
/// # Returns
///
/// - A file descriptor on success.
/// - `-1` on failure.
pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let token = current_user_token();
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    let path = translated_str(token, path);
    let path = get_full_path(&process_inner.cwd, &path);

    drop(process_inner);

    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut process_inner = process.inner_exclusive_access();
        let fd = process_inner.alloc_fd();
        process_inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

/// Closes an open file descriptor.
///
/// # Arguments
///
/// - `fd`: The file descriptor to close.
///
/// # Returns
///
/// - `0` on success.
/// - `-1` if the file descriptor is invalid.
pub fn sys_close(fd: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    if fd >= process_inner.fd_table.len() {
        return -1;
    }

    match process_inner.fd_table[fd].take() {
        Some(_) => 0,
        None => -1,
    }
}

/// Reads data from an open file descriptor into a buffer.
///
/// # Arguments
///
/// - `fd`: The file descriptor from which to read.
/// - `buf`: A pointer to the buffer where data will be stored.
/// - `len`: The maximum number of bytes to read.
///
/// # Returns
///
/// - The number of bytes read on success.
/// - `-1` on failure or if the file descriptor is invalid.
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    if fd >= process_inner.fd_table.len() {
        return -1;
    }

    if let Some(file) = &process_inner.fd_table[fd] {
        let file = file.clone();
        if !file.is_readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(process_inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

/// Writes data to an open file descriptor from a buffer.
///
/// # Arguments
///
/// - `fd`: The file descriptor to write to.
/// - `buf`: A pointer to the buffer containing the data to write.
/// - `len`: The number of bytes to write.
///
/// # Returns
///
/// - The number of bytes written on success,
/// - `-1` on failure or if the file descriptor is invalid.
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    if fd >= process_inner.fd_table.len() {
        return -1;
    }

    if let Some(file) = &process_inner.fd_table[fd] {
        if !file.is_writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(process_inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

/// Retrieves file status information, writing it to a specified buffer.
///
/// # Arguments
///
/// - `fd`: The file descriptor of the file.
/// - `stat`: A pointer to a buffer where file status information will be written.
///
/// # Returns
///
/// - `0` on success.
/// - `-1` if the file descriptor is invalid.
pub fn sys_fstat(fd: usize, stat: *mut u8) -> isize {
    let token = current_user_token();
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    let mut user_buffer = UserBuffer::new(translated_byte_buffer(
        token,
        stat,
        core::mem::size_of::<Stat>(),
    ));

    let fd_table = &process_inner.fd_table;
    if fd >= fd_table.len() || fd_table[fd].is_none() {
        return -1;
    }
    let file = fd_table[fd].clone().unwrap();
    drop(process_inner);

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

/// Creates a pipe, a unidirectional data channel, and returns file descriptors for the read and write ends.
///
/// # Arguments
///
/// - `pipe`: A pointer where the file descriptors for the read and write ends of the pipe will be stored.
///
/// # Returns
///
/// - `0` on success.
pub fn sys_pipe(pipe: *mut usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let (pipe_read, pipe_write) = make_pipe();

    let read_fd = process_inner.alloc_fd();
    process_inner.fd_table[read_fd] = Some(pipe_read);

    let write_fd = process_inner.alloc_fd();
    process_inner.fd_table[write_fd] = Some(pipe_write);

    *translated_mut_ref(token, pipe) = read_fd;
    *translated_mut_ref(token, unsafe { pipe.add(1) }) = write_fd;

    0
}
