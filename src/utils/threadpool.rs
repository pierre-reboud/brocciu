use crate::game::BotGame;
use log::debug;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

type JobFunc = dyn Send + 'static + FnMut() -> String;

pub struct ThreadPool {
    workers: Vec<Worker>,
    task_sender: mpsc::Sender<Job>,
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
    task_receiver: Arc<Mutex<Receiver<Job>>>,
}

pub struct Job {
    function: Box<JobFunc>,
    results_sender: Arc<Mutex<Sender<String>>>,
}
// type Job = Box<dyn Send + 'static + FnMut() -> String>;

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (task_sender, task_receiver) = mpsc::channel();
        // let (results_sender, results_receiver) = mpsc::channel();

        let task_receiver = Arc::new(Mutex::new(task_receiver));
        // let results_sender = Arc::new(Mutex::new(results_sender));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, task_receiver.clone()));
        }
        let tp = ThreadPool {
            workers,
            task_sender,
        };
        debug!("Threadpool insantiated");
        tp
    }

    pub fn schedule_job<F>(&self, f: F) -> Receiver<String>
    where
        F: Send + 'static + FnMut() -> String,
    {
        let (results_sender, results_receiver) = mpsc::channel();
        let results_sender = Arc::new(Mutex::new(results_sender));

        let job = Job::new(Box::new(f), results_sender.clone());
        self.task_sender.send(job).unwrap();
        debug!("Job scheduled");
        results_receiver
    }
}

impl Worker {
    pub fn new(id: usize, task_receiver: Arc<Mutex<Receiver<Job>>>) -> Worker {
        let t_receiver = task_receiver.clone();
        let thread = thread::spawn(move || loop {
            let mut job = task_receiver.lock().unwrap().recv().unwrap();
            let result: String = (*job.function)();
            debug!("Worker {id} executed job; sending result ...");
            let _ = job.results_sender.lock().unwrap().send(result);
        });
        Worker {
            id,
            thread,
            task_receiver: t_receiver,
        }
    }
}

impl Job {
    fn new<F>(f: F, results_sender: Arc<Mutex<Sender<String>>>) -> Job
    where
        F: Send + 'static + FnMut() -> String,
    {
        let function = Box::new(f);
        Job {
            function,
            results_sender,
        }
    }
}
