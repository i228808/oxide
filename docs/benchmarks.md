# Benchmarks

Oxide is a thin abstraction over Axum. These benchmarks quantify the exact cost of the framework's middleware stack (state injection, panic recovery, rate limiting, CORS, request timeout) compared to bare Axum.

This page reflects benchmark code in:

- `oxide-framework-core/benches/overhead.rs`
- `oxide-framework-core/examples/loadtest.rs`
- `oxide-framework-core/examples/bench_raw_axum.rs`
- `oxide-framework-core/examples/bench_oxide.rs`

## Running Benchmarks

### Criterion (micro-benchmarks)

```bash
cargo bench -p oxide-framework-core --bench overhead
```

Measures per-request latency with statistical rigor (100-500 samples per benchmark). Results in `target/criterion/`.

### Load Test (sustained throughput)

```bash
cargo run -p oxide-framework-core --release --example loadtest
```

Configurable via environment variables:

```bash
# 30 seconds, 100 concurrent connections
DURATION=30 CONCURRENCY=100 cargo run -p oxide-framework-core --release --example loadtest
```

### External Tools (wrk / k6)

Start the comparison servers:

```bash
# Terminal 1 — bare Axum on port 3001
cargo run -p oxide-framework-core --release --example bench_raw_axum

# Terminal 2 — Oxide (full middleware) on port 3002
cargo run -p oxide-framework-core --release --example bench_oxide
```

Then benchmark with wrk:

```bash
# Bare Axum
wrk -t4 -c100 -d30s http://127.0.0.1:3001/json

# Oxide (full middleware stack)
wrk -t4 -c100 -d30s http://127.0.0.1:3002/json

# Oxide (controller route)
wrk -t4 -c100 -d30s http://127.0.0.1:3002/api/json
```

Or with k6:

```javascript
import http from 'k6/http';
export default function () {
  http.get('http://127.0.0.1:3002/json');
}
```

```bash
k6 run --vus 100 --duration 30s script.js
```

## Baseline Results (Example)

Machine: Windows 10, AMD Ryzen / Intel i7 (results vary by hardware, Rust
version, and benchmark sample sizes).

### Criterion — Per-Request Latency (single request, no concurrency)

| Benchmark | Raw Axum | Oxide (minimal) | Oxide (full stack) | Oxide (controller) |
|---|---|---|---|---|
| **GET /json** | ~84 µs | ~87 µs (+3%) | ~97 µs (+15%) | ~100 µs (+19%) |
| **GET /users/42** | ~85 µs | — | ~104 µs (+22%) | — |
| **POST /create** | ~99 µs | — | ~114 µs (+15%) | — |

*Full stack = state injection + panic recovery + rate limiting + CORS + request timeout.*

The ~15-20% overhead includes **five middleware layers** wrapping every request. On absolute terms, Oxide adds roughly **13-19 µs** per request compared to bare Axum.

### Criterion — In-Process (no network, oneshot)

| Benchmark | Latency |
|---|---|
| Raw Axum GET /json | ~1.6 µs |
| Raw Axum GET /users/42 | ~2.0 µs |
| Raw Axum POST /create | ~2.7 µs |

These are the pure routing + serialization cost with no network overhead. Useful as a baseline for understanding how much is network vs framework.

### Criterion — Concurrent Throughput (batch of N parallel requests)

| Concurrency | Raw Axum | Oxide (full) | Overhead |
|---|---|---|---|
| 10 | ~308 µs | ~331 µs | +7% |
| 50 | ~1.9 ms | ~1.3 ms | -36% (faster)* |
| 100 | ~2.4 ms | ~2.4 ms | ~0% |

*At higher concurrency, Oxide's connection pooling and middleware pipeline can amortize overhead. Variance is high in these benchmarks — run them yourself for hardware-specific numbers.*

### Load Test — Sustained Throughput (5 seconds, 50 concurrent)

| Server | Requests | req/s | avg | p50 | p95 | p99 | errors |
|---|---|---|---|---|---|---|---|
| Raw Axum | 210K | ~42K | 0.49ms | 0.33ms | 1.38ms | 1.94ms | 0 |
| Oxide (minimal) | 213K | ~43K | 0.26ms | 0.24ms | 0.43ms | 0.59ms | 0 |
| Oxide (full middleware) | 206K | ~41K | 0.30ms | 0.27ms | 0.50ms | 0.89ms | 0 |
| Oxide (controller + full) | 199K | ~40K | 0.31ms | 0.28ms | 0.53ms | 1.02ms | 0 |

**Key takeaway (from this sample run)**: Oxide with the full middleware stack
can sustain roughly **~40K req/s** at 50 concurrency and may land within a small
single-digit percentage of raw Axum throughput on similar hardware.

## What the Benchmarks Test

### Oxide's Middleware Stack (full)

Every request passes through:

1. **State injection** — clones `Arc<AppState>`, inserts into request extensions
2. **Panic recovery** — `CatchPanicLayer` wraps the handler
3. **Rate limiting** — per-IP HashMap lookup + atomic counter
4. **Request timeout** — `tokio::time::timeout` wrapper
5. **CORS** — header injection on every response

### Endpoints

All endpoints return the same JSON shape to ensure a fair comparison:

```json
{"status": 200, "data": {"text": "hello"}}
```

- `GET /json` — simple JSON response
- `GET /users/{id}` — path parameter extraction + dynamic JSON
- `POST /create` — JSON body deserialization + 201 response

## Interpreting Results

- **< 5% overhead** at sustained throughput → the abstraction is essentially free under real workloads
- **~15-20% overhead** per-request in micro-benchmarks → expected for 5 middleware layers; most of this is the rate limiter's HashMap lock
- **Zero errors** in these sampled runs → indicates healthy behavior for this benchmark setup
- **Controllers add ~3-5%** on top of functional handlers → the `Arc<Self>` clone per request is near-zero cost

## Tips for Production

1. **Rate limiter** is the most expensive middleware (~5-10 µs). If you're behind a reverse proxy with its own rate limiting, disable it for better throughput.
2. **Request logging** (disabled in benchmarks) adds ~2-5 µs per request for tracing. Leave it on in production — the observability is worth it.
3. **Build with `--release`** — debug builds are 10-50x slower due to lack of optimizations.

## Reproducibility Notes

To compare results over time, record:

- Rust toolchain version (`rustc --version`)
- CPU model + core/thread count
- OS + power mode
- benchmark command and duration/concurrency settings

Treat percent deltas as directional unless runs are repeated and averaged.

