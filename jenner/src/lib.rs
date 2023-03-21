//! A set of traits and proc-macros involved in making and using generators
//! to create [`AsyncIterator`](std::async_iter::AsyncIterator)s, [`Future`](std::future::Future)s and [`Iterator`]s
//!
//! # Asynchronous example
//!
//! ```rust
//! // a couple required nightly features
//! #![feature(generators)]
//!
//! use effective::{Effective, EffectiveExt, Async, Multiple};
//! use jenner::effect;
//! use std::{time::{Instant, Duration}, convert::Infallible};
//!
//! /// Creates a stream that yields u32s that countdown from 5 to 0.
//! /// Waiting 0.2s between each (1s total)
//! #[effect]
//! #[yields]
//! async fn countdown() -> u32 {
//!     yield 5;
//!     for i in (0..5).rev() {
//!         tokio::time::sleep(Duration::from_millis(200)).await;
//!         yield i;
//!     }
//! }
//!
//! /// Iterates over the provided stream, printing the value and
//! /// pushing it to a vec that is returned
//! #[effect]
//! async fn collect(
//!     input: impl Effective<Item = u32, Failure = Infallible, Produces = Multiple, Async = Async>,
//! ) -> Vec<u32> {
//!     let mut v = vec![];
//!
//!     #[effect(async)]
//!     for i in input {
//!         println!("{:?}", i);
//!         v.push(i)
//!     }
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
//!     let v = collect(countdown()).shim().await;
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
//! use effective::EffectiveExt;
//! use jenner::effect;
//!
//! #[effect]
//! #[yields]
//! fn fibonacii() -> usize {
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
//!     let v: Vec<_> = fibonacii().shim().take(10).collect();
//!     assert_eq!(v, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
//! }
//! ```
#![feature(generator_trait)]

/// Apply to a function to convert it into an iterator, allowing the use of the `yield` keyword.
/// Iterators can be synchronous or asynchronous.
///
/// # Asynchronous example
///
/// ```
/// // a couple required nightly features
/// #![feature(generators)]
///
/// use effective::{Effective, EffectiveExt, Async, Multiple};
/// use jenner::effect;
/// use std::{time::{Instant, Duration}, convert::Infallible};
///
/// /// Creates a stream that yields u32s that countdown from 5 to 0.
/// /// Waiting 0.2s between each (1s total)
/// #[effect]
/// #[yields]
/// async fn countdown() -> u32 {
///     yield 5;
///     for i in (0..5).rev() {
///         tokio::time::sleep(Duration::from_millis(200)).await;
///         yield i;
///     }
/// }
///
/// /// Iterates over the provided stream, printing the value and
/// /// pushing it to a vec that is returned
/// #[effect]
/// async fn collect(
///     input: impl Effective<Item = u32, Failure = Infallible, Produces = Multiple, Async = Async>,
/// ) -> Vec<u32> {
///     let mut v = vec![];
///
///     #[effect(async)]
///     for i in input {
///         println!("{:?}", i);
///         v.push(i)
///     }
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
///     let v = collect(countdown()).shim().await;
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
/// use effective::EffectiveExt;
/// use jenner::effect;
///
/// #[effect]
/// #[yields]
/// fn fibonacii() -> usize {
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
///     // fibonacii().shim() is a valid `Iterator<Item = usize>`
///     let v: Vec<_> = fibonacii().shim().take(10).collect();
///     assert_eq!(v, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
/// }
/// ```
pub use jenner_macro::effect;

mod asynch;
mod sync;

#[doc(hidden)]
pub mod __private {
    pub use crate::asynch::UnsafeContextRef;
    pub use crate::asynch::{AsyncFallibleGeneratorImpl, AsyncGeneratorImpl, AsyncImpl};
    pub use crate::sync::{SyncFallibleGeneratorImpl, SyncGeneratorImpl};
    pub use effective;
    pub use std::future::{Future, IntoFuture};
    pub use std::{ops::GeneratorState, pin, task};
}
