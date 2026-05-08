//! Binance WebSocket implementation for real-time L2 books and trades.

use crate::error::{Error, Result};
use crate::market::{Exchange, MarketEvent, TradeUpdate};
use crate::market::book::L2OrderBook;
use async_trait::async_trait;
use rust_decimal::Decimal;
use std::str::FromStr;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

pub struct BinanceExchange {
    name: String,
    base_url: String,
}

impl BinanceExchange {
    pub fn new() -> Self {
        Self {
            name: "Binance".to_string(),
            base_url: "wss://stream.binance.com:9443/ws".to_string(),
        }
    }

    async fn handle_websocket(
        &self,
        symbol: &str,
        stream_name: &str,
        tx: broadcast::Sender<MarketEvent>,
    ) {
        let url = format!("{}/{}@{}", self.base_url, symbol.to_lowercase(), stream_name);
        let mut backoff = Duration::from_secs(1);

        loop {
            info!(url = %url, "Connecting to Binance WebSocket");
            match connect_async(&url).await {
                Ok((mut ws_stream, _)) => {
                    backoff = Duration::from_secs(1);
                    info!("Binance WebSocket connected");

                    while let Some(msg) = ws_stream.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(event) = self.parse_message(&text, stream_name) {
                                    let _ = tx.send(event);
                                }
                            }
                            Ok(Message::Ping(p)) => {
                                let _ = ws_stream.send(Message::Pong(p)).await;
                            }
                            Ok(Message::Close(_)) => break,
                            Err(e) => {
                                error!(error = %e, "WebSocket error");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to connect to Binance");
                }
            }

            warn!(retry_in = ?backoff, "Reconnecting...");
            sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(60));
        }
    }

    fn parse_message(&self, text: &str, stream_name: &str) -> Result<MarketEvent> {
        let v: Value = serde_json::from_str(text)?;
        
        match stream_name {
            "depth20@100ms" => {
                let mut book = L2OrderBook::new(&self.name, "BTC/USDT");
                if let Some(bids) = v["bids"].as_array() {
                    for b in bids {
                        let price = Decimal::from_str(b[0].as_str().unwrap()).unwrap();
                        let qty = Decimal::from_str(b[1].as_str().unwrap()).unwrap();
                        book.update_bid(price, qty);
                    }
                }
                if let Some(asks) = v["asks"].as_array() {
                    for a in asks {
                        let price = Decimal::from_str(a[0].as_str().unwrap()).unwrap();
                        let qty = Decimal::from_str(a[1].as_str().unwrap()).unwrap();
                        book.update_ask(price, qty);
                    }
                }
                book.local_timestamp = chrono::Utc::now().timestamp_millis();
                Ok(MarketEvent::OrderBookUpdate(book))
            }
            "trade" => {
                Ok(MarketEvent::Trade(TradeUpdate {
                    exchange: self.name.clone(),
                    price: Decimal::from_str(v["p"].as_str().unwrap()).unwrap(),
                    quantity: Decimal::from_str(v["q"].as_str().unwrap()).unwrap(),
                    is_buyer_maker: v["m"].as_bool().unwrap(),
                    timestamp: v["T"].as_i64().unwrap(),
                }))
            }
            _ => Err(Error::MalformedResponse("Unknown Binance stream".into())),
        }
    }
}

#[async_trait]
impl Exchange for BinanceExchange {
    fn name(&self) -> &str {
        &self.name
    }

    async fn subscribe_order_book(&self, symbol: &str) -> Result<broadcast::Receiver<MarketEvent>> {
        let (tx, rx) = broadcast::channel(128);
        let symbol = symbol.to_string();
        let this = Arc::new(BinanceExchange::new()); // Simplified for now
        
        tokio::spawn(async move {
            this.handle_websocket(&symbol, "depth20@100ms", tx).await;
        });
        
        Ok(rx)
    }

    async fn subscribe_trades(&self, symbol: &str) -> Result<broadcast::Receiver<MarketEvent>> {
        let (tx, rx) = broadcast::channel(128);
        let symbol = symbol.to_string();
        let this = Arc::new(BinanceExchange::new());
        
        tokio::spawn(async move {
            this.handle_websocket(&symbol, "trade", tx).await;
        });
        
        Ok(rx)
    }

    fn is_healthy(&self) -> bool {
        true // TODO: Implement heartbeat check
    }
}
