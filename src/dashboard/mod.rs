//! Real-time dashboard and API server.

use crate::engine::scanner::ArbOpportunity;
use axum::{
    extract::State,
    response::Html,
    routing::get,
    Json, Router,
};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Arc;
use tracing::info;
use crate::market::book::L2OrderBook;
use rust_decimal::Decimal;

#[derive(Serialize)]
pub struct PriceDisplay {
    pub exchange: String,
    pub last: Decimal,
    pub ask: Decimal,
    pub taker_fee: f64,
    pub withdrawal_fee: f64,
}

#[derive(Serialize)]
pub struct DashboardState {
    pub prices: Vec<PriceDisplay>,
    pub opportunities: Vec<ArbOpportunity>,
    pub scan_id: u64,
}

pub struct AppState {
    pub opportunities: RwLock<VecDeque<ArbOpportunity>>,
    pub books: DashMap<String, L2OrderBook>,
    pub scan_id: std::sync::atomic::AtomicU64,
}

pub async fn run_server(state: Arc<AppState>) {
    let app = Router::new()
        .route("/", get(serve_dashboard))
        .route("/api/state", get(get_state))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Dashboard available at http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn serve_dashboard() -> Html<&'static str> {
    Html(include_str!("../../crypto_arbitrage_dashboard.html"))
}

async fn get_state(State(state): State<Arc<AppState>>) -> Json<DashboardState> {
    let opps = state.opportunities.read();
    
    let prices: Vec<PriceDisplay> = state.books.iter().map(|r| {
        let book = r.value();
        PriceDisplay {
            exchange: book.exchange.clone(),
            last: book.mid_price().unwrap_or_default(),
            ask: book.best_ask().unwrap_or_default(),
            taker_fee: 0.001, // Placeholder
            withdrawal_fee: 0.0005, // Placeholder
        }
    }).collect();

    Json(DashboardState {
        prices,
        opportunities: opps.iter().cloned().collect(),
        scan_id: state.scan_id.load(std::sync::atomic::Ordering::Relaxed),
    })
}
