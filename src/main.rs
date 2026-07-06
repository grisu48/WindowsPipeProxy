mod named_pipe;
mod splice;

use std::{fs, time::Duration};

use anyhow::Result;
use serde::Deserialize;
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

#[derive(Deserialize)]
struct Config {
    #[serde(rename = "Pipe")]
    pipes: Vec<Pipe>, // Collection of pipe definitions we should act on
}

#[derive(Deserialize)]
struct Pipe {
    #[serde(rename = "Pipe")]
    src: String, // Path to named pipe
    #[serde(rename = "Address")]
    addr: String, // Local server address
}

impl Config {
    // Parse program arguments
    pub fn parse_file(filename: &str) -> Result<Config> {
        let contents = fs::read_to_string(filename)?;
        let config: Config = toml::from_str(contents.as_str())?;
        Ok(config)
    }
}

#[tokio::main]
async fn main() {
    let cf_filename = "C:\\pipe-proxy.toml";
    if try_exists(cf_filename).await.expect("error checking configuration file") == false {
        eprintln!("configuration file {cf_filename} doesn't exist. Cannot start");
        std::process::exit(ERR_NO_CONFIG);
    }

    let cf = match Config::parse_file(cf_filename) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("configuration error: {err}");
            std::process::exit(ERR_CONFIG_INVAL);
        }
    };

    eprintln!("Windows Pipe Proxy v0.2");

    let mut tasks = Vec::new();
    for pipe in cf.pipes {
        let listener = match TcpListener::bind(pipe.addr.as_str()).await {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("error binding 'tcp socket to {}: {err}", pipe.addr);
                std::process::exit(ERR_SOCKET_ERROR);
            }
        };

        let task = task::spawn(worker_loop(pipe.src.clone(), listener));
        tasks.push(task);
    }
    for task in tasks {
        if let Err(err) = task.await {
            eprintln!("task error: {err}");
            std::process::exit(ERR_RUNTIME);
        }
    }
}

async fn worker_loop(named_pipe: String, listener : TcpListener) {
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
                        if let Err(err) = pipe.reconnect(10) {
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
