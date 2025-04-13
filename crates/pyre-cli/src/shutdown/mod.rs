use tokio::{
    signal::unix::{signal, Signal, SignalKind},
    sync::broadcast,
};
use tracing::info;

pub struct Shutdown {
    signals: Vec<(&'static str, Signal)>,
}

impl Shutdown {
    /// Creates a new `Shutdown` instance with the specified signals.
    /// The default signals are `SIGINT`, `SIGTERM`, `SIGQUIT`, and `SIGHUP`.
    ///
    /// # Panics
    /// This function will panic if the signals handlers cannot be created.
    /// See `tokio::signal::unix::signal` for more details.
    #[must_use]
    pub fn new_with_all_signals() -> Self {
        Self {
            signals: vec![
                ("SIGINT", signal(SignalKind::interrupt()).unwrap()),
                ("SIGTERM", signal(SignalKind::terminate()).unwrap()),
                ("SIGQUIT", signal(SignalKind::quit()).unwrap()),
                ("SIGHUP", signal(SignalKind::hangup()).unwrap()),
            ],
        }
    }

    pub fn install(self) -> broadcast::Sender<()> {
        let handlers = self
            .signals
            .iter()
            .map(|(kind, _)| *kind)
            .collect::<Vec<_>>()
            .join("|");

        info!(%handlers, "installing shutdown handlers");

        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        let shutdown_signal = shutdown_tx.clone();

        for (kind, mut signal) in self.signals {
            let shutdown_signal = shutdown_signal.clone();
            tokio::spawn(async move {
                let mut shutdown_received = false;
                loop {
                    signal.recv().await;

                    if shutdown_received {
                        tracing::warn!(%kind, "forcing shutdown");
                        std::process::exit(1);
                    } else {
                        tracing::info!(%kind, "gracefully shutting down");
                        let _ = shutdown_signal.send(());
                        shutdown_received = true;
                    }
                }
            });
        }

        shutdown_tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown() {
        let shutdown_tx = Shutdown::new_with_all_signals().install();
        let mut receiver = shutdown_tx.subscribe();
        shutdown_tx.send(()).unwrap();
        assert!(receiver.recv().await.is_ok());
    }
}
