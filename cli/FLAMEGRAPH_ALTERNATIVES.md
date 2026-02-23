This document lists alternative libraries and tools for generating flamegraphs and visualizing profiling data, with short notes and integration suggestions.

- Inferno (Rust): A Rust crate for generating flame graphs and collapsed stacks. Good for native Rust profiling and producing standard SVG flamegraphs. Use when you have aggregated stack traces and want a well-tested renderer.

- speedscope (speedscope.app / JSON): Export profile data to speedscope's JSON format and view interactively in the speedscope web UI or VS Code extension. Useful for interactive exploration and large traces.

- Brendan Gregg's FlameGraph (Perl scripts): The original toolchain to collapse stacks and generate flamegraphs. Works well with perf and other trace producers; requires stack collapse preprocessing.

- Hotspot / Flame (JavaScript tools): For web-oriented flamegraphs, consider libraries that render in-browser (d3-based) for interactive zooming and panning. Export a JSON structure and use the JS renderer.

Integration notes
- Precompute collapsed stack texts (name and total samples) and feed into renderers that accept the collapsed format (Inferno, FlameGraph scripts).
- For interactive workflows prefer `speedscope` JSON export; for static reports prefer SVG via Inferno or FlameGraph.
- If memory or allocation pressure is a concern, stream output (write to file progressively) instead of building giant strings in memory.

Links
- Inferno crate (Rust) — search `inferno` on crates.io
- speedscope — https://www.speedscope.app/
- FlameGraph scripts — https://github.com/brendangregg/FlameGraph

