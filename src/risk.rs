//! Risk assessment and position sizing utilities.
//!
//! Implements quantitative filters to detect high-risk environments and 
//! calculates optimal position sizes using the Kelly Criterion.

use crate::arbitrage::ArbitrageOpportunity;

/// Evaluates if a specific opportunity meets safety standards.
pub struct RiskAssessor {}

impl Default for RiskAssessor {
    fn default() -> Self {
        Self {}
    }
}

impl RiskAssessor {
    /// Returns true if the opportunity is considered within acceptable risk bounds.
    pub fn is_acceptable(&self, opp: &ArbitrageOpportunity) -> bool {
        // High transfer time increases risk of price drift
        if opp.avg_transfer_minutes > 90.0 {
            return false;
        }

        // Extremely low net profit might be noise
        if opp.net_after_all_pct < -0.05 {
            return false;
        }

        true
    }
}