//! Settings, read once at startup from the environment into a typed [`AppConfig`].

mod app_config;
pub mod env_file;

pub use app_config::{
    AppConfig, AppEnv, AuthConfig, ConfigError, DatabaseConfig, EmailConfig, PaddlePrices,
    SmtpSecurity, StorageConfig, UPLOADS_PER_ADDRESS_FACTOR,
};
