//! Semaphore

use crate::sync::deadlock::DetectInfo;
use crate::sync::{DeadLockDetect, UPSafeCell};
use crate::task::{
    block_current_and_run_next, current_process, current_task, wakeup_task, TaskControlBlock,
};
use alloc::collections::btree_map::BTreeMap;
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub allocation: BTreeMap<usize, usize>,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    allocation: BTreeMap::new(),
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        // tid
        let binding = current_task().unwrap();
        let task_inner = binding.inner_exclusive_access();
        let tid = task_inner.res.as_ref().unwrap().tid;
        drop(task_inner);

        let mut inner = self.inner.exclusive_access();
        inner.count += 1;

        inner
            .allocation
            .entry(tid)
            .and_modify(|elem| {
                if *elem > 0 {
                    *elem -= 1
                }
            })
            .or_insert(0);

        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                drop(inner); //
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) -> isize {
        trace!("kernel: Semaphore::down");
        // tid
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
            inner.count -= 1;
            inner.wait_queue.push_back(current_task().unwrap());
            // inner
            //     .allocation
            //     .entry(tid)
            //     .and_modify(|elem| *elem += 1)
            //     .or_insert(1);
            drop(inner);

            // detect
            let is_dead_lock = process_inner.detect();

            // clean up
            let mut inner = self.inner.exclusive_access();
            inner.count += 1;
            inner.wait_queue.pop_back();
            // inner.allocation.entry(tid).and_modify(|elem| *elem -= 1);
            drop(inner);

            if is_dead_lock {
                return -0xDEAD;
            }
        }

        drop(process_inner);

        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());

            inner
                .allocation
                .entry(tid)
                .and_modify(|elem| *elem += 1)
                .or_insert(1);

            drop(inner);
            block_current_and_run_next();
        }

        0
    }
}

impl DetectInfo for Semaphore {
    /// get wait queue (tids) as need
    fn get_need(&self) -> BTreeMap<usize, usize> {
        let mut need = BTreeMap::new();
        let inner = self.inner.exclusive_access();

        inner.wait_queue.iter().for_each(|tcb| {
            let inner = tcb.inner_exclusive_access();

            need.entry(inner.res.as_ref().unwrap().tid)
                .and_modify(|res| *res += 1)
                .or_insert(1);
        });

        need
    }
    /// get allocation
    fn get_allocation(&self) -> BTreeMap<usize, usize> {
        let inner = self.inner.exclusive_access();

        inner.allocation.clone()
    }
}
