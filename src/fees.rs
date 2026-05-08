//! Fee modeling and breakeven analytics.
//!
//! Provides granular breakdowns of the costs associated with a cross-venue trade.

use serde::Serialize;
use std::fmt;

/// Summary of all costs for a specific arbitrage route.
#[derive(Debug, Clone, Serialize)]
pub struct FeeSummary {
    pub buy_taker_fee_usd: f64,
    pub sell_taker_fee_usd: f64,
    pub withdrawal_fee_usd: f64,
    pub network_fee_usd: f64,
    pub total_fee_usd: f64,
    pub total_fee_pct: f64,
}

impl fmt::Display for FeeSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  Buy Fee:        ${:<10.2}", self.buy_taker_fee_usd)?;
        writeln!(f, "  Sell Fee:       ${:<10.2}", self.sell_taker_fee_usd)?;
        writeln!(f, "  Withdrawal:     ${:<10.2}", self.withdrawal_fee_usd)?;
        writeln!(f, "  Network:        ${:<10.2}", self.network_fee_usd)?;
        writeln!(f, "  ──────────────────────────")?;
        write!(f, "  TOTAL:          ${:<10.2} ({:.3}%)", self.total_fee_usd, self.total_fee_pct)
    }
}

pub struct FeeCalculator;

impl FeeCalculator {
    /// Estimates the total round-trip cost for a proposed trade.
    pub fn calculate_summary(
        trade_btc: f64,
        buy_price: f64,
        buy_taker: f64,
        sell_taker: f64,
        withdrawal_btc: f64,
    ) -> FeeSummary {
        let trade_usd = trade_btc * buy_price;
        let network_fee_btc = 0.0000125;

        let buy_fee_usd = trade_usd * buy_taker;
        let sell_fee_usd = trade_usd * sell_taker;
        let withdrawal_fee_usd = withdrawal_btc * buy_price;
        let network_fee_usd = network_fee_btc * buy_price;

        let total_fee_usd = buy_fee_usd + sell_fee_usd + withdrawal_fee_usd + network_fee_usd;
        let total_fee_pct = (total_fee_usd / trade_usd) * 100.0;

        FeeSummary {
            buy_taker_fee_usd: buy_fee_usd,
            sell_taker_fee_usd: sell_fee_usd,
            withdrawal_fee_usd,
            network_fee_usd,
            total_fee_usd,
            total_fee_pct,
        }
    }
}