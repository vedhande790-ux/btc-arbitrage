//! Quantum Arbitrage Engine v5.0.0 — Production Orchestrator
//! 
//! Fault-tolerant system utilizing direct exchange feeds, L2 book depth analysis,
//! fixed-point financial precision, and task supervision.

pub mod config;
pub mod error;
pub mod market;
pub mod engine;
#[path = "risk/mod.rs"]
pub mod risk;
pub mod storage;
pub mod dashboard;

use crate::market::{MarketEvent, Exchange};
use crate::market::binance::BinanceExchange;
use crate::market::kraken::KrakenExchange;
use crate::market::normalization::NormalizationEngine;
use crate::engine::scanner::ArbitrageScanner;
use crate::storage::AsyncStorage;
use crate::dashboard::{AppState, run_server};
use crate::error::Result;

use rust_decimal_macros::dec;
use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    // 0. Initialize crypto provider (Required for rustls 0.23+)
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // 1. Initialize logging
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    info!("Starting Quantum Arbitrage Engine v5.0.0 (Production Hardened)");

    // 2. Setup high-performance async storage
    let storage = Arc::new(AsyncStorage::new("sqlite://arbitrage.db").await?);

    // 3. Initialize Shared App State for Dashboard
    let app_state = Arc::new(AppState {
        opportunities: parking_lot::RwLock::new(VecDeque::with_capacity(100)),
        books: dashmap::DashMap::new(),
        scan_id: std::sync::atomic::AtomicU64::new(0),
    });

    // 4. Initialize Domain Engines
    let norm_engine = Arc::new(NormalizationEngine::new());
    let scanner = Arc::new(ArbitrageScanner::new(dec!(0.1), Arc::clone(&norm_engine)));
    
    // 5. Global Event Distribution Channel
    let (global_tx, mut global_rx) = broadcast::channel(2048);

    // 6. Spawn Domain Scanning Task
    let _scanner_task = {
        let scanner = Arc::clone(&scanner);
        let storage = Arc::clone(&storage);
        let app_state = Arc::clone(&app_state);
        
        tokio::spawn(async move {
            info!("Arbitrage detection engine active");
            while let Ok(event) = global_rx.recv().await {
                match event {
                    MarketEvent::OrderBookUpdate(book) => {
                        // Update dashboard state
                        app_state.books.insert(book.exchange.clone(), book.clone());
                        app_state.scan_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        
                        // Run detection
                        if let Some(opp) = scanner.update_book(book) {
                            info!(
                                "OPPORTUNITY: {} -> {} | Net: {}% | Profit: ${}",
                                opp.buy_exchange, opp.sell_exchange, opp.net_pct, opp.net_profit_usd
                            );
                            
                            // Update dashboard history
                            let mut opps = app_state.opportunities.write();
                            if opps.len() >= 100 { opps.pop_back(); }
                            opps.push_front(opp.clone());
                            
                            storage.log_opportunity(opp);
                        }
                    }
                    _ => {}
                }
            }
        })
    };

    // 7. Start Dashboard Server
    let dashboard_state = Arc::clone(&app_state);
    tokio::spawn(async move {
        run_server(dashboard_state).await;
    });

    // 8. Supervised Exchange Feeds
    let exchanges: Vec<(Arc<dyn Exchange>, String)> = vec![
        (Arc::new(BinanceExchange::new()), "BTCUSDT".to_string()),
        (Arc::new(KrakenExchange::new()), "BTC/USD".to_string()),
    ];

    for (ex, symbol) in exchanges {
        let tx = global_tx.clone();
        tokio::spawn(async move {
            let mut backoff = Duration::from_secs(1);
            loop {
                info!(exchange = ex.name(), symbol = %symbol, "Starting exchange feed");
                match ex.subscribe_order_book(&symbol).await {
                    Ok(mut rx) => {
                        backoff = Duration::from_secs(1);
                        while let Ok(event) = rx.recv().await {
                            let _ = tx.send(event);
                        }
                    }
                    Err(e) => error!(exchange = ex.name(), error = %e, "Feed failed"),
                }
                warn!(exchange = ex.name(), "Restarting feed in {:?}...", backoff);
                sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(60));
            }
        });
    }

    // Keep the main thread alive
    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received. Cleaning up...");

    Ok(())
}