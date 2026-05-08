//! Terminal-based telemetry and reporting.
//!
//! Provides a high-fidelity CLI interface for real-time monitoring of the
//! arbitrage engine's state and detected opportunities.

use crate::arbitrage::ArbitrageOpportunity;
use crate::config::Config;
use crate::exchanges::ExchangePrice;
use chrono::Utc;

pub struct Reporter {
    config: Config,
}

impl Reporter {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn print_banner(&self) {
        if self.config.no_color {
            println!("\n+----------------------------------------------------------------------------+");
            println!("|               BTC QUANTUM ARBITRAGE ENGINE v3.0                            |");
            println!("+----------------------------------------------------------------------------+\n");
        } else {
            println!("\n\x1b[1;33m╔══════════════════════════════════════════════════════════════════════════╗\x1b[0m");
            println!("\x1b[1;33m║               \x1b[1;37m₿  QUANTUM ARBITRAGE ENGINE  v3.0\x1b[1;33m                  ║\x1b[0m");
            println!("\x1b[1;33m║                 \x1b[0;37mBTC/USD Live Arbitrage Scanner\x1b[1;33m                       ║\x1b[0m");
            println!("\x1b[1;33m╚══════════════════════════════════════════════════════════════════════════╝\x1b[0m\n");
        }
    }

    pub fn print_scan_header(&self, scan_id: u64) {
        let timestamp = Utc::now().format("%H:%M:%S");
        let date = Utc::now().format("%Y-%m-%d");
        println!("\x1b[1;34m[SCAN #{}]\x1b[0m — {} | {}", scan_id, timestamp, date);
        println!("\x1b[0;90m───────────────────────────────────────────────────────────────\x1b[0m");
    }

    pub fn print_prices(&self, prices: &[ExchangePrice]) {
        println!("\n\x1b[1;37mLIVE PRICES\x1b[0m (Top 10 by volume)\n");
        for p in prices.iter().take(10) {
            println!("{:>18} → \x1b[1;32m${:<10.2}\x1b[0m  Fee: {:.2}%", 
                p.exchange, p.last, p.taker_fee * 100.0);
        }
        println!();
    }

    pub fn print_opportunities(&self, opps: &[ArbitrageOpportunity]) {
        if opps.is_empty() {
            println!("\x1b[0;90mNo positive net opportunities found this scan.\x1b[0m\n");
            return;
        }

        println!("\x1b[1;32m🚀 {} OPPORTUNITIES FOUND\x1b[0m\n", opps.len());
        for (i, opp) in opps.iter().take(10).enumerate() {
            let conf_color = match opp.confidence {
                crate::arbitrage::Confidence::High => "\x1b[1;32m",
                crate::arbitrage::Confidence::Medium => "\x1b[1;33m",
                crate::arbitrage::Confidence::Low => "\x1b[1;31m",
                crate::arbitrage::Confidence::Risky => "\x1b[1;35m",
            };

            println!(
                "{:>2}. {} → {} | Gross: {:.2}% | \x1b[1;37mNet: {:.2}%\x1b[0m | Profit: \x1b[1;32m${:.2}\x1b[0m | {}[{}]\x1b[0m",
                i + 1,
                opp.buy_exchange,
                opp.sell_exchange,
                opp.gross_spread_pct,
                opp.net_after_all_pct,
                opp.net_profit_usd,
                conf_color,
                opp.confidence
            );
            println!("   \x1b[0;90m↳ {} | Transfer: {:.0}m\x1b[0m", opp.notes, opp.avg_transfer_minutes);
        }
        println!();
    }

    pub fn print_footer(&self) {
        println!("\x1b[0;90mNext scan in {} seconds... (Ctrl + C to exit)\x1b[0m\n", self.config.scan_interval_secs);
    }
}