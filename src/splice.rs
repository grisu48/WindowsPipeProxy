use tokio::io::{self, AsyncWriteExt};

pub trait AsyncReadable {
    fn try_read(&self, buf: &mut [u8]) -> io::Result<usize>;
    async fn readable(&self) -> io::Result<()>;
}

// Read from input and write to the output using the given buffer
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

// Read from input and write to the two outputs using the given buffer
async fn pump2(
    input: &mut impl AsyncReadable,
    output1: &mut (impl AsyncWriteExt + Unpin),
    output2: &mut (impl AsyncWriteExt + Unpin),
    buf: &mut [u8],
) -> io::Result<()> {
    match input.try_read(buf) {
        Ok(0) => Err(io::Error::new(io::ErrorKind::Other, "connection closed")),
        Ok(n) => {
            output1.write_all(&buf[0..n]).await?;
            output2.write_all(&buf[0..n]).await?;
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(()),
        Err(e) => Err(e),
    }
}

// Splice and input to and output, i.e. transfer all bytes from the source to the sink
pub async fn splice(
    source: &mut (impl AsyncReadable + Unpin),
    sink: &mut (impl AsyncWriteExt + Unpin),
) -> io::Result<()> {
    let mut buf = vec![0; 2 * 1024];
    loop {
        tokio::select! {
            Ok(_) = source.readable() =>
                pump(source, sink, &mut buf).await?,

        }
    }
}

// Splice and input to and outputs, i.e. transfer all bytes from the source to the two given sinks
pub async fn splice2(
    source: &mut (impl AsyncReadable + Unpin),
    sink1: &mut (impl AsyncWriteExt + Unpin),
    sink2: &mut (impl AsyncWriteExt + Unpin),
) -> io::Result<()> {
    let mut buf = vec![0; 2 * 1024];
    loop {
        tokio::select! {
            Ok(_) = source.readable() =>
                pump2(source, sink1, sink2, &mut buf).await?,

        }
    }
}
