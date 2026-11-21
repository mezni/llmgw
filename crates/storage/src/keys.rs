use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

mod repository;

pub use repository::*;

pub const KEY_BYTES: usize = 32;
pub const KEY_PREFIX_LEN: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedKey {
    pub secret: String,
    pub prefix: String,
    pub hash: Vec<u8>,
}

pub fn generate_key() -> GeneratedKey {
    let mut bytes = [0_u8; KEY_BYTES];
    OsRng.fill_bytes(&mut bytes);
    let encoded = URL_SAFE_NO_PAD.encode(bytes);
    let secret = format!("gw-{encoded}");
    let prefix = format!("gw-{}", &encoded[..KEY_PREFIX_LEN]);
    let hash = hash_key(&secret);
    GeneratedKey {
        secret,
        prefix,
        hash,
    }
}

pub fn hash_key(secret: &str) -> Vec<u8> {
    Sha256::digest(secret.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_gw_prefixed_secret_and_derived_metadata() {
        let key = generate_key();
        let encoded = key.secret.strip_prefix("gw-").expect("prefix");
        assert_eq!(encoded.len(), 43);
        assert!(encoded
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert!(key.secret.starts_with(&key.prefix));
        assert_eq!(key.hash, hash_key(&key.secret));
        assert_eq!(key.hash.len(), 32);
        assert_ne!(key.secret, key.prefix);
    }

    #[test]
    fn generated_secrets_are_unique() {
        assert_ne!(generate_key().secret, generate_key().secret);
    }
}
