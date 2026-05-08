//! Structured concurrency and task supervision.

use crate::error::Result;
use crate::market::{Exchange, MarketEvent};
use std::sync::Arc;
use tokio::task::JoinSet;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

#[allow(dead_code)]
pub enum TaskStatus {
    Starting,
    Running,
    Crashed(String),
    Restarting,
}

pub struct TaskSupervisor {
    exchanges: Vec<Arc<dyn Exchange>>,
    symbol: String,
}

impl TaskSupervisor {
    pub fn new(symbol: &str) -> Self {
        Self {
            exchanges: Vec::new(),
            symbol: symbol.to_string(),
        }
    }

    pub fn add_exchange(&mut self, exchange: Arc<dyn Exchange>) {
        self.exchanges.push(exchange);
    }

    /// Runs all exchange feeds and monitors their health.
    pub async fn run(&self, global_tx: broadcast::Sender<MarketEvent>) -> Result<()> {
        let mut set = JoinSet::new();

        for ex in &self.exchanges {
            let ex = Arc::clone(ex);
            let tx = global_tx.clone();
            let symbol = self.symbol.clone();

            set.spawn(async move {
                Self::supervise_exchange(ex, symbol, tx).await;
            });
        }

        while let Some(res) = set.join_next().await {
            match res {
                Ok(_) => warn!("Supervisor: A task finished unexpectedly"),
                Err(e) => error!(error = %e, "Supervisor: Task panicked"),
            }
        }

        Ok(())
    }

    async fn supervise_exchange(ex: Arc<dyn Exchange>, symbol: String, global_tx: broadcast::Sender<MarketEvent>) {
        let mut backoff = Duration::from_secs(1);
        
        loop {
            info!(exchange = ex.name(), "Starting supervised feed");
            match ex.subscribe_order_book(&symbol).await {
                Ok(mut rx) => {
                    backoff = Duration::from_secs(1);
                    while let Ok(event) = rx.recv().await {
                        if let Err(e) = global_tx.send(event) {
                            error!(error = %e, "Supervisor: Global broadcast channel closed");
                            return;
                        }
                    }
                }
                Err(e) => {
                    error!(exchange = ex.name(), error = %e, "Feed failed to start");
                }
            }

            warn!(exchange = ex.name(), retry_in = ?backoff, "Restarting exchange feed...");
            sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(60));
        }
    }
}
