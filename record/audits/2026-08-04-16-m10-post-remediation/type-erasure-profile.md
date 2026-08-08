# Parser-consumer type-erasure profile

Profile captured on 2026-08-08 on the project development machine (Apple M3
Pro, macOS) using the release benchmark profile and Divan's 41 ns timer. Each
run collected 100 samples. Inputs were the deterministic benchmark crate's
Kubernetes-style document and generated 1,000-record document.

```bash
cargo bench -p yamalgam-bench --bench dispatch
```

All values below are medians. The typed prototype duplicated
`ResolvedEvents` behavior while retaining a concrete upstream iterator and the
same `NoopResolver`, `LoaderConfig`, buffering, and error termination.

| Workload | Run 1 | Run 2 | Run 3 |
|----------|------:|------:|------:|
| small events, boxed | 51.45 us | 51.29 us | 54.12 us |
| small events, typed | 48.81 us | 48.99 us | 50.87 us |
| medium events, boxed | 2.211 ms | 2.212 ms | 2.203 ms |
| medium events, typed | 2.144 ms | 2.127 ms | 2.147 ms |
| small composer, boxed | 75.62 us | 77.27 us | 75.83 us |
| small composer, typed | 71.79 us | 74.29 us | 72.04 us |
| medium composer, boxed | 3.222 ms | 3.262 ms | 3.250 ms |
| medium composer, typed | 3.127 ms | 3.183 ms | 3.106 ms |
| concrete tag resolution | 123.3 ns | 120.8 ns | 124.7 ns |
| dynamic tag resolution | 122.0 ns | 119.5 ns | 128.6 ns |

The event-adapter improvement repeats at 4.5–6.0% for the small document and
2.5–3.8% for the medium document. End-to-end composition repeats at 3.9–5.1%
and 2.4–4.4%, respectively. Dynamic tag resolution shows no consistent penalty
at this scale, so the tag-resolver box remains.

Current erased serde baselines were 93.64, 94.12, and 94.22 us for the small
document and 4.159, 4.136, and 4.134 ms for the medium document. They establish
repeatable production workloads but do not justify a generic public
deserializer without an equivalent measured implementation.
