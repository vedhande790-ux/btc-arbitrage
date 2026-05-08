//! Kraken WebSocket implementation.

use crate::error::{Error, Result};
use crate::market::{Exchange, MarketEvent};
use crate::market::book::L2OrderBook;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use std::str::FromStr;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::error;

pub struct KrakenExchange {
    name: String,
    base_url: String,
}

impl KrakenExchange {
    pub fn new() -> Self {
        Self {
            name: "Kraken".to_string(),
            base_url: "wss://ws.kraken.com/v2".to_string(),
        }
    }

    async fn handle_websocket(&self, tx: broadcast::Sender<MarketEvent>) {
        let mut backoff = Duration::from_secs(1);

        loop {
            match connect_async(&self.base_url).await {
                Ok((mut ws_stream, _)) => {
                    backoff = Duration::from_secs(1);
                    
                    // Subscribe to book
                    let sub = serde_json::json!({
                        "method": "subscribe",
                        "params": {
                            "channel": "book",
                            "symbol": ["BTC/USD"],
                            "depth": 10
                        }
                    });
                    let _ = ws_stream.send(Message::Text(sub.to_string().into())).await;

                    while let Some(msg) = ws_stream.next().await {
                        if let Ok(Message::Text(text)) = msg {
                            if let Ok(event) = self.parse_message(&text) {
                                let _ = tx.send(event);
                            }
                        }
                    }
                }
                Err(e) => error!(error = %e, "Kraken connection failed"),
            }
            sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(60));
        }
    }

    fn parse_message(&self, text: &str) -> Result<MarketEvent> {
        let v: Value = serde_json::from_str(text)?;
        if v["channel"] == "book" && v["type"] == "update" {
            let data = &v["data"][0];
            let mut book = L2OrderBook::new(&self.name, "BTC/USD");
            
            if let Some(bids) = data["bids"].as_array() {
                for b in bids {
                    let p = Decimal::from_str(b["price"].as_str().unwrap_or("0")).unwrap_or_default();
                    let q = Decimal::from_str(b["qty"].as_str().unwrap_or("0")).unwrap_or_default();
                    book.update_bid(p, q);
                }
            }
            if let Some(asks) = data["asks"].as_array() {
                for a in asks {
                    let p = Decimal::from_str(a["price"].as_str().unwrap_or("0")).unwrap_or_default();
                    let q = Decimal::from_str(a["qty"].as_str().unwrap_or("0")).unwrap_or_default();
                    book.update_ask(p, q);
                }
            }
            return Ok(MarketEvent::OrderBookUpdate(book));
        }
        Err(Error::MalformedResponse("Unhandled Kraken msg".into()))
    }
}

#[async_trait]
impl Exchange for KrakenExchange {
    fn name(&self) -> &str { &self.name }

    async fn subscribe_order_book(&self, _symbol: &str) -> Result<broadcast::Receiver<MarketEvent>> {
        let (tx, rx) = broadcast::channel(128);
        let this = Arc::new(KrakenExchange::new());
        tokio::spawn(async move { this.handle_websocket(tx).await; });
        Ok(rx)
    }

    async fn subscribe_trades(&self, _symbol: &str) -> Result<broadcast::Receiver<MarketEvent>> {
        Err(Error::MalformedResponse("Not implemented".into()))
    }

    fn is_healthy(&self) -> bool { true }
}
