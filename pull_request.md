Pull Request: Contract Dependency Analysis and Visualization

This pull request introduces a system for analyzing and visualizing dependencies between Soroban contracts. It's designed to help developers understand how their contracts interact and to safely manage updates across the ecosystem.

The core of this update is the automated detection of dependencies. When a new contract version is published, the registry now scans its ABI to find any interface or client declarations. These relationships are stored and used to build a comprehensive dependency graph.

For the API, I've added several new endpoints:
- A dependency lookup to see what a contract calls.
- A dependent lookup to see what other contracts rely on it.
- An impact analysis tool that shows the full ripple effect of a contract change.
- A global graph endpoint that exports data ready for D3.js visualization.

To keep things fast, I've integrated these lookups with our existing caching system. I've also added a cycle detection feature that will flag any circular dependencies during the publication process to prevent potential deployment issues.

Testing included unit checks for the ABI parsing logic and manual verification of the graph data structure and cache performance.

#253
