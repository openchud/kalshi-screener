use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower_http::cors::CorsLayer;

mod kalshi;
mod scorer;

#[derive(Clone)]
struct AppState {
    cache: Arc<RwLock<MarketCache>>,
}

#[derive(Default)]
struct MarketCache {
    markets: Vec<ScoredMarket>,
    updated_at: Option<DateTime<Utc>>,
    total_markets: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoredMarket {
    pub ticker: String,
    pub title: String,
    pub category: String,
    pub yes_bid: f64,
    pub yes_ask: f64,
    pub spread: f64,
    pub volume: i64,
    pub volume_24h: i64,
    pub open_interest: i64,
    pub liquidity_score: f64,
    pub composite_score: f64,
    pub close_time: Option<String>,
    pub hours_to_close: Option<f64>,
    pub status: String,
    pub last_price: f64,
    pub event_ticker: String,
    pub subtitle: String,
}

#[derive(Serialize)]
struct ApiResponse {
    markets: Vec<ScoredMarket>,
    updated_at: Option<String>,
    total_markets: usize,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState {
        cache: Arc::new(RwLock::new(MarketCache::default())),
    };

    // Spawn background fetcher
    let cache = state.cache.clone();
    tokio::spawn(async move {
        loop {
            tracing::info!("Fetching Kalshi markets...");
            match kalshi::fetch_all_markets().await {
                Ok(markets) => {
                    let scored = scorer::score_markets(markets);
                    let total = scored.len();
                    let mut c = cache.write();
                    c.markets = scored;
                    c.updated_at = Some(Utc::now());
                    c.total_markets = total;
                    tracing::info!("Cached {} scored markets", total);
                }
                Err(e) => {
                    tracing::error!("Failed to fetch markets: {}", e);
                }
            }
            tokio::time::sleep(Duration::from_secs(120)).await;
        }
    });

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/markets", get(markets_handler))
        .route("/api/health", get(health_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8888));
    tracing::info!("Screener running on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn markets_handler(State(state): State<AppState>) -> impl IntoResponse {
    let cache = state.cache.read();
    Json(ApiResponse {
        markets: cache.markets.clone(),
        updated_at: cache.updated_at.map(|t| t.to_rfc3339()),
        total_markets: cache.total_markets,
    })
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}
