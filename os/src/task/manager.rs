//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;

/// for stride algorithm
const BIG_STRIDE: usize = 998244353;

///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: Vec<Arc<TaskControlBlock>>,
}

/// Impl Stride schedule algorithm
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: Vec::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(task);

        // reverse sort order
        self.ready_queue.sort_by(|task_a, task_b| {
            let inner_a = task_a.inner_exclusive_access();
            let stride_a = inner_a.stride;
            drop(inner_a);
            let inner_b = task_b.inner_exclusive_access();
            let stride_b = inner_b.stride;
            drop(inner_b);

            stride_b.cmp(&stride_a)
        });
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let task = self.ready_queue.pop();

        match task {
            Some(task) => {
                // update runtime stride info

                task.change_program_stride(BIG_STRIDE);
                Some(task)
            }

            None => None,
        }
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
