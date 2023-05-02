use super::*;
use bytes::Bytes;
use h2::server::SendResponse;
use std::future::poll_fn;

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
        let mut sender = self.send_stream()?;
        sender.write(data).await?;
        sender.end()
    }
}

pub struct Sender {
    pub inner: h2::SendStream<Bytes>,
}

impl Sender {
    #[inline]
    pub async fn write(&mut self, data: impl Into<Bytes>) -> Result<()> {
        let mut bytes: Bytes = data.into();
        while !bytes.is_empty() {
            let len = bytes.len();
            self.inner.reserve_capacity(len);
            match poll_fn(|cx| self.inner.poll_capacity(cx)).await {
                None => return Err(h2::Error::from(h2::Reason::CANCEL)),
                Some(nbytes) => {
                    let data = bytes.split_to(nbytes?.min(len));
                    self.inner.send_data(data, false)?;
                }
            }
        }
        Ok(())
    }

    #[inline]
    pub fn end(mut self) -> Result<()> {
        self.inner.send_data(EMPTY_DATA, true)?;
        Ok(())
    }
}
