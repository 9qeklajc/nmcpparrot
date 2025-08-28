use crate::goose_mcp::{commands::GooseCommands, types::*};
use crate::mcp::chat::{Chat, ProgressMessageRequest, SendMessageRequest};
use crate::searxng_mcp::{SearXNGServer, SearXNGWebSearchRequest};
use nostr_sdk::prelude::*;
use rmcp::{
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, Error as RmcpError, ServerHandler,
};

#[derive(Debug, Clone)]
pub struct CombinedServer {
    chat: Chat,
    searxng: SearXNGServer,
}

#[tool(tool_box)]
impl CombinedServer {
    pub fn new(
        client: Client,
        progress_client: Option<Client>,
        our_pubkey: PublicKey,
        target_pubkey: PublicKey,
        searxng_url: String,
    ) -> Self {
        Self {
            chat: Chat::new(
                client.clone(),
                progress_client.clone(),
                our_pubkey,
                target_pubkey,
            ),
            searxng: SearXNGServer::new(
                searxng_url,
                client,
                progress_client,
                our_pubkey,
                target_pubkey,
            ),
        }
    }

    #[tool(description = "Send a message to the user via Nostr DM")]
    async fn send(
        &self,
        #[tool(aggr)] request: SendMessageRequest,
    ) -> Result<CallToolResult, RmcpError> {
        self.chat.send(request).await
    }

    #[tool(description = "Send a progress/debug message to the user via the progress identity")]
    async fn progress(
        &self,
        #[tool(aggr)] request: ProgressMessageRequest,
    ) -> Result<CallToolResult, RmcpError> {
        self.chat.progress(request).await
    }

    #[tool(description = "Listen and wait for the user's next message")]
    async fn wait(&self) -> Result<CallToolResult, RmcpError> {
        // The Chat wait method already includes response reminders
        self.chat.wait().await
    }

    #[tool(
        description = "Execute a Goose task with the given instructions. Supports both text instructions and instruction files."
    )]
    async fn runtask(
        &self,
        #[tool(aggr)] request: RunTaskRequest,
    ) -> Result<CallToolResult, RmcpError> {
        // Check for active sessions first
        if GooseCommands::has_active_sessions() {
            let warning_message = "⚠️ Active Goose sessions detected. Use 'killsessions' to terminate them before starting new tasks.".to_string();
            let _ = self
                .chat
                .send(SendMessageRequest {
                    message: warning_message,
                })
                .await;
            return Ok(CallToolResult::error(vec![Content::text(
                "Active sessions must be terminated first".to_string(),
            )]));
        }

        // Send progress update
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: "Starting Goose task execution...".to_string(),
            })
            .await;

        let result = GooseCommands::run_task(request).await;

        // Send result to user via chat
        let message = if result.success {
            let has_completion_marker = result.output.contains("🔚 EXECUTION COMPLETED");
            let base_message =
                format!("✅ Goose task completed successfully:\n\n{}", result.output);

            if has_completion_marker {
                format!("{}\n\n🔚 Task execution finished. Use 'killsessions' to cleanup and terminate.", base_message)
            } else {
                base_message
            }
        } else {
            let error_msg = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            format!(
                "❌ Goose task failed (exit code {}):\n\n{}",
                result.exit_code, error_msg
            )
        };

        let _ = self.chat.send(SendMessageRequest { message }).await;

        Self::convert_goose_result(result)
    }

    #[tool(
        description = "Start a new Goose session or resume an existing one with specified configuration."
    )]
    async fn startsession(
        &self,
        #[tool(aggr)] request: SessionRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let session_name = request
            .name
            .clone()
            .unwrap_or_else(|| "new session".to_string());

        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: format!("Starting Goose session: {}", session_name),
            })
            .await;

        let result = GooseCommands::start_session(request).await;

        // Send result to user via chat
        let message = if result.success {
            format!(
                "✅ Goose session started successfully:\n\n{}",
                result.output
            )
        } else {
            let error_msg = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            format!(
                "❌ Failed to start Goose session (exit code {}):\n\n{}",
                result.exit_code, error_msg
            )
        };

        let _ = self.chat.send(SendMessageRequest { message }).await;

        Self::convert_goose_result(result)
    }

    #[tool(description = "List all saved Goose sessions with optional filtering and formatting.")]
    async fn listsessions(
        &self,
        #[tool(aggr)] request: SessionListRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: "Retrieving Goose sessions...".to_string(),
            })
            .await;

        let result = GooseCommands::list_sessions(request).await;

        // Send result to user via chat
        let message = if result.success {
            if result.output.trim().is_empty() {
                "📋 No Goose sessions found.".to_string()
            } else {
                format!("📋 Goose sessions:\n\n{}", result.output)
            }
        } else {
            let error_msg = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            format!(
                "❌ Failed to list sessions (exit code {}):\n\n{}",
                result.exit_code, error_msg
            )
        };

        let _ = self.chat.send(SendMessageRequest { message }).await;

        Self::convert_goose_result(result)
    }

    #[tool(description = "Remove one or more Goose sessions by ID, name, or regex pattern.")]
    async fn removesession(
        &self,
        #[tool(aggr)] request: SessionRemoveRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::remove_session(request).await;
        Self::convert_goose_result(result)
    }

    #[tool(description = "Export a Goose session to Markdown format for sharing or documentation.")]
    async fn exportsession(
        &self,
        #[tool(aggr)] request: SessionExportRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let session_name = request
            .name
            .clone()
            .unwrap_or_else(|| "session".to_string());

        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: format!("Exporting Goose session: {}", session_name),
            })
            .await;

        let result = GooseCommands::export_session(request).await;

        // Send result to user via chat
        let message = if result.success {
            format!("✅ Session exported successfully:\n\n{}", result.output)
        } else {
            let error_msg = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            format!(
                "❌ Failed to export session (exit code {}):\n\n{}",
                result.exit_code, error_msg
            )
        };

        let _ = self.chat.send(SendMessageRequest { message }).await;

        Self::convert_goose_result(result)
    }

    #[tool(
        description = "Configure Goose settings including providers, extensions, and other options."
    )]
    async fn configure(
        &self,
        #[tool(aggr)] request: ConfigureRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::configure(request).await;
        Self::convert_goose_result(result)
    }

    #[tool(
        description = "Update Goose CLI to a newer version with optional canary or reconfiguration."
    )]
    async fn update(
        &self,
        #[tool(aggr)] request: UpdateRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::update(request).await;
        Self::convert_goose_result(result)
    }

    #[tool(
        description = "Show Goose information including version, configuration, and system details."
    )]
    async fn info(&self, #[tool(aggr)] request: InfoRequest) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: "Retrieving Goose system information...".to_string(),
            })
            .await;

        let result = GooseCommands::info(request).await;

        // Send result to user via chat
        let message = if result.success {
            format!("ℹ️ Goose system information:\n\n{}", result.output)
        } else {
            let error_msg = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            format!(
                "❌ Failed to get Goose info (exit code {}):\n\n{}",
                result.exit_code, error_msg
            )
        };

        let _ = self.chat.send(SendMessageRequest { message }).await;

        Self::convert_goose_result(result)
    }

    #[tool(description = "Get the current Goose version.")]
    async fn version(&self) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::version().await;

        // Send result to user via chat
        let message = if result.success {
            format!("🔢 Goose version:\n\n{}", result.output)
        } else {
            let error_msg = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            format!(
                "❌ Failed to get Goose version (exit code {}):\n\n{}",
                result.exit_code, error_msg
            )
        };

        let _ = self.chat.send(SendMessageRequest { message }).await;

        Self::convert_goose_result(result)
    }

    #[tool(description = "Display Goose help information.")]
    async fn goose_help(&self) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::help().await;
        Self::convert_goose_result(result)
    }

    #[tool(description = "List available or installed MCP servers for Goose.")]
    async fn mcp_list(
        &self,
        #[tool(aggr)] request: McpListRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::mcp_list(request).await;
        Self::convert_goose_result(result)
    }

    #[tool(description = "Install an MCP server for use with Goose.")]
    async fn mcp_install(
        &self,
        #[tool(aggr)] request: McpInstallRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::mcp_install(request).await;
        Self::convert_goose_result(result)
    }

    #[tool(
        description = "Manage Goose projects - start working on existing or create new projects."
    )]
    async fn projectmanagement(
        &self,
        #[tool(aggr)] request: ProjectRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::project_management(request).await;
        Self::convert_goose_result(result)
    }

    #[tool(description = "List all available Goose projects.")]
    async fn listprojects(&self) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::list_projects().await;
        Self::convert_goose_result(result)
    }

    #[tool(description = "Force terminate all active Goose sessions and cleanup execution state.")]
    async fn killsessions(&self) -> Result<CallToolResult, RmcpError> {
        let _ = self
            .chat
            .progress(ProgressMessageRequest {
                message: "Terminating all active Goose sessions...".to_string(),
            })
            .await;

        let result = GooseCommands::kill_all_sessions().await;

        // Send result to user via chat
        let message = if result.success {
            format!("🔚 All Goose sessions terminated:\n\n{}", result.output)
        } else {
            let error_msg = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            format!(
                "❌ Failed to terminate sessions (exit code {}):\n\n{}",
                result.exit_code, error_msg
            )
        };

        let _ = self.chat.send(SendMessageRequest { message }).await;
        Self::convert_goose_result(result)
    }

    #[tool(description = "Check if any Goose sessions are currently active.")]
    async fn checksessions(&self) -> Result<CallToolResult, RmcpError> {
        let has_active = GooseCommands::has_active_sessions();
        let message = if has_active {
            "⚠️ Active Goose sessions detected - use killsessions to terminate".to_string()
        } else {
            "✅ No active Goose sessions".to_string()
        };

        let _ = self.chat.send(SendMessageRequest { message }).await;
        Ok(CallToolResult::success(vec![Content::text(
            if has_active {
                "Active sessions detected"
            } else {
                "No active sessions"
            }
            .to_string(),
        )]))
    }

    #[tool(description = "Execute web searches with pagination")]
    async fn searxng_web_search(
        &self,
        #[tool(aggr)] request: SearXNGWebSearchRequest,
    ) -> Result<CallToolResult, RmcpError> {
        self.searxng.searxng_web_search(request).await
    }

    fn convert_goose_result(result: CommandResult) -> Result<CallToolResult, RmcpError> {
        if result.success {
            Ok(CallToolResult::success(vec![Content::text(result.output)]))
        } else {
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
            let formatted_error = format!(
                "Command failed (exit code {}): {}",
                result.exit_code, error_msg
            );
            Ok(CallToolResult::error(vec![Content::text(formatted_error)]))
        }
    }
}

impl ServerHandler for CombinedServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This combined server provides both Nostr chat capabilities and comprehensive Goose AI agent command execution.\n\n🚨 ABSOLUTELY MANDATORY FOR EVERY USER MESSAGE:\n\n1️⃣ **IMMEDIATE PROGRESS RESPONSE**: The INSTANT you receive a user message, you MUST send a progress update\n   Example: {\"tool\": \"progress\", \"arguments\": {\"message\": \"I'm starting your Goose operation...\"}}\n\n2️⃣ **SESSION CHECK**: Before ANY Goose operation, check for active sessions\n   Example: {\"tool\": \"checksessions\"}\n\n3️⃣ **EXECUTE OPERATION**: Use requested Goose tool (runtask, startsession, etc.)\n\n4️⃣ **SESSION CLEANUP**: After completion, terminate sessions\n   Example: {\"tool\": \"killsessions\"}\n\n5️⃣ **MANDATORY FINAL SEND**: You MUST ALWAYS end with a 'send' tool call - NO EXCEPTIONS\n   Example: {\"tool\": \"send\", \"arguments\": {\"message\": \"✅ Goose operation completed and cleaned up\"}}\n\n🔴 CRITICAL: EVERY conversation turn MUST follow this pattern:\n   wait → progress → checksessions → [goose operations] → killsessions → send\n\n🚨 **DUPLICATE PREVENTION & SESSION MANAGEMENT**:\n• NEVER execute same command multiple times for one request\n• ALWAYS check sessions before starting new operations\n• ALWAYS terminate sessions after completion\n• If \"already being executed\" error: STOP and inform user\n• Look for \"🔚 EXECUTION COMPLETED\" marker in outputs\n• Use 'killsessions' to force cleanup when needed\n\n📢 USER VISIBILITY RULES:\n• Users can ONLY see messages sent via 'send' and 'progress' tools\n• Users CANNOT see your thinking, reasoning, or stdout output\n• If you don't use 'send', the user sees NOTHING\n• If you don't use 'progress', users think you're not working\n• Goose operations automatically send results, but you MUST still send final confirmation\n\n❌ FORBIDDEN BEHAVIORS:\n• Never end a turn without 'send'\n• Never start Goose work without 'progress'\n• Never execute operations without checking sessions first\n• Never leave sessions active after completion\n• Never execute duplicate commands\n• Never assume the user knows what you're doing\n• Never skip final confirmation even if Goose auto-sends results\n• Never send follow-up messages asking if user needs help\n• Never ask \"Is there anything else I can help you with?\"\n• Never send unsolicited check-in messages after task completion\n\n🛡️ **SESSION MANAGEMENT TOOLS**:\n• 'checksessions' - Check for active sessions (use before operations)\n• 'killsessions' - Force terminate all sessions (use after completion)\n• 'runtask' - Execute with deduplication protection\n• 'startsession' - Start with session tracking\n\n🔧 CRITICAL JSON PARAMETER RULES:\n• Parameters MUST be a SINGLE, complete JSON object: {\"instructions\": \"text\"}\n• Use ONLY double quotes, never single quotes\n• ABSOLUTELY NO text, characters, or content after the closing brace }\n• NO comments, explanations, or additional text outside the JSON\n• Properly escape quotes and backslashes inside strings\n• Example of CORRECT format: {\"instructions\": \"analyze the code\"}\n• Example of WRONG format: {\"instructions\": \"analyze code\"}\\nExecuting now...\n• Example of WRONG format: {\"instructions\": \"analyze code\"} // starting analysis\n\n⚠️ TRAILING CHARACTERS ERROR: If you see \"trailing characters\" errors, you have text after the JSON.\n\n💀 PARAMETER PARSING FAILURES WILL BREAK THE ENTIRE SYSTEM\n💀 SESSION MANAGEMENT FAILURES WILL CAUSE DUPLICATE RESPONSES".to_string()),
        }
    }
}
