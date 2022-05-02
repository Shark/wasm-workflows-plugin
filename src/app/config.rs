use anyhow::anyhow;
use clap::Parser;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    /// IP address to bind the HTTP server to
    #[clap(short = 'b', long = "bind", default_value_t = String::from("127.0.0.1"))]
    pub bind_ip: String,

    /// Port to open the HTTP server on
    #[clap(short = 'p', long = "port", default_value_t = 3000)]
    pub bind_port: u16,

    /// Comma-separated list of insecure OCI registry hosts
    #[clap(
        long = "insecure-oci-registries",
        env = "INSECURE_OCI_REGISTRIES",
        use_value_delimiter = true
    )]
    pub insecure_oci_registries: Vec<String>,

    #[clap(long = "fs-cache-dir", env = "FS_CACHE_DIR")]
    pub fs_cache_dir: Option<String>,

    #[clap(long = "log-level", env = "LOG_LEVEL")]
    pub log_level: LogLevel,

    #[clap(long = "enable-telemetry", env = "OTEL_ENABLE")]
    pub enable_telemetry: bool,
}

pub enum LogLevel {
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn to_str(&self) -> &str {
        match self {
            LogLevel::Info => "Info",
            LogLevel::Debug => "Debug",
            LogLevel::Trace => "Trace",
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl FromStr for LogLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(anyhow!(format!("Unknown log level '{}'", s))),
        }
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl Debug for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}
