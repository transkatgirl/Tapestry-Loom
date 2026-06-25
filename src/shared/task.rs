use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};

use futures::FutureExt;
use tokio::task::{self, JoinError, JoinHandle};

pub const BACKGROUND_REFRESH_WAIT: Duration = Duration::from_millis(500);

pub fn spawn_blocking_abortable<F, T>(f: F) -> AbortableBlockingTaskHandle<T>
where
    F: FnOnce(Arc<AtomicBool>) -> T + Send + 'static,
    T: Send + 'static,
{
    AbortableBlockingTaskHandle::new(f)
}

#[derive(Debug)]
pub struct AbortableBlockingTaskHandle<T>
where
    T: Send + 'static,
{
    handle: JoinHandle<T>,
    abort: Arc<AtomicBool>,
}

impl<T> AbortableBlockingTaskHandle<T>
where
    T: Send + 'static,
{
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce(Arc<AtomicBool>) -> T + Send + 'static,
    {
        let abort = Arc::new(AtomicBool::new(false));
        let fn_abort = abort.clone();

        Self {
            handle: task::spawn_blocking(move || f(fn_abort)),
            abort,
        }
    }
    pub fn abort(&self) {
        self.handle.abort();
        self.abort.store(true, Ordering::SeqCst);
    }
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}

impl<T> Drop for AbortableBlockingTaskHandle<T>
where
    T: Send + 'static,
{
    fn drop(&mut self) {
        self.handle.abort();
        self.abort.store(true, Ordering::Relaxed);
    }
}

impl<T> Future for AbortableBlockingTaskHandle<T>
where
    T: Send + 'static,
{
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.handle.poll_unpin(cx)
    }
}
