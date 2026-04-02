use core::{future::Future, pin::Pin, task::{Context, Poll}};
use futures_util::task::AtomicWaker;
use crate::interrupts::ticks;

static TIMER_WAKER: AtomicWaker = AtomicWaker::new();

pub fn wake_timer() {
    TIMER_WAKER.wake();
}

pub struct TimerFuture {
    target_tick: u64,
}

impl TimerFuture {
    pub fn new(ticks_to_wait: u64) -> Self {
        TimerFuture { target_tick: ticks() + ticks_to_wait }
    }
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        if ticks() >= self.target_tick {
            Poll::Ready(())
        } else {
            TIMER_WAKER.register(cx.waker());
            Poll::Pending
        }
    }
}