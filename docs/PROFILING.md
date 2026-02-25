# Profiling Tools & Guidelines

When performance metrics fall below our baselines, use these profiling tools to identify bottlenecks.

## 1. CPU Profiling with Flamegraphs
Flamegraphs visualize where your code spends the most CPU time. 

**Generation Command:**
```bash
cargo install flamegraph
cargo flamegraph --bin soroban-registry
```

## 2. Benchmarking with Criterion
Do not guess if a code change is faster; measure it using the `criterion` crate for micro-benchmarking. Run benchmarks via `cargo bench`.

## 3. Memory Profiling 
Memory leaks degrade performance over time.
* **Heaptrack:** Excellent for tracking memory allocations in Rust binaries.
* **Valgrind (Massif):** Measures heap memory usage over the lifetime of the program.

## 4. PR Checklist Addition
When submitting code, include the following in your PR description:
- [ ] Checked for N+1 query issues.
- [ ] Database migrations include appropriate indexes.
- [ ] `cargo bench` run confirms no performance regression.
- [ ] Caching considered for new read-heavy endpoints.