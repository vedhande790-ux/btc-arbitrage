//! High-performance asynchronous persistence layer.

use crate::engine::ArbOpportunity;
use crate::error::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use tokio::sync::mpsc;
use tracing::{error, info};

pub enum StorageEvent {
    LogOpportunity(ArbOpportunity),
}

pub struct AsyncStorage {
    tx: mpsc::Sender<StorageEvent>,
}

impl AsyncStorage {
    pub async fn new(database_url: &str) -> Result<Self> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::from_str(database_url)?
                .create_if_missing(true)
        )
        .await?;

        // Schema initialization
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS opportunities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                buy_exchange TEXT NOT NULL,
                sell_exchange TEXT NOT NULL,
                symbol TEXT NOT NULL,
                size_btc TEXT NOT NULL,
                buy_price_avg TEXT NOT NULL,
                sell_price_avg TEXT NOT NULL,
                net_profit_usd TEXT NOT NULL,
                net_pct TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            )"
        ).execute(&pool).await?;

        let (tx, rx) = mpsc::channel(1000);
        
        // Spawn persistence worker
        tokio::spawn(async move {
            Self::worker(pool, rx).await;
        });

        Ok(Self { tx })
    }

    pub fn log_opportunity(&self, opp: ArbOpportunity) {
        if let Err(e) = self.tx.try_send(StorageEvent::LogOpportunity(opp)) {
            error!(error = %e, "Storage queue overflowed");
        }
    }

    async fn worker(pool: SqlitePool, mut rx: mpsc::Receiver<StorageEvent>) {
        info!("Storage worker started");
        while let Some(event) = rx.recv().await {
            match event {
                StorageEvent::LogOpportunity(opp) => {
                    let res = sqlx::query(
                        "INSERT INTO opportunities (
                            buy_exchange, sell_exchange, symbol, size_btc, 
                            buy_price_avg, sell_price_avg, net_profit_usd, 
                            net_pct, timestamp
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&opp.buy_exchange)
                    .bind(&opp.sell_exchange)
                    .bind(&opp.symbol)
                    .bind(opp.size_btc.to_string())
                    .bind(opp.buy_price.to_string())
                    .bind(opp.sell_price.to_string())
                    .bind(opp.net_profit_usd.to_string())
                    .bind(opp.net_pct.to_string())
                    .bind(opp.timestamp)
                    .execute(&pool)
                    .await;

                    if let Err(e) = res {
                        error!(error = %e, "Failed to persist opportunity");
                    }
                }
            }
        }
    }
}
