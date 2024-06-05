use super::*;
use h2::{RecvStream, StreamId};
use http::HeaderMap;

/// Represents an HTTP request object. It consists of the request headers and body.
pub struct Request {
    pub(crate) head: http::request::Parts,
    pub(crate) body: RecvStream,
}

impl Request {
    #[inline]
    /// Retrieve the next chunk of data from the request body.
    pub async fn data(&mut self) -> Option<Result<Bytes>> {
        poll_fn(|cx| self.poll_data(cx)).await
    }

    #[inline]
    #[doc(hidden)]
    pub fn poll_data(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        self.body.poll_data(cx).map(|out| match out {
            Some(Ok(data)) => {
                let _ = self.body.flow_control().release_capacity(data.len());
                Some(Ok(data))
            }
            v => v,
        })
    }

    /// Returns the stream ID of this stream.
    ///
    /// # Panics
    ///
    /// If the lock on the stream store has been poisoned.
    #[inline]
    pub fn stream_id(&self) -> StreamId {
        self.body.stream_id()
    }

    /// Returns true if the receive half has reached the end of stream.
    ///
    /// A return value of `true` means that calls to `poll` and `poll_trailers`
    /// will both return `None`.
    #[inline]
    pub fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    /// Get optional trailers for this stream.
    #[inline]
    pub async fn trailers(&mut self) -> Result<Option<HeaderMap>> {
        self.body.trailers().await
    }
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Request")?;
        self.head.fmt(f)
    }
}

impl std::ops::Deref for Request {
    type Target = http::request::Parts;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.head
    }
}

impl std::ops::DerefMut for Request {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.head
    }
}
