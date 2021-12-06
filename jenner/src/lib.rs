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
#![feature(generator_trait, never_type, unwrap_infallible)]

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

mod asynch;
mod stream;
mod sync;

pub use asynch::AsyncGenerator;
pub use sync::SyncGenerator;

#[doc(hidden)]
pub mod __private {
    pub use crate::asynch::UnsafeContextRef;
    pub use crate::stream::IntoAsyncGenerator;
    pub use futures_core::{Future, Stream};
    pub use std::{ops::GeneratorState, pin, task};
}

#[pin_project::pin_project]
#[doc(hidden)]
pub struct GeneratorImpl<G> {
    #[pin]
    generator: G,
}

/// Type returned by `.await`ed for loops
pub enum ForResult<B, F> {
    /// Value `break`ed from the for loop
    Break(B),
    /// If the for loop has a `Finally`/`Future` type, the value will be here
    Finally(F),
}

impl<B, F> ForResult<B, F> {
    pub fn into_finally(self) -> F
    where
        B: Into<!>,
    {
        self.finished().into_ok()
    }
    pub fn finished(self) -> Result<F, B> {
        match self {
            ForResult::Break(b) => Err(b),
            ForResult::Finally(f) => Ok(f),
        }
    }
}
