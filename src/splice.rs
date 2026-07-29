use tokio::{
    io::{self, AsyncWriteExt, BufWriter},
    time::{self},
};

// Default buffer capacity for a write sync
const BUFWRITER_CAP: usize = 8192;

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
    bufsize: usize,
) -> io::Result<()> {
    let mut buf = vec![0; bufsize];
    // Wrap output in BufWriter to avoid that a slow network connection would congest the input stream on peaks
    let mut sink = BufWriter::with_capacity(BUFWRITER_CAP, sink);
    let mut ticker = time::interval(time::Duration::from_secs(1));
    loop {
        tokio::select! {
            Ok(_) = source.readable() =>
                pump(source, &mut sink, &mut buf).await?,
            _ = &mut Box::pin(ticker.tick()) => sink.flush().await?,
        }
    }
}

// Splice and input to and outputs, i.e. transfer all bytes from the source to the two given sinks
pub async fn splice2(
    source: &mut (impl AsyncReadable + Unpin),
    sink1: &mut (impl AsyncWriteExt + Unpin),
    sink2: &mut (impl AsyncWriteExt + Unpin),
    bufsize: usize,
) -> io::Result<()> {
    let mut buf = vec![0; bufsize];
    // Wrap output in BufWriter to avoid that a slow network connection would congest the input stream on peaks
    let mut sink1 = BufWriter::with_capacity(BUFWRITER_CAP, sink1);
    let mut sink2 = BufWriter::with_capacity(BUFWRITER_CAP, sink2);
    let mut ticker = time::interval(time::Duration::from_secs(1));
    loop {
        tokio::select! {
            Ok(_) = source.readable() =>
                pump2(source, &mut sink1, &mut sink2, &mut buf).await?,
            _ = &mut Box::pin(ticker.tick()) => {
                sink1.flush().await?;
                sink2.flush().await?;
            }
        }
    }
}
