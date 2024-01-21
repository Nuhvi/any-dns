// use std::sync::mpsc::{Sender, Receiver, channel};

// use threadpool::ThreadPool;



// pub struct TaskQueue {
//     pool: ThreadPool,
//     tx: Sender<i32>,
//     rx: Receiver<i32>,
// }

// impl TaskQueue {
//     pub fn new(n_workers: usize) -> Self {
//         let (add_tx, add_rx) = channel();
//         TaskQueue {
//             pool:  ThreadPool::with_name("dnsWorker".into(), n_workers),
//             rx: add_rx,
//             tx: add_tx
//         }
//     }

//     pub fn add(&self, value: &str) {
//         let value = value.to_string();
//         let tx = self.tx.clone();
//         self.pool.execute(move|| {
//             println!("{} {}", std::thread::current().name().unwrap(), value);
//             tx.send(1).expect("channel will be there waiting for the pool");
//         });
//     }

//     pub fn get_pending_results(&self) -> Vec<i32> {
//         let mut results: Vec<i32> = vec![];
//         match self.rx.try_recv() {
//             Ok(val) => {
//                 results.push(val);
//             }
//             _ => {}
//         };
//         results
//     }


// }


// #[cfg(test)]
// mod tests {
//     use threadpool::ThreadPool;
//     use std::{sync::mpsc::channel, thread, time::Duration};

//     use crate::task_queue::TaskQueue;


//     #[test]
//     fn run_once() {
//         let queue = TaskQueue::new(5);
//         queue.add("yolo");
//         thread::sleep(Duration::from_secs(1));
//         let res = queue.get_pending_results();
//         println!("Res {:?}", res);
//     }

// }
