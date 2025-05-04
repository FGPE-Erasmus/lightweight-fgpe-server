use clap::Parser;
use std::net::SocketAddr;
use url::Url;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Database connection string (e.g., "postgres://user:password@host:port/database")
    /// Can also be set using the DATABASE_URL environment variable.
    #[arg(long, env = "DATABASE_URL")]
    pub connection_str: String,

    /// Database connection pool size
    /// Can also be set using the DB_POOL_MAX_SIZE environment variable.
    /// Default value: 10
    #[arg(long, env = "DB_POOL_MAX_SIZE", default_value = "10")]
    pub db_pool_max_size: u32,

    /// Server listen address and port (e.g., "127.0.0.1:3000")
    /// Can also be set using the SERVER_ADDRESS environment variable.
    /// Default value: 127.0.0.1:3000
    #[arg(long, env = "SERVER_ADDRESS", default_value = "127.0.0.1:3000")]
    pub server_address: SocketAddr,

    /// Keycloak server address and port (e.g., "127.0.0.1:8080")
    /// Can also be set using the KEYCLOAK_SERVER_URL environment variable.
    /// Default value: 127.0.0.1:8080
    #[arg(long, env = "KEYCLOAK_SERVER_URL", default_value = "http://127.0.0.1:8080")]
    pub keycloak_server_url: Url,

    /// Keycloak realm name
    /// Can also be set using the KEYCLOAK_REALM environment variable.
    /// Default value: fgpe
    #[arg(long, env = "KEYCLOAK_REALM", default_value = "fgpe")]
    pub keycloak_realm: String,

    /// Keycloak allowed audiences (e.g., "account")
    /// Can also be set using the KEYCLOAK_AUDIENCES environment variable.
    /// Default value: account
    #[arg(long, env = "KEYCLOAK_AUDIENCES", default_value = "account")]
    pub keycloak_audiences: String,

    /// Log level (e.g., "info")
    /// Can also be set using the RUST_LOG environment variable.
    /// Default value: info
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    pub log_level: String,
}
