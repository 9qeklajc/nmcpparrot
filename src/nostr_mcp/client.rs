use super::encryption::{EncryptionError, MemoryEncryption};
use super::types::*;
use chrono::{DateTime, Utc};
use nostr_sdk::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Error types for Nostr memory operations
#[derive(Debug)]
pub enum NostrMemoryError {
    NostrError(String),
    EncryptionError(EncryptionError),
    #[allow(dead_code)] // Future timeout handling
    TimeoutError,
    InvalidData(String),
}

impl From<EncryptionError> for NostrMemoryError {
    fn from(err: EncryptionError) -> Self {
        NostrMemoryError::EncryptionError(err)
    }
}

impl std::fmt::Display for NostrMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NostrMemoryError::NostrError(e) => write!(f, "Nostr error: {}", e),
            NostrMemoryError::EncryptionError(e) => write!(f, "Encryption error: {}", e),
            NostrMemoryError::TimeoutError => write!(f, "Operation timed out"),
            NostrMemoryError::InvalidData(e) => write!(f, "Invalid data: {}", e),
        }
    }
}

impl std::error::Error for NostrMemoryError {}

/// Client for Nostr memory operations with local fallback
#[derive(Debug, Clone)]
pub struct NostrMemoryClient {
    client: Client,
    encryption: MemoryEncryption,
    our_pubkey: PublicKey,
    // Local memory storage as fallback
    local_memories: Arc<RwLock<HashMap<uuid::Uuid, MemoryEntry>>>,
}

impl NostrMemoryClient {
    /// Create a new Nostr memory client
    pub fn new(client: Client, keys: Keys, our_pubkey: PublicKey) -> Self {
        let encryption = MemoryEncryption::new(keys);
        Self {
            client,
            encryption,
            our_pubkey,
            local_memories: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a memory entry by sending it as an encrypted DM to ourselves
    pub async fn store_memory(&self, memory: &MemoryEntry) -> Result<bool, NostrMemoryError> {
        let dm_content = self.encryption.create_memory_dm_content(memory)?;

        // Store locally as a backup/fallback
        {
            let mut local_memories = self.local_memories.write().await;
            local_memories.insert(memory.id, memory.clone());
        }

        // Send the encrypted memory as a DM to ourselves (Nostr storage)
        let _result = self
            .client
            .send_private_msg(self.our_pubkey, dm_content, [])
            .await
            .map_err(|e| NostrMemoryError::NostrError(e.to_string()))?;

        Ok(true)
    }

    /// Retrieve memory entries with optional filtering
    pub async fn retrieve_memories(
        &self,
        filter: &RetrieveMemoryRequest,
    ) -> Result<Vec<MemoryEntry>, NostrMemoryError> {
        // Build the Nostr filter to get our DMs
        let mut nostr_filter = Filter::new()
            .kind(Kind::EncryptedDirectMessage)
            .pubkey(self.our_pubkey) // DMs sent by us
            .limit(filter.limit.unwrap_or(100) as usize); // Get more than requested to allow for filtering

        // Add time filters if specified
        if let Some(since_str) = &filter.since {
            if let Ok(since_dt) = DateTime::parse_from_rfc3339(since_str) {
                let timestamp = Timestamp::from_secs(since_dt.timestamp() as u64);
                nostr_filter = nostr_filter.since(timestamp);
            }
        }

        if let Some(until_str) = &filter.until {
            if let Ok(until_dt) = DateTime::parse_from_rfc3339(until_str) {
                let timestamp = Timestamp::from_secs(until_dt.timestamp() as u64);
                let _nostr_filter = nostr_filter.until(timestamp);
            }
        }

        // TODO: Implement actual Nostr event retrieval
        let events: Vec<Event> = Vec::new();

        let mut memories = Vec::new();

        for event in events {
            let content = &event.content;

            // Try to extract memory from the DM content
            if let Ok(Some(memory)) = self
                .encryption
                .extract_memory_from_dm::<MemoryEntry>(content)
            {
                // Apply filters
                if self.matches_filter(&memory, filter) {
                    memories.push(memory);
                }
            }
        }

        // If no memories found from Nostr, fallback to local memory
        if memories.is_empty() {
            let local_memories = self.local_memories.read().await;
            for (_, memory) in local_memories.iter() {
                if self.matches_filter(memory, filter) {
                    memories.push(memory.clone());
                }
            }
        }

        // Sort by timestamp (newest first)
        memories.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply limit
        let limit = filter.limit.unwrap_or(10) as usize;
        if memories.len() > limit {
            memories.truncate(limit);
        }

        Ok(memories)
    }

    /// Delete a memory by ID (this is complex in Nostr, so we'll mark it as deleted)
    pub async fn delete_memory(&self, memory_id: &str) -> Result<bool, NostrMemoryError> {
        // Parse the UUID
        let uuid = uuid::Uuid::parse_str(memory_id)
            .map_err(|e| NostrMemoryError::InvalidData(format!("Invalid UUID: {}", e)))?;

        // Remove from local memory first
        {
            let mut local_memories = self.local_memories.write().await;
            local_memories.remove(&uuid);
        }

        // In Nostr, we can't actually delete messages, so we'll store a deletion marker
        let deletion_marker = format!("MEMORY_DELETED:{}", uuid);

        self.client
            .send_private_msg(self.our_pubkey, deletion_marker, [])
            .await
            .map_err(|e| NostrMemoryError::NostrError(e.to_string()))?;

        Ok(true)
    }

    /// Update a memory entry (stores a new version)
    pub async fn update_memory(
        &self,
        memory_id: &str,
        update: &UpdateMemoryRequest,
    ) -> Result<MemoryEntry, NostrMemoryError> {
        // First, find the existing memory
        let retrieve_filter = RetrieveMemoryRequest {
            query: None,
            memory_type: None,
            category: None,
            tags: None,
            limit: Some(1000), // Get many to find the specific ID
            since: None,
            until: None,
        };

        let memories = self.retrieve_memories(&retrieve_filter).await?;

        let mut existing_memory = memories
            .into_iter()
            .find(|m| m.id.to_string() == memory_id)
            .ok_or_else(|| NostrMemoryError::InvalidData("Memory not found".to_string()))?;

        // Apply updates
        if let Some(title) = &update.title {
            existing_memory.content.title = title.clone();
        }
        if let Some(description) = &update.description {
            existing_memory.content.description = description.clone();
        }
        if let Some(tags) = &update.tags {
            existing_memory.content.metadata.tags = tags.clone();
        }
        if let Some(priority) = &update.priority {
            existing_memory.content.metadata.priority = Some(priority.clone());
        }
        if let Some(expiry_str) = &update.expiry {
            if let Ok(expiry_dt) = DateTime::parse_from_rfc3339(expiry_str) {
                existing_memory.content.metadata.expiry = Some(expiry_dt.with_timezone(&Utc));
            }
        }

        // Update timestamp
        existing_memory.timestamp = Utc::now();

        // Store the updated memory
        self.store_memory(&existing_memory).await?;

        Ok(existing_memory)
    }

    /// Get memory statistics
    pub async fn get_memory_stats(&self) -> Result<MemoryStats, NostrMemoryError> {
        let retrieve_filter = RetrieveMemoryRequest {
            query: None,
            memory_type: None,
            category: None,
            tags: None,
            limit: Some(10000), // Get all memories for stats
            since: None,
            until: None,
        };

        let memories = self.retrieve_memories(&retrieve_filter).await?;

        let mut by_type = std::collections::HashMap::new();
        let mut by_category = std::collections::HashMap::new();
        let mut oldest = None;
        let mut newest = None;

        for memory in &memories {
            // Count by type
            *by_type.entry(memory.memory_type.clone()).or_insert(0) += 1;

            // Count by category
            if let Some(category) = &memory.category {
                *by_category.entry(category.clone()).or_insert(0) += 1;
            }

            // Track oldest and newest
            if oldest.is_none() || memory.timestamp < oldest.unwrap() {
                oldest = Some(memory.timestamp);
            }
            if newest.is_none() || memory.timestamp > newest.unwrap() {
                newest = Some(memory.timestamp);
            }
        }

        Ok(MemoryStats {
            total_memories: memories.len(),
            by_type,
            by_category,
            oldest,
            newest,
        })
    }

    /// Check if a memory matches the given filter
    fn matches_filter(&self, memory: &MemoryEntry, filter: &RetrieveMemoryRequest) -> bool {
        // Skip expired memories
        if memory.is_expired() {
            return false;
        }

        // Check query match
        if let Some(query) = &filter.query {
            if !memory.matches_query(query) {
                return false;
            }
        }

        // Check type filter
        if let Some(filter_type) = &filter.memory_type {
            if &memory.memory_type != filter_type {
                return false;
            }
        }

        // Check category filter
        if let Some(filter_category) = &filter.category {
            match &memory.category {
                Some(memory_category) => {
                    if memory_category != filter_category {
                        return false;
                    }
                }
                None => return false,
            }
        }

        // Check tags filter (must contain all specified tags)
        if let Some(filter_tags) = &filter.tags {
            for filter_tag in filter_tags {
                if !memory.content.metadata.tags.contains(filter_tag) {
                    return false;
                }
            }
        }

        true
    }
}
