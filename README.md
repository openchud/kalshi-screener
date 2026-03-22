# 🦞 Kalshi Screener

A free, real-time market screener for [Kalshi](https://kalshi.com) prediction markets.

**Live:** See all open markets ranked by a composite score factoring in liquidity, 24h volume, spread tightness, time to expiry, and price positioning.

## Features

- **1400+ markets** scored and ranked in real-time
- **Category detection**: Sports, Finance, Economics, Weather, Politics, Multi-leg
- **Sortable columns**: Click any header to sort
- **Filters**: Search by ticker/title, filter by category and expiry window
- **Dark theme** — easy on the eyes for late-night market analysis
- **Auto-refresh** every 2 minutes
- **Zero dependencies** frontend (vanilla JS)
- **Rate limit handling** with exponential backoff

## Scoring

Each market gets a composite score (0-1) based on:

| Factor | Weight | What it measures |
|--------|--------|-----------------|
| Liquidity | 30% | Open interest + total volume |
| 24h Volume | 25% | Recent trading activity |
| Spread | 20% | Tighter spread = higher score |
| Time to Close | 15% | Prefers 1-72h (actionable) |
| Price | 10% | Mid-range prices = more interesting |

## Run it yourself

```bash
# Just Python 3.10+ required, no pip install needed
python3 -u server.py
# → http://localhost:8888
```

## API

```bash
# Get all scored markets
curl http://localhost:8888/api/markets

# Health check
curl http://localhost:8888/api/health
```

## Systemd

```bash
sudo cp screener.service /etc/systemd/system/
sudo systemctl enable --now screener
```

## Stack

- **Backend**: Python 3 (stdlib only — no dependencies!)
- **Frontend**: Vanilla HTML/CSS/JS
- **Data**: Kalshi public API (no auth required)
- **Also includes**: Rust/Axum prototype for future performance upgrade

## License

MIT

---

Built by [@openchud](https://github.com/openchud) 🦞
