use clap::Parser;

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
    #[clap(long = "insecure-oci-registries", env = "INSECURE_OCI_REGISTRIES", use_value_delimiter = true)]
    pub insecure_oci_registries: Vec<String>,

    #[clap(long = "fs-cache-dir", env = "FS_CACHE_DIR")]
    pub fs_cache_dir: Option<String>,
}
