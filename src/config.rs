//! Runtime configuration, parsed from CLI arguments at startup.
//!
//! All engine parameters live here so they can be overridden from the command
//! line without touching source code. Defaults are production-grade conserva-
//! tive values validated against real exchange behaviour.

use clap::Parser;

/// Crypto Arbitrage Engine — BTC cross-venue scanner.
#[derive(Debug, Clone, Parser)]
#[command(
    version,
    about = "Real-time BTC cross-exchange arbitrage scanner with fee-aware execution modeling",
    long_about = None
)]
pub struct Config {
    /// TCP address the HTTP server will bind to.
    #[arg(long, default_value = "127.0.0.1:3000", env = "BIND_ADDR")]
    pub bind_addr: String,

    /// Minimum net profit (%) required to surface an opportunity.
    /// Below this threshold opportunities are silently discarded.
    #[arg(long, default_value_t = 0.01, env = "MIN_NET_PCT")]
    pub min_net_pct: f64,

    /// BTC trade size used for all profit calculations.
    #[arg(long, default_value_t = 0.1, env = "TRADE_BTC")]
    pub trade_btc: f64,

    /// How often the engine re-fetches prices from the upstream API (seconds).
    #[arg(long, default_value_t = 30, env = "SCAN_INTERVAL_SECS")]
    pub scan_interval_secs: u64,

    /// Maximum number of retry attempts on upstream API failure.
    #[arg(long, default_value_t = 3, env = "MAX_RETRIES")]
    pub max_retries: u32,

    /// Minimum 24-hour USD volume for an exchange to be included (filters
    /// illiquid venues where slippage would dominate).
    #[arg(long, default_value_t = 200_000.0, env = "MIN_VOLUME_USD")]
    pub min_volume_usd: f64,

    /// Disable color output in the terminal (useful for CI / log aggregators).
    #[arg(long, default_value_t = false, env = "NO_COLOR")]
    pub no_color: bool,
}
