# jenner

A proc-macro to make use of nightly coroutine syntax in order to create and manipulate
streams using a much easier syntax, much akin to how async/await futures work.

## Example

```rust
#![feature(coroutines)] // required nightly feature
use jenner::{effect, effective::{wrappers::future, Async, Effective, EffectiveExt, Multiple}};
use std::convert::Infallible;

/// Creating brand new streams
#[effect(yields)]
async fn countdown() -> u32 {
    yield 5;
    for i in (0..5).rev() {
        // futures can be awaited in these streams
        future(tokio::time::sleep(std::time::Duration::from_secs(1))).await;
        // yielding values corresponds to the stream item
        yield i;
    }
}

/// Consuming streams to create new streams (akin to input.map())
#[effect(yields)]
async fn double(
    input: impl Effective<Item = u32, Failure = Infallible, Produces = Multiple, Async = Async>,
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
    input: impl Effective<Item = T, Failure = Infallible, Produces = Multiple, Async = Async>,
) -> Vec<T> {
    let mut v = vec![];
    #[effect(async)]
    for i in input {
        println!("{:?}", i);
        v.push(i)
    }
    // Return value of the function is the output of the future adapter
    v
}
```

## Breakdown

The `effect` attribute macro works in a very simple way, making a few simple but crucial transformations.
The expansion snippets are schematic: `$expr`, `$stream` and `$body` stand for user code.

### Generator

Firstly, the function signature is re-written to

```rust,ignore
fn countdown() -> impl Effective<
    Item = u32, Failure = Infallible, Produces = Multiple, Async = Async,
>;
fn double(
    input: impl Effective<Item = u32, Failure = Infallible, Produces = Multiple, Async = Async>,
) -> impl Effective<Item = u32, Failure = Infallible, Produces = Multiple, Async = Async>;
fn collect<T: std::fmt::Debug>(
    input: impl Effective<Item = T, Failure = Infallible, Produces = Multiple, Async = Async>,
) -> impl Effective<Item = Vec<T>, Failure = Infallible, Produces = Single, Async = Async>;
```

Then, a yielding async function's body is wrapped in this expression
(`$body` stands for the transformed body, and `__cx` for the generated context variable)

```rust,ignore
::jenner::__private::AsyncGeneratorImpl::create(
    #[coroutine]
    static move |mut __cx: ::jenner::__private::UnsafeContextRef| {
        $body
    }
)
```

The `create` function is fairly simple.
It accepts a `Coroutine<UnsafeContextRef, Yield = Poll<Y>, Return = ()>` and returns
an `Effective<Item = Y, Failure = Infallible, Produces = Multiple, Async = Async>` value.
Calling `.shim()` on this value provides a stream. For infallible async functions
without `yields`, the `AsyncImpl` wrapper provides a single item, and `.shim()`
provides a future.

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
To get around this, the following rule is applied. The operand is an infallible,
single-item `Effective`; `wrappers::future` adapts a standard Rust future.

Any `.await` keywords in the body are modified from

```rust,ignore
$expr.await
```

into

```rust,ignore
{
    let mut fut = ::std::pin::pin!($expr);
    loop {
        let polled = {
            // The context borrow is confined to this child poll, before any yield.
            let cx = unsafe { __cx.get_context() };
            Effective::poll_effect(fut.as_mut(), cx)
        };
        match polled {
            EffectResult::Item(r) => break r,
            EffectResult::Pending(_) => {
                __cx = yield Poll::Pending;
            }
            EffectResult::Done(_) | EffectResult::Failure(_) => unreachable!(),
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
Instead, we provide a simple syntax to iterate any generator asynchronously.

```rust,ignore
#[effect(async)]
for i in $stream {
    $body
}
```

The loop consumes an infallible asynchronous `Effective` with `Produces = Multiple`.
Each item is passed to the body, and `EffectResult::Done` finishes the loop.
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
                Effective::poll_effect(gen.as_mut(), cx)
            };
            match polled {
                EffectResult::Done(_) => break None,
                EffectResult::Item(i) => break Some(i),
                EffectResult::Failure(_) => unreachable!(),
                EffectResult::Pending(_) => {
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

An `#[effect] async fn` without `yields` produces a single value. Calling
`.shim()` adapts it to a future whose output is the function's return type.
For example, `collect(double(countdown())).shim().await` returns a `Vec<u32>`.

The `AsyncImpl` wrapper requires `Yield = Poll<Infallible>`, which determines the
yield type for a function that never yields an item. Its coroutine return value
becomes `EffectResult::Item`, with `Produces = Single`.

### Error Handling

Since these effects can fail, we can use the try `?` syntax to return early
from functions marked `fallible`. With `yields`, the success type in the return
annotation describes the yielded items.

```rust
#![feature(coroutines)]
use jenner::{effect, effective::wrappers::future};

#[effect(yields, fallible)]
async fn make_requests() -> Result<u32, &'static str> {
    for i in 0..5 {
        let resp = future(async move {
            // imagine this makes a http request that could fail
            let req = if i == 4 { Err("4 is a random number") } else { Ok(i) };
            req
        }).await;

        // Using the `?` syntax to return early with the error
        // but continue with any good values. (can be used anywhere and not exclusively with yields)
        yield resp?;
    }
}
```

The fallible wrapper maps the coroutine's `Err` return to `EffectResult::Failure`.
Successful completion is wrapped in `Ok` by the macro; a yielding function's body
finishes with `()`.

For an asynchronous effect, `?` is expanded through `effective::Fallible::branch`:
continuation values remain in the body, and failures return `Err` from the coroutine.
The helper traits determine the item and failure types from the return annotation.
