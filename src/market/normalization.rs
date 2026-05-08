//! Asset normalization and parity management.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Serialize;
use std::collections::HashMap;
use parking_lot::RwLock;

#[derive(Debug, Clone, Serialize)]
pub struct ParityState {
    pub usdt_usd: Decimal,
    pub usdc_usd: Decimal,
    pub busd_usd: Decimal,
}

impl Default for ParityState {
    fn default() -> Self {
        Self {
            usdt_usd: dec!(1.0),
            usdc_usd: dec!(1.0),
            busd_usd: dec!(1.0),
        }
    }
}

/// Normalizes prices across different quote currencies (USD, USDT, USDC).
pub struct NormalizationEngine {
    parity: RwLock<ParityState>,
}

impl NormalizationEngine {
    pub fn new() -> Self {
        Self {
            parity: RwLock::new(ParityState::default()),
        }
    }

    /// Updates a parity rate (e.g. USDT/USD).
    #[allow(dead_code)]
    pub fn update_parity(&self, asset: &str, rate: Decimal) {
        let mut p = self.parity.write();
        match asset {
            "USDT" => p.usdt_usd = rate,
            "USDC" => p.usdc_usd = rate,
            "BUSD" => p.busd_usd = rate,
            _ => {}
        }
    }

    /// Converts a price in a given quote currency to canonical USD.
    pub fn to_canonical(&self, price: Decimal, quote: &str) -> Decimal {
        let p = self.parity.read();
        match quote {
            "USDT" => price * p.usdt_usd,
            "USDC" => price * p.usdc_usd,
            "BUSD" => price * p.busd_usd,
            "USD" => price,
            _ => price, // Fallback
        }
    }

    /// Returns the parity threshold alert if any stablecoin depegs.
    #[allow(dead_code)]
    pub fn check_depeg(&self, threshold: Decimal) -> Vec<String> {
        let p = self.parity.read();
        let mut alerts = Vec::new();
        if (p.usdt_usd - dec!(1.0)).abs() > threshold { alerts.push("USDT".to_string()); }
        if (p.usdc_usd - dec!(1.0)).abs() > threshold { alerts.push("USDC".to_string()); }
        alerts
    }
}

pub struct AssetMapper {
    // Map of Exchange Symbol -> (Base, Quote)
    mapping: HashMap<String, (String, String)>,
}

impl AssetMapper {
    pub fn new() -> Self {
        let mut mapping = HashMap::new();
        mapping.insert("BTCUSDT".to_string(), ("BTC".to_string(), "USDT".to_string()));
        mapping.insert("BTC/USD".to_string(), ("BTC".to_string(), "USD".to_string()));
        mapping.insert("BTC-USD".to_string(), ("BTC".to_string(), "USD".to_string()));
        Self { mapping }
    }

    pub fn resolve(&self, symbol: &str) -> Option<(String, String)> {
        self.mapping.get(symbol).cloned()
    }
}
