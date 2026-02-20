//! Field validators for input validation
//!
//! This module provides reusable validation functions for common field types
//! in the Soroban Registry API.

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Stellar contract ID pattern: 56 characters starting with 'C'
    static ref CONTRACT_ID_REGEX: Regex = Regex::new(r"^C[A-Z0-9]{55}$").unwrap();
    
    /// Stellar address pattern: 56 characters starting with 'G'
    static ref STELLAR_ADDRESS_REGEX: Regex = Regex::new(r"^G[A-Z0-9]{55}$").unwrap();
    
    /// Semver pattern: major.minor.patch with optional pre-release
    static ref SEMVER_REGEX: Regex = Regex::new(
        r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$"
    ).unwrap();
    
    /// URL pattern for source URLs
    static ref URL_REGEX: Regex = Regex::new(
        r"^https?://[^\s/$.?#].[^\s]*$"
    ).unwrap();
    
    /// HTML tag detection pattern
    static ref HTML_TAG_REGEX: Regex = Regex::new(r"<[^>]+>").unwrap();
    
    /// Script/event handler pattern for XSS detection
    static ref XSS_PATTERN_REGEX: Regex = Regex::new(
        r"(?i)(javascript:|on\w+\s*=|<script|<iframe|<object|<embed)"
    ).unwrap();
}

/// Validate that a string is not empty after trimming
pub fn validate_required(value: &str, field_name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{} is required", field_name));
    }
    Ok(())
}

/// Validate string length within bounds
pub fn validate_length(value: &str, min: usize, max: usize) -> Result<(), String> {
    let len = value.chars().count();
    if len < min {
        return Err(format!("must be at least {} characters", min));
    }
    if len > max {
        return Err(format!("must be at most {} characters", max));
    }
    Ok(())
}

/// Validate that a string length is within bounds, returning field-specific error
pub fn validate_length_with_field(
    value: &str,
    min: usize,
    max: usize,
    field_name: &str,
) -> Result<(), String> {
    let len = value.chars().count();
    if len < min {
        return Err(format!("{} must be at least {} characters", field_name, min));
    }
    if len > max {
        return Err(format!("{} must be at most {} characters", field_name, max));
    }
    Ok(())
}

/// Validate Stellar contract ID format
/// Must be 56 characters starting with 'C'
pub fn validate_contract_id(contract_id: &str) -> Result<(), String> {
    let trimmed = contract_id.trim();
    
    if trimmed.is_empty() {
        return Err("contract_id is required".to_string());
    }
    
    if !CONTRACT_ID_REGEX.is_match(trimmed) {
        return Err(
            "must be a valid Stellar contract ID (56 characters starting with 'C')".to_string()
        );
    }
    
    Ok(())
}

/// Validate Stellar address format
/// Must be 56 characters starting with 'G'
pub fn validate_stellar_address(address: &str) -> Result<(), String> {
    let trimmed = address.trim();
    
    if trimmed.is_empty() {
        return Err("stellar address is required".to_string());
    }
    
    if !STELLAR_ADDRESS_REGEX.is_match(trimmed) {
        return Err(
            "must be a valid Stellar address (56 characters starting with 'G')".to_string()
        );
    }
    
    Ok(())
}

/// Validate optional Stellar address (only validates if Some)
pub fn validate_stellar_address_optional(address: &Option<String>) -> Result<(), String> {
    match address {
        Some(addr) if !addr.trim().is_empty() => validate_stellar_address(addr),
        _ => Ok(()),
    }
}

/// Validate semver version string
pub fn validate_semver(version: &str) -> Result<(), String> {
    let trimmed = version.trim();
    
    if trimmed.is_empty() {
        return Err("version is required".to_string());
    }
    
    if !SEMVER_REGEX.is_match(trimmed) {
        return Err("must be a valid semantic version (e.g., 1.0.0)".to_string());
    }
    
    Ok(())
}

/// Validate URL format
pub fn validate_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    
    if trimmed.is_empty() {
        return Ok(()); // Empty URLs are allowed (optional field)
    }
    
    if !URL_REGEX.is_match(trimmed) {
        return Err("must be a valid URL (starting with http:// or https://)".to_string());
    }
    
    Ok(())
}

/// Validate optional URL
pub fn validate_url_optional(url: &Option<String>) -> Result<(), String> {
    match url {
        Some(u) if !u.trim().is_empty() => validate_url(u),
        _ => Ok(()),
    }
}

/// Validate that a string contains no HTML tags
pub fn validate_no_html(value: &str) -> Result<(), String> {
    if HTML_TAG_REGEX.is_match(value) {
        return Err("HTML tags are not allowed".to_string());
    }
    Ok(())
}

/// Validate that a string contains no potential XSS patterns
pub fn validate_no_xss(value: &str) -> Result<(), String> {
    if XSS_PATTERN_REGEX.is_match(value) {
        return Err("potentially unsafe content detected".to_string());
    }
    Ok(())
}

/// Validate a list of tags
pub fn validate_tags(tags: &[String], max_tags: usize, max_tag_length: usize) -> Result<(), String> {
    if tags.len() > max_tags {
        return Err(format!("at most {} tags are allowed", max_tags));
    }
    
    for (i, tag) in tags.iter().enumerate() {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            return Err(format!("tag at index {} cannot be empty", i));
        }
        if trimmed.len() > max_tag_length {
            return Err(format!(
                "tag '{}' exceeds maximum length of {} characters",
                trimmed, max_tag_length
            ));
        }
        if let Err(e) = validate_no_xss(trimmed) {
            return Err(format!("tag '{}': {}", trimmed, e));
        }
    }
    
    Ok(())
}

/// Validate source code size
pub fn validate_source_code_size(source: &str, max_bytes: usize) -> Result<(), String> {
    let size = source.len();
    if size > max_bytes {
        let max_mb = max_bytes as f64 / (1024.0 * 1024.0);
        let actual_mb = size as f64 / (1024.0 * 1024.0);
        return Err(format!(
            "source code size ({:.2} MB) exceeds maximum allowed ({:.2} MB)",
            actual_mb, max_mb
        ));
    }
    Ok(())
}

/// Validate JSON value is not deeply nested (prevent DoS)
pub fn validate_json_depth(value: &serde_json::Value, max_depth: usize) -> Result<(), String> {
    fn check_depth(v: &serde_json::Value, current: usize, max: usize) -> Result<(), String> {
        if current > max {
            return Err(format!("JSON exceeds maximum nesting depth of {}", max));
        }
        match v {
            serde_json::Value::Array(arr) => {
                for item in arr {
                    check_depth(item, current + 1, max)?;
                }
            }
            serde_json::Value::Object(obj) => {
                for (_, val) in obj {
                    check_depth(val, current + 1, max)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    check_depth(value, 0, max_depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_contract_id() {
        // Valid contract ID
        let valid_id = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
        assert!(validate_contract_id(valid_id).is_ok());
        
        // Invalid: starts with G
        let invalid_g = "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
        assert!(validate_contract_id(invalid_g).is_err());
        
        // Invalid: too short
        assert!(validate_contract_id("CABC123").is_err());
        
        // Invalid: empty
        assert!(validate_contract_id("").is_err());
    }

    #[test]
    fn test_validate_stellar_address() {
        // Valid address
        let valid = "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
        assert!(validate_stellar_address(valid).is_ok());
        
        // Invalid: starts with C
        let invalid_c = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
        assert!(validate_stellar_address(invalid_c).is_err());
    }

    #[test]
    fn test_validate_length() {
        assert!(validate_length("hello", 1, 10).is_ok());
        assert!(validate_length("", 1, 10).is_err());
        assert!(validate_length("hello world!", 1, 5).is_err());
    }

    #[test]
    fn test_validate_no_html() {
        assert!(validate_no_html("plain text").is_ok());
        assert!(validate_no_html("<script>alert('xss')</script>").is_err());
        assert!(validate_no_html("<b>bold</b>").is_err());
    }

    #[test]
    fn test_validate_no_xss() {
        assert!(validate_no_xss("normal text").is_ok());
        assert!(validate_no_xss("javascript:alert(1)").is_err());
        assert!(validate_no_xss("onclick=alert(1)").is_err());
    }

    #[test]
    fn test_validate_tags() {
        let valid_tags = vec!["defi".to_string(), "token".to_string()];
        assert!(validate_tags(&valid_tags, 10, 50).is_ok());
        
        // Too many tags
        let many_tags: Vec<String> = (0..15).map(|i| format!("tag{}", i)).collect();
        assert!(validate_tags(&many_tags, 10, 50).is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://github.com/user/repo").is_ok());
        assert!(validate_url("http://example.com").is_ok());
        assert!(validate_url("not-a-url").is_err());
        assert!(validate_url("ftp://invalid.com").is_err());
    }

    #[test]
    fn test_validate_semver() {
        assert!(validate_semver("1.0.0").is_ok());
        assert!(validate_semver("0.1.0-alpha").is_ok());
        assert!(validate_semver("2.0.0-rc.1+build.123").is_ok());
        assert!(validate_semver("not-a-version").is_err());
    }
}
