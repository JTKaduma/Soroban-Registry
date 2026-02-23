/// Database writer module
/// Handles writing detected contracts to the database

use shared::{Contract, Network};
use sqlx::{PgPool, Row};
use thiserror::Error;
use uuid::Uuid;
use tracing::{debug, error, info};
use crate::rpc::ContractDeployment;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    SqlError(String),
    #[error("Contract already exists: {0}")]
    DuplicateContract(String),
}

/// Database writer for storing discovered contracts
pub struct DatabaseWriter {
    pool: PgPool,
}

impl DatabaseWriter {
    /// Create new database writer
    pub fn new(pool: PgPool) -> Self {
        DatabaseWriter { pool }
    }

    /// Write discovered contract to database
    /// Returns true if new contract was inserted, false if already existed
    pub async fn write_contract(
        &self,
        deployment: &ContractDeployment,
        network: &Network,
    ) -> Result<bool, DatabaseError> {
        debug!(
            "Writing contract to database: contract_id={}, network={:?}",
            deployment.contract_id, network
        );

        let network_str = network_to_str(network);

        // Check if contract already exists
        let existing = sqlx::query(
            r#"
            SELECT id FROM contracts
            WHERE contract_id = $1 AND network = $2::network_type
            LIMIT 1
            "#,
        )
        .bind(&deployment.contract_id)
        .bind(network_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to check for existing contract: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        if existing.is_some() {
            debug!(
                "Contract already exists in database: {}",
                deployment.contract_id
            );
            return Ok(false);
        }

        // Create a publisher record for the deployer if it doesn't exist
        let publisher_id = self
            .get_or_create_publisher(&deployment.deployer)
            .await?;

        // Insert new contract with is_verified = false
        let contract_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        sqlx::query(r#"
            INSERT INTO contracts (
                id,
                contract_id,
                wasm_hash,
                name,
                publisher_id,
                network,
                is_verified,
                created_at,
                updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6::network_type, $7, $8, $9)
        "#)
            .bind(contract_id)
            .bind(&deployment.contract_id)
            .bind(format!("{}_{}", deployment.contract_id, deployment.op_id))
            .bind(&deployment.contract_id)
            .bind(publisher_id)
            .bind(network_str)
            .bind(false)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "Failed to insert contract record: {} ({})",
                    deployment.contract_id, e
                );
                DatabaseError::SqlError(e.to_string())
            })?;

        info!(
            "Contract record created: contract_id={}, network={}, publisher={}",
            deployment.contract_id, network_str, deployment.deployer
        );

        Ok(true)
    }

    /// Write multiple contracts in a single transaction
    pub async fn write_contracts_batch(
        &self,
        deployments: &[ContractDeployment],
        network: &Network,
    ) -> Result<(usize, usize), DatabaseError> {
        let mut new_count = 0;
        let mut duplicate_count = 0;

        for deployment in deployments {
            match self.write_contract(deployment, network).await {
                Ok(true) => new_count += 1,
                Ok(false) => duplicate_count += 1,
                Err(e) => {
                    error!("Failed to write contract: {}, error: {}", deployment.contract_id, e);
                    // Continue with next contract, don't fail the entire batch
                }
            }
        }

        info!(
            "Batch write complete: new={}, duplicates={}",
            new_count, duplicate_count
        );

        Ok((new_count, duplicate_count))
    }

    /// Get or create a publisher record for a deployer address
    async fn get_or_create_publisher(&self, address: &str) -> Result<Uuid, DatabaseError> {
        debug!("Getting or creating publisher for address: {}", address);

        // Try to find existing publisher
        let existing = sqlx::query(
            r#"
            SELECT id FROM publishers
            WHERE stellar_address = $1
            LIMIT 1
            "#,
        )
        .bind(address)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query publishers: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        if let Some(row) = existing {
            let id_bytes: Vec<u8> = row.try_get("id").map_err(|e| {
                DatabaseError::SqlError(format!("Failed to extract publisher id: {}", e))
            })?;
            let id = Uuid::from_slice(&id_bytes).map_err(|e| {
                DatabaseError::SqlError(format!("Failed to parse publisher uuid: {}", e))
            })?;
            debug!("Found existing publisher: {}", address);
            return Ok(id);
        }

        // Create new publisher
        let publisher_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO publishers (id, stellar_address, created_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (stellar_address) DO UPDATE
            SET id = EXCLUDED.id
            "#,
        )
        .bind(publisher_id)
        .bind(address)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to create publisher: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        debug!("Created new publisher: {} ({})", address, publisher_id);

        Ok(publisher_id)
    }

    /// Get recently indexed contracts (for verification)
    pub async fn get_recent_contracts(
        &self,
        network: &Network,
        limit: i32,
    ) -> Result<Vec<Contract>, DatabaseError> {
        let network_str = network_to_str(network);

        let rows = sqlx::query_as::<_, Contract>(
            r#"
            SELECT 
                id, contract_id, wasm_hash, name, description,
                publisher_id, network, is_verified, category, tags,
                created_at, updated_at
            FROM contracts
            WHERE network = $1::network_type AND is_verified = false
            ORDER BY created_at DESC
            LIMIT $2
            "#
        )
        .bind(network_str)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch recent contracts: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        debug!("Fetched {} recent unverified contracts", rows.len());

        Ok(rows)
    }

    /// Check if a contract exists
    pub async fn contract_exists(
        &self,
        contract_id: &str,
        network: &Network,
    ) -> Result<bool, DatabaseError> {
        let network_str = network_to_str(network);

        let result = sqlx::query(
            r#"
            SELECT id FROM contracts
            WHERE contract_id = $1 AND network = $2::network_type
            LIMIT 1
            "#,
        )
        .bind(contract_id)
        .bind(network_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to check contract existence: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        Ok(result.is_some())
    }
}

/// Convert Network enum to string for database queries
fn network_to_str(network: &Network) -> &str {
    match network {
        Network::Mainnet => "mainnet",
        Network::Testnet => "testnet",
        Network::Futurenet => "futurenet",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_to_str() {
        assert_eq!(network_to_str(&Network::Mainnet), "mainnet");
        assert_eq!(network_to_str(&Network::Testnet), "testnet");
        assert_eq!(network_to_str(&Network::Futurenet), "futurenet");
    }
}
