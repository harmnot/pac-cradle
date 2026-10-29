//! Log output: JSON lines in production (one object per event, with the request's span and
//! its `request_id`), compact text in development. `RUST_LOG` overrides the level.

use tracing_subscriber::EnvFilter;

use crate::config::AppEnv;

/// Used when `RUST_LOG` is unset.
const DEFAULT_FILTER: &str = "info";

/// Starts logging. Call once, first thing at startup.
pub fn init(env: AppEnv) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    let result = match env {
        AppEnv::Production => builder
            .json()
            .flatten_event(true)
            .with_current_span(true)
            .with_span_list(false)
            .try_init(),
        AppEnv::Development => builder.compact().try_init(),
    };
    if let Err(error) = result {
        eprintln!("paccradle API: logging was already set up: {error}");
    }
}
