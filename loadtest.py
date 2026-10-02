#!/usr/bin/env python3
"""Hammer one HTTP endpoint with N concurrent connections for T seconds.
Usage: loadtest.py <url> <method> [body_file] [duration_s] [concurrency]
Reports req/s, p50/p99 latency ms. Uses one persistent connection per worker thread.
"""
import json, statistics, sys, time, urllib.request

def main():
    url = sys.argv[1]
    method = sys.argv[2].upper()
    body = None
    idx = 3
    if method == "POST":
        body = open(sys.argv[3], "rb").read()
        idx = 4
    duration = float(sys.argv[idx]) if len(sys.argv) > idx else 15.0
    conc = int(sys.argv[idx + 1]) if len(sys.argv) > idx + 1 else 50

    from concurrent.futures import ThreadPoolExecutor

    latencies = []
    errors = [0]
    stop_at = time.time() + duration

    def worker():
        # one persistent connection per thread
        import http.client
        from urllib.parse import urlparse
        u = urlparse(url)
        conn = http.client.HTTPConnection(u.hostname, u.port, timeout=30)
        path = u.path or "/"
        if u.query:
            path += "?" + u.query
        headers = {}
        if body:
            headers["Content-Type"] = "application/json"
            headers["Content-Length"] = str(len(body))
        local = []
        while time.time() < stop_at:
            t0 = time.perf_counter()
            try:
                conn.request(method, path, body=body, headers=headers)
                r = conn.getresponse()
                r.read()
                if r.status != 200:
                    errors[0] += 1
                else:
                    local.append((time.perf_counter() - t0) * 1000.0)
            except Exception:
                errors[0] += 1
                try:
                    conn.close()
                except Exception:
                    pass
                conn = http.client.HTTPConnection(u.hostname, u.port, timeout=30)
        return local

    t0 = time.time()
    with ThreadPoolExecutor(max_workers=conc) as ex:
        results = list(ex.map(lambda _: worker(), range(conc)))
    wall = time.time() - t0
    for local in results:
        latencies.extend(local)
    latencies.sort()
    n = len(latencies)
    p50 = latencies[n // 2] if n else float("nan")
    p99 = latencies[int(n * 0.99)] if n else float("nan")
    print(json.dumps({
        "url": url, "method": method,
        "concurrency": conc, "wall_s": round(wall, 2),
        "requests": n, "errors": errors[0],
        "req_per_s": round(n / wall, 1),
        "p50_ms": round(p50, 3), "p99_ms": round(p99, 3),
    }, indent=2))

main()
