use super::*;
use http::HeaderMap;

/// Represents an HTTP request object. It consists of the request headers and body.
pub struct Request {
    /// Component parts of an HTTP `Request`
    ///
    /// The HTTP request head consists of a method, uri, version, and a set of
    /// header fields.
    pub head: http::request::Parts,

    /// Receives the body stream and trailers from the remote peer
    pub body: RecvStream,
}

/// Receives the body stream and trailers from the remote peer
pub struct RecvStream {
    #[doc(hidden)]
    pub inner: h2::RecvStream,
}

impl RecvStream {
    #[inline]
    /// Retrieve the next chunk of data from the request body.
    pub async fn data(&mut self) -> Option<Result<Bytes>> {
        poll_fn(|cx| self.poll_data(cx)).await
    }

    /// Poll for the next data frame.
    #[inline]
    pub fn poll_data(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes>>> {
        self.inner.poll_data(cx).map(|out| match out {
            Some(Ok(data)) => {
                let data = self
                    .inner
                    .flow_control()
                    .release_capacity(data.len())
                    .map(|_| data);

                Some(data)
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
    pub fn stream_id(&self) -> h2::StreamId {
        self.inner.stream_id()
    }

    /// Returns true if the receive half has reached the end of stream.
    ///
    /// A return value of `true` means that calls to `poll` and `poll_trailers`
    /// will both return `None`.
    #[inline]
    pub fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    /// Get optional trailers for this stream.
    #[inline]
    pub async fn trailers(&mut self) -> Result<Option<HeaderMap>> {
        self.inner.trailers().await
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
