Benchmarks and memory verification for flamegraph SVG generation

Run criterion benchmarks:

```bash
cd cli
cargo bench
```

This will run two benches: `generate_old` and `generate_builder` comparing the original formatting-heavy implementation and the new builder-based implementation.

Memory verification helper (prints RSS in KB):

```bash
cd cli
cargo run --bin verify_flamegraph_memory
```

On Windows you can monitor process memory with Task Manager or use this helper which samples process memory after each iteration.

Notes:
- The benches create temporary files and read them back to ensure generation completed.
- For CI, consider running `cargo bench -- --quiet` and collecting the output artifacts.
