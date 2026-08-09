# Scanner cold-path profile

Profile captured on 2026-08-08 on the project development machine (Apple M3
Pro, macOS). Divan used its 41 ns timer and collected 100 samples per workload.
The benchmark scans deterministic small, 1,000-record, and 10,000-record YAML
inputs.

```bash
cargo bench -p yamalgam-bench --bench comparative -- yamalgam_scan
```

## Baseline

| Workload | Run 1 | Run 2 | Run 3 |
|----------|------:|------:|------:|
| small scan | 29.64 us | 29.18 us | 29.16 us |
| medium scan | 1.141 ms | 1.135 ms | 1.178 ms |
| large scan | 12.99 ms | 13.09 ms | 13.00 ms |

Optimized ThinLTO code in the benchmark executable inlined
`scan_to_next_token` into `fetch_next_token`. The latter occupied 28,344 bytes,
measured from its symbol address to the next text symbol with `nm -nm`. The
three static diagnostic allocations identified by the audit were therefore in
the hot scanner body.

## Targeted cold boundary

A `#[cold]` and `#[inline(never)]` helper now owns the three static error
allocations in `scan_to_next_token`. Optimized code places that helper in a
separate 132-byte symbol. The containing `fetch_next_token` body is 28,052
bytes, 292 bytes smaller. The two symbols use 160 fewer text bytes in total.

| Workload | Run 1 | Run 2 | Run 3 | Confirmation |
|----------|------:|------:|------:|-------------:|
| small scan | 29.70 us | 29.33 us | 28.33 us | 29.35 us |
| medium scan | 1.117 ms | 1.070 ms | 1.090 ms | 1.146 ms |
| large scan | 12.89 ms | 12.30 ms | 12.32 ms | 13.21 ms |

The wall-clock distributions overlap and do not support a throughput claim.
The generated layout change is direct and repeatable: rare string allocation
left the hot symbol with a net text-size reduction and no stable slowdown.

## Rejected expansion

A broader prototype routed every static scanner error through the helper. It
caused LLVM to change its inlining decisions: `fetch_next_token` grew to 30,796
bytes, 8.7% above baseline. That expansion was discarded. The committed scope
is limited to the three diagnostics named by the finding and supported by the
optimized layout evidence.
