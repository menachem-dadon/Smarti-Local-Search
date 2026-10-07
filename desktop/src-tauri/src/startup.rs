use std::sync::{Condvar, Mutex};

/// Registered before webviews load, so early IPC waits for setup to finish.
pub struct Startup<T> {
    result: Mutex<Option<Result<T, String>>>,
    ready: Condvar,
}

impl<T> Default for Startup<T> {
    fn default() -> Self {
        Self {
            result: Mutex::new(None),
            ready: Condvar::new(),
        }
    }
}

impl<T: Clone> Startup<T> {
    pub fn complete(&self, result: Result<T, String>) {
        *self.result.lock().unwrap() = Some(result);
        self.ready.notify_all();
    }

    /// Only call on a blocking worker; never block the native UI thread.
    pub fn wait(&self) -> Result<T, String> {
        let result = self
            .ready
            .wait_while(self.result.lock().unwrap(), |result| result.is_none())
            .unwrap();
        result.as_ref().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::Startup;
    use std::sync::{Arc, mpsc};

    #[test]
    fn early_requests_wait_and_all_receive_the_initialized_service() {
        let startup = Arc::new(Startup::default());
        let (send, receive) = mpsc::channel();
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let startup = startup.clone();
                let send = send.clone();
                std::thread::spawn(move || send.send(startup.wait()).unwrap())
            })
            .collect();
        assert_eq!(receive.try_recv(), Err(mpsc::TryRecvError::Empty));
        startup.complete(Ok(42));
        for _ in &threads {
            assert_eq!(receive.recv().unwrap(), Ok(42));
        }
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(startup.wait(), Ok(42));
    }

    #[test]
    fn startup_failure_is_returned_to_every_request() {
        let startup = Startup::<u64>::default();
        startup.complete(Err("Index could not be opened".into()));
        assert_eq!(startup.wait(), Err("Index could not be opened".into()));
    }
}
