use crossbeam_channel::{unbounded, Receiver, Sender};
use log::info;
use std::{
    mem::ManuallyDrop,
    num::NonZeroUsize,
    thread::{self, JoinHandle},
};

type Task = Box<dyn FnOnce() + Send>;

pub struct ThreadPool {
    sender: ManuallyDrop<Sender<Task>>,
    workers: Vec<Worker>,
}

impl ThreadPool {
    /// Create a thread pool with the given thread number
    pub fn new(thread_num: NonZeroUsize) -> Self {
        let (sender, receiver) = unbounded();

        let workers = (0..thread_num.get())
            .map(|id| Worker::spawn(id, receiver.clone()))
            .collect();

        Self {
            sender: ManuallyDrop::new(sender),
            workers,
        }
    }

    /// Execute a task
    pub fn execute<Task>(&self, task: Task)
    where
        Task: FnOnce() + Send + 'static,
    {
        self.sender.send(Box::new(task)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Drop sender first so that workers can terminate
        // SAFETY: sender will not be used anymore
        unsafe { ManuallyDrop::drop(&mut self.sender) };

        // Wait for all workers to finish
        for worker in self.workers.drain(..) {
            worker.join();
        }
    }
}

struct Worker {
    thread: JoinHandle<()>,
}

impl Worker {
    fn spawn(id: usize, receiver: Receiver<Task>) -> Self {
        Self {
            thread: thread::spawn(move || Self::task_loop(id, receiver)),
        }
    }

    fn join(self) {
        self.thread.join().unwrap();
    }

    fn task_loop(id: usize, receiver: Receiver<Task>) {
        while let Ok(task) = receiver.recv() {
            info!("Worker[{id}] received a task!");
            task();
        }

        info!("Worker[{id}] terminated!");
    }
}
