//! A set of traits and proc-macros involved in making and using generators
//! to create [`Stream`]s, [`Future`]s and [`Iterator`]s
//!
//! # Asynchronous example
//!
//! ```rust
//! // a couple required nightly features
//! #![feature(generators, generator_trait, never_type)]
//!
//! use jenner::generator;
//! use futures_core::Stream;
//! use std::time::{Instant, Duration};
//!
//! /// Creates a stream that yields u32s that countdown from 5 to 0.
//! /// Waiting 0.2s between each (1s total)
//! #[generator]
//! #[yields(u32)]
//! async fn countdown() {
//!     yield 5;
//!     for i in (0..5).rev() {
//!         tokio::time::sleep(Duration::from_millis(200)).await;
//!         yield i;
//!     }
//! }
//!
//! /// Iterates over the provided stream, printing the value and
//! /// pushing it to a vec that is returned
//! #[generator]
//! async fn collect(input: impl Stream<Item = u32>) -> Vec<u32> {
//!     let mut v = vec![];
//!
//!     for i in input {
//!         println!("{:?}", i);
//!         v.push(i)
//!     }.await; // special syntax to consume a stream
//!
//!     v
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let start = Instant::now();
//!
//!     // countdown() is a valid stream
//!     // collect(...) is a valid future
//!     let v = collect(countdown()).await;
//!     assert_eq!(v, vec![5, 4, 3, 2, 1, 0]);
//!
//!     assert!(start.elapsed() > Duration::from_millis(200 * 5));
//! }
//! ```
//!
//! # Synchronous example
//!
//! ```rust
//! #![feature(generators)]
//!
//! use jenner::generator;
//!
//! #[generator]
//! #[yields(usize)]
//! fn fibonacii() {
//!     use std::mem;
//!
//!     let mut a = 0;
//!     let mut b = 1;
//!     loop {
//!         yield a;
//!
//!         mem::swap(&mut a, &mut b);
//!         b += a;
//!     }
//! }
//!
//! fn main() {
//!     // fibonacii() is a valid `Iterator<Item = usize>`
//!     let v: Vec<_> = fibonacii().take(10).collect();
//!     assert_eq!(v, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
//! }
//! ```

#![feature(generator_trait, never_type)]

use futures_core::{Future, Stream};

/// From the provided generator body, it creates an `impl [AsyncGenerator]<Y, R>` type that implements
/// both `Future<Output = R>` and `Stream<Item = Y>`.
///
/// ```
/// #![feature(generators, generator_trait, never_type)]
///
/// use jenner::async_generator;
/// use futures_core::Stream;
/// use std::future::Future;
/// use std::time::{Instant, Duration};
///
/// /// Creates a stream that yields u32s that countdown from 5 to 0.
/// /// Waiting 0.2s between each (1s total)
/// fn countdown() -> impl Stream<Item = u32> {
///     async_generator!{
///         yield 5;
///         for i in (0..5).rev() {
///             tokio::time::sleep(Duration::from_millis(200)).await;
///             yield i;
///         }
///     }
/// }
///
/// /// Iterates over the provided stream, printing the value and
/// /// pushing it to a vec that is returned
/// fn collect(input: impl Stream<Item = u32>) -> impl Future<Output = Vec<u32>> {
///     async_generator!{
///         let mut v = vec![];
///
///         for i in input {
///             println!("{:?}", i);
///             v.push(i)
///         }.await; // special syntax to consume a stream
///
///         v
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let start = Instant::now();
///
///     // countdown() is a valid stream
///     // collect(...) is a valid future
///     let v = collect(countdown()).await;
///     assert_eq!(v, vec![5, 4, 3, 2, 1, 0]);
///
///     assert!(start.elapsed() > Duration::from_millis(200 * 5));
/// }
/// ```
pub use jenner_macro::async_generator;

/// Apply to a function to convert it into an iterator, allowing the use of the `yield` keyword.
/// Iterators can be synchronous or asynchronous.
///
/// # Asynchronous example
///
/// ```
/// // a couple required nightly features
/// #![feature(generators, generator_trait, never_type)]
///
/// use jenner::generator;
/// use futures_core::Stream;
/// use std::time::{Instant, Duration};
///
/// /// Creates a stream that yields u32s that countdown from 5 to 0.
/// /// Waiting 0.2s between each (1s total)
/// #[generator]
/// #[yields(u32)]
/// async fn countdown() {
///     yield 5;
///     for i in (0..5).rev() {
///         tokio::time::sleep(Duration::from_millis(200)).await;
///         yield i;
///     }
/// }
///
/// /// Iterates over the provided stream, printing the value and
/// /// pushing it to a vec that is returned
/// #[generator]
/// async fn collect(input: impl Stream<Item = u32>) -> Vec<u32> {
///     let mut v = vec![];
///
///     for i in input {
///         println!("{:?}", i);
///         v.push(i)
///     }.await; // special syntax to consume a stream
///
///     v
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let start = Instant::now();
///
///     // countdown() is a valid stream
///     // collect(...) is a valid future
///     let v = collect(countdown()).await;
///     assert_eq!(v, vec![5, 4, 3, 2, 1, 0]);
///
///     assert!(start.elapsed() > Duration::from_millis(200 * 5));
/// }
/// ```
///
/// # Synchronous example ([`Iterator`])
///
/// ```
/// #![feature(generators)]
///
/// use jenner::generator;
///
/// #[generator]
/// #[yields(usize)]
/// fn fibonacii() {
///     use std::mem;
///
///     let mut a = 0;
///     let mut b = 1;
///     loop {
///         yield a;
///
///         mem::swap(&mut a, &mut b);
///         b += a;
///     }
/// }
///
/// fn main() {
///     // fibonacii() is a valid `Iterator<Item = usize>`
///     let v: Vec<_> = fibonacii().take(10).collect();
///     assert_eq!(v, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
/// }
/// ```
pub use jenner_macro::generator;

use pin_project::pin_project;
use std::{
    mem,
    ops::{Generator, GeneratorState},
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

#[doc(hidden)]
pub mod __private {
    pub use futures_core::{Future, Stream};
    pub use std::{ops::GeneratorState, pin, task};
}

#[doc(hidden)]
pub struct UnsafeContextRef(NonNull<Context<'static>>);

impl UnsafeContextRef {
    #[doc(hidden)]
    pub unsafe fn get_context(&mut self) -> &mut Context<'_> {
        mem::transmute(self.0)
    }
}

impl<'a> From<&mut Context<'a>> for UnsafeContextRef {
    fn from(cx: &mut Context<'a>) -> Self {
        Self(unsafe { mem::transmute(cx) })
    }
}

unsafe impl Send for UnsafeContextRef {}

#[pin_project]
#[doc(hidden)]
pub struct GeneratorImpl<G> {
    #[pin]
    generator: G,
}

impl<G> GeneratorImpl<G> {
    #[doc(hidden)]
    pub unsafe fn new_async<Y, R>(generator: G) -> Self
    where
        G: Generator<UnsafeContextRef, Yield = Poll<Y>, Return = R>,
    {
        Self { generator }
    }

    #[doc(hidden)]
    pub unsafe fn new_sync<Y, R>(generator: G) -> Self
    where
        G: Generator<(), Yield = Y, Return = R>,
    {
        Self { generator }
    }
}

impl<Y, G> Stream for GeneratorImpl<G>
where
    G: Generator<UnsafeContextRef, Yield = Poll<Y>>,
{
    type Item = Y;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().generator.resume(cx.into()) {
            GeneratorState::Yielded(p) => p.map(Some),
            GeneratorState::Complete(_) => Poll::Ready(None),
        }
    }
}

/// Future evaluates to the return value of the async stream
impl<R, G> Future for GeneratorImpl<G>
where
    G: Generator<UnsafeContextRef, Return = R>,
{
    type Output = R;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().generator.resume(cx.into()) {
            GeneratorState::Yielded(_) => Poll::Pending,
            GeneratorState::Complete(r) => Poll::Ready(r),
        }
    }
}

impl<Y, R, G> AsyncGenerator<Y, R> for GeneratorImpl<G>
where
    G: Generator<UnsafeContextRef, Yield = Poll<Y>, Return = R>,
{
    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<Y, G::Return>> {
        match self.project().generator.resume(cx.into()) {
            GeneratorState::Yielded(p) => p.map(GeneratorState::Yielded),
            GeneratorState::Complete(r) => Poll::Ready(GeneratorState::Complete(r)),
        }
    }
}

/// This trait is a combination of [`Stream`], [`Future`] and [`Generator`] all in one neat package.
pub trait AsyncGenerator<Y, R>: Stream<Item = Y> + Future<Output = R> {
    /// Poll the async generator, resuming it's execution until the next yield or await.
    ///
    /// Possible outcomes:
    ///     `future.poll()` is `Pending` => returns `Poll::Pending`,
    ///     `future.poll()` is `Ready(_)` => execution of generator continues until next yield point,
    ///     `yield item;` => returns `Poll::Ready(GeneratorState::Yielded(item))`,
    ///     `return item;` => returns `Poll::Ready(GeneratorState::Completed(item))`,
    fn poll_resume(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<GeneratorState<Y, R>>;

    #[doc(hidden)]
    fn into_async_generator(self) -> Self
    where
        Self: Sized,
    {
        self
    }
}

/// This trait is a combination of [`Iterator`], [`Finally`] and [`Generator`] all in one neat package.
pub trait SyncGenerator<Y, R>: Iterator<Item = Y> + Finally<Output = R> {
    /// Same as [`Generator::resume`] but with no argument, to match normal iterators
    fn resume(self: Pin<&mut Self>) -> GeneratorState<Y, R>;
}

/// This allows synchronous generators a way to return a value
/// once the execution is complete.
pub trait Finally {
    /// Type to return
    type Output;
    /// Consume to get the output.
    fn finally(self) -> Self::Output;
}

impl<R, G> Finally for GeneratorImpl<G>
where
    G: Generator<(), Return = R>,
{
    type Output = R;

    fn finally(self) -> Self::Output {
        let mut gen = self.generator;
        loop {
            // SAFETY: since gen never moves during the lifetime of this loop
            // the pin assumptions are never violated during the usage of the generator
            let gen = unsafe { Pin::new_unchecked(&mut gen) };
            match gen.resume(()) {
                GeneratorState::Yielded(_) => (),
                GeneratorState::Complete(r) => break r,
            }
        }
    }
}

impl<Y, G> Iterator for GeneratorImpl<G>
where
    G: Generator<(), Yield = Y>,
{
    type Item = Y;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: validate the safety of this...
        let gen = unsafe { Pin::new_unchecked(&mut self.generator) };
        match gen.resume(()) {
            GeneratorState::Yielded(y) => Some(y),
            GeneratorState::Complete(_) => None,
        }
    }
}

impl<Y, R, G> SyncGenerator<Y, R> for GeneratorImpl<G>
where
    G: Generator<(), Yield = Y, Return = R>,
{
    fn resume(self: Pin<&mut Self>) -> GeneratorState<Y, R> {
        self.project().generator.resume(())
    }
}

#[doc(hidden)]
pub trait IntoAsyncGenerator {
    type Yield;
    type Return;
    type AsyncGenerator: AsyncGenerator<Self::Yield, Self::Return>;
    fn into_async_generator(self) -> Self::AsyncGenerator;
}

impl<S> IntoAsyncGenerator for S
where
    S: Stream,
{
    type Yield = S::Item;
    type Return = ();
    type AsyncGenerator = StreamGenerator<Self>;

    fn into_async_generator(self) -> Self::AsyncGenerator {
        StreamGenerator { stream: self }
    }
}

#[doc(hidden)]
#[pin_project]
pub struct StreamGenerator<S> {
    #[pin]
    stream: S,
}

impl<S> Stream for StreamGenerator<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().stream.poll_next(cx)
    }
}

impl<S> Future for StreamGenerator<S>
where
    S: Stream,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().stream.poll_next(cx) {
            Poll::Pending | Poll::Ready(Some(_)) => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(()),
        }
    }
}

impl<S> AsyncGenerator<S::Item, ()> for StreamGenerator<S>
where
    S: Stream,
{
    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<S::Item, ()>> {
        match self.project().stream.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(r)) => Poll::Ready(GeneratorState::Yielded(r)),
            Poll::Ready(None) => Poll::Ready(GeneratorState::Complete(())),
        }
    }
}

/// Type returned by `.await`ed for loops
pub enum ForResult<B, F> {
    /// Value `break`ed from the for loop
    Break(B),
    /// If the for loop has a `Finally`/`Future` type, the value will be here
    Finally(F),
}

impl<B, F> ForResult<B, F> {
    pub fn into_break(self) -> B
    where
        F: Into<!>,
    {
        match self {
            ForResult::Break(b) => b,
            ForResult::Finally(f) => f.into(),
        }
    }
    pub fn into_finally(self) -> F
    where
        B: Into<!>,
    {
        match self {
            ForResult::Break(b) => b.into(),
            ForResult::Finally(f) => f,
        }
    }
}
