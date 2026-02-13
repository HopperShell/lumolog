#!/usr/bin/env python3
"""Generate a realistic ~1000-line JSON log for demo videos.

Simulates an API server over a 4-hour window with:
- Natural traffic patterns (quiet → busy → incident → recovery → quiet)
- Time gaps between phases
- Realistic IPs, URLs, UUIDs, durations, status codes
- All log levels represented
- Varied message templates for similar-line filtering demos
"""

import json
import random
import uuid
from datetime import datetime, timedelta, timezone

random.seed(42)

# --- Config ---
BASE_TIME = datetime(2025, 1, 15, 6, 0, 0, tzinfo=timezone.utc)
OUTPUT = "testdata/demo.log"

# --- Data pools ---
SERVICES = ["api-gateway", "auth-service", "user-service", "payment-service", "notification-service"]
ENDPOINTS = [
    ("GET", "/api/v1/users", "user-service"),
    ("GET", "/api/v1/users/{id}", "user-service"),
    ("POST", "/api/v1/users", "user-service"),
    ("POST", "/api/v1/auth/login", "auth-service"),
    ("POST", "/api/v1/auth/refresh", "auth-service"),
    ("GET", "/api/v1/orders", "payment-service"),
    ("POST", "/api/v1/orders", "payment-service"),
    ("POST", "/api/v1/payments/charge", "payment-service"),
    ("GET", "/api/v1/notifications", "notification-service"),
    ("POST", "/api/v1/notifications/send", "notification-service"),
    ("GET", "/healthz", "api-gateway"),
    ("GET", "/api/v1/products", "api-gateway"),
    ("GET", "/api/v1/products/{id}", "api-gateway"),
    ("DELETE", "/api/v1/users/{id}", "user-service"),
    ("PATCH", "/api/v1/users/{id}", "user-service"),
]

IPS = [
    "192.168.1.42", "10.0.3.17", "172.16.0.100", "10.0.3.18",
    "203.0.113.50", "198.51.100.23", "192.168.1.105", "10.0.3.22",
    "172.16.0.55", "192.168.2.200", "10.0.3.99", "203.0.113.12",
    "45.33.32.156",  # suspicious IP for AbuseIPDB demo
    "185.220.101.1",  # known tor exit for demo
]

USER_AGENTS = [
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
    "curl/8.4.0",
    "PostmanRuntime/7.36.0",
    "python-requests/2.31.0",
    "Go-http-client/2.0",
    "okhttp/4.12.0",
]

DB_TABLES = ["users", "orders", "payments", "sessions", "notifications", "products"]

lines = []

def ts(t):
    return t.strftime("%Y-%m-%dT%H:%M:%S.") + f"{t.microsecond // 1000:03d}Z"

def log(t, level, msg, **extra):
    entry = {"timestamp": ts(t), "level": level, "message": msg}
    entry.update(extra)
    lines.append(json.dumps(entry))

def request_id():
    return str(uuid.uuid4())

def rand_ip():
    return random.choice(IPS)

def rand_duration(base_ms=20, spread=200):
    return f"{base_ms + random.randint(0, spread)}ms"

def rand_endpoint():
    method, path, service = random.choice(ENDPOINTS)
    # Replace {id} with a realistic UUID or int
    if "{id}" in path:
        path = path.replace("{id}", str(uuid.uuid4())[:8])
    return method, path, service

# --- Phase 1: Early morning quiet (06:00–07:00) — ~80 lines ---
t = BASE_TIME
for _ in range(80):
    t += timedelta(seconds=random.randint(30, 90))
    method, path, service = rand_endpoint()
    rid = request_id()
    ip = rand_ip()

    if random.random() < 0.05:
        log(t, "warn", f"Slow query on {random.choice(DB_TABLES)} table",
            service=service, duration=f"{random.randint(800, 3000)}ms", request_id=rid)
    elif random.random() < 0.03:
        log(t, "debug", f"Cache miss for key user:{random.randint(1000,9999)}",
            service="user-service", request_id=rid)
    else:
        status = random.choices([200, 201, 204, 301, 404], weights=[70, 10, 5, 3, 2])[0]
        lvl = "info"
        if status == 404:
            lvl = "warn"
        log(t, lvl, f"{method} {path} completed",
            status=status, duration=rand_duration(15, 100), client_ip=ip,
            service=service, request_id=rid)

# --- Gap: 15 min of silence (07:00–07:15) ---
t += timedelta(minutes=15)
log(t, "info", "Health check passed", service="api-gateway", endpoint="/healthz")

# --- Phase 2: Morning ramp-up (07:15–08:30) — ~200 lines ---
for _ in range(200):
    t += timedelta(seconds=random.randint(10, 40))
    method, path, service = rand_endpoint()
    rid = request_id()
    ip = rand_ip()
    ua = random.choice(USER_AGENTS)

    roll = random.random()
    if roll < 0.03:
        log(t, "error", f"Connection refused to downstream service",
            service=service, target=f"{random.choice(SERVICES)}:8080",
            request_id=rid, client_ip=ip)
    elif roll < 0.08:
        log(t, "warn", f"Request timeout after {random.randint(5000, 30000)}ms",
            service=service, method=method, path=path,
            request_id=rid, client_ip=ip)
    elif roll < 0.12:
        log(t, "warn", f"Rate limit approaching for {ip}",
            service="api-gateway", current_rate=f"{random.randint(80, 99)}/100",
            client_ip=ip)
    elif roll < 0.15:
        log(t, "debug", f"JWT token refreshed for user {random.randint(1000, 9999)}",
            service="auth-service", token_expiry="3600s", client_ip=ip)
    elif roll < 0.17:
        log(t, "info", f"New user registered",
            service="user-service", user_id=str(uuid.uuid4()),
            email=f"user{random.randint(100,999)}@example.com", client_ip=ip)
    else:
        status = random.choices([200, 201, 204, 301, 400, 404, 500], weights=[60, 10, 5, 3, 5, 3, 1])[0]
        lvl = "info" if status < 400 else ("warn" if status < 500 else "error")
        log(t, lvl, f"{method} {path} completed",
            status=status, duration=rand_duration(20, 150), client_ip=ip,
            service=service, request_id=rid, user_agent=ua)

# --- Phase 3: Incident begins (08:30–09:00) — ~300 lines, error-heavy ---
t += timedelta(minutes=2)
log(t, "error", "Database connection pool exhausted",
    service="payment-service", pool_size=20, active_connections=20,
    waiting_queries=47, host="db-primary.internal:5432")

t += timedelta(seconds=1)
log(t, "fatal", "Circuit breaker OPEN for payment-service",
    service="api-gateway", failure_rate="0.87", threshold="0.50",
    consecutive_failures=34)

for _ in range(300):
    t += timedelta(seconds=random.randint(2, 12))
    method, path, service = rand_endpoint()
    rid = request_id()
    ip = rand_ip()

    roll = random.random()
    if roll < 0.25:
        log(t, "error", f"Database query timeout after {random.randint(5000, 30000)}ms",
            service=service, query=f"SELECT * FROM {random.choice(DB_TABLES)} WHERE id = $1",
            host="db-primary.internal:5432", request_id=rid)
    elif roll < 0.40:
        log(t, "error", f"{method} {path} failed",
            status=503, duration=rand_duration(5000, 25000), client_ip=ip,
            service=service, request_id=rid,
            error="upstream service unavailable")
    elif roll < 0.50:
        log(t, "error", f"Connection reset by peer",
            service=service, target="db-primary.internal:5432",
            request_id=rid, retry_attempt=random.randint(1, 5))
    elif roll < 0.55:
        log(t, "warn", f"Retry attempt {random.randint(2, 5)} for request {rid}",
            service=service, method=method, path=path, client_ip=ip)
    elif roll < 0.60:
        log(t, "error", f"Payment processing failed",
            service="payment-service", amount=f"${random.randint(10, 500)}.{random.randint(0,99):02d}",
            currency="USD", request_id=rid, client_ip=ip,
            error="database connection timeout")
    elif roll < 0.65:
        log(t, "warn", f"Response queue depth critical",
            service="api-gateway", queue_depth=random.randint(500, 2000),
            max_depth=1000)
    elif roll < 0.68:
        log(t, "error", f"Failed to send notification",
            service="notification-service", type="email",
            recipient=f"user{random.randint(100,999)}@example.com",
            error="SMTP connection timeout", request_id=rid)
    elif roll < 0.70:
        suspicious = random.choice(["45.33.32.156", "185.220.101.1"])
        log(t, "warn", f"Suspicious login attempt from {suspicious}",
            service="auth-service", client_ip=suspicious,
            user="admin", attempts=random.randint(3, 15),
            geo="unknown", user_agent="python-requests/2.31.0")
    else:
        status = random.choices([200, 500, 502, 503, 504], weights=[20, 25, 15, 30, 10])[0]
        lvl = "info" if status < 400 else "error"
        log(t, lvl, f"{method} {path} completed",
            status=status, duration=rand_duration(100, 10000), client_ip=ip,
            service=service, request_id=rid)

# --- Phase 4: Recovery (09:00–09:15) — ~150 lines ---
t += timedelta(seconds=30)
log(t, "info", "Database connection pool recovered",
    service="payment-service", pool_size=20, active_connections=8,
    host="db-primary.internal:5432")

t += timedelta(seconds=5)
log(t, "info", "Circuit breaker CLOSED for payment-service",
    service="api-gateway", failure_rate="0.03", threshold="0.50")

for _ in range(150):
    t += timedelta(seconds=random.randint(5, 25))
    method, path, service = rand_endpoint()
    rid = request_id()
    ip = rand_ip()

    roll = random.random()
    if roll < 0.05:
        log(t, "warn", f"Elevated latency on {random.choice(DB_TABLES)} queries",
            service=service, duration=f"{random.randint(500, 1500)}ms",
            request_id=rid, host="db-primary.internal:5432")
    elif roll < 0.08:
        log(t, "error", f"Stale connection closed",
            service=service, target="db-primary.internal:5432",
            idle_time=f"{random.randint(300, 900)}s")
    elif roll < 0.12:
        log(t, "info", f"Cache warmed for {random.choice(DB_TABLES)} table",
            service=service, entries=random.randint(100, 5000))
    else:
        status = random.choices([200, 201, 204, 301, 404], weights=[70, 10, 5, 3, 2])[0]
        lvl = "info" if status < 400 else "warn"
        log(t, lvl, f"{method} {path} completed",
            status=status, duration=rand_duration(20, 200), client_ip=ip,
            service=service, request_id=rid)

# --- Gap: 20 min quiet (09:15–09:35) ---
t += timedelta(minutes=20)
log(t, "info", "Scheduled job: cleanup expired sessions",
    service="auth-service", deleted=random.randint(50, 300))

t += timedelta(seconds=30)
log(t, "info", "Scheduled job: aggregate daily metrics",
    service="api-gateway", records_processed=random.randint(10000, 50000))

# --- Phase 5: Normal afternoon traffic (09:35–10:00) — ~200 lines ---
for _ in range(200):
    t += timedelta(seconds=random.randint(5, 20))
    method, path, service = rand_endpoint()
    rid = request_id()
    ip = rand_ip()

    roll = random.random()
    if roll < 0.02:
        log(t, "error", f"Unexpected null in response from {random.choice(SERVICES)}",
            service=service, field="user.email", request_id=rid)
    elif roll < 0.05:
        log(t, "warn", f"Deprecated API version called",
            service="api-gateway", version="v0", path=path.replace("v1", "v0"),
            client_ip=ip, user_agent=random.choice(USER_AGENTS))
    elif roll < 0.08:
        log(t, "debug", f"Feature flag evaluated",
            service=service, flag=random.choice(["dark_mode", "new_checkout", "beta_api"]),
            result=random.choice(["true", "false"]), user_id=random.randint(1000, 9999))
    elif roll < 0.10:
        log(t, "trace", f"SQL query executed",
            service=service, table=random.choice(DB_TABLES),
            duration=f"{random.randint(1, 50)}ms", rows=random.randint(0, 100))
    elif roll < 0.12:
        log(t, "info", f"Webhook delivered successfully",
            service="notification-service", url=f"https://hooks.example.com/{uuid.uuid4().hex[:8]}",
            response_code=200, duration=rand_duration(50, 300))
    else:
        status = random.choices([200, 201, 204, 301, 400, 404], weights=[65, 10, 5, 3, 5, 2])[0]
        lvl = "info" if status < 400 else "warn"
        log(t, lvl, f"{method} {path} completed",
            status=status, duration=rand_duration(15, 120), client_ip=ip,
            service=service, request_id=rid)

# --- Write output ---
with open(OUTPUT, "w") as f:
    for line in lines:
        f.write(line + "\n")

print(f"Generated {len(lines)} lines to {OUTPUT}")
