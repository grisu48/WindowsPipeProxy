mod config;
mod named_pipe;
mod splice;

use std::time::Duration;
use tokio::{
    fs::try_exists,
    net::{TcpListener, TcpStream},
    task,
    time::sleep,
};

use crate::named_pipe::NamedPipe;

impl splice::AsyncReadable for TcpStream {
    fn try_read(&self, buf: &mut [u8]) -> tokio::io::Result<usize> {
        Self::try_read(&self, buf)
    }

    async fn readable(&self) -> tokio::io::Result<()> {
        Self::readable(&self).await
    }
}

const ERR_RUNTIME: i32 = 100; // General runtime error
const ERR_NO_CONFIG: i32 = 101; // Configuration file doesn't exist
const ERR_CONFIG_INVAL: i32 = 102; // Configuration (file) is invalid
const ERR_SOCKET_ERROR: i32 = 103; // Socket errot

#[tokio::main]
async fn main() {
    let cf_filename = "C:\\pipe-proxy.toml";
    if try_exists(cf_filename)
        .await
        .expect("error checking configuration file")
        == false
    {
        eprintln!("configuration file {cf_filename} doesn't exist. Cannot start");
        std::process::exit(ERR_NO_CONFIG);
    }

    let cf = match config::Config::parse_file(cf_filename) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("configuration error: {err}");
            std::process::exit(ERR_CONFIG_INVAL);
        }
    };

    eprintln!("Windows Pipe Proxy - version 0.3");

    let mut tasks = Vec::new();
    for pipe in cf.clone().pipes {
        let listener = match TcpListener::bind(pipe.addr.as_str()).await {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("error binding 'tcp socket to {}: {err}", pipe.addr);
                std::process::exit(ERR_SOCKET_ERROR);
            }
        };

        let cf = cf.clone();
        let task = task::spawn(worker_loop(pipe.src.clone(), listener, cf));
        tasks.push(task);
    }
    for task in tasks {
        if let Err(err) = task.await {
            eprintln!("task error: {err}");
            std::process::exit(ERR_RUNTIME);
        }
    }
}

async fn worker_loop(named_pipe: String, listener: TcpListener, cf: config::Config) {
    let address = match listener.local_addr() {
        Ok(addr) => addr.to_string(),
        Err(_) => "???".to_string(),
    };

    println!("{named_pipe} => {address} is listening");

    loop {
        let mut pipe = match NamedPipe::new(named_pipe.as_str()) {
            Ok(pipe) => pipe,
            Err(err) => {
                // TODO: Better error matching, this works for now but can be improved.
                if !err
                    .to_string()
                    .contains("The system cannot find the file specified.")
                {
                    eprintln!("pipe {named_pipe} error: {err}");
                }
                sleep(Duration::from_millis(500)).await;
                continue;
            }
        };
        let (mut socket, addr) = match listener.accept().await {
            Ok((sock, addr)) => (sock, addr),
            Err(err) => {
                eprintln!("error accepting client for {named_pipe}: {err}");
                continue;
            }
        };
        println!("Connected: {} <==> {}", pipe.path(), addr.to_string());
        loop {
            match splice::splice(&mut pipe, &mut socket).await {
                Ok(_) => {
                    eprintln!("Unexpected EOF for {named_pipe}");
                    break;
                }
                Err(e) => {
                    // Allow client to reconnect
                    if e.kind() == std::io::ErrorKind::ConnectionAborted
                        || e.to_string() == "connection closed"
                    {
                        eprintln!("{named_pipe}: client disconnected ({})", e.to_string());
                    }
                    // Reconnect named pipe on pipe errors (e.g. when the VM reboots)
                    else if let Err(_) = pipe.info() {
                        if let Err(err) = pipe.reconnect(
                            cf.plumber.reconnect_attempts,
                            cf.plumber.reconnect_delay,
                            false,
                        ) {
                            eprintln!("{named_pipe}: pipe error: {err}");
                        } else {
                            // Pipe is reconnected, let's continue
                            continue;
                        }
                    } else {
                        eprintln!("{named_pipe}: splice error: {e} ({})", e.kind());
                    }
                    break;
                }
            };
        }
        eprintln!("{} closed", pipe.path())
    }
}
