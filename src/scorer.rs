use crate::{kalshi::Market, ScoredMarket};
use chrono::Utc;

pub fn score_markets(markets: Vec<Market>) -> Vec<ScoredMarket> {
    let mut scored: Vec<ScoredMarket> = markets
        .into_iter()
        .filter_map(|m| score_market(m))
        .collect();

    scored.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

fn score_market(m: Market) -> Option<ScoredMarket> {
    let spread = m.yes_ask - m.yes_bid;

    // Skip markets with no meaningful pricing
    if m.yes_bid <= 0.0 && m.yes_ask <= 0.0 {
        return None;
    }

    let hours_to_close = m.close_time.as_ref().and_then(|ct| {
        ct.parse::<chrono::DateTime<Utc>>().ok().map(|close| {
            let diff = close - Utc::now();
            diff.num_minutes() as f64 / 60.0
        })
    });

    let oi = m.open_interest.max(0.0);
    let vol = m.volume.max(0.0);
    let liq_score = ((oi.max(1.0).ln() / 12.0) * 0.5 + (vol.max(1.0).ln() / 14.0) * 0.5).min(1.0).max(0.0);

    let vol_24h = m.volume_24h.max(0.0);
    let vol_score = (vol_24h.max(1.0).ln() / 10.0).min(1.0).max(0.0);

    let spread_score = if spread > 0.0 { (1.0 - spread).max(0.0) } else { 0.5 };

    let time_score = hours_to_close.map(|h| {
        if h < 0.0 { 0.0 }
        else if h < 1.0 { 0.3 }
        else if h <= 72.0 { 1.0 }
        else if h <= 168.0 { 0.7 }
        else if h <= 720.0 { 0.4 }
        else { 0.2 }
    }).unwrap_or(0.3);

    let mid = (m.yes_bid + m.yes_ask) / 2.0;
    let price_score = if mid > 0.0 {
        1.0 - (2.0 * (mid - 0.5)).abs().min(1.0) * 0.5
    } else {
        0.3
    };

    let composite = liq_score * 0.30
        + vol_score * 0.25
        + spread_score * 0.20
        + time_score * 0.15
        + price_score * 0.10;

    Some(ScoredMarket {
        ticker: m.ticker,
        title: m.title,
        category: m.category,
        yes_bid: m.yes_bid,
        yes_ask: m.yes_ask,
        spread: (spread * 100.0).round() / 100.0,
        volume: m.volume as i64,
        volume_24h: m.volume_24h as i64,
        open_interest: m.open_interest as i64,
        liquidity_score: (liq_score * 100.0).round() / 100.0,
        composite_score: (composite * 1000.0).round() / 1000.0,
        close_time: m.close_time,
        hours_to_close: hours_to_close.map(|h| (h * 10.0).round() / 10.0),
        status: m.status,
        last_price: m.last_price,
        event_ticker: m.event_ticker,
        subtitle: m.subtitle,
    })
}
