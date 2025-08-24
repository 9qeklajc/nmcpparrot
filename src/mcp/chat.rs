use crate::response_tracker::{create_response_reminder, ResponseTracker};
use crate::utils::wait_for_message;
use nostr_sdk::prelude::*;
use rmcp::{
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, Error as RmcpError, ServerHandler,
};
use tokio::time::{sleep, Duration};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SendMessageRequest {
    #[schemars(description = "The message to send to the user")]
    pub message: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProgressMessageRequest {
    #[schemars(description = "The progress/debug message to send to the user")]
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Chat {
    client: Client,
    progress_client: Option<Client>,
    our_pubkey: PublicKey,
    target_pubkey: PublicKey,
    response_tracker: ResponseTracker,
}

#[tool(tool_box)]
impl Chat {
    pub fn new(
        client: Client,
        progress_client: Option<Client>,
        our_pubkey: PublicKey,
        target_pubkey: PublicKey,
    ) -> Self {
        Self {
            client,
            progress_client,
            our_pubkey,
            target_pubkey,
            response_tracker: ResponseTracker::new(),
        }
    }

    #[tool(description = "Send a message to the user")]
    pub async fn send(
        &self,
        #[tool(aggr)] SendMessageRequest { message }: SendMessageRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = self.send_with_retry(&self.client, message).await;
        if result.is_ok() {
            self.response_tracker.mark_response_sent();
        }
        result
    }

    #[tool(description = "Send a progress/debug message to the user via the progress identity")]
    pub async fn progress(
        &self,
        #[tool(aggr)] ProgressMessageRequest { message }: ProgressMessageRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = match &self.progress_client {
            Some(c) => self.send_with_retry(c, message).await,
            None => Err(RmcpError::internal_error(
                "Progress identity not configured",
                None,
            )),
        };
        if result.is_ok() {
            self.response_tracker.mark_progress_sent();
        }
        result
    }

    #[tool(description = "Listen and wait for the user's next message")]
    pub async fn wait(&self) -> Result<CallToolResult, RmcpError> {
        let message = wait_for_message(&self.client, &self.our_pubkey, &self.target_pubkey)
            .await
            .map_err(|e| RmcpError::internal_error(e.to_string(), None))?;

        self.response_tracker.start_conversation();

        let reminder = create_response_reminder();
        let enhanced_message = format!("{}\n\n{}", message, reminder);

        Ok(CallToolResult::success(vec![Content::text(
            enhanced_message,
        )]))
    }

    async fn send_with_retry(
        &self,
        client: &Client,
        message: String,
    ) -> Result<CallToolResult, RmcpError> {
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 1000;
        let mut last_error = String::new();

        for attempt in 0..MAX_RETRIES {
            let result = client
                .send_private_msg(self.target_pubkey, message.clone(), [])
                .await;
            match result {
                Ok(_) => {
                    let msg = if attempt == 0 {
                        "Sent message"
                    } else {
                        "Sent message after retry"
                    };
                    return Ok(CallToolResult::success(vec![Content::text(
                        msg.to_string(),
                    )]));
                }
                Err(e) => {
                    last_error = e.to_string();
                    log::warn!("Attempt {} failed: {}", attempt + 1, last_error);
                }
            }

            if attempt < MAX_RETRIES - 1 {
                let delay = Duration::from_millis(BASE_DELAY_MS * (1 << attempt));
                log::info!("Retrying in {}ms...", delay.as_millis());
                sleep(delay).await;
            }
        }

        Err(RmcpError::internal_error(
            format!(
                "Failed to send message after {} attempts: {}",
                MAX_RETRIES, last_error
            ),
            None,
        ))
    }
}

#[tool(tool_box)]
impl ServerHandler for Chat {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides tools for talking to a specific user over the Nostr protocol via encrypted DMs.\n\n🚨 ABSOLUTELY MANDATORY FOR EVERY USER MESSAGE:\n\n1️⃣ **IMMEDIATE PROGRESS RESPONSE**: The INSTANT you receive a user message, you MUST send a progress update\n   Example: {\"tool\": \"progress\", \"arguments\": {\"message\": \"I'm working on your request...\"}}\n\n2️⃣ **PERFORM OPERATIONS**: Execute the requested tasks\n\n3️⃣ **MANDATORY FINAL SEND**: You MUST ALWAYS end with a 'send' tool call - NO EXCEPTIONS\n   Example: {\"tool\": \"send\", \"arguments\": {\"message\": \"Here are the results...\"}}\n\n🔴 CRITICAL: EVERY conversation turn MUST follow this pattern:\n   wait → progress → [operations] → send\n\n📢 USER VISIBILITY RULES:\n• Users can ONLY see messages sent via 'send' and 'progress' tools\n• Users CANNOT see your thinking, reasoning, or stdout output\n• If you don't use 'send', the user sees NOTHING\n• If you don't use 'progress', users think you're not working\n\n❌ FORBIDDEN BEHAVIORS:\n• Never end a turn without 'send'\n• Never start work without 'progress'\n• Never assume the user knows what you're doing\n• Never output to stdout/terminal\n\n🔧 CRITICAL JSON PARAMETER RULES:\n• Parameters MUST be a SINGLE, complete JSON object: {\"message\": \"text\"}\n• Use ONLY double quotes, never single quotes\n• ABSOLUTELY NO text, characters, or content after the closing brace }\n• NO comments, explanations, or additional text outside the JSON\n• Properly escape quotes and backslashes inside strings\n• Example of CORRECT format: {\"message\": \"Hello world\"}\n• Example of WRONG format: {\"message\": \"Hello world\"}\\nI'm working on this\n• Example of WRONG format: {\"message\": \"Hello world\"} // sending message\n\n⚠️ TRAILING CHARACTERS ERROR: If you see \"trailing characters\" errors, you have text after the JSON.\n\n💀 PARAMETER PARSING FAILURES WILL BREAK THE ENTIRE SYSTEM".to_string()),
        }
    }
}
