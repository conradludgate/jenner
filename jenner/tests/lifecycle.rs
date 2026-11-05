#![feature(coroutines)]

use std::{
    cell::Cell,
    future::Future,
    marker::PhantomPinned,
    pin::{pin, Pin},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Wake, Waker},
};

use jenner::{
    effect,
    effective::{wrappers::future, EffectResult, Effective},
};

#[derive(Default)]
struct Wakes(AtomicUsize);
impl Wake for Wakes {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

struct PinnedFuture {
    address: Cell<usize>,
    polls: Cell<usize>,
    drops: Arc<AtomicUsize>,
    _pin: PhantomPinned,
}
impl PinnedFuture {
    fn new(drops: Arc<AtomicUsize>) -> Self {
        Self {
            address: Cell::new(0),
            polls: Cell::new(0),
            drops,
            _pin: PhantomPinned,
        }
    }
}
impl Future for PinnedFuture {
    type Output = u32;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        let this = self.as_ref().get_ref();
        let address = this as *const Self as usize;
        let previous = this.address.replace(address);
        assert!(
            previous == 0 || previous == address,
            "future moved after polling"
        );
        let polls = this.polls.get();
        this.polls.set(polls + 1);
        cx.waker().wake_by_ref();
        if polls == 0 {
            Poll::Pending
        } else {
            Poll::Ready(7)
        }
    }
}
impl Drop for PinnedFuture {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
}

#[effect]
async fn await_pinned(drops: Arc<AtomicUsize>) -> u32 {
    future(PinnedFuture::new(drops)).await
}

#[effect(yields)]
async fn yield_and_await(drops: Arc<AtomicUsize>) -> u32 {
    yield 1;
    let value = future(PinnedFuture::new(drops)).await;
    yield value;
}

#[effect(yields)]
fn borrowing_generator() -> u32 {
    let value = Cell::new(3);
    let borrow = &value;
    yield borrow.get();
    value.set(4);
    yield borrow.get();
}

#[test]
fn a_synchronous_generator_can_borrow_across_yields() {
    let mut effect = pin!(borrowing_generator());
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        effect.as_mut().poll_effect(&mut cx),
        EffectResult::Item(3)
    ));
    assert!(matches!(
        effect.as_mut().poll_effect(&mut cx),
        EffectResult::Item(4)
    ));
    assert!(matches!(
        effect.as_mut().poll_effect(&mut cx),
        EffectResult::Done(_)
    ));
}

#[effect]
async fn panic_in_poll(drops: Arc<AtomicUsize>) {
    future(async move {
        let _guard = PinnedFuture::new(drops);
        panic!("child poll panicked");
    })
    .await
}

#[test]
fn panic_during_poll_drops_locals_once() {
    let drops = Arc::new(AtomicUsize::new(0));
    {
        let mut effect = pin!(panic_in_poll(drops.clone()));
        let mut cx = Context::from_waker(Waker::noop());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            effect.as_mut().poll_effect(&mut cx)
        }));
        assert!(result.is_err());
    }
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

#[test]
fn pending_uses_the_context_from_each_poll() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut effect = pin!(await_pinned(drops.clone()));
    let first = Arc::new(Wakes::default());
    {
        let waker = Waker::from(first.clone());
        let mut cx = Context::from_waker(&waker);
        assert!(matches!(
            effect.as_mut().poll_effect(&mut cx),
            EffectResult::Pending(_)
        ));
    }
    let second = Arc::new(Wakes::default());
    {
        let waker = Waker::from(second.clone());
        let mut cx = Context::from_waker(&waker);
        assert!(matches!(
            effect.as_mut().poll_effect(&mut cx),
            EffectResult::Item(7)
        ));
    }
    assert_eq!(first.0.load(Ordering::Relaxed), 1);
    assert_eq!(second.0.load(Ordering::Relaxed), 1);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

#[test]
fn cancelling_a_pending_effect_drops_its_pinned_future_once() {
    let drops = Arc::new(AtomicUsize::new(0));
    {
        let mut effect = pin!(await_pinned(drops.clone()));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(matches!(
            effect.as_mut().poll_effect(&mut cx),
            EffectResult::Pending(_)
        ));
        assert_eq!(drops.load(Ordering::Relaxed), 0);
    }
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

#[test]
fn yielding_refreshes_the_context_before_the_next_await() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut effect = pin!(yield_and_await(drops.clone()));
    {
        let mut cx = Context::from_waker(Waker::noop());
        assert!(matches!(
            effect.as_mut().poll_effect(&mut cx),
            EffectResult::Item(1)
        ));
    }
    let wakes = Arc::new(Wakes::default());
    let waker = Waker::from(wakes.clone());
    let mut cx = Context::from_waker(&waker);
    assert!(matches!(
        effect.as_mut().poll_effect(&mut cx),
        EffectResult::Pending(_)
    ));
    assert!(matches!(
        effect.as_mut().poll_effect(&mut cx),
        EffectResult::Item(7)
    ));
    assert!(matches!(
        effect.as_mut().poll_effect(&mut cx),
        EffectResult::Done(_)
    ));
    assert_eq!(wakes.0.load(Ordering::Relaxed), 2);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

#[test]
fn a_pending_effect_can_move_to_another_thread() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut effect = Box::pin(await_pinned(drops.clone()));
    {
        let mut cx = Context::from_waker(Waker::noop());
        assert!(matches!(
            effect.as_mut().poll_effect(&mut cx),
            EffectResult::Pending(_)
        ));
    }
    std::thread::spawn(move || {
        let mut cx = Context::from_waker(Waker::noop());
        assert!(matches!(
            effect.as_mut().poll_effect(&mut cx),
            EffectResult::Item(7)
        ));
    })
    .join()
    .unwrap();
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}
