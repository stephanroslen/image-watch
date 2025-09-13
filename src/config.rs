use crate::error::Result;
use shellexpand::tilde;
use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Config {
    pub auth_pass_argon2: String,
    pub auth_user: String,
    pub auth_token_cleanup_interval: std::time::Duration,
    pub auth_token_ttl: std::time::Duration,
    pub auth_token_max_per_user: usize,
    pub file_extensions: Vec<String>,
    pub rescrape_interval: std::time::Duration,
    pub serve_dir: PathBuf,
    pub listen_address: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let auth_pass_argon2 = env::var("AUTH_PASS_ARGON2")?;
        let auth_user = env::var("AUTH_USER")?;

        let raw_file_extensions = env::var("FILE_EXTENSIONS").unwrap_or("jpg,jpeg".to_string());
        let file_extensions = raw_file_extensions
            .split(',')
            .map(|s| s.to_string())
            .collect();

        let auth_token_cleanup_interval =
            env::var("AUTH_TOKEN_CLEANUP_INTERVAL_MILLIS").unwrap_or("1000".to_string());
        let auth_token_cleanup_interval =
            std::time::Duration::from_millis(auth_token_cleanup_interval.parse::<u64>()?);

        let auth_token_ttl = env::var("AUTH_TOKEN_TTL_SECS").unwrap_or("3600".to_string());
        let auth_token_ttl = std::time::Duration::from_secs(auth_token_ttl.parse::<u64>()?);

        let auth_token_max_per_user =
            env::var("AUTH_TOKEN_MAX_PER_USER").unwrap_or("16".to_string());
        let auth_token_max_per_user = auth_token_max_per_user.parse::<usize>()?;

        let raw_rescrape_interval =
            env::var("RESCRAPE_INTERVAL_MILLIS").unwrap_or("1000".to_string());
        let rescrape_interval =
            std::time::Duration::from_millis(raw_rescrape_interval.parse::<u64>()?);

        let raw_serve_dir = env::var("SERVE_DIR")?;
        let serve_dir = Path::new(&tilde(&raw_serve_dir).to_string()).to_path_buf();

        let listen_address = env::var("LISTEN_ADDRESS").unwrap_or("127.0.0.1:3000".to_string());

        let config = Self {
            auth_pass_argon2,
            auth_user,
            auth_token_cleanup_interval,
            auth_token_ttl,
            auth_token_max_per_user,
            file_extensions,
            rescrape_interval,
            serve_dir,
            listen_address,
        };

        tracing::debug!("Configuration extraction successful: {:?}", config);

        Ok(config)
    }
}
