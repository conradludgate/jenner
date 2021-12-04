# streams-generator

A proc-macro to make use of nightly generator syntax in order to create and manipulate
streams using a much easier syntax, much akin to how async/await futures work today.

## Example

```rust
#![feature(generators)] // required nightly feature
use streams_generator::stream;
use std::future::Future; // Futures provided by std
use futures_core::Stream; // Streams provided by futures

/// Creating brand new streams
fn zero_to_three() -> impl Stream<Item = u32> {
    stream! {
        for i in (0..5).rev() {
            // futures can be awaited in these streams
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            // yielding values corresponds to the stream item
            yield i;
        }
    }
}

/// Consuming streams to create new streams (akin to input.map())
fn double(input: impl Stream<Item = u32>) -> impl Stream<Item = u32> {
    stream! {
        // custom async for syntax handles the polling of the stream automatically for you
        async for i in input {
            yield i * 2;
        }
    }
}

/// Futures are also supported
fn collect<T: std::fmt::Debug>(input: impl Stream<Item = T>) -> impl Future<Output = Vec<T>> {
    stream! {
        let mut v = vec![];
        async for i in input {
            println!("got {:?}", i);
            v.push(i)
        }
        /// Return value of the stream is the output of the future
        v
    }
}
```

## Breakdown

The `stream!` macro works in a very simple way, making a few simple but crucial transformations.

### Generator

Firstly, the entire block body is wrapped in this expression

```rust
unsafe {
    ::streams_generator::new_stream_generator(|mut __cx: ::streams_generator::UnsafeContextRef|{
        $body
    })
}
```

The `new_stream_generator` function is fairly simple.
It accepts a `Generator<Yield = Poll<Y>, Return = R>` and returns an `AsyncStream` type,
which implements `StreamGenerator<Y, R>` (and by extension, `Stream<Item = Y>` and `Future<Output = R>`).

### Yields

Any `yield` keywords in the body are modified from

```rust
yield $expr
```

into

```rust
yield ::std::task::Poll::Ready($expr)
```

This allows the generator to tell the stream that a new value is now ready.

### Awaits

Currently, with the state of generators in nightly, you cannot mix `yield`s and `await`s.
To get around this, the following rule is applied

Any `.await` keywords in the body are modified from

```rust
$expr.await
```

into

```rust
{
    let mut fut = $expr;

    loop {
        let polled = unsafe {
            ::std::future::Future::poll(
                ::std::pin::Pin::new_unchecked(&mut fut),
                __cx.get_context()
            )
        };
        match polled {
            ::std::task::Poll::Ready(r) => break r,
            ::std::task::Poll::Pending => {
                yield ::std::task::Poll::Pending;
            }
        }
    }
}
```

This change is quite big in comparison to the `yield`.

We create a loop to allow us to repeatedly poll the future.
If the future is still pending, then we just yield that back up to the stream.
This tells the stream that it's currently waiting for some asynchronous task to complete.

If the future's output is now ready, we `break` the value from the loop. This uses the fact
that loops are an expression. This allows us to assign the value from the future into our stream's scope.

This is pretty close to how `await` works in regular rust's `async` blocks.

### Async For

Iterating over streams is currently a very poor experience.
Instead, we provide a simple syntax to iterate the stream asynchronously.

```rust
async for i in $stream {
    $body
}
```

becomes

```rust
{
    let mut stream = $stream
    loop {
        let next = loop {
            let polled = unsafe {
                ::futures_core::stream::Stream::poll_next(
                    ::std::pin::Pin::new_unchecked(&mut stream),
                    __cx.get_context()
                )
            };
            match polled {
                ::std::task::Poll::Ready(r) => break r,
                ::std::task::Poll::Pending => {
                    yield::std::task::Poll::Pending;
                }
            }
        };
        match next {
            Some(i) => $body,
            _ => break,
        }
    }
}
```

This is pretty similar to the `await` case, but repeated.

### Futures

While these stream generators are automatically valid futures,
and edge case occurs when you never actually call `yield` since the
`Yield` type cannot be inferred from the context.

We solve this by counting the number of `yield` statements we see in the body.
If no `yield` tokens are found, we hard encode the `Yield` type in the function to `()`.
This is similar to how omitting a return from a function results in `()` being the returned value.

### Error Handling

Since these generators are also functions that can return value,
we can use the try `?` syntax to return early from functions.

```rust
fn make_requests() -> impl StreamGenerator<u32, anyhow::Result<()>> {
    stream! {
        for i in 0..5 {
            let resp = async move {
                // imagine this makes a http request that could fail
                let req = if i == 4 { Err("4 is a random number") } else { Ok(i) };
                req
            }.await

            // Using the `?` syntax to return early with the error
            // but continue with any good values. (can be used anywhere and not exclusively with yields)
            yield resp?;
        }

        // we don't care about the return value, but rust needs one anyway
        Ok(())
    }
}
```
