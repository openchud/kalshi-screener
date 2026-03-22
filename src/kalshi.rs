use serde::Deserialize;
use std::time::Duration;

const KALSHI_API: &str = "https://api.elections.kalshi.com/trade-api/v2";

fn parse_dollar(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(0.0)
}

fn parse_fp(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(0.0)
}

#[derive(Debug, Deserialize)]
pub struct RawMarket {
    pub ticker: String,
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub event_ticker: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub close_time: Option<String>,
    #[serde(default)]
    pub yes_bid_dollars: Option<String>,
    #[serde(default)]
    pub yes_ask_dollars: Option<String>,
    #[serde(default)]
    pub last_price_dollars: Option<String>,
    #[serde(default)]
    pub volume_fp: Option<String>,
    #[serde(default)]
    pub volume_24h_fp: Option<String>,
    #[serde(default)]
    pub open_interest_fp: Option<String>,
    // Legacy integer fields (fallback)
    #[serde(default)]
    pub yes_bid: Option<i64>,
    #[serde(default)]
    pub yes_ask: Option<i64>,
    #[serde(default)]
    pub last_price: Option<i64>,
    #[serde(default)]
    pub volume: Option<i64>,
    #[serde(default)]
    pub volume_24h: Option<i64>,
    #[serde(default)]
    pub open_interest: Option<i64>,
    #[serde(default)]
    pub strike_type: Option<String>,
}

/// Normalized market with clean f64 values
pub struct Market {
    pub ticker: String,
    pub title: String,
    pub subtitle: String,
    pub event_ticker: String,
    pub category: String,
    pub status: String,
    pub close_time: Option<String>,
    pub yes_bid: f64,   // 0.0-1.0
    pub yes_ask: f64,   // 0.0-1.0
    pub last_price: f64, // 0.0-1.0
    pub volume: f64,
    pub volume_24h: f64,
    pub open_interest: f64,
}

impl RawMarket {
    pub fn normalize(self) -> Market {
        let yes_bid = self.yes_bid_dollars.as_deref().map(parse_dollar)
            .unwrap_or_else(|| self.yes_bid.unwrap_or(0) as f64 / 100.0);
        let yes_ask = self.yes_ask_dollars.as_deref().map(parse_dollar)
            .unwrap_or_else(|| self.yes_ask.unwrap_or(0) as f64 / 100.0);
        let last_price = self.last_price_dollars.as_deref().map(parse_dollar)
            .unwrap_or_else(|| self.last_price.unwrap_or(0) as f64 / 100.0);
        let volume = self.volume_fp.as_deref().map(parse_fp)
            .unwrap_or_else(|| self.volume.unwrap_or(0) as f64);
        let volume_24h = self.volume_24h_fp.as_deref().map(parse_fp)
            .unwrap_or_else(|| self.volume_24h.unwrap_or(0) as f64);
        let open_interest = self.open_interest_fp.as_deref().map(parse_fp)
            .unwrap_or_else(|| self.open_interest.unwrap_or(0) as f64);

        // Extract category from ticker prefix
        let category = extract_category(&self.ticker);

        Market {
            ticker: self.ticker,
            title: self.title,
            subtitle: self.subtitle,
            event_ticker: self.event_ticker,
            category,
            status: self.status,
            close_time: self.close_time,
            yes_bid,
            yes_ask,
            last_price,
            volume,
            volume_24h,
            open_interest,
        }
    }
}

fn extract_category(ticker: &str) -> String {
    let t = ticker.to_uppercase();
    if t.starts_with("KXNBA") || t.starts_with("KXNFL") || t.starts_with("KXNHL") || t.starts_with("KXMLB")
        || t.starts_with("KXNCAA") || t.starts_with("KXMLS") || t.starts_with("KXEPL")
        || t.contains("GAME") || t.contains("SPORT") || t.contains("MVP")
    {
        "Sports".into()
    } else if t.starts_with("KXSP500") || t.starts_with("KXBTC") || t.starts_with("KXETH")
        || t.starts_with("KXNAS") || t.starts_with("KXDOW") || t.contains("STOCK")
    {
        "Finance".into()
    } else if t.starts_with("KXCPI") || t.starts_with("KXGDP") || t.starts_with("KXFED")
        || t.starts_with("KXINFL") || t.starts_with("KXJOB") || t.starts_with("KXUNEMPLOY")
    {
        "Economics".into()
    } else if t.starts_with("KXHIGH") || t.starts_with("KXLOW") || t.starts_with("KXRAIN")
        || t.starts_with("KXTEMP") || t.starts_with("KXSNOW") || t.contains("WEATHER")
    {
        "Weather".into()
    } else if t.starts_with("KXPRES") || t.starts_with("KXSEN") || t.starts_with("KXHOUSE")
        || t.contains("TRUMP") || t.starts_with("KXGOV") || t.starts_with("KXPOL")
        || t.starts_with("KXBIDEN") || t.starts_with("KXELECT")
    {
        "Politics".into()
    } else if t.starts_with("KXMVE") || t.starts_with("KXMULTI") {
        "Multi-leg".into()
    } else if t.starts_with("KXBALANCE") || t.starts_with("KXTARIFF") {
        "Policy".into()
    } else {
        "Other".into()
    }
}

#[derive(Deserialize)]
struct MarketsResponse {
    markets: Vec<RawMarket>,
    cursor: Option<String>,
}

pub async fn fetch_all_markets() -> Result<Vec<Market>, String> {
    let client = reqwest::Client::builder()
        .user_agent("kalshi-screener/0.1 (github.com/openchud)")
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Client build error: {}", e))?;

    let mut all_markets = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut url = format!("{}/markets?limit=200&status=open", KALSHI_API);
        if let Some(ref c) = cursor {
            if !c.is_empty() {
                url.push_str(&format!("&cursor={}", c));
            }
        }

        let mut attempts = 0;
        let resp = loop {
            attempts += 1;
            let r = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("HTTP error: {}", e))?;

            if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if attempts > 5 {
                    return Err("Rate limited after 5 retries".into());
                }
                let wait = Duration::from_secs(2u64.pow(attempts));
                tracing::warn!("Rate limited, retrying in {}s (attempt {})", wait.as_secs(), attempts);
                tokio::time::sleep(wait).await;
                continue;
            }

            break r;
        };

        if !resp.status().is_success() {
            return Err(format!("API returned {}", resp.status()));
        }

        let body: MarketsResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        let count = body.markets.len();
        all_markets.extend(body.markets.into_iter().map(|m| m.normalize()));

        // Small delay between pages to avoid rate limits
        if count == 200 {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        match body.cursor {
            Some(c) if !c.is_empty() && count == 200 => cursor = Some(c),
            _ => break,
        }
    }

    Ok(all_markets)
}
