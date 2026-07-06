use anyhow::Result;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "Pipe")]
    pub pipes: Vec<Pipe>, // Collection of pipe definitions we should act on

    #[serde(rename = "Plumber")]
    #[serde(default)]
    pub plumber: Plumber, // Named pipe configuration
}

#[derive(Deserialize, Clone)]
pub struct Pipe {
    #[serde(rename = "Pipe")]
    pub src: String, // Path to named pipe
    #[serde(rename = "Address")]
    pub addr: String, // Local server address
}

// Configuration for the named pipe
#[derive(Deserialize, Clone)]
pub struct Plumber {
    #[serde(rename = "ReconnectAttempts")]
    #[serde(default = "default_reconnect_attempts")]
    pub reconnect_attempts: i32,
    #[serde(rename = "ReconnectDelay")]
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay: u64,
}

impl Config {
    // Parse program arguments
    pub fn parse_file(filename: &str) -> Result<Config> {
        let contents = fs::read_to_string(filename)?;
        let config: Config = toml::from_str(contents.as_str())?;
        Ok(config)
    }
}

impl Default for Plumber {
    fn default() -> Self {
        Self {
            reconnect_attempts: default_reconnect_attempts(),
            reconnect_delay: default_reconnect_delay(),
        }
    }
}

fn default_reconnect_attempts() -> i32 {
    10
}

fn default_reconnect_delay() -> u64 {
    1
}
