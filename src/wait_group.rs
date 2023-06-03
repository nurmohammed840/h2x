use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

/// A WaitGroup is a synchronization primitive that will resolve when all of the tasks in the group have finished.
#[derive(Debug)]
pub struct WaitGroup<T>(pub Arc<T>);

impl<T> WaitGroup<T> {
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
