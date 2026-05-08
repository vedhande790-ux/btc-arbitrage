//! Rolling realized volatility engine.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::VecDeque;
use parking_lot::RwLock;

pub struct VolatilityEngine {
    window_size: usize,
    prices: RwLock<VecDeque<f64>>,
}

impl VolatilityEngine {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            prices: RwLock::new(VecDeque::with_capacity(window_size)),
        }
    }

    pub fn update(&self, price: Decimal) {
        let mut p = self.prices.write();
        if let Some(price_f64) = price.to_f64() {
            if p.len() >= self.window_size {
                p.pop_front();
            }
            p.push_back(price_f64);
        }
    }

    /// Calculates annualized realized volatility based on returns in the window.
    pub fn realized_volatility(&self) -> f64 {
        let p = self.prices.read();
        if p.len() < 2 { return 0.0; }

        let mut returns = Vec::with_capacity(p.len() - 1);
        for i in 1..p.len() {
            let r = (p[i] - p[i-1]) / p[i-1];
            returns.push(r);
        }

        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / (returns.len() - 1) as f64;

        let stdev = variance.sqrt();
        
        // Annualize (assuming 1s updates, 31,536,000 seconds per year)
        stdev * (31536000.0f64).sqrt()
    }
}
