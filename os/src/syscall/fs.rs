//! File and filesystem-related syscalls

use alloc::sync::Arc;

use crate::fs::{
    get_inode_number, get_nlink, link_file, open_file, unlink_file, OSInode, OpenFlags, Stat,
    StatMode,
};
use crate::mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
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
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
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
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    trace!("kernel:pid[{}] sys_fstat", current_task().unwrap().pid.0);

    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.inner_exclusive_access();

    if fd >= inner.fd_table.len() {
        return -1;
    }
    if fd == 0 || fd == 1 || fd == 2 {
        return -1; // stdin / stdout / stderr
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }

    // Attempt to downcast the Arc<dyn File> to Arc<OSInode>

    let arc_file = Arc::clone(&inner.fd_table[fd].as_ref().unwrap());
    let os_inode = unsafe { Arc::from_raw(Arc::into_raw(arc_file) as *const OSInode) };

    let st_mm_ref = translated_refmut(token, st);

    // dev always 0, StatMode always FILE ?
    *st_mm_ref = Stat::new(
        0,
        get_inode_number(&*os_inode).into(),
        StatMode::FILE,
        get_nlink(&*os_inode),
    );

    0
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    // only have one dir in the fs, so have to parse path strings
    let token = current_user_token();

    let old_name = translated_str(token, old_name);
    let new_name = translated_str(token, new_name);

    if old_name == new_name {
        return -1;
    }

    link_file(old_name.as_str(), new_name.as_str());
    0
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    let token = current_user_token();
    let name = translated_str(token, name);

    if unlink_file(name.as_str()) {
        0
    } else {
        -1
    }
}
