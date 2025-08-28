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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    task_completed: Arc<AtomicBool>,
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
            task_completed: Arc::new(AtomicBool::new(false)),
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
            // Mark task as completed when final response is sent
            self.task_completed.store(true, Ordering::Relaxed);
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
        // Check if task has been completed - if so, exit instead of waiting
        if self.task_completed.load(Ordering::Relaxed) {
            return Ok(CallToolResult::success(vec![Content::text(
                "Task completed - agent session ending".to_string(),
            )]));
        }

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

impl ServerHandler for Chat {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides tools for talking to a specific user over the Nostr protocol via encrypted DMs.\n\nMANDATORY WORKFLOW FOR EVERY USER MESSAGE:\n\n1. IMMEDIATE PROGRESS RESPONSE: Send a progress update\n   Example: {\"tool\": \"progress\", \"arguments\": {\"message\": \"I'm working on your request...\"}}\n\n2. PERFORM OPERATIONS: Execute the requested tasks\n\n3. MANDATORY FINAL SEND: End with a 'send' tool call containing your complete response\n   Example: {\"tool\": \"send\", \"arguments\": {\"message\": \"Here are the results...\"}}\n\n4. TASK COMPLETION: After sending final response, agent session ends automatically\n\nCRITICAL: Pattern is wait -> progress -> [operations] -> send -> EXIT\n\nUSER VISIBILITY RULES:\n- Users can ONLY see messages sent via 'send' and 'progress' tools\n- If you don't use 'send', the user sees NOTHING\n\nFORBIDDEN BEHAVIORS:\n- Never end a turn without 'send'\n- Never start work without 'progress'\n- Never send follow-up messages asking if user needs help\n- Never ask \"Is there anything else I can help you with?\"\n- Never send unsolicited check-in messages\n- Never continue waiting after sending final response\n- Provide complete answers in single 'send' call then EXIT\n\nJSON PARAMETER RULES:\n- Parameters MUST be valid JSON: {\"message\": \"text\"}\n- Use double quotes only\n- No trailing characters after closing brace\n- No comments outside JSON\n\nPARAMETER PARSING FAILURES WILL BREAK THE SYSTEM".to_string()),
        }
    }
}
