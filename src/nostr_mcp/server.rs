use super::client::NostrMemoryClient;
use super::memory_manager::MemoryManager;
use super::types::*;
use crate::mcp::chat::{Chat, ProgressMessageRequest, SendMessageRequest};
use nostr_sdk::prelude::*;
use rmcp::{
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, Error as RmcpError, ServerHandler,
};

#[derive(Debug, Clone)]
pub struct NostrMemoryServer {
    memory_manager: MemoryManager,
    chat: Chat,
}

#[tool(tool_box)]
impl NostrMemoryServer {
    /// Create a new Nostr Memory MCP server
    pub fn new(
        nostr_client: Client,
        progress_client: Option<Client>,
        keys: Keys,
        our_pubkey: PublicKey,
        target_pubkey: PublicKey,
    ) -> Self {
        let memory_client = NostrMemoryClient::new(nostr_client.clone(), keys, our_pubkey);
        let memory_manager = MemoryManager::new(memory_client);
        let chat = Chat::new(nostr_client, progress_client, our_pubkey, target_pubkey);

        Self {
            memory_manager,
            chat,
        }
    }

    #[tool(description = "Store a new memory entry in Nostr")]
    pub async fn store_memory(
        &self,
        #[tool(aggr)] request: StoreMemoryRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: format!("Storing memory: {}", request.title),
            })
            .await;

        match self
            .memory_manager
            .store_memory_from_request(&request)
            .await
        {
            Ok(memory) => {
                let message = format!(
                    "🧠 Memory stored successfully!\n\n\
                     📝 **Title:** {}\n\
                     🆔 **ID:** {}\n\
                     📅 **Created:** {}\n\
                     🏷️ **Type:** {:?}\n\
                     {}{}",
                    memory.content.title,
                    memory.id,
                    memory.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                    memory.memory_type,
                    memory
                        .category
                        .as_ref()
                        .map(|c| format!("📂 **Category:** {:?}\n", c))
                        .unwrap_or_default(),
                    if memory.content.metadata.tags.is_empty() {
                        String::new()
                    } else {
                        format!("🏷️ **Tags:** {}\n", memory.content.metadata.tags.join(", "))
                    }
                );

                let _ = self.chat.send(SendMessageRequest { message }).await;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Memory stored with ID: {}",
                    memory.id
                ))]))
            }
            Err(e) => {
                let error_message = format!("❌ Failed to store memory: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_message.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_message)]))
            }
        }
    }

    #[tool(description = "Retrieve and search memory entries")]
    pub async fn retrieve_memory(
        &self,
        #[tool(aggr)] request: RetrieveMemoryRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let query_desc = if let Some(query) = &request.query {
            format!("Searching memories for: {}", query)
        } else {
            "Retrieving memories".to_string()
        };

        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: query_desc,
            })
            .await;

        match self.memory_manager.retrieve_memories(&request).await {
            Ok(response) => {
                let message = if response.memories.is_empty() {
                    "🔍 No memories found matching your criteria.".to_string()
                } else {
                    let mut message = format!("🧠 Found {} memories:\n\n", response.memories.len());

                    for (i, memory) in response.memories.iter().enumerate() {
                        message.push_str(&format!(
                            "{}. **{}**\n\
                             🆔 ID: {}\n\
                             📅 Created: {}\n\
                             🏷️ Type: {:?}\n\
                             {}\
                             📝 {}\n\
                             {}\n",
                            i + 1,
                            memory.content.title,
                            memory.id,
                            memory.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                            memory.memory_type,
                            memory
                                .category
                                .as_ref()
                                .map(|c| format!("📂 Category: {:?}\n", c))
                                .unwrap_or_default(),
                            memory.content.description,
                            if memory.content.metadata.tags.is_empty() {
                                String::new()
                            } else {
                                format!("🏷️ Tags: {}\n", memory.content.metadata.tags.join(", "))
                            }
                        ));
                    }

                    message
                };

                let _ = self.chat.send(SendMessageRequest { message }).await;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Retrieved {} memories",
                    response.memories.len()
                ))]))
            }
            Err(e) => {
                let error_message = format!("❌ Failed to retrieve memories: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_message.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_message)]))
            }
        }
    }

    #[tool(description = "Update an existing memory entry")]
    pub async fn update_memory(
        &self,
        #[tool(aggr)] request: UpdateMemoryRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: format!("Updating memory: {}", request.id),
            })
            .await;

        match self.memory_manager.update_memory(&request).await {
            Ok(memory) => {
                let message = format!(
                    "✅ Memory updated successfully!\n\n\
                     📝 **Title:** {}\n\
                     🆔 **ID:** {}\n\
                     📅 **Updated:** {}\n\
                     🏷️ **Type:** {:?}\n\
                     {}{}",
                    memory.content.title,
                    memory.id,
                    memory.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                    memory.memory_type,
                    memory
                        .category
                        .as_ref()
                        .map(|c| format!("📂 **Category:** {:?}\n", c))
                        .unwrap_or_default(),
                    if memory.content.metadata.tags.is_empty() {
                        String::new()
                    } else {
                        format!("🏷️ **Tags:** {}\n", memory.content.metadata.tags.join(", "))
                    }
                );

                let _ = self.chat.send(SendMessageRequest { message }).await;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Memory {} updated successfully",
                    memory.id
                ))]))
            }
            Err(e) => {
                let error_message = format!("❌ Failed to update memory: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_message.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_message)]))
            }
        }
    }

    #[tool(description = "Delete a memory entry by ID")]
    pub async fn delete_memory(
        &self,
        #[tool(aggr)] request: DeleteMemoryRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: format!("Deleting memory: {}", request.id),
            })
            .await;

        match self.memory_manager.delete_memory(&request).await {
            Ok(_) => {
                let message = format!("🗑️ Memory {} deleted successfully", request.id);
                let _ = self.chat.send(SendMessageRequest { message }).await;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Memory {} deleted",
                    request.id
                ))]))
            }
            Err(e) => {
                let error_message = format!("❌ Failed to delete memory: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_message.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_message)]))
            }
        }
    }

    #[tool(description = "Get statistics about stored memories")]
    pub async fn memory_stats(&self) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: "Gathering memory statistics...".to_string(),
            })
            .await;

        match self.memory_manager.get_memory_stats().await {
            Ok(stats) => {
                let mut message = "📊 **Memory Statistics**\n\n".to_string();
                message.push_str(&format!(
                    "🧠 **Total Memories:** {}\n\n",
                    stats.total_memories
                ));

                if !stats.by_type.is_empty() {
                    message.push_str("📋 **By Type:**\n");
                    for (type_name, count) in &stats.by_type {
                        message.push_str(&format!("  • {}: {}\n", type_name, count));
                    }
                    message.push('\n');
                }

                if !stats.by_category.is_empty() {
                    message.push_str("📂 **By Category:**\n");
                    for (category_name, count) in &stats.by_category {
                        message.push_str(&format!("  • {}: {}\n", category_name, count));
                    }
                    message.push('\n');
                }

                if let Some(oldest) = stats.oldest {
                    message.push_str(&format!(
                        "📅 **Oldest:** {}\n",
                        oldest.format("%Y-%m-%d %H:%M:%S UTC")
                    ));
                }

                if let Some(newest) = stats.newest {
                    message.push_str(&format!(
                        "📅 **Newest:** {}\n",
                        newest.format("%Y-%m-%d %H:%M:%S UTC")
                    ));
                }

                let _ = self.chat.send(SendMessageRequest { message }).await;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Memory statistics: {} total memories",
                    stats.total_memories
                ))]))
            }
            Err(e) => {
                let error_message = format!("❌ Failed to get memory statistics: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_message.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_message)]))
            }
        }
    }

    #[tool(description = "Clean up expired memories")]
    pub async fn cleanup_expired_memories(&self) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: "Cleaning up expired memories...".to_string(),
            })
            .await;

        match self.memory_manager.cleanup_expired_memories().await {
            Ok(expired_count) => {
                let message = if expired_count == 0 {
                    "✅ No expired memories found. All memories are current.".to_string()
                } else {
                    format!("🧹 Cleaned up {} expired memories", expired_count)
                };

                let _ = self.chat.send(SendMessageRequest { message }).await;

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Cleaned up {} expired memories",
                    expired_count
                ))]))
            }
            Err(e) => {
                let error_message = format!("❌ Failed to cleanup expired memories: {}", e);
                let _ = self
                    .chat
                    .send(SendMessageRequest {
                        message: error_message.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_message)]))
            }
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for NostrMemoryServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This Nostr Memory MCP server provides persistent memory storage for AI agents using encrypted Nostr direct messages.\n\n🧠 **MEMORY OPERATIONS**:\n\n📝 **store_memory**: Store new memory entries with type, category, tags, and optional expiry\n🔍 **retrieve_memory**: Search and filter memories by query, type, category, tags, or date range\n✏️ **update_memory**: Modify existing memory entries\n🗑️ **delete_memory**: Remove memory entries by ID\n📊 **memory_stats**: Get statistics about stored memories\n🧹 **cleanup_expired_memories**: Remove expired memory entries\n\n🔐 **PRIVACY & SECURITY**:\n• All memories are encrypted using Nostr NIP-17 private messages\n• Memories are stored as DMs to yourself for maximum privacy\n• Each memory has a unique UUID for precise identification\n• Memories can have expiry dates for automatic cleanup\n\n📋 **MEMORY TYPES**:\n• user_preference: User preferences and settings\n• context: Contextual information about conversations\n• fact: Important facts to remember\n• instruction: Instructions or commands to remember\n• note: General notes and observations\n\n📂 **CATEGORIES**:\n• personal: Personal information\n• work: Work-related memories\n• project: Project-specific information\n• general: General purpose memories\n\n🏷️ **FEATURES**:\n• Full-text search across titles and descriptions\n• Tag-based organization and filtering\n• Priority levels (high, medium, low)\n• Date range filtering\n• Automatic expiry handling\n• Comprehensive statistics\n\n💡 **USAGE TIPS**:\n• Use descriptive titles for easy searching\n• Add relevant tags for better organization\n• Set expiry dates for temporary information\n• Use appropriate types and categories for filtering\n• Regular cleanup of expired memories keeps storage optimal".to_string()),
        }
    }
}
