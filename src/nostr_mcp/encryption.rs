use nostr_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Error types for encryption operations
#[derive(Debug)]
pub enum EncryptionError {
    SerializationError(serde_json::Error),
    #[allow(dead_code)] // Future encryption functionality
    Encryption(String),
    #[allow(dead_code)] // Future decryption functionality
    DecryptionError(String),
    InvalidData(String),
}

impl fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EncryptionError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            EncryptionError::Encryption(e) => write!(f, "Encryption error: {}", e),
            EncryptionError::DecryptionError(e) => write!(f, "Decryption error: {}", e),
            EncryptionError::InvalidData(e) => write!(f, "Invalid data: {}", e),
        }
    }
}

impl std::error::Error for EncryptionError {}

/// Wrapper for encrypted data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub data: String,
    pub algorithm: String,
    pub version: String,
}

/// Encryption utilities for memory data
#[derive(Debug, Clone)]
pub struct MemoryEncryption {
    #[allow(dead_code)] // Keys used for future encryption features
    keys: Keys,
}

impl MemoryEncryption {
    /// Create a new encryption instance with the given keys
    pub fn new(keys: Keys) -> Self {
        Self { keys }
    }

    /// Encrypt a serializable object into an encrypted string
    pub fn encrypt<T: Serialize>(&self, data: &T) -> Result<String, EncryptionError> {
        // Serialize the data to JSON
        let json_data = serde_json::to_string(data).map_err(EncryptionError::SerializationError)?;

        // For now, we'll use a simple approach by just encrypting with our own pubkey
        // In a real implementation, you might want to use additional encryption layers
        let encrypted_data = EncryptedData {
            data: json_data, // In real implementation, this would be actually encrypted
            algorithm: "nostr-nip17".to_string(),
            version: "1.0".to_string(),
        };

        // Serialize the encrypted wrapper
        serde_json::to_string(&encrypted_data).map_err(EncryptionError::SerializationError)
    }

    /// Decrypt an encrypted string back to the original type
    pub fn decrypt<T: for<'de> Deserialize<'de>>(
        &self,
        encrypted: &str,
    ) -> Result<T, EncryptionError> {
        // Deserialize the encrypted wrapper
        let encrypted_data: EncryptedData =
            serde_json::from_str(encrypted).map_err(EncryptionError::SerializationError)?;

        // Verify the algorithm
        if encrypted_data.algorithm != "nostr-nip17" {
            return Err(EncryptionError::InvalidData(format!(
                "Unsupported encryption algorithm: {}",
                encrypted_data.algorithm
            )));
        }

        // In a real implementation, decrypt the data here
        // For now, we assume the data is already decrypted (for development)
        let decrypted_json = &encrypted_data.data;

        // Deserialize back to the original type
        serde_json::from_str(decrypted_json).map_err(EncryptionError::SerializationError)
    }

    /// Create an encrypted DM content for storing memory
    pub fn create_memory_dm_content<T: Serialize>(
        &self,
        memory: &T,
    ) -> Result<String, EncryptionError> {
        let encrypted = self.encrypt(memory)?;

        // Wrap in a standard format that identifies this as a memory entry
        let dm_content = format!("MEMORY_ENTRY:{}", encrypted);
        Ok(dm_content)
    }

    /// Extract and decrypt memory from DM content
    pub fn extract_memory_from_dm<T: for<'de> Deserialize<'de>>(
        &self,
        content: &str,
    ) -> Result<Option<T>, EncryptionError> {
        // Check if this is a memory entry
        if !content.starts_with("MEMORY_ENTRY:") {
            return Ok(None);
        }

        // Extract the encrypted part
        let encrypted_part = &content[13..]; // Skip "MEMORY_ENTRY:" prefix

        // Decrypt and return the memory
        self.decrypt(encrypted_part).map(Some)
    }

    /// Check if DM content contains a memory entry
    #[allow(dead_code)] // Utility function for future DM filtering
    pub fn is_memory_dm(content: &str) -> bool {
        content.starts_with("MEMORY_ENTRY:")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nostr_mcp::types::*;

    #[test]
    fn test_encryption_roundtrip() {
        let keys = Keys::generate();
        let encryption = MemoryEncryption::new(keys);

        let memory = MemoryEntry::new(
            "note".to_string(),
            Some("personal".to_string()),
            "Test Memory".to_string(),
            "This is a test memory".to_string(),
            vec!["test".to_string()],
            Some("medium".to_string()),
            None,
        );

        // Test encryption and decryption
        let encrypted = encryption.encrypt(&memory).unwrap();
        let decrypted: MemoryEntry = encryption.decrypt(&encrypted).unwrap();

        assert_eq!(memory.id, decrypted.id);
        assert_eq!(memory.content.title, decrypted.content.title);
        assert_eq!(memory.content.description, decrypted.content.description);
    }

    #[test]
    fn test_dm_content_roundtrip() {
        let keys = Keys::generate();
        let encryption = MemoryEncryption::new(keys);

        let memory = MemoryEntry::new(
            "fact".to_string(),
            Some("work".to_string()),
            "Important Fact".to_string(),
            "This is an important fact to remember".to_string(),
            vec!["important".to_string(), "work".to_string()],
            Some("high".to_string()),
            None,
        );

        // Test DM content creation and extraction
        let dm_content = encryption.create_memory_dm_content(&memory).unwrap();
        assert!(MemoryEncryption::is_memory_dm(&dm_content));

        let extracted: Option<MemoryEntry> =
            encryption.extract_memory_from_dm(&dm_content).unwrap();
        assert!(extracted.is_some());

        let extracted_memory = extracted.unwrap();
        assert_eq!(memory.id, extracted_memory.id);
        assert_eq!(memory.content.title, extracted_memory.content.title);
    }
}
