//! The HTTPS client providers share (object storage now; Paddle and the AI helpers later):
//! rustls on `ring` with Mozilla's root certificates, as sqlx and lettre already build it, so
//! no second crypto library (reqwest's default, aws-lc-rs, is a C build).

use std::sync::Arc;
use std::time::Duration;

use crate::error::{AppError, ErrorCode};

/// How long a provider call may take.
#[derive(Debug, Clone, Copy)]
pub struct Timeouts {
    /// To open the connection (TCP and TLS).
    pub connect: Duration,
    /// For the whole exchange, body included.
    pub total: Duration,
}

/// A client speaking HTTP/1.1 over TLS 1.2 or 1.3. It never follows redirects (a signed
/// request must not go elsewhere) and ignores proxy settings in the environment.
pub fn client(timeouts: Timeouts) -> Result<reqwest::Client, AppError> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let mut tls = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(internal)?
        .with_root_certificates(roots)
        .with_no_client_auth();
    tls.alpn_protocols = vec![b"http/1.1".to_vec()];
    reqwest::Client::builder()
        .tls_backend_preconfigured(tls)
        .connect_timeout(timeouts.connect)
        .timeout(timeouts.total)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(internal)
}

fn internal(error: impl std::error::Error + Send + Sync + 'static) -> AppError {
    AppError::new(ErrorCode::Internal).with_source(error)
}
