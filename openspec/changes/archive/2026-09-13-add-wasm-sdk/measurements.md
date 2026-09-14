# Measurements

Taken on the change branch after the whole change landed. Machine: WSL2, Linux 6.18, Node 24.15,
Chromium via Playwright 1.62. The site was the built image (`docker run -p 8099:8080 nx-playground`),
so every number is against what deploys, not against the dev server.

## Module size

| | Raw | Gzipped |
|---|---|---|
| `nx.wasm` (the compiler and language service) | 2,049,345 B (1.95 MiB) | 681,395 B (665 KiB) |
| `nx.worker-*.js` (the worker shell and loader) | 44,635 B | 12,080 B |

The design expected **under 3 MB raw**. It is 1.95 MiB, comfortably inside, and close to the spike's
2.6 MB before the `wasm-release` profile's symbol stripping. No `wasm-opt`; that remains available.

## Compile latency

Through `compileWithCatalog` over the wasm host, under Node, against the site's own catalog. Seven
compiles per example; "cold" is the first, "median" is the median of the rest.

| Example | Source | Cold | Median | NX IR |
|---|---|---|---|---|
| `layouts.nx` (largest) | 23,469 B | 106.3 ms | 55.3 ms | 1,386 KiB |
| `scroll.nx` | 18,778 B | 45.3 ms | 45.6 ms | 1,264 KiB |
| `shapes.nx` | 12,765 B | 36.0 ms | 35.9 ms | 1,183 KiB |

The design expected **under 200 ms** for the largest example. The largest is 106 ms cold and 55 ms
warm, inside it either way.

Creating a host from the already-compiled module — what recovery from a trap costs — is **1.7 ms**,
well under the 12 to 21 ms the spike measured. Memory settles at **31 MiB** after twenty-one
compiles of the three largest examples and does not grow with further ones (the SDK's own test
proves that over a thousand calls).

## First editor view

Time from navigation to the first compiled drawing, on `/playground/shapes`:

| Connection | Compiler module arrives | First compiled drawing |
|---|---|---|
| Unthrottled (localhost) | 412 ms | 873 ms |
| Chrome "Fast 3G" (1.6 Mbit/s, 150 ms RTT) | 73.5 s | 83.0 s |

**The throttled number is a deviation worth reading carefully, and it is not the compiler's.** The
editor view transfers **17.23 MiB** in 20 responses, and the origin sends every byte uncompressed —
compression is Cloudflare's job in production, and there is no edge in front of a local container.
The compiler module is 1.96 MiB of that 17.23 MiB, **11%**:

| Asset | Transferred |
|---|---|
| `canvaskit-*.wasm` | 7.86 MiB |
| `index-*.js` (the app, Monaco, DrawnUI, Shiki) | 4.73 MiB |
| **`nx-*.wasm` (this change)** | **1.96 MiB** |
| `NotoColorEmoji-Subset.ttf` | 0.88 MiB |
| everything else (17 responses) | 1.80 MiB |

So on a slow connection the editor was already dominated by CanvasKit and the application bundle;
this change adds about an eighth to that, and removes a network round trip from every compile
afterwards. Behind Cloudflare the module is served gzipped (665 KiB) and `immutable`, and it is a
cache hit for every visitor after the first in a region.

The design's own expectation here was qualitative — "started when the editor mounts rather than on
the gallery" — and that is where the shipped behaviour differs from the design; see the deviation
note in the change's tasks. The gallery draws a live preview of every example, so it compiles too
and pulls the module as its previews compile, just after first paint rather than before it.

## What the numbers do not cover

- A real trap in a browser. The SDK's trap tests force one against a module built with the crate's
  `debug-trap` feature and prove the crashed-host contract; the worker's session test proves the
  replacement. The shipped module exports nothing that traps on purpose, so the browser check
  covered a failed module load and an overrun deadline instead, both read from the diagnostics pane.
- Production timings behind Cloudflare. Those need a deploy.
