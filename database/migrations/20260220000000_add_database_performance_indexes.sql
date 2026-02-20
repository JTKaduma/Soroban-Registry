-- no-transaction
-- Add performance indexes for high-traffic contract and verification queries.

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_contracts_publisher_network
    ON contracts (publisher_id, network);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_contracts_created
    ON contracts (created_at DESC);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_versions_contract_id
    ON contract_versions (contract_id);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_status
    ON verifications (status);
