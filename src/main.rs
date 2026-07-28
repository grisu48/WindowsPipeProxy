mod config;
mod named_pipe;
mod splice;

use std::time::Duration;
use tokio::{
    fs::{File, OpenOptions, try_exists},
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    task,
    time::sleep,
};

use crate::{
    config::{Args, Pipe},
    named_pipe::NamedPipe,
};

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
    let args = match Args::create() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("argument error: {err}");
            std::process::exit(ERR_CONFIG_INVAL);
        }
    };
    let cf_filename = args.config_file.as_str();
    if try_exists(cf_filename)
        .await
        .expect("error checking configuration file")
        == false
    {
        eprintln!("configuration file {cf_filename} doesn't exist. Cannot start");
        std::process::exit(ERR_NO_CONFIG);
    }

    let mut cf = match config::Config::parse_file(cf_filename) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("configuration error: {err}");
            std::process::exit(ERR_CONFIG_INVAL);
        }
    };
    cf.apply_args(&args);

    eprintln!("Windows Pipe Proxy - version 0.3");
    if cf.verbose {
        eprintln!("verbose mode on");
    }

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
        let task = task::spawn(worker_loop(pipe, listener, cf));
        tasks.push(task);
    }
    for task in tasks {
        if let Err(err) = task.await {
            eprintln!("task error: {err}");
            std::process::exit(ERR_RUNTIME);
        }
    }
}

async fn worker_loop(named_pipe: Pipe, listener: TcpListener, cf: config::Config) {
    let address = match listener.local_addr() {
        Ok(addr) => addr.to_string(),
        Err(_) => "???".to_string(),
    };

    if cf.verbose {
        println!("{} => {address} is listening", named_pipe.src);
    }

    loop {
        let mut pipe = match NamedPipe::new(named_pipe.src.as_str()) {
            Ok(pipe) => pipe,
            Err(err) => {
                // TODO: Better error matching, this works for now but can be improved.
                let msg = err.to_string();
                let mut print = true;
                // this error happens on every attempt and is very noisy. We want to hide it unconditionally.
                if msg.contains("The system cannot find the file specified.") {
                    print = false;
                }
                // exclusion list of errors to be printed unless verbose
                if print && !cf.verbose {
                    print = !msg.contains("All pipe instances are busy");
                }
                if print {
                    eprintln!("pipe {} error: {err}", named_pipe.src);
                }
                sleep(Duration::from_millis(500)).await;
                continue;
            }
        };
        let (mut socket, addr) = match listener.accept().await {
            Ok((sock, addr)) => (sock, addr),
            Err(err) => {
                eprintln!("error accepting client for {}: {err}", named_pipe.src);
                continue;
            }
        };
        println!("Connected: {} <==> {}", pipe.path(), addr.to_string());
        loop {
            let ret;
            if named_pipe.file.is_empty() {
                ret = splice::splice(&mut pipe, &mut socket).await;
            } else {
                // Try to open the pipe log file, continue normally if that's not possible
                // We always append to a log file.
                let mut file = OpenOptions::new();
                let file = file
                    .read(false)
                    .append(true)
                    .create(true)
                    .open(named_pipe.file.as_str())
                    .await;
                ret = match file {
                    Ok(mut file) => {
                        let _ = file
                            .write_all(
                                format!("#### Named pipe log for: {} ####\n\n", named_pipe.src)
                                    .as_bytes(),
                            )
                            .await; // swallow error if any
                        splice::splice2(&mut pipe, &mut socket, &mut file).await
                    }
                    Err(err) => {
                        eprintln!("{} error to open pipe log file: {err}", named_pipe.src);
                        // Continue without pipe log file
                        splice::splice(&mut pipe, &mut socket).await
                    }
                };
            }

            match ret {
                Ok(_) => {
                    eprintln!("Unexpected EOF for {}", named_pipe.src);
                    break;
                }
                Err(e) => {
                    // Allow client to reconnect
                    if e.kind() == std::io::ErrorKind::ConnectionAborted
                        || e.to_string() == "connection closed"
                    {
                        eprintln!(
                            "{}: client disconnected ({})",
                            named_pipe.src,
                            e.to_string()
                        );
                    }
                    // Reconnect named pipe on pipe errors (e.g. when the VM reboots)
                    else if let Err(_) = pipe.info() {
                        if let Err(err) = pipe.reconnect(
                            cf.plumber.reconnect_attempts,
                            cf.plumber.reconnect_delay,
                            cf.verbose,
                        ) {
                            eprintln!("{}: pipe error: {err}", named_pipe.src);
                        } else {
                            // Pipe is reconnected, let's continue
                            continue;
                        }
                    } else {
                        eprintln!("{}: splice error: {e} ({})", named_pipe.src, e.kind());
                    }
                    break;
                }
            };
        }
        if cf.verbose {
            eprintln!("{} closed", pipe.path())
        }
    }
}
