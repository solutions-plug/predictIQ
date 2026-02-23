use std::{env, net::SocketAddr};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub redis_url: String,
    pub database_url: String,
    pub blockchain_rpc_url: String,
    pub featured_limit: i64,
    pub content_default_page_size: i64,
}

impl Config {
    pub fn from_env() -> Self {
        let bind_addr = env::var("API_BIND_ADDR")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| "0.0.0.0:8080".parse().expect("valid bind addr"));

        Self {
            bind_addr,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1/predictiq".to_string()),
            blockchain_rpc_url: env::var("BLOCKCHAIN_RPC_URL")
                .unwrap_or_else(|_| "https://soroban-testnet.stellar.org:443".to_string()),
            featured_limit: env::var("FEATURED_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            content_default_page_size: env::var("CONTENT_DEFAULT_PAGE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
        }
    }
}
