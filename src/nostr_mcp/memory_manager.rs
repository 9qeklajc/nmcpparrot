use super::client::{NostrMemoryClient, NostrMemoryError};
use super::types::*;
use chrono::{DateTime, Utc};

/// High-level memory manager that handles business logic
#[derive(Debug, Clone)]
pub struct MemoryManager {
    client: NostrMemoryClient,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(client: NostrMemoryClient) -> Self {
        Self { client }
    }

    /// Store a new memory from a request
    pub async fn store_memory_from_request(
        &self,
        request: &StoreMemoryRequest,
    ) -> Result<MemoryEntry, NostrMemoryError> {
        // Parse expiry if provided
        let expiry = if let Some(expiry_str) = &request.expiry {
            match DateTime::parse_from_rfc3339(expiry_str) {
                Ok(dt) => Some(dt.with_timezone(&Utc)),
                Err(_) => {
                    return Err(NostrMemoryError::InvalidData(
                        "Invalid expiry date format. Use ISO 8601 format.".to_string(),
                    ))
                }
            }
        } else {
            None
        };

        // Create the memory entry
        let memory = MemoryEntry::new(
            request.memory_type.clone(),
            request.category.clone(),
            request.title.clone(),
            request.description.clone(),
            request.tags.clone().unwrap_or_default(),
            request.priority.clone(),
            expiry,
        );

        // Store it via the client
        let _ = self.client.store_memory(&memory).await?;

        Ok(memory)
    }

    /// Retrieve memories with filtering and business logic
    pub async fn retrieve_memories(
        &self,
        request: &RetrieveMemoryRequest,
    ) -> Result<MemoryResponse, NostrMemoryError> {
        let memories = self.client.retrieve_memories(request).await?;

        let total = memories.len();
        let limit = request.limit.unwrap_or(10) as usize;
        let page = 1; // For now, we don't support pagination

        Ok(MemoryResponse {
            memories,
            total,
            page,
            per_page: limit as u32,
        })
    }

    /// Update an existing memory
    pub async fn update_memory(
        &self,
        request: &UpdateMemoryRequest,
    ) -> Result<MemoryEntry, NostrMemoryError> {
        self.client.update_memory(&request.id, request).await
    }

    /// Delete a memory by ID
    pub async fn delete_memory(
        &self,
        request: &DeleteMemoryRequest,
    ) -> Result<bool, NostrMemoryError> {
        self.client.delete_memory(&request.id).await
    }

    /// Get memory statistics
    pub async fn get_memory_stats(&self) -> Result<MemoryStats, NostrMemoryError> {
        self.client.get_memory_stats().await
    }

    /// Search for memories by content (convenience method)
    #[allow(dead_code)] // Convenience method for future use
    pub async fn search_memories(
        &self,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<MemoryEntry>, NostrMemoryError> {
        let request = RetrieveMemoryRequest {
            query: Some(query.to_string()),
            memory_type: None,
            category: None,
            tags: None,
            limit,
            since: None,
            until: None,
        };

        self.client.retrieve_memories(&request).await
    }

    /// Get memories by type (convenience method)
    #[allow(dead_code)] // Convenience method for future use
    pub async fn get_memories_by_type(
        &self,
        memory_type: String,
        limit: Option<u32>,
    ) -> Result<Vec<MemoryEntry>, NostrMemoryError> {
        let request = RetrieveMemoryRequest {
            query: None,
            memory_type: Some(memory_type),
            category: None,
            tags: None,
            limit,
            since: None,
            until: None,
        };

        self.client.retrieve_memories(&request).await
    }

    /// Get memories by category (convenience method)
    #[allow(dead_code)] // Convenience method for future use
    pub async fn get_memories_by_category(
        &self,
        category: String,
        limit: Option<u32>,
    ) -> Result<Vec<MemoryEntry>, NostrMemoryError> {
        let request = RetrieveMemoryRequest {
            query: None,
            memory_type: None,
            category: Some(category),
            tags: None,
            limit,
            since: None,
            until: None,
        };

        self.client.retrieve_memories(&request).await
    }

    /// Get memories by tags (convenience method)
    #[allow(dead_code)] // Convenience method for future use
    pub async fn get_memories_by_tags(
        &self,
        tags: Vec<String>,
        limit: Option<u32>,
    ) -> Result<Vec<MemoryEntry>, NostrMemoryError> {
        let request = RetrieveMemoryRequest {
            query: None,
            memory_type: None,
            category: None,
            tags: Some(tags),
            limit,
            since: None,
            until: None,
        };

        self.client.retrieve_memories(&request).await
    }

    /// Get recent memories (last N memories)
    #[allow(dead_code)] // Convenience method for future use
    pub async fn get_recent_memories(
        &self,
        limit: Option<u32>,
    ) -> Result<Vec<MemoryEntry>, NostrMemoryError> {
        let request = RetrieveMemoryRequest {
            query: None,
            memory_type: None,
            category: None,
            tags: None,
            limit,
            since: None,
            until: None,
        };

        self.client.retrieve_memories(&request).await
    }

    /// Clean up expired memories (returns count of expired memories found)
    pub async fn cleanup_expired_memories(&self) -> Result<usize, NostrMemoryError> {
        let request = RetrieveMemoryRequest {
            query: None,
            memory_type: None,
            category: None,
            tags: None,
            limit: Some(10000), // Get all to check for expired
            since: None,
            until: None,
        };

        let all_memories = self.client.retrieve_memories(&request).await?;
        let mut expired_count = 0;

        for memory in all_memories {
            if memory.is_expired() {
                // Mark as deleted
                let delete_request = DeleteMemoryRequest {
                    id: memory.id.to_string(),
                };
                self.delete_memory(&delete_request).await?;
                expired_count += 1;
            }
        }

        Ok(expired_count)
    }
}
