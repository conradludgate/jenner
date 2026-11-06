//! Coroutine macros for futures, streams and synchronous generators.
//!
//! # Asynchronous example
//!
//! ```rust
//! // add the required nightly feature
//! #![feature(coroutines)]
//!
//! use jenner::{effect, AsyncStream};
//! use std::time::{Instant, Duration};
//!
//! /// Creates a stream that yields u32s that countdown from 5 to 0.
//! /// Waiting 0.2s between each (1s total)
//! #[effect(yields)]
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
//!     input: impl AsyncStream<u32>,
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
//! #![feature(coroutines)]
//!
//! use jenner::{effect, SyncGeneratorExt};
//! use std::pin::pin;
//!
//! #[effect(yields)]
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
//!     // Pin the generator, then borrow it as an iterator.
//!     let mut generator = pin!(fibonacii());
//!     let v: Vec<_> = generator.as_mut().iter().take(10).collect();
//!     assert_eq!(v, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
//! }
//! ```
#![feature(coroutine_trait)]
#![feature(trait_alias)]

mod generator;
mod iter;

pub use futures_core::Stream;
pub use generator::{
    AsyncGenerator, AsyncStream, AsyncValue, SyncStream, SyncValue, TryAsyncStream, TrySyncStream,
};
pub use iter::{Iter, SyncGeneratorExt, TryIter};
pub use std::ops::{Coroutine, CoroutineState};

/// Apply to a function to produce a synchronous or asynchronous generator.
///
/// `yields` uses the return annotation as the item type. `yields(T)` specifies
/// the item type separately and preserves the function's final return value.
/// `fallible` wraps the successful body in `Ok`, allowing ordinary `?` errors.
///
/// # Asynchronous example
///
/// ```
/// // add the required nightly feature
/// #![feature(coroutines)]
///
/// use jenner::{effect, AsyncStream};
/// use std::time::{Instant, Duration};
///
/// /// Creates a stream that yields u32s that countdown from 5 to 0.
/// /// Waiting 0.2s between each (1s total)
/// #[effect(yields)]
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
///     input: impl AsyncStream<u32>,
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
/// #![feature(coroutines)]
///
/// use jenner::{effect, SyncGeneratorExt};
/// use std::pin::pin;
///
/// #[effect(yields)]
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
///     // Pin the generator, then borrow it as an iterator.
///     let mut generator = pin!(fibonacii());
///     let v: Vec<_> = generator.as_mut().iter().take(10).collect();
///     assert_eq!(v, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
/// }
/// ```
pub use jenner_macro::effect;

mod asynch;

#[doc(hidden)]
pub mod __private {
    pub use crate::asynch::UnsafeContextRef;
    pub use crate::generator::{Generator, TryAsyncStreamImpl};

    // Extract types from Result aliases without parsing their spelling in macros.
    pub trait ResultTypes {
        type Ok;
        type Error;
    }
    impl<T, E> ResultTypes for Result<T, E> {
        type Ok = T;
        type Error = E;
    }
}
