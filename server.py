#!/usr/bin/env python3
"""Kalshi Market Screener — by @openchud"""
import json, math, threading, time, urllib.request, urllib.error, traceback, sys
from datetime import datetime, timezone
from http.server import HTTPServer, SimpleHTTPRequestHandler
from pathlib import Path

sys.stdout.reconfigure(line_buffering=True)

KALSHI_API = "https://api.elections.kalshi.com/trade-api/v2"
MAX_PAGES = 10  # Cap at ~2000 markets to avoid rate limits
CACHE = {"markets": [], "updated_at": None, "total_markets": 0}
LOCK = threading.Lock()


def fetch_page(cursor=None):
    url = f"{KALSHI_API}/markets?limit=200&status=open"
    if cursor:
        url += f"&cursor={cursor}"
    req = urllib.request.Request(url, headers={"User-Agent": "kalshi-screener/0.1"})
    resp = urllib.request.urlopen(req, timeout=15)
    return json.loads(resp.read())


def fetch_pages(extra_params="", max_pages=MAX_PAGES):
    """Fetch up to max_pages of markets with optional extra URL params."""
    markets = []
    cursor = None
    for page in range(1, max_pages + 1):
        url = f"{KALSHI_API}/markets?limit=200&status=open{extra_params}"
        if cursor:
            url += f"&cursor={cursor}"
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "kalshi-screener/0.1"})
            resp = urllib.request.urlopen(req, timeout=15)
            data = json.loads(resp.read())
        except urllib.error.HTTPError as e:
            if e.code == 429:
                print(f"  Rate limited on page {page}, got {len(markets)}", flush=True)
                break
            raise
        except Exception as e:
            print(f"  Error on page {page}: {e}", flush=True)
            break
        batch = data.get("markets", [])
        markets.extend(batch)
        cursor = data.get("cursor")
        if not cursor or len(batch) < 200:
            break
        time.sleep(1.5)
    return markets


# Series to fetch specifically (high-value non-sports categories)
PRIORITY_SERIES = [
    # Finance
    "KXBTC", "KXETH", "KXSP500", "KXNAS", "KXDOW",
    "KXTSLA", "KXNVDA", "KXAAPL", "KXAMZN", "KXGOOG", "KXCOIN",
    # Economics
    "KXCPI", "KXGDP", "KXFED", "KXPCE", "KXISM", "KXJOB", "KXUNEMPLOY",
    # Weather
    "KXHIGHNY", "KXHIGHCHI", "KXHIGHLA", "KXHIGHMIA", "KXHIGHATL",
    "KXLOWNY", "KXLOWCHI",
    # Politics/Policy
    "KXPRES", "KXTARIFF", "KXCEASEFIRE", "KXUKRAINE",
    # Index/Other
    "KXINX", "KXBALANCE",
]


def fetch_markets():
    seen_tickers = set()
    all_markets = []

    # Fetch priority series first (1 page each, plenty)
    for series in PRIORITY_SERIES:
        try:
            batch = fetch_pages(f"&series_ticker={series}", max_pages=2)
            for m in batch:
                if m["ticker"] not in seen_tickers:
                    seen_tickers.add(m["ticker"])
                    all_markets.append(m)
            if batch:
                print(f"  {series}: +{len(batch)}", flush=True)
            time.sleep(0.5)
        except Exception as e:
            print(f"  {series} error: {e}", flush=True)

    # Then fetch general markets for broader coverage
    print(f"  Fetching general markets...", flush=True)
    general = fetch_pages("", max_pages=5)
    for m in general:
        if m["ticker"] not in seen_tickers:
            seen_tickers.add(m["ticker"])
            all_markets.append(m)
    print(f"  General: +{len(general)} ({len(all_markets)} unique total)", flush=True)

    return all_markets


def pdol(s):
    try: return float(s)
    except: return 0.0


def category(ticker):
    t = ticker.upper()
    sports = ["KXNBA","KXNFL","KXNHL","KXMLB","KXNCAA","KXMLS","KXEPL","KXWTA","KXATP","KXUFC","KXPGA","KXF1"]
    finance = ["KXSP500","KXBTC","KXETH","KXNAS","KXDOW","KXTSLA","KXAAPL","KXAMZN","KXGOOG","KXNVDA","KXCOIN"]
    econ = ["KXCPI","KXGDP","KXFED","KXINFL","KXJOB","KXUNEMPLOY","KXPCE","KXISM","KXRATE"]
    weather = ["KXHIGH","KXLOW","KXRAIN","KXTEMP","KXSNOW"]
    politics = ["KXPRES","KXSEN","KXHOUSE","KXGOV","KXPOL","KXELECT"]
    if any(t.startswith(p) for p in sports) or "GAME" in t or "MATCH" in t: return "Sports"
    if any(t.startswith(p) for p in finance): return "Finance"
    if any(t.startswith(p) for p in econ): return "Economics"
    if any(t.startswith(p) for p in weather): return "Weather"
    if any(t.startswith(p) for p in politics) or "TRUMP" in t: return "Politics"
    if t.startswith("KXMVE"): return "Multi-leg"
    if any(x in t for x in ["TARIFF","CEASEFIRE","UKRAINE","BALANCE"]): return "Policy"
    return "Other"


def score(m):
    yb = pdol(m.get("yes_bid_dollars", "0"))
    ya = pdol(m.get("yes_ask_dollars", "0"))
    lp = pdol(m.get("last_price_dollars", "0"))
    vol = pdol(m.get("volume_fp", "0"))
    v24 = pdol(m.get("volume_24h_fp", "0"))
    oi = pdol(m.get("open_interest_fp", "0"))
    sp = ya - yb
    if yb <= 0 and ya <= 0: return None

    htc = None
    ct = m.get("close_time")
    if ct:
        try:
            close = datetime.fromisoformat(ct.replace("Z", "+00:00"))
            htc = round((close - datetime.now(timezone.utc)).total_seconds() / 3600, 1)
        except: pass

    ls = max(0, min(1, math.log(max(oi,1))/12*0.5 + math.log(max(vol,1))/14*0.5))
    vs = max(0, min(1, math.log(max(v24,1))/10))
    ss = max(0, 1-sp) if sp > 0 else 0.5
    ts = (0 if htc and htc<0 else 0.3 if htc and htc<1 else 1.0 if htc and htc<=72 else 0.7 if htc and htc<=168 else 0.4 if htc and htc<=720 else 0.2 if htc else 0.3)
    mid = (yb+ya)/2
    ps = (1.0 - min(1, abs(2*(mid-0.5)))*0.5) if mid > 0 else 0.3
    comp = ls*0.30 + vs*0.25 + ss*0.20 + ts*0.15 + ps*0.10

    # Signal detection: flag interesting market conditions
    signals = []
    if v24 > 1000 and sp <= 0.02:
        signals.append("tight")      # High-volume tight spread = liquid & efficient
    if v24 > 500 and sp >= 0.10:
        signals.append("wide")       # High-volume wide spread = opportunity
    if htc and 0 < htc < 6 and v24 > 100:
        signals.append("expiring")   # Expiring soon with activity
    if lp > 0 and yb > 0:
        move = abs(lp - yb) / max(lp, 0.01)
        if move > 0.15 and v24 > 50:
            signals.append("moving")  # Price moved significantly from last trade
    if oi > 5000 and 0.35 <= mid <= 0.65:
        signals.append("contested")  # High OI near 50/50 = genuine uncertainty

    return {
        "ticker": m["ticker"], "title": m.get("title",""), "subtitle": m.get("subtitle",""),
        "event_ticker": m.get("event_ticker",""), "category": category(m["ticker"]),
        "yes_bid": yb, "yes_ask": ya, "spread": round(sp,4), "last_price": lp,
        "volume": int(vol), "volume_24h": int(v24), "open_interest": int(oi),
        "liquidity_score": round(ls,3), "composite_score": round(comp,4),
        "close_time": ct, "hours_to_close": htc, "status": m.get("status",""),
        "signals": signals,
    }


def refresh_loop():
    while True:
        try:
            print(f"[{datetime.now(timezone.utc).isoformat()}] Fetching...", flush=True)
            raw = fetch_markets()
            scored = sorted([s for s in (score(m) for m in raw) if s], key=lambda x: -x["composite_score"])
            with LOCK:
                CACHE["markets"] = scored
                CACHE["updated_at"] = datetime.now(timezone.utc).isoformat()
                CACHE["total_markets"] = len(scored)
            print(f"[{datetime.now(timezone.utc).isoformat()}] Cached {len(scored)} markets", flush=True)
        except Exception as e:
            print(f"[{datetime.now(timezone.utc).isoformat()}] ERROR: {e}", flush=True)
            traceback.print_exc()
        time.sleep(180)


class Handler(SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/api/markets":
            with LOCK: data = json.dumps(CACHE)
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(data.encode())
        elif self.path == "/api/health":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"ok")
        elif self.path in ("/", "/index.html"):
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.end_headers()
            self.wfile.write(Path(__file__).parent.joinpath("static/index.html").read_bytes())
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, *a): pass


if __name__ == "__main__":
    threading.Thread(target=refresh_loop, daemon=True).start()
    print("Screener running on http://0.0.0.0:8888", flush=True)
    HTTPServer(("0.0.0.0", 8888), Handler).serve_forever()
