//! Process management syscalls

use crate::config::PAGE_SIZE;

use crate::mm::address::StepByOne;
use crate::mm::{translated_byte_ptr, MapPermission, PageTable, VirtAddr};
use crate::task::{
    change_program_brk, delete_framed_area, exit_current_and_run_next, insert_framed_area,
    suspend_current_and_run_next,
};
use crate::task::{current_user_token, query_current_syscall_times};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");

    // common compiler and user space mm allocator will always align the memory
    // of struct to 4K
    // so in Linux Os, usually we don't have to handle cross page issue

    let us = get_time_us();

    if let Some(buffer) = translated_byte_ptr(current_user_token(), ts as *const u8) {
        unsafe {
            *(buffer as *mut TimeVal) = TimeVal {
                sec: us / 1_000_000,
                usec: us % 1_000_000,
            };
        }
        0
    } else {
        -1
    }

    // println!("[Kernel] sbi : get_time_ms: {}", us);

    // println!("[Kernel] sys_get_time: finished");
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
const READ_DATA: usize = 0;
const WRITE_DATA: usize = 1;
const QUERY_SYSCALL_TIME: usize = 2;

pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");

    let va = VirtAddr::from(id);

    let vpn = va.floor();

    println!("[kernel] sys_trace: va {:?} vpn {:?}", va, vpn);

    let pt = PageTable::from_token(current_user_token());

    // panic if invisible
    // or maybe return -1 ? <-- this one
    let pte = pt.translate(vpn);

    match trace_request {
        READ_DATA => {
            let ptr = translated_byte_ptr(current_user_token(), id as *const u8);

            if pte.is_none() || ptr.is_none() {
                println!("[kernel] sys_trace: pass an invalid `id` as ptr");
                return -1;
            }

            let pte = pte.unwrap();

            if !pte.is_valid() || !pte.readable() || !pte.is_user() {
                println!("[kernel] sys_trace: `id` ptr permission denied ");
                -1
            } else {
                unsafe { *(ptr.unwrap() as *mut u8) as isize }
            }
        }
        WRITE_DATA => {
            let ptr = translated_byte_ptr(current_user_token(), id as *const u8);

            if pte.is_none() || ptr.is_none() {
                println!("[kernel] sys_trace pass an invalid `id` as ptr");
                return -1;
            }

            let pte = pte.unwrap();

            if !pte.is_valid() || !pte.writable() || !pte.is_user() {
                println!("[kernel] sys_trace: `id` ptr permission denied ");
                -1
            } else {
                unsafe { *(ptr.unwrap() as *mut u8) = data as u8 };
                0
            }
        }
        QUERY_SYSCALL_TIME => query_current_syscall_times(id) as isize,
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");

    if (port & !0x7) != 0 || (port & 0x7) == 0 {
        return -1;
    }
    let end: usize = start + len;
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(end);

    if start_va.page_offset() != 0 {
        return -1;
    }

    let mut start_vpn = start_va.floor();
    let pt = PageTable::from_token(current_user_token());
    for _ in 0..len.div_ceil(PAGE_SIZE) {
        match pt.translate(start_vpn) {
            Some(pte) => {
                if pte.is_valid() {
                    println!("[kernel] sys_mmap: page {:?} already alloc", start_vpn);
                    return -1;
                }
            }
            None => {}
        }
        start_vpn.step();
    }

    let mut permission = MapPermission::empty();
    if port & 0x1 != 0 {
        permission |= MapPermission::R;
    }
    if port & 0x2 != 0 {
        permission |= MapPermission::W;
    }
    if port & 0x4 != 0 {
        permission |= MapPermission::X;
    }
    permission |= MapPermission::U;

    // frame insert

    insert_framed_area(start_va, end_va, permission);

    println!("[kernel] mmap on va {}", start);

    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");

    let end = start + len;
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(end);
    if start_va.page_offset() != 0 {
        return -1;
    }

    let mut start_vpn = start_va.floor();
    let pt = PageTable::from_token(current_user_token());
    for _ in 0..len.div_ceil(PAGE_SIZE) {
        match pt.translate(start_vpn) {
            Some(pte) => {
                if !pte.is_valid() {
                    return -1;
                }
            }
            None => {
                return -1;
            }
        }
        start_vpn.step();
    }

    delete_framed_area(start_va, end_va);

    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
