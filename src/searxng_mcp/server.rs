use super::client::SearXNGClient;
use super::types::*;
use crate::mcp::chat::{Chat, ProgressMessageRequest, SendMessageRequest};
use nostr_sdk::prelude::*;
use rmcp::{
    model::{CallToolResult, Content},
    tool, Error as RmcpError,
};

#[derive(Debug, Clone)]
pub struct SearXNGServer {
    client: SearXNGClient,
    chat: Chat,
}

#[tool(tool_box)]
impl SearXNGServer {
    pub fn new(
        base_url: String,
        nostr_client: Client,
        progress_client: Option<Client>,
        our_pubkey: PublicKey,
        target_pubkey: PublicKey,
    ) -> Self {
        Self {
            client: SearXNGClient::new(base_url),
            chat: Chat::new(nostr_client, progress_client, our_pubkey, target_pubkey),
        }
    }

    #[tool(description = "Execute web searches with pagination")]
    pub async fn searxng_web_search(
        &self,
        #[tool(aggr)] request: SearXNGWebSearchRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: format!("Searching for: {}", request.query),
            })
            .await;

        match self.client.search(request).await {
            Ok(response) => {
                let message = if response.results.is_empty() {
                    format!("🔍 No results found for query: {}", response.query)
                } else {
                    let mut message = format!(
                        "🔍 Found {} results for: {} (Page {}, {} per page)\n\n",
                        response.total_results, response.query, response.page, response.per_page
                    );

                    if let Some(answers) = &response.answers {
                        if !answers.is_empty() {
                            message.push_str("💡 **Answers:**\n");
                            for answer in answers {
                                message.push_str(&format!("• {}\n", answer));
                            }
                            message.push('\n');
                        }
                    }

                    message.push_str("📋 **Results:**\n");
                    for (i, result) in response.results.iter().enumerate() {
                        let result_num = (response.page - 1) * response.per_page + i as u32 + 1;
                        message.push_str(&format!(
                            "{}. **{}**\n   🔗 {}\n",
                            result_num, result.title, result.url
                        ));
                        if let Some(content) = &result.content {
                            let truncated_content = if content.len() > 150 {
                                format!("{}...", &content[..150])
                            } else {
                                content.clone()
                            };
                            message.push_str(&format!("   📄 {}\n", truncated_content));
                        }
                        if let Some(engine) = &result.engine {
                            message.push_str(&format!("   🔧 {}\n", engine));
                        }
                        message.push('\n');
                    }

                    if response.total_results > response.results.len() {
                        let remaining =
                            response.total_results - (response.page * response.per_page) as usize;
                        if remaining > 0 {
                            message
                                .push_str(&format!("... {} more results available\n", remaining));
                        }
                    }

                    if let Some(suggestions) = &response.suggestions {
                        if !suggestions.is_empty() {
                            message.push_str("\n💭 **Suggestions:**\n");
                            for suggestion in suggestions {
                                message.push_str(&format!("• {}\n", suggestion));
                            }
                        }
                    }

                    if let Some(corrections) = &response.corrections {
                        if !corrections.is_empty() {
                            message.push_str("\n✏️ **Did you mean:**\n");
                            for correction in corrections {
                                message.push_str(&format!("• {}\n", correction));
                            }
                        }
                    }

                    message
                };

                let _ = self.chat.send(SendMessageRequest { message }).await;

                let search_summary = format!(
                    "Search completed: {} results found for '{}' (page {})",
                    response.total_results, response.query, response.page
                );
                Ok(CallToolResult::success(vec![Content::text(search_summary)]))
            }
            Err(e) => {
                let error_message = format!("❌ Search failed: {}", e);
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
