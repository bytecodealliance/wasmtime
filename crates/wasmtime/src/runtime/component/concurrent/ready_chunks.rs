//! Like `futures::stream::ReadyChunks` but without fusing the inner stream.
//!
//! We use this with `FuturesUnordered` which may produce `Poll::Ready(None)` but later produce more elements due
//! to additional futures having been added, so fusing is not appropriate.

use {
    futures::{Stream, StreamExt},
    std::{
        pin::Pin,
        task::{Context, Poll},
        vec::Vec,
    },
};

pub struct ReadyChunks<S: Stream> {
    stream: S,
    capacity: usize,
}

impl<S: Stream> ReadyChunks<S> {
    pub fn new(stream: S, capacity: usize) -> Self {
        Self { stream, capacity }
    }

    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }
}

impl<S: Stream + Unpin> Stream for ReadyChunks<S> {
    type Item = Vec<S::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut items = Vec::new();

        loop {
            match self.stream.poll_next_unpin(cx) {
                Poll::Pending => {
                    break if items.is_empty() {
                        Poll::Pending
                    } else {
                        Poll::Ready(Some(items))
                    }
                }

                Poll::Ready(Some(item)) => {
                    items.push(item);
                    if items.len() >= self.capacity {
                        break Poll::Ready(Some(items));
                    }
                }

                Poll::Ready(None) => {
                    break Poll::Ready(if items.is_empty() { None } else { Some(items) });
                }
            }
        }
    }
}
