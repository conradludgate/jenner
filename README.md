# jenner

A proc-macro to make use of nightly coroutine syntax in order to create and manipulate
streams using a much easier syntax, much akin to how async/await futures work.

## Example

```rust
#![feature(coroutines)] // required nightly feature
use jenner::{effect, AsyncStream};

/// Creating brand new streams
#[effect(yields)]
async fn countdown() -> u32 {
    yield 5;
    for i in (0..5).rev() {
        // futures can be awaited in these streams
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        // yielding values corresponds to the stream item
        yield i;
    }
}

/// Consuming streams to create new streams (akin to input.map())
#[effect(yields)]
async fn double(
    input: impl AsyncStream<u32>,
) -> u32 {
    // custom async for syntax handles the polling of the stream automatically for you
    #[effect(async)]
    for i in input {
        yield i * 2;
    }
}

/// Futures are also supported
#[effect]
async fn collect<T: std::fmt::Debug>(
    input: impl AsyncStream<T>,
) -> Vec<T> {
    let mut v = vec![];
    #[effect(async)]
    for i in input {
        println!("{:?}", i);
        v.push(i)
    }
    // Return value of the function is the output of the future
    v
}
```

## Breakdown

The `effect` attribute macro works in a very simple way, making a few simple but crucial transformations.
The expansion snippets are schematic: `$expr`, `$stream` and `$body` stand for user code.

### Generator

Firstly, the function signature is re-written to

```rust,ignore
fn countdown() -> impl AsyncStream<u32>;
fn double(input: impl AsyncStream<u32>) -> impl AsyncStream<u32>;
fn collect<T: std::fmt::Debug>(input: impl AsyncStream<T>) -> impl AsyncValue<Vec<T>>;
```

Then, a yielding async function's body is wrapped in this expression
(`$body` stands for the transformed body, and `__cx` for the generated context variable)

```rust,ignore
::jenner::__private::Generator::new(
    #[coroutine]
    static move |mut __cx: ::jenner::__private::UnsafeContextRef| {
        $body
    }
)
```

The wrapper implements `AsyncGenerator` and projects the pinned coroutine. Its
`poll_resume` method returns `Poll<CoroutineState<Yield, Return>>`: a pending
operation, a yielded item, or the final return value.

Trait aliases describe the common shapes. `AsyncStream<T>` combines
`AsyncGenerator<Yield = T, Return = ()>` with `Stream<Item = T>`, and
`AsyncValue<T>` combines `AsyncGenerator<Yield = Infallible, Return = T>` with
`Future<Output = T>`. These values support ordinary stream combinators and `.await`.

To yield items and return a separate final value, specify the yield type:

```rust
#![feature(coroutines)]
use jenner::effect;

#[effect(yields(u32))]
async fn counted() -> usize {
    yield 10;
    yield 20;
    2
}
```

This returns `impl AsyncGenerator<Yield = u32, Return = usize>`. Its final value is
available through `poll_resume`; it is not discarded by a stream adapter.
Synchronous functions use `Coroutine<(), Yield = Y, Return = R>` with the same
annotations. Pin a synchronous stream with `pin!`, then use `.as_mut().iter()`
from `SyncGeneratorExt` to borrow it as an iterator without allocating.

### Yields

Any `yield` keywords in an async body are modified from

```rust,ignore
yield $expr
```

into

```rust,ignore
__cx = yield Poll::Ready($expr)
```

This allows the generator to tell the stream that a value is ready. Resuming
the generator supplies the context for the next poll, which is assigned to `__cx`.

### Awaits

Native `.await` syntax is not available inside a coroutine body.
To get around this, the following rule is applied. The operand implements
`IntoFuture`, just as in an ordinary async function. Nested async blocks and
closures keep their own suspension and return boundaries.

Any `.await` keywords in the body are modified from

```rust,ignore
$expr.await
```

into

```rust,ignore
{
    let mut fut = ::core::pin::pin!(IntoFuture::into_future($expr));
    loop {
        let polled = {
            // The context borrow is confined to this child poll, before any yield.
            let cx = unsafe { __cx.get_context() };
            Future::poll(fut.as_mut(), cx)
        };
        match polled {
            Poll::Ready(r) => break r,
            Poll::Pending => {
                __cx = yield Poll::Pending;
            }
        }
    }
}
```

This expansion is quite big in comparison to the `yield`.

We create a loop to allow us to repeatedly poll the future.
If the future is still pending, then we just yield that back up to the stream.
This tells the stream that it's currently waiting for some asynchronous task to complete.

If the future's output is ready, we `break` the value from the loop. This uses the fact
that loops are an expression. This allows us to assign the value from the future into our stream's scope.

This is pretty close to how `await` works in regular rust's `async` blocks.

### For Await

Iterating over streams is currently a very poor experience.
Instead, we provide a simple syntax to iterate any `Stream` asynchronously.

```rust,ignore
#[effect(async)]
for i in $stream {
    $body
}
```

The loop consumes any `Stream`. Each item is passed to the body, and `None`
finishes the loop.
An ordinary `break` exits the loop; its output is `()`.

When processed, the code turns into

```rust,ignore
{
    let mut gen = ::std::pin::pin!($stream); // evaluate and pin the stream
    loop {
        let next = loop {
            let polled = {
                // The context borrow ends before any suspension.
                let cx = unsafe { __cx.get_context() };
                Stream::poll_next(gen.as_mut(), cx)
            };
            match polled {
                Poll::Ready(next) => break next,
                Poll::Pending => {
                    __cx = yield Poll::Pending;
                }
            }
        };
        if let Some(i) = next { $body } else { break };
    }
}
```

This is pretty similar to the `await` case, but repeated.

### Futures

An `#[effect] async fn` without `yields` returns `impl AsyncValue<T>`, where `T`
is the function's return type. For example,
`collect(double(countdown())).await` returns a `Vec<u32>`.

Its coroutine has `Yield = Poll<Infallible>`, which determines the yield type
for a function that never yields an item. The wrapper maps its final return
value to `Poll::Ready` in the `Future` implementation.

### Error Handling

Since these effects can fail, we can use the try `?` syntax to return early
from functions marked `fallible`. With `yields`, the success type in the return
annotation describes the yielded items.

```rust
#![feature(coroutines)]
use jenner::effect;

#[effect(yields, fallible)]
async fn make_requests() -> Result<u32, &'static str> {
    for i in 0..5 {
        let resp = async move {
            // imagine this makes a http request that could fail
            let req = if i == 4 { Err("4 is a random number") } else { Ok(i) };
            req
        }.await;

        // Using the `?` syntax to return early with the error
        // but continue with any good values. (can be used anywhere and not exclusively with yields)
        yield resp?;
    }
}
```

Successful completion is wrapped in `Ok` by the macro; a yielding function's body
finishes with `()`. Ordinary Rust `?` propagates errors from the coroutine and
supports error conversion through `From`.

`TryAsyncStream<T, E>` has `Yield = T` and `Return = Result<(), E>`. Its `Stream`
implementation emits each item as `Ok`, then a terminal error as `Err` once,
followed by `None`. For example, `make_requests()` produces `Ok(0)` through
`Ok(3)`, then `Err("4 is a random number")`. Synchronous fallible streams expose
the same sequence through `.as_mut().try_iter()`.

Functions without `yields` return a single `Result<T, E>`. A `fallible` body has
an unwrapped successful tail expression; a body that writes its own `Ok(...)`
can use `#[effect]` without the `fallible` flag.
