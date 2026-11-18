// SPDX-License-Identifier: GPL-3.0-or-later
//! A job on a thread of its own, for the window: started with a name and a [`Cancel`] it
//! can check and report progress on, polled each frame, its result taken once. A panic in
//! the job is caught and comes back as [`Stopped::Panicked`], a thread that ends without a
//! word as [`Stopped::Lost`]: the window says so and goes on. Used by generation,
//! simulation, nesting, *Export all setups…*, reading models and tracing pictures.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use chipload_pipeline::Cancel;

/// Why a job gave no result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Stopped {
    /// It panicked, with the panic's text.
    Panicked(String),
    /// Its thread ended without sending anything.
    Lost,
}

impl Stopped {
    /// The panic's text, or `otherwise` when there is none.
    pub(super) fn text_or(self, otherwise: String) -> String {
        match self {
            Stopped::Panicked(text) => text,
            Stopped::Lost => otherwise,
        }
    }
}

/// A job running on its own thread, giving a `T`.
pub(super) struct Worker<T> {
    cancel: Arc<Cancel>,
    receiver: Receiver<Result<T, String>>,
}

impl<T: Send + 'static> Worker<T> {
    /// Run `job` on a thread named `name`. The error is the system's, when no thread could
    /// be had.
    pub(super) fn start(
        name: &str,
        job: impl FnOnce(&Cancel) -> T + Send + 'static,
    ) -> std::io::Result<Worker<T>> {
        let (sender, receiver) = channel();
        let cancel = Arc::new(Cancel::new());
        let flag = Arc::clone(&cancel);
        std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| job(&flag)))
                    .map_err(|payload| panic_text(payload.as_ref()));
                // The window may have moved on and dropped the receiver: nothing to tell.
                let _ = sender.send(outcome);
            })?;
        Ok(Worker { cancel, receiver })
    }
}

impl<T> Worker<T> {
    /// The result once it has come; None while the job runs.
    pub(super) fn poll(&self) -> Option<Result<T, Stopped>> {
        match self.receiver.try_recv() {
            Ok(outcome) => Some(outcome.map_err(Stopped::Panicked)),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(Stopped::Lost)),
        }
    }

    /// Wait for the result (tests, screenshots).
    #[cfg(test)]
    pub(super) fn wait(&self) -> Result<T, Stopped> {
        match self.receiver.recv() {
            Ok(outcome) => outcome.map_err(Stopped::Panicked),
            Err(_) => Err(Stopped::Lost),
        }
    }

    /// Ask the job to stop at its next check.
    pub(super) fn cancel(&self) {
        self.cancel.cancel();
    }

    /// The share of the job done, 0 to 1, as it reports it.
    pub(super) fn progress(&self) -> f64 {
        self.cancel.progress()
    }
}

/// A panic's message, as far as it can be read.
pub(super) fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "unknown error".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_gives_its_result_and_a_panic_its_text() {
        let worker = Worker::start("test", |_| 6 * 7).unwrap();
        assert_eq!(worker.wait(), Ok(42));
        let worker: Worker<u32> = Worker::start("test", |_| panic!("broken")).unwrap();
        assert_eq!(worker.wait(), Err(Stopped::Panicked("broken".into())));
    }

    #[test]
    fn a_job_sees_its_cancel_and_reports_progress() {
        let worker = Worker::start("test", |cancel: &Cancel| {
            cancel.stage_done(0.5);
            while !cancel.is_cancelled() {
                std::thread::yield_now();
            }
            "stopped"
        })
        .unwrap();
        while worker.progress() < 0.5 {
            std::thread::yield_now();
        }
        worker.cancel();
        assert_eq!(worker.wait(), Ok("stopped"));
    }
}
