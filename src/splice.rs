use tokio::io::{self, AsyncWriteExt};

pub trait AsyncReadable {
    fn try_read(&self, buf: &mut [u8]) -> io::Result<usize>;
    async fn readable(&self) -> io::Result<()>;
}

async fn pump(
    input: &mut impl AsyncReadable,
    output: &mut (impl AsyncWriteExt + Unpin),
    buf: &mut [u8],
) -> io::Result<()> {
    match input.try_read(buf) {
        Ok(0) => Err(io::Error::new(io::ErrorKind::Other, "connection closed")),
        Ok(n) => {
            output.write_all(&buf[0..n]).await?;
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(()),
        Err(e) => Err(e),
    }
}

pub async fn splice(
    pipe: &mut (impl AsyncReadable + AsyncWriteExt + Unpin),
    socket: &mut (impl AsyncReadable + AsyncWriteExt + Unpin),
) -> io::Result<()> {
    let mut buf = vec![0; 2 * 1024];
    loop {
        tokio::select! {
            Ok(_) = pipe.readable() =>
                pump(pipe, socket, &mut buf).await?,
            Ok(_) = socket.readable() =>
                pump(socket, pipe, &mut buf).await?,

        }
    }
}
