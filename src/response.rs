use super::*;
use h2::server::SendResponse;

const EMPTY_DATA: Bytes = Bytes::new();

#[derive(Debug)]
pub struct Response {
    pub status: http::StatusCode,
    pub headers: http::HeaderMap,
    pub(crate) sender: SendResponse<Bytes>,
}

impl Response {
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

    #[inline]
    pub fn send_headers(self) -> Result<()> {
        self.create_response(true)?;
        Ok(())
    }

    #[inline]
    pub fn send_stream(self) -> Result<Sender> {
        let inner = self.create_response(false)?;
        Ok(Sender { inner })
    }

    #[inline]
    pub async fn write(self, data: impl Into<Bytes>) -> Result<()> {
        self.send_stream()?.end_write(data).await
    }
}

pub struct Sender {
    pub inner: h2::SendStream<Bytes>,
}

impl Sender {
    async fn _write(&mut self, mut bytes: Bytes, end: bool) -> Result<()> {
        loop {
            let len = bytes.len();
            self.inner.reserve_capacity(len);
            match std::future::poll_fn(|cx| self.inner.poll_capacity(cx)).await {
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

    pub async fn write(&mut self, bytes: impl Into<Bytes>) -> Result<()> {
        self._write(bytes.into(), false).await
    }

    pub fn write_unbound(&mut self, bytes: impl Into<Bytes>) -> Result<()> {
        self.inner.send_data(bytes.into(), false)
    }

    pub async fn end_write(mut self, bytes: impl Into<Bytes>) -> Result<()> {
        self._write(bytes.into(), true).await
    }

    pub fn end_write_unbound(mut self, bytes: impl Into<Bytes>) -> Result<()> {
        self.inner.send_data(bytes.into(), true)
    }

    #[inline]
    pub fn end(mut self) -> Result<()> {
        self.inner.send_data(EMPTY_DATA, true)
    }
}
