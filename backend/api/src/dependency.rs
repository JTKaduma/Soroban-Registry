use shared::{ContractDependency, DependencyDeclaration, DependencyTreeNode, GraphEdge, GraphNode, GraphResponse};
use uuid::Uuid;
use std::collections::{HashMap, HashSet, VecDeque};
use sqlx::PgPool;
use anyhow::Result;
use crate::error::ApiError;

/// Detect dependencies from a contract ABI JSON
pub fn detect_dependencies_from_abi(abi_json: &serde_json::Value) -> Vec<DependencyDeclaration> {
    let mut dependencies = Vec::new();
    let mut seen = HashSet::new();

    // In Soroban, external contract calls often use a 'Client' or are defined in the spec.
    // We look for 'custom' types that might represent other contracts, 
    // or specific annotations if they exist.
    // For now, we'll scan for common patterns.
    
    if let Some(specs) = abi_json.as_array() {
        for spec in specs {
            // Look for interface/contract client definitions
            if let Some(spec_type) = spec.get("type").and_then(|t| t.as_str()) {
                if spec_type == "contract_spec_interface" || spec_type == "interface" {
                    if let Some(name) = spec.get("name").and_then(|n| n.as_str()) {
                        if seen.insert(name.to_string()) {
                            dependencies.push(DependencyDeclaration {
                                name: name.to_string(),
                                version_constraint: "*".to_string(), // Default constraint
                            });
                        }
                    }
                }
            }
            
            // Scan function inputs/outputs for custom types that might be contract IDs?
            // Actually, usually they are addressed by Symbol or Address.
            // But sometimes the 'type' field in the spec itself points to another contract.
        }
    }

    dependencies
}

/// Calculate transitive closure of dependencies (all recursive dependencies)
pub async fn get_transitive_dependencies(
    pool: &PgPool,
    root_id: Uuid,
) -> Result<Vec<Uuid>> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(root_id);
    visited.insert(root_id);

    let mut result = Vec::new();

    while let Some(current_id) = queue.pop_front() {
        let deps: Vec<Uuid> = sqlx::query_scalar(
            "SELECT dependency_contract_id FROM contract_dependencies WHERE contract_id = $1 AND dependency_contract_id IS NOT NULL"
        )
        .bind(current_id)
        .fetch_all(pool)
        .await?;

        for dep_id in deps {
            if !visited.contains(&dep_id) {
                visited.insert(dep_id);
                queue.push_back(dep_id);
                result.push(dep_id);
            }
        }
    }

    Ok(result)
}

/// Calculate transitive closure of dependents (all contracts affected by this one)
pub async fn get_transitive_dependents(
    pool: &PgPool,
    root_id: Uuid,
) -> Result<Vec<Uuid>> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(root_id);
    visited.insert(root_id);

    let mut result = Vec::new();

    while let Some(current_id) = queue.pop_front() {
        let dependents: Vec<Uuid> = sqlx::query_scalar(
            "SELECT contract_id FROM contract_dependencies WHERE dependency_contract_id = $1"
        )
        .bind(current_id)
        .fetch_all(pool)
        .await?;

        for dep_id in dependents {
            if !visited.contains(&dep_id) {
                visited.insert(dep_id);
                queue.push_back(dep_id);
                result.push(dep_id);
            }
        }
    }

    Ok(result)
}

/// Detect if adding a dependency would create a cycle
pub async fn detect_cycle(
    pool: &PgPool,
    start_node: Uuid,
    potential_dep: Uuid,
) -> Result<bool> {
    if start_node == potential_dep {
        return Ok(true);
    }

    // If potential_dep already depends on start_node (directly or indirectly), adding start_node -> potential_dep creates a cycle
    let transitive_deps = get_transitive_dependencies(pool, potential_dep).await?;
    Ok(transitive_deps.contains(&start_node))
}

/// Build D3-compatible graph representation
pub async fn build_dependency_graph(pool: &PgPool) -> Result<GraphResponse> {
    let contracts: Vec<GraphNode> = sqlx::query_as(
        "SELECT id, contract_id, name, network, is_verified, category, tags FROM contracts"
    )
    .fetch_all(pool)
    .await?;

    let edges: Vec<GraphEdge> = sqlx::query_as(
        "SELECT contract_id as source, dependency_contract_id as target, 'calls' as dependency_type 
         FROM contract_dependencies 
         WHERE dependency_contract_id IS NOT NULL"
    )
    .fetch_all(pool)
    .await?;

    Ok(GraphResponse {
        nodes: contracts,
        edges,
    })
}

/// Resolve a dependency name/id to a contract UUID if it exists in the registry
pub async fn resolve_contract_id(pool: &PgPool, identifier: &str) -> Result<Option<Uuid>> {
    // Try UUID first
    if let Ok(id) = Uuid::parse_str(identifier) {
        return Ok(Some(id));
    }

    // Try contract_id (public key)
    let id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM contracts WHERE contract_id = $1"
    )
    .bind(identifier)
    .fetch_optional(pool)
    .await?;

    if id.is_some() {
        return Ok(id);
    }

    // Try name
    let id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM contracts WHERE name = $1"
    )
    .bind(identifier)
    .fetch_optional(pool)
    .await?;

    Ok(id)
}

/// Save dependencies for a contract, resolving them if possible
pub async fn save_dependencies(
    pool: &PgPool,
    contract_id: Uuid,
    decls: &[DependencyDeclaration],
) -> Result<()> {
    // Clear existing dependencies (optional, depends on if we want to merge or replace)
    sqlx::query("DELETE FROM contract_dependencies WHERE contract_id = $1")
        .bind(contract_id)
        .execute(pool)
        .await?;

    for decl in decls {
        let dep_contract_id = resolve_contract_id(pool, &decl.name).await?;

        if let Some(dep_id) = dep_contract_id {
            if detect_cycle(pool, contract_id, dep_id).await.unwrap_or(false) {
                tracing::warn!(
                    "Circular dependency detected: contract {} -> {}",
                    contract_id,
                    dep_id
                );
            }
        }

        sqlx::query(
            "INSERT INTO contract_dependencies (contract_id, dependency_name, dependency_contract_id, version_constraint) 
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (contract_id, dependency_name) DO UPDATE SET 
                dependency_contract_id = EXCLUDED.dependency_contract_id,
                version_constraint = EXCLUDED.version_constraint"
        )
        .bind(contract_id)
        .bind(&decl.name)
        .bind(dep_contract_id)
        .bind(&decl.version_constraint)
        .execute(pool)
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_dependencies() {
        let abi = json!([
            {
                "type": "interface",
                "name": "TokenInterface"
            },
            {
                "type": "function",
                "name": "hello"
            }
        ]);

        let deps = detect_dependencies_from_abi(&abi);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "TokenInterface");
    }

    #[test]
    fn test_detect_dependencies_duplicate() {
        let abi = json!([
            { "type": "interface", "name": "Auth" },
            { "type": "interface", "name": "Auth" }
        ]);

        let deps = detect_dependencies_from_abi(&abi);
        assert_eq!(deps.len(), 1);
    }
}
