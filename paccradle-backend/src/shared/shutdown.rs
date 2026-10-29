//! Stopping cleanly: on Ctrl-C or SIGTERM (what `docker stop` sends) the server stops taking
//! requests and the workers stop claiming jobs, and both finish what they are doing.

use tokio::sync::watch;

/// Resolves on Ctrl-C or SIGTERM.
pub async fn signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(%error, "cannot listen for Ctrl-C");
            std::future::pending::<()>().await;
        }
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::warn!(%error, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}

/// Resolves once `stop` turns `true`, or its sender is gone.
pub async fn stopped(mut stop: watch::Receiver<bool>) {
    // An Err means the sender was dropped, which also means stop.
    let _ = stop.wait_for(|stopping| *stopping).await;
}
