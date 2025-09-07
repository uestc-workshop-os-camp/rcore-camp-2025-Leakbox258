//! Types related to task management

use super::TaskContext;

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// sys_write times
    pub sys_write_times: usize,
    /// sys_exit times
    pub sys_exit_times: usize,
    /// sys_yield times
    pub sys_yield_times: usize,
    /// sys get_time times
    pub sys_get_time_times: usize,
    /// sys trace times
    pub sys_trace_times: usize,
}

impl TaskControlBlock {
    /// quick init of TCB
    pub fn new() -> TaskControlBlock {
        TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
            sys_write_times: 0,
            sys_exit_times: 0,
            sys_yield_times: 0,
            sys_get_time_times: 0,
            sys_trace_times: 0,
        }
    }
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
