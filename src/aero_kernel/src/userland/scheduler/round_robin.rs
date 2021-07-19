/*
 * Copyright (C) 2021 The Aero Project Developers.
 *
 * This file is part of The Aero Project.
 *
 * Aero is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Aero is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Aero. If not, see <https://www.gnu.org/licenses/>.
 */

use alloc::sync::Arc;
use core::mem;

use intrusive_collections::LinkedList;

use crate::arch::interrupts::IPI_RESCHEDULE;
use crate::userland::task::{Task, TaskAdapter};
use crate::utils::{downcast, PerCpu};
use crate::{apic, arch};

use super::SchedulerInterface;

#[thread_local]
static mut CURRENT_PROCESS: Option<Arc<Task>> = None;

/// Scheduler queue containing a vector of all of the task of the enqueued
/// taskes.
struct TaskQueue {
    /// The kernel idle task is a special kind of task that is run when
    /// no taskes in the scheduler's queue are avaliable to execute. The idle task
    /// is to be created for each CPU.
    idle_process: Arc<Task>,
    runnable: LinkedList<TaskAdapter>,
}

impl TaskQueue {
    /// Creates a new task queue with no taskes by default.
    #[inline]
    fn new() -> Self {
        Self {
            idle_process: Task::new_idle(),
            runnable: LinkedList::new(TaskAdapter::new()),
        }
    }

    #[inline]
    fn push_runnable(&mut self, task: Arc<Task>) {
        debug_assert!(task.link.is_linked() == false); // Make sure the task is not already linked in the queue
        debug_assert!(task.task_id() != self.idle_process.task_id()); // Make sure we are not adding the IDLE task in the queue

        self.runnable.push_back(task);
    }
}

/// Round Robin is the simplest algorithm for a preemptive scheduler. When the
/// system timer fires, the next task in the queue is switched to, and the
/// preempted task is put back into the queue.
///
/// ## Notes
/// * <https://en.wikipedia.org/wiki/Round-robin_scheduling>
pub struct RoundRobin {
    /// The per-cpu scheduler queues.
    queue: PerCpu<TaskQueue>,
}

impl RoundRobin {
    /// Creates a new instance of the round robin scheduler and return a
    /// reference-counting pointer to itself. The task of this function
    /// is to initialize the per-cpu queues that the round robin scheduling
    /// algorithm requires.
    pub fn new() -> Arc<Self> {
        let this = Arc::new(Self {
            queue: PerCpu::new(|| TaskQueue::new()),
        });

        this
    }
}

impl SchedulerInterface for RoundRobin {
    fn register_task(&self, task: Arc<Task>) {
        let queue = self.queue.get_mut();

        queue.push_runnable(task);
    }

    fn init(&self) {
        let queue = self.queue.get();

        unsafe {
            CURRENT_PROCESS = Some(queue.idle_process.clone());
        }
    }

    fn current_task(&self) -> Arc<Task> {
        unsafe {
            CURRENT_PROCESS
                .as_ref()
                .expect("`current_task` was invoked before the current task was initialized")
                .clone()
        }
    }
}

unsafe impl Send for RoundRobin {}
unsafe impl Sync for RoundRobin {}

pub fn exit_current_task(_: usize) -> ! {
    loop {}
}

/// Yields execution to another task. The task of this function is to get the
/// task which is on the front of the task queue and jump to it. If no task are
/// avaviable for execution then the IDLE process task is executed.
///
/// ## Overview
/// Instead of adding `reschedule` as a method in the [SchedulerInterface] trait we are making
/// this a normal function as in the trait case, the scheduler will be locked for a longer time. The
/// scheduler only needs lock protection for reserving the task id allocated.
pub fn reschedule() {
    unsafe {
        apic::get_local_apic().send_ipi(1, IPI_RESCHEDULE);
    }

    let scheduler = super::get_scheduler();
    let round_robin: Option<Arc<RoundRobin>> = downcast(&scheduler.inner);
    let scheduler_ref = round_robin.expect("Failed to downcast the scheduler");

    let queue = scheduler_ref.queue.get_mut();

    mem::drop(scheduler); // Unlock the scheduler

    let previous_task = unsafe {
        CURRENT_PROCESS
            .as_ref()
            .expect("`reschedule` was invoked with no active previous task")
            .clone()
    };

    if let Some(new_task) = queue.runnable.pop_front() {
        // Swap Cr3 if neccessary.
        if previous_task.task_id() != new_task.task_id() {
            // Switch Cr3 and return to the thread.
            arch::task::arch_task_spinup(new_task.arch_task_mut(), true);
        } else {
            // Do not switch Cr3 and return to the thread.
            arch::task::arch_task_spinup(previous_task.arch_task_mut(), false);
        }
    }
}
