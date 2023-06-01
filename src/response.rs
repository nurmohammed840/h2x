use super::*;
use h2::server::SendResponse;

/// Represents an HTTP response object.
#[derive(Debug)]
pub struct Response {
    /// Represens status code of HTTP response.
    pub status: http::StatusCode,
    /// Represens headers of HTTP response.
    pub headers: http::HeaderMap,
    /// Responsible for sending the HTTP response body
    #[doc(hidden)]
    pub sender: SendResponse<Bytes>,
}

impl Response {
    /// Returns the stream ID of the response stream.
    ///
    /// # Panics
    ///
    /// If the lock on the stream store has been poisoned.
    #[inline]
    pub fn stream_id(&self) -> h2::StreamId {
        self.sender.stream_id()
    }

    fn create_response(mut self, end: bool) -> Result<h2::SendStream<Bytes>> {
        let mut response = http::Response::new(());
        *response.status_mut() = self.status;
        *response.headers_mut() = self.headers;
        self.sender.send_response(response, end)
    }

    /// Send the response headers.
    #[inline]
    pub fn send_headers(self) -> Result<()> {
        self.create_response(true)?;
        Ok(())
    }

    /// This method is used to obtain a [Responder] that can be used to send the response body.
    #[inline]
    pub fn send_stream(self) -> Result<Responder> {
        let inner = self.create_response(false)?;
        Ok(Responder { inner })
    }

    /// Sends response data to the remote peer.
    #[inline]
    pub async fn write(self, bytes: impl Into<Bytes>) -> Result<()> {
        self.send_stream()?.end_write(bytes).await
    }

    /// The data is buffered and the capacity is implicitly requested. Once the
    /// capacity becomes available, the data is flushed to the connection.
    ///
    /// However, this buffering is unbounded. As such, sending large amounts of
    /// data without reserving capacity before hand could result in large
    /// amounts of data being buffered in memory.
    #[inline]
    pub fn write_unbound(self, bytes: impl Into<Bytes>) -> Result<()> {
        self.send_stream()?.end_write_unbound(bytes)
    }
}

/// The [Responder] struct created from `Response::send_stream`
///
/// It is responsible for sending the HTTP response body.
pub struct Responder {
    #[doc(hidden)]
    pub inner: h2::SendStream<Bytes>,
}

impl Responder {
    #[doc(hidden)]
    pub async fn write_bytes(&mut self, mut bytes: Bytes, end: bool) -> Result<()> {
        loop {
            let len = bytes.len();
            self.inner.reserve_capacity(len);
            match poll_fn(|cx| self.inner.poll_capacity(cx)).await {
                None => return Err(h2::Error::from(h2::Reason::CANCEL)),
                Some(nbytes) => {
                    let nbytes = nbytes?;
                    if len <= nbytes {
                        return self.inner.send_data(bytes, end);
                    }
                    self.inner.send_data(bytes.split_to(nbytes), false)?;
                }
            };
        }
    }

    /// Sends a single data frame to the remote peer.
    pub async fn write(&mut self, bytes: impl Into<Bytes>) -> Result<()> {
        self.write_bytes(bytes.into(), false).await
    }

    /// Sends final chunk of data to the remote peer.
    pub async fn end_write(mut self, bytes: impl Into<Bytes>) -> Result<()> {
        self.write_bytes(bytes.into(), true).await
    }

    /// The data is buffered and the capacity is implicitly requested. Once the
    /// capacity becomes available, the data is flushed to the connection.
    ///
    /// However, this buffering is unbounded. As such, sending large amounts of
    /// data without reserving capacity before hand could result in large
    /// amounts of data being buffered in memory.
    pub fn write_unbound(&mut self, bytes: impl Into<Bytes>) -> Result<()> {
        self.inner.send_data(bytes.into(), false)
    }

    /// Sends final chunk of data to the remote peer.
    pub fn end_write_unbound(mut self, bytes: impl Into<Bytes>) -> Result<()> {
        self.inner.send_data(bytes.into(), true)
    }

    /// Signals the end of writing the response body.
    #[inline]
    pub fn end(mut self) -> Result<()> {
        self.inner.send_data(Bytes::new(), true)
    }
}
