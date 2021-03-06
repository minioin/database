use async_std::fs::File;
use async_std::io::{self, Read, SeekFrom, Write};
use async_std::sync::Arc;
use async_std::task::{Context, Poll};
use bytes::BytesMut;
use std::pin::Pin;
use std::sync::Mutex;
use tempfile::NamedTempFile;

fn empty_file_named() -> NamedTempFile {
    NamedTempFile::new().expect("Failed to create tempfile")
}

fn empty_file() -> File {
    empty_file_named().reopen().expect("empty file").into()
}

fn file_with(content: Vec<&[u8]>) -> File {
    use std::io::{Seek, Write};

    let named_temp_file = empty_file_named();
    let mut file = named_temp_file.reopen().expect("file with content");
    for bytes in content {
        file.write(bytes).unwrap();
    }
    file.seek(SeekFrom::Start(0))
        .expect("set position at the beginning of a file");
    named_temp_file.reopen().expect("reopen file").into()
}

#[derive(Clone, Debug)]
pub struct TestCase {
    request: Arc<File>,
    response: Arc<Mutex<File>>,
}

impl TestCase {
    pub async fn empty() -> Self {
        Self::new(empty_file()).await
    }

    pub async fn with_content(content: Vec<&[u8]>) -> Self {
        Self::new(file_with(content)).await
    }

    pub async fn new(request: File) -> TestCase {
        let temp = tempfile::tempfile().expect("Failed to create tempfile");
        let result = Arc::new(Mutex::new(temp.into()));

        TestCase {
            request: Arc::new(request),
            response: result,
        }
    }

    pub async fn read_result(&self) -> BytesMut {
        use async_std::prelude::*;

        let mut result = Vec::new();
        let mut file = self.response.lock().unwrap();
        file.seek(SeekFrom::Start(0)).await.unwrap();
        file.read_to_end(&mut result).await.unwrap();

        BytesMut::from(result.as_slice())
    }
}

impl Read for TestCase {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self.request).poll_read(cx, buf)
    }
}

impl Write for TestCase {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self.response.lock().unwrap()).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self.response.lock().unwrap()).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self.response.lock().unwrap()).poll_close(cx)
    }
}
