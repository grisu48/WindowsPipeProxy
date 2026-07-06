use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use tokio::io::AsyncWrite;
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::net::windows::named_pipe::NamedPipeClient;
use tokio::net::windows::named_pipe::PipeInfo;

use crate::splice;

pub struct NamedPipe {
    path: String,
    pipe: NamedPipeClient,
}

impl NamedPipe {
    pub fn new(path: &str) -> Result<NamedPipe> {
        let options = ClientOptions::new();
        let pipe = match options.open(path) {
            Ok(pipe) => pipe,
            Err(err) => {
                bail!("{err}");
            }
        };

        let instance = NamedPipe {
            path: path.to_string(),
            pipe: pipe,
        };
        Ok(instance)
    }

    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn info(&self) -> std::io::Result<PipeInfo> {
        self.pipe.info()
    }

    // Attempt to reconnect the named pipe. attempts defines the number of reconnection attempts with 1 second delay between then.
    pub fn reconnect(&mut self, attempts: i32) -> Result<()> {
        let options = ClientOptions::new();
        for attempt in 1..attempts {
            match options.open(self.path()) {
                Ok(pipe) => {
                    self.pipe = pipe;
                    return Ok(());
                }
                Err(err) => {
                    eprintln!("reconnection attempt {attempt}/{attempts}: {err}");
                }
            };
            sleep(Duration::from_secs(1));
        }
        bail!("timeout");
    }
}

impl splice::AsyncReadable for NamedPipe {
    fn try_read(&self, buf: &mut [u8]) -> tokio::io::Result<usize> {
        self.pipe.try_read(buf)
    }

    async fn readable(&self) -> tokio::io::Result<()> {
        self.pipe.readable().await
    }
}

impl AsyncWrite for NamedPipe {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        Poll::Ready(self.pipe.try_write(buf))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        panic!("poll_flush is not needed");
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        panic!("poll_shutdown is not needed");
    }
}
