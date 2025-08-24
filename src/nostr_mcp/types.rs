use chrono::{DateTime, Utc};
use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Memory entry stored in Nostr DMs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub memory_type: String,
    pub category: Option<String>,
    pub content: MemoryContent,
    pub encrypted: bool,
    pub version: String,
}

/// Memory content structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContent {
    pub title: String,
    pub description: String,
    pub metadata: MemoryMetadata,
}

/// Metadata for memory entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub tags: Vec<String>,
    pub priority: Option<String>,
    pub expiry: Option<DateTime<Utc>>,
}

/// Request to store a new memory
#[derive(Debug, Deserialize, JsonSchema)]
pub struct StoreMemoryRequest {
    #[schemars(description = "Type of memory (user_preference, context, fact, instruction, note)")]
    pub memory_type: String,
    #[schemars(
        description = "Optional category classification (personal, work, project, general)"
    )]
    pub category: Option<String>,
    #[schemars(description = "Short title for the memory")]
    pub title: String,
    #[schemars(description = "Detailed description or content")]
    pub description: String,
    #[schemars(description = "Optional tags for categorization")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Optional priority level (high, medium, low)")]
    pub priority: Option<String>,
    #[schemars(description = "Optional expiry date (ISO 8601 format)")]
    pub expiry: Option<String>,
}

/// Request to retrieve memories with filtering
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RetrieveMemoryRequest {
    #[schemars(description = "Search query to match in title or description")]
    pub query: Option<String>,
    #[schemars(
        description = "Filter by memory type (user_preference, context, fact, instruction, note)"
    )]
    pub memory_type: Option<String>,
    #[schemars(description = "Filter by category (personal, work, project, general)")]
    pub category: Option<String>,
    #[schemars(description = "Filter by tags (must contain all specified tags)")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Maximum number of results to return (default 10)")]
    pub limit: Option<u32>,
    #[schemars(description = "Return memories created since this date (ISO 8601)")]
    pub since: Option<String>,
    #[schemars(description = "Return memories created until this date (ISO 8601)")]
    pub until: Option<String>,
}

/// Request to update an existing memory
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateMemoryRequest {
    #[schemars(description = "UUID of the memory to update")]
    pub id: String,
    #[schemars(description = "New title (optional)")]
    pub title: Option<String>,
    #[schemars(description = "New description (optional)")]
    pub description: Option<String>,
    #[schemars(description = "New tags (optional, replaces existing)")]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "New priority (optional, high, medium, low)")]
    pub priority: Option<String>,
    #[schemars(description = "New expiry date (optional, ISO 8601)")]
    pub expiry: Option<String>,
}

/// Request to delete a memory
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteMemoryRequest {
    #[schemars(description = "UUID of the memory to delete")]
    pub id: String,
}

/// Response for memory operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResponse {
    pub memories: Vec<MemoryEntry>,
    pub total: usize,
    pub page: u32,
    pub per_page: u32,
}

/// Summary information about stored memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_memories: usize,
    pub by_type: std::collections::HashMap<String, usize>,
    pub by_category: std::collections::HashMap<String, usize>,
    pub oldest: Option<DateTime<Utc>>,
    pub newest: Option<DateTime<Utc>>,
}

impl MemoryEntry {
    pub fn new(
        memory_type: String,
        category: Option<String>,
        title: String,
        description: String,
        tags: Vec<String>,
        priority: Option<String>,
        expiry: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            memory_type,
            category,
            content: MemoryContent {
                title,
                description,
                metadata: MemoryMetadata {
                    tags,
                    priority,
                    expiry,
                },
            },
            encrypted: true,
            version: "1.0".to_string(),
        }
    }

    /// Check if memory matches the given query
    pub fn matches_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.content.title.to_lowercase().contains(&query_lower)
            || self
                .content
                .description
                .to_lowercase()
                .contains(&query_lower)
            || self
                .content
                .metadata
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&query_lower))
    }

    /// Check if memory has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expiry) = self.content.metadata.expiry {
            Utc::now() > expiry
        } else {
            false
        }
    }
}
