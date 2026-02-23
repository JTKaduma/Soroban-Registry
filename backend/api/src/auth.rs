use chrono::{Duration, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Clone)]
pub struct ChallengeRecord {
    pub nonce: String,
    pub expires_at: i64,
}

pub struct AuthManager {
    challenges: HashMap<String, ChallengeRecord>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthManager {
    pub fn new(secret: String) -> Self {
        Self {
            challenges: HashMap::new(),
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn from_env() -> Self {
        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-only-secret".to_string());
        Self::new(secret)
    }

    pub fn create_challenge(&mut self, address: &str) -> String {
        let nonce: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        let expires_at = (Utc::now() + Duration::minutes(5)).timestamp();
        self.challenges.insert(
            address.to_string(),
            ChallengeRecord {
                nonce: nonce.clone(),
                expires_at,
            },
        );
        nonce
    }

    pub fn verify_and_issue_jwt(
        &mut self,
        address: &str,
        public_key_hex: &str,
        signature_hex: &str,
    ) -> Result<String, &'static str> {
        let challenge = self
            .challenges
            .remove(address)
            .ok_or("challenge_not_found")?;
        if Utc::now().timestamp() > challenge.expires_at {
            return Err("challenge_expired");
        }
        if address != public_key_hex {
            return Err("address_public_key_mismatch");
        }
        let public_key = decode_hex_32(public_key_hex).ok_or("invalid_public_key_hex")?;
        let signature = decode_hex_64(signature_hex).ok_or("invalid_signature_hex")?;
        let vk = VerifyingKey::from_bytes(&public_key).map_err(|_| "invalid_public_key")?;
        let sig = Signature::from_bytes(&signature);
        vk.verify(challenge.nonce.as_bytes(), &sig)
            .map_err(|_| "invalid_signature")?;
        let iat = Utc::now().timestamp();
        let exp = (Utc::now() + Duration::hours(24)).timestamp();
        let claims = AuthClaims {
            sub: address.to_string(),
            iat,
            exp,
        };
        encode(&Header::default(), &claims, &self.encoding_key).map_err(|_| "jwt_encode_failed")
    }

    pub fn validate_jwt(&self, token: &str) -> Result<AuthClaims, &'static str> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        decode::<AuthClaims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| "invalid_token")
    }
}

fn decode_hex_32(value: &str) -> Option<[u8; 32]> {
    let bytes = decode_hex(value)?;
    let mut out = [0u8; 32];
    if bytes.len() != out.len() {
        return None;
    }
    out.copy_from_slice(&bytes);
    Some(out)
}

fn decode_hex_64(value: &str) -> Option<[u8; 64]> {
    let bytes = decode_hex(value)?;
    let mut out = [0u8; 64];
    if bytes.len() != out.len() {
        return None;
    }
    out.copy_from_slice(&bytes);
    Some(out)
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn hex_encode(data: &[u8]) -> String {
        data.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    }

    #[test]
    fn challenge_verify_and_jwt_works() {
        let mut auth = AuthManager::new("test-secret".to_string());
        let seed = [7u8; 32];
        let sk = SigningKey::from_bytes(&seed);
        let vk_hex = hex_encode(sk.verifying_key().as_bytes());
        let nonce = auth.create_challenge(&vk_hex);
        let sig = sk.sign(nonce.as_bytes());
        let token = auth
            .verify_and_issue_jwt(&vk_hex, &vk_hex, &hex_encode(&sig.to_bytes()))
            .expect("jwt must be issued");
        let claims = auth.validate_jwt(&token).expect("token must be valid");
        assert_eq!(claims.sub, vk_hex);
    }

    #[test]
    fn nonce_is_single_use() {
        let mut auth = AuthManager::new("test-secret".to_string());
        let seed = [9u8; 32];
        let sk = SigningKey::from_bytes(&seed);
        let vk_hex = hex_encode(sk.verifying_key().as_bytes());
        let nonce = auth.create_challenge(&vk_hex);
        let sig = sk.sign(nonce.as_bytes());
        let sig_hex = hex_encode(&sig.to_bytes());
        let first = auth.verify_and_issue_jwt(&vk_hex, &vk_hex, &sig_hex);
        assert!(first.is_ok());
        let second = auth.verify_and_issue_jwt(&vk_hex, &vk_hex, &sig_hex);
        assert!(second.is_err());
    }
}
