use crate::utils::wait_for_message;
use nostr_sdk::prelude::*;
use rmcp::{
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, Error as RmcpError, ServerHandler,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SendMessageRequest {
    #[schemars(description = "The message to send to the user")]
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Chat {
    client: Client,
    our_pubkey: PublicKey,
    target_pubkey: PublicKey,
}

#[tool(tool_box)]
impl Chat {
    pub fn new(client: Client, our_pubkey: PublicKey, target_pubkey: PublicKey) -> Self {
        Self {
            client,
            our_pubkey,
            target_pubkey,
        }
    }

    #[tool(description = "Send a message to the user")]
    async fn send(
        &self,
        #[tool(aggr)] SendMessageRequest { message }: SendMessageRequest,
    ) -> Result<CallToolResult, RmcpError> {
        self.client
            .send_private_msg(self.target_pubkey, message, [])
            .await
            .map_err(|e| RmcpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text("Sent message")]))
    }

    #[tool(description = "Listen and wait for the user's next message")]
    async fn wait(&self) -> Result<CallToolResult, RmcpError> {
        let message = wait_for_message(&self.client, &self.our_pubkey, &self.target_pubkey)
            .await
            .map_err(|e| RmcpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(message)]))
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
            instructions: Some("This server provides tools for talking to a specific user over the Nostr protocol via encrypted DMs.".to_string()),
        }
    }
}
