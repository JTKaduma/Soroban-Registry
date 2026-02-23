## New Feature: Automated Contract Health Scoring (#246)

Implemented a comprehensive health scoring system to track the quality, activity, and security of Soroban contracts.

- **Dynamic Scoring Engine**: Calculates a 0-100 score based on verification status (40%), deployment activity (20%), update frequency (20%), and security scan results (10%).
- **Daily Automation**: Integrated a daily background job into the aggregation worker that triggers at 2 AM UTC to ensure scores stay current.
- **Security Deductions**: Automatically penalizes contracts with critical or high-severity CVE scan results.
- **Abandonment Detection**: Identifies and flags abandoned contracts (no updates in >1 year) with a maintenance penalty.
- **API Integration**: Health scores are now reflected in all contract listings and detail endpoints.

#### Key Files
- [health.rs](file:///c:/Users/hp/Downloads/Soroban-Registry/backend/api/src/health.rs): Core scoring logic and bulk update job.
- [aggregation.rs](file:///c:/Users/hp/Downloads/Soroban-Registry/backend/api/src/aggregation.rs): background task integration.
- [models.rs](file:///c:/Users/hp/Downloads/Soroban-Registry/backend/shared/src/models.rs): Updated shared `Contract` struct.
- [038_add_health_score.sql](file:///c:/Users/hp/Downloads/Soroban-Registry/database/migrations/038_add_health_score.sql): Database schema migration.

Fixes #246
