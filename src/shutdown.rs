use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
    task::{Context, Poll},
};

#[doc(hidden)]
/// A WaitGroup is a synchronization primitive that will resolve when all of the tasks in the group have finished.
#[derive(Debug)]
pub struct WaitGroup<T>(pub Arc<T>);

impl<T> WaitGroup<T> {
    #[doc(hidden)]
    pub fn new(v: T) -> (Arc<T>, Self) {
        let inner = Arc::new(v);
        let value = inner.clone();
        (value, Self(inner))
    }
}

impl<T> Future for WaitGroup<T> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // spin loop
        if Arc::strong_count(&self.0) == 1 {
            return Poll::Ready(());
        }
        std::thread::yield_now();
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

/// It used to signal a shutdown to a running server.
pub trait SignalShutdown: Sync + Send + 'static {
    /// Signals a shutdown.
    ///
    /// This method is called to signal to close the running server.
    /// Implementors of this trait should update their internal state to indicate that a shutdown has been requested.
    ///
    /// The [SignalShutdown::is_shutdown] method should return `true` if the server has been signaled to shutdown.
    fn shutdown(&self);

    /// This method is called to check if the server should perform a graceful shutdown.
    ///
    /// This method should return `true` if [SignalShutdown::shutdown] has been received.
    fn is_shutdown(&self) -> bool;
}

/// A struct that implements the [ShutDownSignal] trait.
///
/// It represents the state of a shutdown process.
#[derive(Debug, Default, Clone)]
pub struct ShutDownState(Arc<AtomicBool>);

impl SignalShutdown for ShutDownState {
    fn is_shutdown(&self) -> bool {
        self.0.load(atomic::Ordering::Acquire)
    }

    fn shutdown(&self) {
        self.0.store(true, atomic::Ordering::Relaxed)
    }
}

impl ShutDownState {
    /// Creates a new [ShutDownState] struct with value `false`.
    pub fn new() -> Self {
        Self::default()
    }
}
