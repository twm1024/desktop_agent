// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

#![allow(dead_code)]
use crate::error::{AppError, Result};
use aes_gcm::aead::{Aead, AeadCore};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::Engine;
use keyring::Entry as KeyringEntry;
use rand::Rng;
use tracing::{info, warn};

/// Configuration encryption using system keychain
pub struct ConfigEncryption {
    cipher: Aes256Gcm,
}

impl ConfigEncryption {
    /// Create a new ConfigEncryption instance
    ///
    /// Retrieves or creates an encryption key from the system keychain
    pub fn new() -> Result<Self> {
        let entry = KeyringEntry::new("desktop-agent", "encryption-key")
            .map_err(|e| AppError::security(format!("Keyring error: {}", e)))?;

        // Try to get existing key
        let key_bytes = match entry.get_password() {
            Ok(key_str) => {
                info!("Loaded encryption key from keychain");
                base64::engine::general_purpose::STANDARD
                    .decode(&key_str)
                    .map_err(|e| AppError::security(format!("Invalid key format: {}", e)))?
            }
            Err(_) => {
                info!("Generating new encryption key");
                // Generate new key
                let key_bytes: [u8; 32] = rand::thread_rng().gen();
                let key_str = base64::engine::general_purpose::STANDARD.encode(key_bytes);

                // Save to keychain
                if let Err(e) = entry.set_password(&key_str) {
                    warn!("Failed to save key to keychain: {}", e);
                }

                key_bytes.to_vec()
            }
        };

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| AppError::security(format!("Failed to create cipher: {}", e)))?;

        Ok(Self { cipher })
    }

    /// Encrypt data
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let nonce = Aes256Gcm::generate_nonce(&mut rand::thread_rng());
        let ciphertext = self.cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| AppError::security(format!("Encryption failed: {}", e)))?;

        // Combine nonce and ciphertext
        let mut result = nonce.to_vec();
        result.extend(ciphertext);

        Ok(base64::engine::general_purpose::STANDARD.encode(result))
    }

    /// Decrypt data
    pub fn decrypt(&self, ciphertext: &str) -> Result<String> {
        let data = base64::engine::general_purpose::STANDARD
            .decode(ciphertext)
            .map_err(|e| AppError::security(format!("Invalid ciphertext format: {}", e)))?;

        if data.len() < 12 {
            return Err(AppError::security("Ciphertext too short"));
        }

        let (nonce, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce);

        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::security(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext)
            .map_err(|e| AppError::security(format!("Invalid UTF-8 in plaintext: {}", e)))
    }

    /// Encrypt sensitive configuration values
    pub fn encrypt_config_value(&self, key: &str, value: &str) -> Result<String> {
        info!("Encrypting config value for key: {}", key);
        self.encrypt(value)
    }

    /// Decrypt sensitive configuration values
    pub fn decrypt_config_value(&self, key: &str, encrypted: &str) -> Result<String> {
        info!("Decrypting config value for key: {}", key);
        self.decrypt(encrypted)
    }

    /// Regenerate the encryption key
    ///
    /// WARNING: This will invalidate all encrypted values
    pub fn regenerate_key() -> Result<()> {
        info!("Regenerating encryption key");

        let entry = KeyringEntry::new("desktop-agent", "encryption-key")
            .map_err(|e| AppError::security(format!("Keyring error: {}", e)))?;

        // Delete old key
        let _ = entry.delete_password();

        // Generate and save new key
        let key_bytes: [u8; 32] = rand::thread_rng().gen();
        let key_str = base64::engine::general_purpose::STANDARD.encode(key_bytes);

        entry.set_password(&key_str)
            .map_err(|e| AppError::security(format!("Failed to save new key: {}", e)))?;

        info!("Encryption key regenerated successfully");
        Ok(())
    }
}

impl Default for ConfigEncryption {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            panic!("Failed to initialize ConfigEncryption: {}", e);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        match ConfigEncryption::new() {
            Ok(encryption) => {
                let plaintext = "This is a secret value";

                let encrypted = encryption.encrypt(plaintext).unwrap();
                assert_ne!(plaintext, encrypted);

                let decrypted = encryption.decrypt(&encrypted).unwrap();
                assert_eq!(plaintext, decrypted);
            }
            Err(_) => {
                println!("Skipping encryption test (keyring not available)");
            }
        }
    }

    #[test]
    fn test_encryption_empty_string() {
        match ConfigEncryption::new() {
            Ok(encryption) => {
                let plaintext = "";

                let encrypted = encryption.encrypt(plaintext).unwrap();
                let decrypted = encryption.decrypt(&encrypted).unwrap();

                assert_eq!(plaintext, decrypted);
            }
            Err(_) => {
                println!("Skipping encryption test (keyring not available)");
            }
        }
    }

    #[test]
    fn test_encryption_unicode() {
        match ConfigEncryption::new() {
            Ok(encryption) => {
                let plaintext = "测试中文 这是一个秘密";

                let encrypted = encryption.encrypt(plaintext).unwrap();
                let decrypted = encryption.decrypt(&encrypted).unwrap();

                assert_eq!(plaintext, decrypted);
            }
            Err(_) => {
                println!("Skipping encryption test (keyring not available)");
            }
        }
    }
}
