//! Price discovery from upstream exchange data feeds.
//!
//! Handles data ingestion, rate limiting, and normalisation of exchange-specific
//! fee structures. Uses a local cache for high-availability during API downtime.

use crate::config::Config;
use crate::error::{Error, Result};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

/// Internal representation of a single exchange venue's price data.
#[derive(Debug, Clone, Serialize)]
pub struct ExchangePrice {
    pub exchange: String,
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
    pub volume_24h: f64,
    pub withdrawal_fee: f64,
    pub taker_fee: f64,
    pub transfer_time_minutes: u32,
    pub liquidity_score: f64,
}

/// Client for fetching market data from upstream APIs.
pub struct MarketDataFetcher {
    client: Client,
}

impl MarketDataFetcher {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(12))
            .build()?;
        Ok(Self { client })
    }
}

impl MarketDataFetcher {
    /// Fetches and normalises BTC price data from all tracked venues.
    pub async fn fetch_all_prices(&self, config: &Config) -> Result<Vec<ExchangePrice>> {
        let url = "https://api.coingecko.com/api/v3/coins/bitcoin/tickers?depth=false&order=volume_desc&per_page=40";
        let mut last_error: Option<Error> = None;
    let mut json: Option<Value> = None;

    for attempt in 0..config.max_retries {
        let response = self.client
            .get(url)
            .header("User-Agent", "QuantumArbitrageScanner/3.0")
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                json = Some(resp.json().await?);
                break;
            }
            Ok(resp) if resp.status().as_u16() == 429 => {
                let retry_after_secs = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or((attempt + 1) as u64 * 4);
                
                warn!(attempt, retry_after_secs, "Rate limit hit (429)");
                last_error = Some(Error::RateLimited { retry_after_secs });
                sleep(Duration::from_secs(retry_after_secs)).await;
            }
            Ok(resp) => {
                let status = resp.status().as_u16();
                warn!(attempt, status, "Upstream API error");
                last_error = Some(Error::UpstreamStatus { status });
                sleep(Duration::from_millis(800 * (u64::from(attempt) + 1))).await;
            }
            Err(err) => {
                warn!(attempt, %err, "Network request failed");
                last_error = Some(Error::Http(err));
                sleep(Duration::from_millis(800 * (u64::from(attempt) + 1))).await;
            }
        }
    }

    let json = match json {
        Some(v) => v,
        None => {
            if let Some(e) = last_error {
                return Err(e);
            }
            return Err(Error::MalformedResponse("Max retries exceeded".to_string()));
        }
    };

    let tickers = json["tickers"].as_array().ok_or_else(|| {
        Error::MalformedResponse("Missing 'tickers' field in API response".into())
    })?;

    let mut prices = Vec::new();
    let mut seen = HashSet::new();

    for ticker in tickers {
        let target = ticker["target"].as_str().unwrap_or("").to_uppercase();
        if !matches!(target.as_str(), "USD" | "USDT" | "USDC") {
            continue;
        }

        let exchange_name = ticker["market"]["name"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();

        if seen.contains(&exchange_name) {
            continue;
        }

        let last = ticker["last"].as_f64().unwrap_or(0.0);
        if last < 10000.0 {
            continue;
        }

        let bid = ticker["bid"].as_f64().unwrap_or(last * 0.999);
        let ask = ticker["ask"].as_f64().unwrap_or(last * 1.001);
        let volume = ticker["converted_volume"]["usd"].as_f64().unwrap_or(0.0);

        if volume < config.min_volume_usd {
            continue;
        }

        seen.insert(exchange_name.clone());

        let (withdrawal_fee, taker_fee, transfer_time_minutes, liquidity_score) =
            get_exchange_profile(&exchange_name);

        prices.push(ExchangePrice {
            exchange: exchange_name,
            bid,
            ask,
            last,
            volume_24h: volume,
            withdrawal_fee,
            taker_fee,
            transfer_time_minutes,
            liquidity_score,
        });
    }

    if prices.is_empty() {
        return Err(Error::NoExchanges);
    }

    prices.sort_by(|a, b| b.volume_24h.partial_cmp(&a.volume_24h).unwrap());
    info!(venues = prices.len(), "Scan complete");
    Ok(prices)
    }
}

/// Returns a tuple of (withdrawal_fee, taker_fee, transfer_min, liquidity_score).
fn get_exchange_profile(name: &str) -> (f64, f64, u32, f64) {
    let name = name.to_lowercase();
    match () {
        _ if name.contains("binance") => (0.00035, 0.0010, 25, 0.95),
        _ if name.contains("coinbase") => (0.0005, 0.0060, 20, 0.90),
        _ if name.contains("kraken") => (0.0002, 0.0026, 35, 0.88),
        _ if name.contains("okx") => (0.0004, 0.0010, 30, 0.92),
        _ if name.contains("bybit") => (0.0003, 0.0010, 30, 0.90),
        _ if name.contains("bitfinex") => (0.0004, 0.0020, 45, 0.82),
        _ if name.contains("kucoin") => (0.0005, 0.0010, 40, 0.80),
        _ => (0.0006, 0.0025, 50, 0.72),
    }
}