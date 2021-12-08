#![feature(
    generators,
    never_type,
    type_alias_impl_trait,
    generic_associated_types
)]

use std::{marker::PhantomData, pin::Pin};

use jenner::{async_generator, AsyncGenerator};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt};

pub struct Buf<R> {
    buffer: Vec<u8>,
    reader: R,
}

#[derive(Clone)] // purposefully not copy
pub struct Cursor {
    idx: usize,
}

#[derive(Clone, Copy)]
pub struct Span {
    pub bytes: (usize, usize), // Range<usize>
}

impl<R: AsyncRead + Unpin> Buf<R> {
    pub fn new(reader: R) -> (Self, Cursor) {
        (
            Self {
                buffer: vec![],
                reader,
            },
            Cursor { idx: 0 },
        )
    }

    pub async fn ensure(&mut self, n: usize) -> io::Result<()> {
        // if not enough bytes available
        if n > self.buffer.len() {
            let diff = n - self.buffer.len();
            self.buffer
                .try_reserve(diff)
                .map_err(|_| io::ErrorKind::OutOfMemory)?;

            let mut n = 0;
            while diff > n {
                let r = self.reader.read_buf(&mut self.buffer).await?;
                if r == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!("needed {} more bytes", diff),
                    ));
                }
                n += r;
            }
        }

        Ok(())
    }
}

impl Cursor {
    pub async fn take_from(
        &mut self,
        buf: &mut Buf<impl AsyncRead + Unpin>,
        n: usize,
    ) -> io::Result<Span> {
        let start = self.idx;
        let end = start + n;
        buf.ensure(end).await?;
        self.idx = end;

        Ok(Span {
            bytes: (start, end),
        })
    }
}

impl Span {
    pub fn view<'buf>(self, buf: &'buf Buf<impl AsyncRead>) -> &'buf [u8] {
        &buf.buffer[self.bytes.0..self.bytes.1]
    }
}

pub struct BufCursor<R> {
    pub buf: Buf<R>,
    pub cur: Cursor,
}

impl<R: AsyncRead + Unpin> BufCursor<R> {
    pub async fn take_from(mut self, n: usize) -> io::Result<(Self, Span)> {
        let span = self.cur.take_from(&mut self.buf, n).await?;
        Ok((self, span))
    }
}

pub trait Parser<'gen, R: AsyncRead + Unpin>
where
    Self: 'gen,
{
    type Yields;
    type Output;
    type AsyncGenerator: AsyncGenerator<Self::Yields, io::Result<(BufCursor<R>, Self::Output)>>
        + 'gen;

    fn parse<'life0>(&'life0 self, cursor: BufCursor<R>) -> Self::AsyncGenerator
    where
        'life0: 'gen,
        Self: 'gen;
}

pub trait Parser3<R: AsyncRead + Unpin> {
    type Yields;
    type Output;

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn parse<'life0, 'async_trait>(
        &'life0 self,
        cursor: BufCursor<R>,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = (BufCursor<R>, Self::Output)>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait;
}

#[repr(transparent)]
pub struct Tag([u8]);
pub fn tag(t: &[u8]) -> &Tag {
    let p = t as *const _ as *const Tag;
    unsafe { &*p }
}

impl<'gen, R: AsyncRead + Unpin + Send + 'static> Parser<'gen, R> for Tag {
    type Yields = !;
    type Output = Span;
    type AsyncGenerator =
        impl AsyncGenerator<Self::Yields, io::Result<(BufCursor<R>, Self::Output)>> + 'gen;

    fn parse<'life0>(&'life0 self, cursor: BufCursor<R>) -> Self::AsyncGenerator
    where
        'life0: 'gen,
        Self: Sync + 'gen,
        BufCursor<R>: Send,
    {
        async_generator! {
            let fut = cursor.take_from(self.0.len());
            let span = fut.await?;
            Ok(span)
        }
        // Box::pin(unsafe {
        //     ::jenner::GeneratorImpl::new_async::<!, _>(
        //         static |mut __cx_OsN5tXI: ::jenner::__private::UnsafeContextRef| {
        //             let span = {
        //                 let mut fut = { cursor.take_from(self.0.len()) };
        //                 loop {
        //                     let polled = unsafe {
        //                         ::jenner::__private::Future::poll(
        //                             ::jenner::__private::pin::Pin::new_unchecked(&mut fut),
        //                             __cx_OsN5tXI.get_context(),
        //                         )
        //                     };
        //                     match polled {
        //                         ::jenner::__private::task::Poll::Ready(r) => break r,
        //                         ::jenner::__private::task::Poll::Pending => {
        //                             yield ::jenner::__private::task::Poll::Pending;
        //                         }
        //                     }
        //                 }
        //             }?;
        //             Ok(span)
        //         },
        //     )
        // })
    }
}

// #[async_trait::async_trait]
// trait Foo {
//     async fn bar(&self, input: Baz) -> Output {
//         x;
//         y.await;
//         z
//     }
//     ::core::pin::Pin<Box<dyn::core::future::Future<Output = Output> + ::core::marker::Send+ 'async_trait> >where 'life0: 'async_trait,Self: ::core::marker::Sync+ 'async_trait
// }

#[tokio::test]
async fn test_parse() {
    let input = b"foobar";
    let (buf, cur) = Buf::new(&input[..]);
    let buffer = BufCursor { buf, cur };

    let (buffer, foo) = tag(b"foo").parse(buffer).await.unwrap();
    let (buffer, bar) = tag(b"bar").parse(buffer).await.unwrap();

    assert_eq!(foo.bytes, (0, 3));
    assert_eq!(bar.bytes, (3, 6));
}
