//! Position sizing and risk mitigation.

use crate::engine::scanner::ArbOpportunity;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub struct RiskAssessor {
    pub max_notional_usd: Decimal,
    pub min_net_pct: Decimal,
    #[allow(dead_code)]
    pub max_volatility: Decimal,
}

impl Default for RiskAssessor {
    fn default() -> Self {
        Self {
            max_notional_usd: dec!(10000.0),
            min_net_pct: dec!(0.05), // 0.05%
            max_volatility: dec!(0.02), // 2% 1h vol
        }
    }
}

impl RiskAssessor {
    pub fn is_acceptable(&self, opp: &ArbOpportunity) -> bool {
        if opp.net_pct < self.min_net_pct {
            return false;
        }

        if opp.size_btc * opp.buy_price > self.max_notional_usd {
            return false;
        }

        true
    }

    /// Kelly Criterion for optimal position sizing.
    #[allow(dead_code)]
    pub fn calculate_kelly_size(&self, win_prob: f64, win_pct: f64, loss_pct: f64, bankroll: f64) -> f64 {
        let q = 1.0 - win_prob;
        let b = win_pct / loss_pct;
        let kelly = (win_prob * b - q) / b;
        
        // Half-Kelly with 25% cap
        let fraction = (kelly / 2.0).clamp(0.0, 0.25);
        bankroll * fraction
    }
}
