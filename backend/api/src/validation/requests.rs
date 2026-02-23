//! Validation implementations for API request types
//!
//! This module implements the `Validatable` trait for all request types
//! that need validation when received from clients.

use shared::models::{
    CreateMigrationRequest, DependencyDeclaration, PublishRequest, UpdateMigrationStatusRequest,
    VerifyRequest,
};

use super::extractors::{FieldError, Validatable, ValidationBuilder};
use super::sanitizers::{
    normalize_contract_id, normalize_stellar_address, sanitize_description_optional, sanitize_name,
    sanitize_tags, sanitize_url_optional, trim,
};
use super::validators::{
    validate_contract_id, validate_json_depth, validate_length, validate_no_xss, validate_semver,
    validate_source_code_size, validate_stellar_address, validate_tags, validate_url_optional,
};

// ─────────────────────────────────────────────────────────────────────────────
// Constants for validation rules
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum length for contract name
const MAX_NAME_LENGTH: usize = 255;
/// Minimum length for contract name
const MIN_NAME_LENGTH: usize = 1;
/// Maximum length for description
const MAX_DESCRIPTION_LENGTH: usize = 5000;
/// Maximum number of tags allowed
const MAX_TAGS_COUNT: usize = 10;
/// Maximum length for each tag
const MAX_TAG_LENGTH: usize = 50;
/// Maximum source code size (1 MB)
const MAX_SOURCE_CODE_BYTES: usize = 1024 * 1024;
/// Maximum JSON nesting depth
const MAX_JSON_DEPTH: usize = 10;
/// Maximum length for category
const MAX_CATEGORY_LENGTH: usize = 100;
/// Maximum length for wasm hash
const MAX_WASM_HASH_LENGTH: usize = 64;
/// Maximum length for dependency name
const MAX_DEPENDENCY_NAME_LENGTH: usize = 255;
/// Maximum length for version constraint
const MAX_VERSION_CONSTRAINT_LENGTH: usize = 100;
/// Maximum number of dependencies
const MAX_DEPENDENCIES_COUNT: usize = 50;

// ─────────────────────────────────────────────────────────────────────────────
// PublishRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for PublishRequest {
    fn sanitize(&mut self) {
        // Normalize contract_id (uppercase, trim)
        self.contract_id = normalize_contract_id(&self.contract_id);

        // Sanitize name (trim, strip HTML, normalize whitespace)
        self.name = sanitize_name(&self.name);

        // Sanitize description (trim, strip HTML)
        sanitize_description_optional(&mut self.description);

        // Normalize publisher address (uppercase, trim)
        self.publisher_address = normalize_stellar_address(&self.publisher_address);

        // Sanitize source URL
        sanitize_url_optional(&mut self.source_url);

        // Sanitize category
        if let Some(ref mut cat) = self.category {
            *cat = trim(cat);
            if cat.is_empty() {
                self.category = None;
            }
        }

        // Sanitize tags
        self.tags = sanitize_tags(&self.tags);

        // Sanitize dependencies
        for dep in &mut self.dependencies {
            dep.name = trim(&dep.name);
            dep.version_constraint = trim(&dep.version_constraint);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        // contract_id: required, valid Stellar contract ID format
        builder.check("contract_id", || validate_contract_id(&self.contract_id));

        // name: required, 1-255 characters
        builder.check("name", || {
            if self.name.is_empty() {
                return Err("name is required".to_string());
            }
            validate_length(&self.name, MIN_NAME_LENGTH, MAX_NAME_LENGTH)
        });

        // name: no XSS patterns
        builder.check("name", || validate_no_xss(&self.name));

        // description: optional, max 5000 characters
        if let Some(ref desc) = self.description {
            builder.check("description", || {
                validate_length(desc, 0, MAX_DESCRIPTION_LENGTH)
            });
            builder.check("description", || validate_no_xss(desc));
        }

        // publisher_address: required, valid Stellar address
        builder.check("publisher_address", || {
            validate_stellar_address(&self.publisher_address)
        });

        // source_url: optional, valid URL format
        builder.check("source_url", || validate_url_optional(&self.source_url));

        // category: optional, max length
        if let Some(ref cat) = self.category {
            builder.check("category", || validate_length(cat, 1, MAX_CATEGORY_LENGTH));
            builder.check("category", || validate_no_xss(cat));
        }

        // tags: max count, each max length
        builder.check("tags", || {
            validate_tags(&self.tags, MAX_TAGS_COUNT, MAX_TAG_LENGTH)
        });

        // dependencies: validate each
        builder.check("dependencies", || {
            if self.dependencies.len() > MAX_DEPENDENCIES_COUNT {
                return Err(format!(
                    "at most {} dependencies are allowed",
                    MAX_DEPENDENCIES_COUNT
                ));
            }
            Ok(())
        });

        for (i, dep) in self.dependencies.iter().enumerate() {
            let field_name = format!("dependencies[{}].name", i);
            if dep.name.is_empty() {
                builder.add_error(&field_name, "dependency name is required");
            } else if dep.name.len() > MAX_DEPENDENCY_NAME_LENGTH {
                builder.add_error(
                    &field_name,
                    format!("must be at most {} characters", MAX_DEPENDENCY_NAME_LENGTH),
                );
            }

            let constraint_field = format!("dependencies[{}].version_constraint", i);
            if dep.version_constraint.is_empty() {
                builder.add_error(&constraint_field, "version constraint is required");
            } else if dep.version_constraint.len() > MAX_VERSION_CONSTRAINT_LENGTH {
                builder.add_error(
                    &constraint_field,
                    format!(
                        "must be at most {} characters",
                        MAX_VERSION_CONSTRAINT_LENGTH
                    ),
                );
            }
        }

        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// VerifyRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for VerifyRequest {
    fn sanitize(&mut self) {
        // Normalize contract_id
        self.contract_id = normalize_contract_id(&self.contract_id);

        // Trim compiler version
        self.compiler_version = trim(&self.compiler_version);

        // Sanitize source code (remove control chars but preserve structure)
        self.source_code = super::sanitizers::sanitize_source_code(&self.source_code);

        // Sanitize JSON build params
        super::sanitizers::sanitize_json_value(&mut self.build_params);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        // contract_id: required, valid format
        builder.check("contract_id", || validate_contract_id(&self.contract_id));

        // source_code: required, max size
        builder.check("source_code", || {
            if self.source_code.trim().is_empty() {
                return Err("source_code is required".to_string());
            }
            validate_source_code_size(&self.source_code, MAX_SOURCE_CODE_BYTES)
        });

        // compiler_version: required, valid semver
        builder.check("compiler_version", || {
            if self.compiler_version.is_empty() {
                return Err("compiler_version is required".to_string());
            }
            validate_semver(&self.compiler_version)
        });

        // build_params: validate JSON depth
        builder.check("build_params", || {
            validate_json_depth(&self.build_params, MAX_JSON_DEPTH)
        });

        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CreateMigrationRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for CreateMigrationRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.wasm_hash = trim(&self.wasm_hash);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        builder.check("contract_id", || validate_contract_id(&self.contract_id));

        builder.check("wasm_hash", || {
            if self.wasm_hash.is_empty() {
                return Err("wasm_hash is required".to_string());
            }
            validate_length(&self.wasm_hash, 1, MAX_WASM_HASH_LENGTH)
        });

        builder.build()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UpdateMigrationStatusRequest validation
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for UpdateMigrationStatusRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut log) = self.log_output {
            *log = trim(log);
            if log.is_empty() {
                self.log_output = None;
            }
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        // Status is an enum, so it's validated by deserialization
        // log_output is optional
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DependencyDeclaration validation (used within PublishRequest)
// ─────────────────────────────────────────────────────────────────────────────

impl Validatable for DependencyDeclaration {
    fn sanitize(&mut self) {
        self.name = trim(&self.name);
        self.version_constraint = trim(&self.version_constraint);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();

        builder.check("name", || {
            if self.name.is_empty() {
                return Err("name is required".to_string());
            }
            validate_length(&self.name, 1, MAX_DEPENDENCY_NAME_LENGTH)
        });

        builder.check("version_constraint", || {
            if self.version_constraint.is_empty() {
                return Err("version_constraint is required".to_string());
            }
            validate_length(&self.version_constraint, 1, MAX_VERSION_CONSTRAINT_LENGTH)
        });

        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::Network;

    fn valid_contract_id() -> String {
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string()
    }

    fn valid_stellar_address() -> String {
        "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string()
    }

    #[test]
    fn test_publish_request_valid() {
        let req = PublishRequest {
            contract_id: valid_contract_id(),
            name: "My Contract".to_string(),
            description: Some("A test contract".to_string()),
            network: Network::Testnet,
            category: Some("DeFi".to_string()),
            tags: vec!["token".to_string(), "defi".to_string()],
            source_url: Some("https://github.com/user/repo".to_string()),
            publisher_address: valid_stellar_address(),
            dependencies: vec![],
        };

        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_publish_request_invalid_contract_id() {
        let req = PublishRequest {
            contract_id: "invalid".to_string(),
            name: "My Contract".to_string(),
            description: None,
            network: Network::Testnet,
            category: None,
            tags: vec![],
            source_url: None,
            publisher_address: valid_stellar_address(),
            dependencies: vec![],
        };

        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "contract_id"));
    }

    #[test]
    fn test_publish_request_empty_name() {
        let req = PublishRequest {
            contract_id: valid_contract_id(),
            name: "".to_string(),
            description: None,
            network: Network::Testnet,
            category: None,
            tags: vec![],
            source_url: None,
            publisher_address: valid_stellar_address(),
            dependencies: vec![],
        };

        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "name"));
    }

    #[test]
    fn test_publish_request_sanitization() {
        let mut req = PublishRequest {
            contract_id: "  cdlzfc3syjydzt7k67vz75hpjvieuvnixf47zg2fb2rmqqvu2hhgcysc  ".to_string(),
            name: "  <b>My Contract</b>  ".to_string(),
            description: Some("  <script>alert('xss')</script>Description  ".to_string()),
            network: Network::Testnet,
            category: Some("  DeFi  ".to_string()),
            tags: vec!["  token  ".to_string(), "<b>defi</b>".to_string()],
            source_url: Some("  https://github.com/user/repo  ".to_string()),
            publisher_address: "  gdlzfc3syjydzt7k67vz75hpjvieuvnixf47zg2fb2rmqqvu2hhgcysc  "
                .to_string(),
            dependencies: vec![],
        };

        req.sanitize();

        // Contract ID should be uppercase and trimmed
        assert_eq!(req.contract_id, valid_contract_id());

        // Name should be trimmed with HTML stripped
        assert_eq!(req.name, "My Contract");

        // Description should have HTML stripped
        assert_eq!(req.description, Some("alert('xss')Description".to_string()));

        // Publisher address should be uppercase and trimmed
        assert_eq!(req.publisher_address, valid_stellar_address());

        // Category should be trimmed
        assert_eq!(req.category, Some("DeFi".to_string()));

        // Tags should be trimmed with HTML stripped
        assert_eq!(req.tags, vec!["token", "defi"]);

        // Source URL should be trimmed
        assert_eq!(req.source_url, Some("https://github.com/user/repo".to_string()));
    }

    #[test]
    fn test_verify_request_valid() {
        let req = VerifyRequest {
            contract_id: valid_contract_id(),
            source_code: "fn main() {}".to_string(),
            build_params: serde_json::json!({"optimize": true}),
            compiler_version: "1.0.0".to_string(),
        };

        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_verify_request_empty_source() {
        let req = VerifyRequest {
            contract_id: valid_contract_id(),
            source_code: "".to_string(),
            build_params: serde_json::json!({}),
            compiler_version: "1.0.0".to_string(),
        };

        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "source_code"));
    }

    #[test]
    fn test_verify_request_invalid_semver() {
        let req = VerifyRequest {
            contract_id: valid_contract_id(),
            source_code: "fn main() {}".to_string(),
            build_params: serde_json::json!({}),
            compiler_version: "not-a-version".to_string(),
        };

        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "compiler_version"));
    }

    #[test]
    fn test_too_many_tags() {
        let req = PublishRequest {
            contract_id: valid_contract_id(),
            name: "My Contract".to_string(),
            description: None,
            network: Network::Testnet,
            category: None,
            tags: (0..15).map(|i| format!("tag{}", i)).collect(),
            source_url: None,
            publisher_address: valid_stellar_address(),
            dependencies: vec![],
        };

        let result = req.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "tags"));
    }
}
