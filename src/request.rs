use super::*;
use h2::RecvStream;

#[derive(Debug)]
pub struct Request {
    pub(crate) head: http::request::Parts,
    pub(crate) body: RecvStream,
}

impl Request {
    #[inline]
    pub fn data(&mut self) -> Data {
        Data(&mut self.body)
    }

    #[inline]
    pub fn stream_id(&self) -> h2::StreamId {
        self.body.stream_id()
    }

    #[inline]
    pub fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    #[inline]
    pub fn trailers(&mut self) -> impl Future<Output = Result<Option<http::HeaderMap>>> + '_ {
        self.body.trailers()
    }
}

pub struct Data<'a>(&'a mut RecvStream);

impl Future for Data<'_> {
    type Output = Option<Result<bytes::Bytes>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_data(cx).map(|out| match out {
            Some(Ok(data)) => {
                let _ = self.0.flow_control().release_capacity(data.len());
                Some(Ok(data))
            }
            v => v,
        })
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
