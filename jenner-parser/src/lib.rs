#![feature(generators, never_type, type_alias_impl_trait)]

use std::marker::PhantomData;

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
    bytes: (usize, usize), // Range<usize>
}

impl<R: AsyncRead + Unpin> Buf<R> {
    pub async fn ensure(&mut self, n: usize) -> io::Result<()> {
        // if not enough bytes available
        if n > self.buffer.len() {
            let mut diff = n - self.buffer.len();
            self.buffer
                .try_reserve(diff)
                .map_err(|_| io::ErrorKind::OutOfMemory)?;

            while diff > 0 {
                let r = self.reader.read_buf(&mut self.buffer).await?;
                if r == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!("needed {} more bytes", diff),
                    ));
                }
                diff -= r;
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

pub trait Parser<R: AsyncRead + Unpin> {
    type Yields;
    type Output;
    type Generator: AsyncGenerator<Self::Yields, io::Result<(BufCursor<R>, Self::Output)>>;

    fn parse(&self, cursor: BufCursor<R>) -> Self::Generator;
}

pub struct Tag(pub [u8]);

impl<R: AsyncRead + Unpin> Parser<R> for Tag {
    type Yields = !;
    type Output = Span;

    type Generator = impl AsyncGenerator<Self::Yields, io::Result<(BufCursor<R>, Self::Output)>>;

    fn parse(&self, mut cursor: BufCursor<R>) -> Self::Generator {
        // async_generator!{
        //     let fut = cursor.cur.take_from(&mut cursor.buf, self.0.len());
        //     let span = fut.await?;
        //     Ok((cursor, span))
        // }
        unsafe {
            ::jenner::GeneratorImpl::new_async::<!, _>(
                move |mut __cx_OsN5tXI: ::jenner::__private::UnsafeContextRef| {
                    let span = {
                        let mut fut = { cursor.take_from(self.0.len()) };
                        loop {
                            let polled = unsafe {
                                ::jenner::__private::Future::poll(
                                    ::jenner::__private::pin::Pin::new_unchecked(&mut fut),
                                    __cx_OsN5tXI.get_context(),
                                )
                            };
                            match polled {
                                ::jenner::__private::task::Poll::Ready(r) => break r,
                                ::jenner::__private::task::Poll::Pending => {
                                    yield ::jenner::__private::task::Poll::Pending;
                                }
                            }
                        }
                    }?;
                    Ok(span)
                },
            )
        }
    }
}
