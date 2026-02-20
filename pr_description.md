# PR Description: Contract Popularity Ranking

## Summary
This PR implements a popularity ranking system for smart contracts, allowing users to discover trending contracts based on recent activity. It introduces a `popularity_score` for each contract, which is recalculated hourly based on deployments, interactions, verification status, and contract age.

## Key Features
- **Database Migration**: Added `popularity_score` and `score_updated_at` columns to the `contracts` table (Migration `003`).
- **Scoring Engine**: Implemented `popularity.rs` with a weighted scoring algorithm:
  - **Deployments (40%)**: Time-decayed count of deployments.
  - **Interactions (30%)**: Time-decayed count of interactions.
  - **Verification (20%)**: Bonus for verified contracts.
  - **Age (10%)**: Boost for newer contracts (exponential decay).
- **Time Decay**: Activity metrics decay over time (default 7 days) to ensure the "trending" list reflects recent usage.
- **API Endpoint**: Added `GET /api/contracts/trending` to fetch top contracts sorted by popularity score.
- **Background Job**: An hourly task automatically updates scores for all contracts.

## Fixes & Improvements
- **Compilation Fixes**: Resolved pre-existing compilation errors in `api` and `shared` crates:
  - Added missing types (`UpdateMigrationStatusRequest`, `TrendingParams`, `TrendingContract`).
  - Fixed `page_size` undefined variable in `list_contracts`.
  - Standardized error return types (`ApiResult` vs `Result<T, StatusCode>`) in multiple handlers.
  - Added missing `PartialEq` and `Display` implementations for deployment enums.
  - Fixed `verifier` crate workspace dependencies.

## API Changes
- **New Endpoint**: `GET /api/contracts/trending?limit=10&timeframe=7d`
- **Response**: Returns a list of `TrendingContract` objects containing contract details + popularity metrics.

## Verification
- Run `cargo check --package api` to verify compilation.
- The background scoring task is spawned in `main.rs` upon server start.
