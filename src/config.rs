use anyhow::Result;
use serde::Deserialize;
use std::{env, fs};

#[derive(Deserialize, Clone)]
pub struct Config {
    #[serde(rename = "Pipe")]
    pub pipes: Vec<Pipe>, // Collection of pipe definitions we should act on

    #[serde(rename = "Plumber")]
    #[serde(default)]
    pub plumber: Plumber, // Named pipe configuration

    #[serde(rename = "Verbose")]
    #[serde(default)]
    pub verbose: bool, // Verbose output
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

// Handler for program arguments
pub struct Args {
    pub config_file: String, // Configuration file
    args: Vec<String>,       // All collected arguments
}

impl Config {
    // Parse program arguments
    pub fn parse_file(filename: &str) -> Result<Config> {
        let contents = fs::read_to_string(filename)?;
        let config: Config = toml::from_str(contents.as_str())?;
        Ok(config)
    }

    // Apply the given program arguments
    pub fn apply_args(&mut self, args: &Args) {
        if args.has_flag("-v") {
            self.verbose = true;
        }
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

impl Args {
    // Create a new Args instance from the given program arguments
    pub fn create() -> Result<Args> {
        let mut args = Args {
            config_file: "pipe-proxy.toml".to_string(),
            args: collect_args(),
        };
        args.parse()?;
        return Ok(args);
    }

    // Create a new empty Args instance (mostly for testing)
    fn new() -> Args {
        Args {
            config_file: "".to_string(),
            args: Vec::new(),
        }
    }

    // Checks if the given flag is present. flag must be passed with the leading dashes, e.g. `-v`
    pub fn has_flag(&self, flag: &str) -> bool {
        self.args.contains(&flag.to_string())
    }

    pub fn parse(&mut self) -> Result<()> {
        for arg in self.args.iter() {
            if arg.starts_with("-") {
                // flag
            } else {
                self.config_file = arg.clone();
            }
        }
        Ok(())
    }
}

fn collect_args() -> Vec<String> {
    let mut args = env::args_os();
    args.next().expect("program name expected");
    args.map(|x| x.into_string().expect("parameter string conversion failed"))
        .collect()
}

fn default_reconnect_attempts() -> i32 {
    10
}

fn default_reconnect_delay() -> u64 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args() {
        let mut args = Args::new();
        // Test if the -v flag gets passed and if the custom configuration file is set.
        let flag_v = "-v".to_string();
        let config_file = "config.toml".to_string();
        assert!(!args.has_flag(&flag_v));
        assert!(args.config_file.is_empty());
        args.args = vec![flag_v.clone(), config_file.clone()];
        args.parse().expect("args.parse failed");
        assert!(args.has_flag(&flag_v));
        assert!(!args.config_file.is_empty());
        assert_eq!(args.config_file, config_file);
    }
}
