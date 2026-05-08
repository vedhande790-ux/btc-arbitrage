//! L2 Order Book implementation with fixed-point precision.

use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct Level {
    pub price: Decimal,
    pub quantity: Decimal,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct L2OrderBook {
    pub exchange: String,
    pub symbol: String,
    pub bids: BTreeMap<Decimal, Decimal>, // Price -> Quantity
    pub asks: BTreeMap<Decimal, Decimal>,
    pub last_update_id: u64,
    pub local_timestamp: i64,
}

impl L2OrderBook {
    pub fn new(exchange: &str, symbol: &str) -> Self {
        Self {
            exchange: exchange.to_string(),
            symbol: symbol.to_string(),
            ..Default::default()
        }
    }

    pub fn update_bid(&mut self, price: Decimal, quantity: Decimal) {
        if quantity.is_zero() {
            self.bids.remove(&price);
        } else {
            self.bids.insert(price, quantity);
        }
    }

    pub fn update_ask(&mut self, price: Decimal, quantity: Decimal) {
        if quantity.is_zero() {
            self.asks.remove(&price);
        } else {
            self.asks.insert(price, quantity);
        }
    }

    /// Estimates the average fill price for a given size by walking the book.
    /// Returns (avg_price, total_filled).
    pub fn estimate_fill_price(&self, size: Decimal, is_buy: bool) -> (Decimal, Decimal) {
        let mut remaining = size;
        let mut total_cost = Decimal::ZERO;
        let mut total_filled = Decimal::ZERO;

        let levels = if is_buy { &self.asks } else { &self.bids };

        let iter: Box<dyn Iterator<Item = (&Decimal, &Decimal)>> = if is_buy {
            Box::new(levels.iter())
        } else {
            Box::new(levels.iter().rev())
        };

        for (price, qty) in iter {
            let take = remaining.min(*qty);
            total_cost += take * price;
            total_filled += take;
            remaining -= take;

            if remaining.is_zero() {
                break;
            }
        }

        if total_filled.is_zero() {
            (Decimal::ZERO, Decimal::ZERO)
        } else {
            (total_cost / total_filled, total_filled)
        }
    }

    #[allow(dead_code)]
    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.keys().last().copied()
    }

    #[allow(dead_code)]
    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.keys().next().copied()
    }

    #[allow(dead_code)]
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(b), Some(a)) => Some((b + a) / Decimal::TWO),
            _ => None,
        }
    }
}
