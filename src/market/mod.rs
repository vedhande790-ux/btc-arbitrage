//! Market data abstraction and feed management.

pub mod book;
pub mod binance;
pub mod kraken;
pub mod normalization;

use crate::error::Result;
use crate::market::book::L2OrderBook;
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub enum MarketEvent {
    OrderBookUpdate(L2OrderBook),
    #[allow(dead_code)]
    Trade(TradeUpdate),
    #[allow(dead_code)]
    Heartbeat(String), // Exchange name
    #[allow(dead_code)]
    Stale(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct TradeUpdate {
    pub exchange: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub is_buyer_maker: bool,
    pub timestamp: i64,
}

#[async_trait]
pub trait Exchange: Send + Sync {
    fn name(&self) -> &str;
    async fn subscribe_order_book(&self, symbol: &str) -> Result<broadcast::Receiver<MarketEvent>>;
    #[allow(dead_code)]
    async fn subscribe_trades(&self, symbol: &str) -> Result<broadcast::Receiver<MarketEvent>>;
    #[allow(dead_code)]
    fn is_healthy(&self) -> bool;
}

#[allow(dead_code)]
pub type SharedOrderBook = Arc<parking_lot::RwLock<L2OrderBook>>;
