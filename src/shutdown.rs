//! Close only owned sessions without blocking the native window thread.
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, TryRecvError},
    },
};

const MAX_CLOSERS: usize = 4;

pub struct ShutdownTask {
    result: Receiver<Result<(), String>>,
    done: Arc<AtomicUsize>,
    total: usize,
}

impl ShutdownTask {
    pub fn start<T, F>(
        items: Vec<T>,
        save: F,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<(), String> + Send + 'static,
    {
        let total = items.len();
        let done = Arc::new(AtomicUsize::new(0));
        let completed = done.clone();
        let (result_tx, result) = mpsc::channel();
        // Wait for the payload on a bounded channel so starting the coordinator
        // never waits for a terminal or its child process.
        let (work_tx, work_rx) = mpsc::sync_channel::<(Vec<T>, F)>(1);
        std::thread::Builder::new()
            .name("session-shutdown".into())
            .spawn(move || {
                let Ok((items, save)) = work_rx.recv() else {
                    return;
                };
                let queue = Arc::new(Mutex::new(VecDeque::from(items)));
                let mut workers = Vec::new();
                for index in 0..total.min(MAX_CLOSERS - 1) {
                    let queue = queue.clone();
                    let done = completed.clone();
                    let wake = wake.clone();
                    if let Ok(worker) = std::thread::Builder::new()
                        .name(format!("session-close-{index}"))
                        .spawn(move || close_items(queue, done, wake))
                    {
                        workers.push(worker);
                    }
                }
                // Saving and closing are independent; neither runs on the UI thread.
                let mut saved = std::panic::catch_unwind(std::panic::AssertUnwindSafe(save))
                    .unwrap_or_else(|_| Err("Session settings could not be saved".into()));
                // If thread creation failed, the coordinator finishes remaining jobs.
                close_items(queue, completed, wake.clone());
                for worker in workers {
                    if worker.join().is_err() && saved.is_ok() {
                        saved = Err("A session could not be closed cleanly".into());
                    }
                }
                let _ = result_tx.send(saved);
                wake();
            })?;
        work_tx
            .send((items, save))
            .map_err(|_| std::io::Error::other("shutdown worker unavailable"))?;
        Ok(Self {
            result,
            done,
            total,
        })
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.done.load(Ordering::Relaxed), self.total)
    }

    pub fn poll(&self) -> Option<Result<(), String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(Err("Shutdown worker stopped unexpectedly".into()))
            }
        }
    }
}

fn close_items<T>(
    queue: Arc<Mutex<VecDeque<T>>>,
    done: Arc<AtomicUsize>,
    wake: Arc<dyn Fn() + Send + Sync>,
) {
    loop {
        let item = queue
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .pop_front();
        let Some(item) = item else { return };
        drop(item);
        done.fetch_add(1, Ordering::Relaxed);
        wake();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    struct BlockingResource {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }
    impl Drop for BlockingResource {
        fn drop(&mut self) {
            let _ = self.entered.send(());
            let _ = self.release.recv_timeout(Duration::from_secs(5));
        }
    }

    #[test]
    fn blocked_cleanup_does_not_block_the_caller_or_report_completion_early() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let task = ShutdownTask::start(
            vec![BlockingResource {
                entered: entered_tx,
                release: release_rx,
            }],
            || Ok(()),
            Arc::new(|| {}),
        )
        .unwrap();
        entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(task.progress(), (0, 1));
        assert!(task.poll().is_none());
        release_tx.send(()).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(result) = task.poll() {
                assert!(result.is_ok());
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(task.progress(), (1, 1));
    }

    #[test]
    fn saving_errors_are_reported_after_all_owned_resources_are_closed() {
        let task = ShutdownTask::start(
            vec![(), (), ()],
            || Err("save failed".into()),
            Arc::new(|| {}),
        )
        .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(result) = task.poll() {
                assert_eq!(result.unwrap_err(), "save failed");
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(task.progress(), (3, 3));
    }
}
