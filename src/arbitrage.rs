//! Core execution engine for arbitrage detection and modeling.
//!
//! Evaluates every possible cross-venue route using a conservative cost model
//! that includes taker fees, withdrawal costs, slippage, and price-drift risk.

use crate::config::Config;
use crate::exchanges::ExchangePrice;
use crate::fees::FeeCalculator;
use crate::risk::RiskAssessor;
use serde::Serialize;
use std::fmt;

/// Represents a validated, profitable arbitrage route.
#[derive(Debug, Clone, Serialize)]
pub struct ArbitrageOpportunity {
    pub buy_exchange: String,
    pub sell_exchange: String,
    pub buy_price: f64,
    pub sell_price: f64,
    pub gross_spread_pct: f64,
    pub net_after_all_pct: f64,
    pub net_profit_usd: f64,
    pub confidence: Confidence,
    pub avg_transfer_minutes: f64,
    pub slippage_cost_usd: f64,
    pub notes: String,
}

/// Qualitative ranking of a trade's execution safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Confidence {
    High,
    Medium,
    Low,
    Risky,
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::High => write!(f, "HIGH"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::Low => write!(f, "LOW"),
            Self::Risky => write!(f, "RISKY"),
        }
    }
}

/// Main engine responsible for scanning the market state for edges.
pub struct ArbitrageEngine {
    config: Config,
}

impl ArbitrageEngine {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Scans a snapshot of exchange prices for profitable routes.
    pub fn find_opportunities(&self, prices: &[ExchangePrice]) -> Vec<ArbitrageOpportunity> {
        let mut opps = Vec::new();
        let risk_assessor = RiskAssessor::default();

        for (i, buy) in prices.iter().enumerate() {
            for (j, sell) in prices.iter().enumerate() {
                if i == j {
                    continue;
                }

                if let Some(opp) = self.analyze_pair(buy, sell) {
                    if risk_assessor.is_acceptable(&opp) {
                        opps.push(opp);
                    }
                }
            }
        }

        // Rank by highest net return first
        opps.sort_by(|a, b| b.net_after_all_pct.partial_cmp(&a.net_after_all_pct).unwrap());
        opps
    }

    /// Models the total profit/loss for a specific buy-sell route.
    fn analyze_pair(&self, buy: &ExchangePrice, sell: &ExchangePrice) -> Option<ArbitrageOpportunity> {
        let size = self.config.trade_btc;

        let effective_buy = buy.ask * (1.0 + buy.taker_fee);
        let effective_sell = sell.bid * (1.0 - sell.taker_fee);

        let gross_spread_pct = ((sell.bid - buy.ask) / buy.ask) * 100.0;

        // Model slippage based on liquidity scores (approximate impact cost)
        let avg_liquidity = ((buy.liquidity_score + sell.liquidity_score) / 2.0).clamp(0.1, 1.0);
        let slippage_pct = (1.0 - avg_liquidity) * 0.35; 
        let slippage_cost_usd = (effective_buy * size) * (slippage_pct / 100.0);

        // Model timing risk: BTC volatility exposure over the transfer window
        let avg_transfer_minutes = f64::from(buy.transfer_time_minutes + sell.transfer_time_minutes) / 2.0;
        let transfer_risk_pct = (avg_transfer_minutes / 60.0) * 0.12; 
        let transfer_risk_cost_usd = (effective_buy * size) * (transfer_risk_pct / 100.0);

        let fee_summary = FeeCalculator::calculate_summary(
            size,
            buy.ask,
            buy.taker_fee,
            sell.taker_fee,
            buy.withdrawal_fee,
        );

        let net_profit_usd = (effective_sell - effective_buy) * size
            - fee_summary.withdrawal_fee_usd
            - slippage_cost_usd
            - transfer_risk_cost_usd;
        let net_after_all_pct = (net_profit_usd / (effective_buy * size)) * 100.0;

        // Discard opportunities below the target threshold
        if net_after_all_pct < self.config.min_net_pct {
            return None;
        }

        let (confidence, notes) = match () {
            _ if net_after_all_pct > 0.8 && avg_liquidity > 0.85 && avg_transfer_minutes < 35.0 => {
                (Confidence::High, "Strong edge after fees and modeled execution costs".to_string())
            }
            _ if net_after_all_pct > 0.5 && avg_liquidity > 0.70 => {
                (Confidence::Medium, "Tradable setup but sensitive to execution timing".to_string())
            }
            _ if net_after_all_pct > 0.35 => {
                (Confidence::Low, "Thin edge; prioritize faster transfer rails".to_string())
            }
            _ => {
                (Confidence::Risky, "Marginal edge likely to vanish under live volatility".to_string())
            }
        };

        Some(ArbitrageOpportunity {
            buy_exchange: buy.exchange.clone(),
            sell_exchange: sell.exchange.clone(),
            buy_price: effective_buy,
            sell_price: effective_sell,
            gross_spread_pct,
            net_after_all_pct,
            net_profit_usd,
            confidence,
            avg_transfer_minutes,
            slippage_cost_usd,
            notes,
        })
    }
}