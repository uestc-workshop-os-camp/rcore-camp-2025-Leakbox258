//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::sync::deadlock::DetectInfo;
use crate::sync::DeadLockDetect;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_process, TaskControlBlock};
use crate::task::{current_task, wakeup_task};
use alloc::collections::btree_map::BTreeMap;
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send + DetectInfo {
    /// Lock the mutex
    fn lock(&self) -> isize;
    /// Unlock the mutex
    fn unlock(&self);
    /// check state (avaliable)
    fn is_lock(&self) -> bool;
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    inner: UPSafeCell<MutexSpinInner>,
}

pub struct MutexSpinInner {
    locked: bool,
    allocation: BTreeMap<usize, usize>,
    wait_queue: BTreeMap<usize, usize>,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        unsafe {
            Self {
                inner: UPSafeCell::new(MutexSpinInner {
                    locked: false,
                    allocation: BTreeMap::new(),
                    wait_queue: BTreeMap::new(),
                }),
            }
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) -> isize {
        trace!("kernel: MutexSpin::lock");

        // get tid
        let binding = current_task().unwrap();
        let task_inner = binding.inner_exclusive_access();
        let tid = task_inner.res.as_ref().unwrap().tid;
        drop(task_inner);

        // apply deadlock check
        let process = current_process();

        let process_inner = process.inner_exclusive_access();

        if process_inner.dead_lock_detect_enable {
            // try lock
            let mut inner = self.inner.exclusive_access();
            inner.wait_queue.insert(tid, 1);
            // inner.allocation.insert(tid, 1);
            drop(inner);

            // detect
            let is_dead_lock = process_inner.detect();

            // clean up
            let mut inner = self.inner.exclusive_access();
            inner.wait_queue.insert(tid, 0);
            // inner.allocation.insert(tid, 0);
            drop(inner);

            if is_dead_lock {
                return -0xDEAD;
            }
        }

        drop(process_inner);

        loop {
            let mut inner = self.inner.exclusive_access();
            let locked = inner.locked;

            if locked {
                inner.wait_queue.insert(tid, 1);

                drop(inner);

                suspend_current_and_run_next();
                continue;
            } else {
                inner.locked = true;

                inner.wait_queue.insert(tid, 0);
                inner.allocation.insert(tid, 1);
                return 0;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut inner = self.inner.exclusive_access();

        let binding = current_task().unwrap();
        let task_inner = binding.inner_exclusive_access();
        let tid = task_inner.res.as_ref().unwrap().tid;

        drop(task_inner);

        inner.allocation.insert(tid, 0);

        inner.locked = false;
    }

    fn is_lock(&self) -> bool {
        let inner = self.inner.exclusive_access();

        inner.locked
    }
}
impl DetectInfo for MutexSpin {
    fn get_allocation(&self) -> BTreeMap<usize, usize> {
        let inner = self.inner.exclusive_access();
        inner.allocation.clone()
    }

    fn get_need(&self) -> BTreeMap<usize, usize> {
        let mut need = BTreeMap::new();
        let inner = self.inner.exclusive_access();

        let wait_queue = inner.wait_queue.clone();

        drop(inner);

        wait_queue.iter().for_each(|(tid, need_res)| {
            // (inner.res.as_ref().unwrap().tid, 1)

            need.entry(*tid)
                .and_modify(|res| *res += need_res)
                .or_insert(1);
        });

        need
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    allocation: BTreeMap<usize, usize>,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    allocation: BTreeMap::new(),
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) -> isize {
        trace!("kernel: MutexBlocking::lock");
        // tid
        let binding = current_task().unwrap();
        let task_inner = binding.inner_exclusive_access();
        let tid = task_inner.res.as_ref().unwrap().tid;
        drop(task_inner);
        drop(binding);

        // apply deadlock check
        let process = current_process();

        let process_inner = process.inner_exclusive_access();

        if process_inner.dead_lock_detect_enable {
            // try lock
            let mut inner = self.inner.exclusive_access();
            inner.wait_queue.push_back(current_task().unwrap());
            // inner.allocation.insert(tid, 0);
            drop(inner);

            // detect
            let is_dead_lock = process_inner.detect();

            // clean up
            let mut inner = self.inner.exclusive_access();
            inner.wait_queue.pop_back();
            // inner.allocation.insert(tid, 0);
            drop(inner);

            if is_dead_lock {
                return -0xDEAD;
            }
        }

        drop(process_inner);
        drop(process);

        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;

            mutex_inner
                .allocation
                .entry(tid)
                .and_modify(|res| *res += 1)
                .or_insert(1);
        }
        0
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        // tid
        let binding = current_task().unwrap();
        let task_inner = binding.inner_exclusive_access();
        let tid = task_inner.res.as_ref().unwrap().tid;
        drop(task_inner);
        drop(binding);

        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);

        mutex_inner
            .allocation
            .entry(tid)
            .and_modify(|res| *res -= 1)
            .or_insert(0);

        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            let task_inner = waking_task.inner_exclusive_access();
            let tid = task_inner.res.as_ref().unwrap().tid;
            drop(task_inner);

            mutex_inner
                .allocation
                .entry(tid)
                .and_modify(|res| *res += 1)
                .or_insert(1);

            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false; //
        }
    }

    fn is_lock(&self) -> bool {
        let mutex_inner = self.inner.exclusive_access();

        mutex_inner.locked
    }
}
impl DetectInfo for MutexBlocking {
    fn get_allocation(&self) -> BTreeMap<usize, usize> {
        let inner = self.inner.exclusive_access();
        inner.allocation.clone()
    }

    fn get_need(&self) -> BTreeMap<usize, usize> {
        let mut need = BTreeMap::new();
        let inner = self.inner.exclusive_access();

        let wait_queue = inner.wait_queue.clone();

        drop(inner);

        wait_queue.iter().for_each(|tcb| {
            let inner = tcb.inner_exclusive_access();

            // (inner.res.as_ref().unwrap().tid, 1)

            need.entry(inner.res.as_ref().unwrap().tid)
                .and_modify(|res| *res += 1)
                .or_insert(1);
        });

        need
    }
}
