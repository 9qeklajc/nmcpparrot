use crate::goose_mcp::{commands::GooseCommands, types::*};
use rmcp::{
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, Error as RmcpError, ServerHandler,
};

#[derive(Debug, Clone)]
pub struct GooseServer;

#[tool]
impl GooseServer {
    pub fn new() -> Self {
        Self
    }

    #[tool(
        description = "Execute a Goose task with the given instructions. Supports both text instructions and instruction files."
    )]
    async fn runtask(
        &self,
        #[tool(aggr)] request: RunTaskRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::run_task(request).await;
        Self::convert_result(result)
    }

    #[tool(
        description = "Start a new Goose session or resume an existing one with specified configuration."
    )]
    async fn startsession(
        &self,
        #[tool(aggr)] request: SessionRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::start_session(request).await;
        Self::convert_result(result)
    }

    #[tool(description = "List all saved Goose sessions with optional filtering and formatting.")]
    async fn listsessions(
        &self,
        #[tool(aggr)] request: SessionListRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::list_sessions(request).await;
        Self::convert_result(result)
    }

    #[tool(description = "Remove one or more Goose sessions by ID, name, or regex pattern.")]
    async fn removesession(
        &self,
        #[tool(aggr)] request: SessionRemoveRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::remove_session(request).await;
        Self::convert_result(result)
    }

    #[tool(description = "Export a Goose session to Markdown format for sharing or documentation.")]
    async fn exportsession(
        &self,
        #[tool(aggr)] request: SessionExportRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::export_session(request).await;
        Self::convert_result(result)
    }

    #[tool(
        description = "Configure Goose settings including providers, extensions, and other options."
    )]
    async fn configure(
        &self,
        #[tool(aggr)] request: ConfigureRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::configure(request).await;
        Self::convert_result(result)
    }

    #[tool(
        description = "Update Goose CLI to a newer version with optional canary or reconfiguration."
    )]
    async fn update(
        &self,
        #[tool(aggr)] request: UpdateRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::update(request).await;
        Self::convert_result(result)
    }

    #[tool(
        description = "Show Goose information including version, configuration, and system details."
    )]
    async fn info(&self, #[tool(aggr)] request: InfoRequest) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::info(request).await;
        Self::convert_result(result)
    }

    #[tool(description = "Get the current Goose version.")]
    async fn version(&self) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::version().await;
        Self::convert_result(result)
    }

    #[tool(description = "Display Goose help information.")]
    async fn goose_help(&self) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::help().await;
        Self::convert_result(result)
    }

    #[tool(description = "List available or installed MCP servers for Goose.")]
    async fn mcp_list(
        &self,
        #[tool(aggr)] request: McpListRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::mcp_list(request).await;
        Self::convert_result(result)
    }

    #[tool(description = "Install an MCP server for use with Goose.")]
    async fn mcp_install(
        &self,
        #[tool(aggr)] request: McpInstallRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::mcp_install(request).await;
        Self::convert_result(result)
    }

    #[tool(
        description = "Manage Goose projects - start working on existing or create new projects."
    )]
    async fn projectmanagement(
        &self,
        #[tool(aggr)] request: ProjectRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::project_management(request).await;
        Self::convert_result(result)
    }

    #[tool(description = "List all available Goose projects.")]
    async fn listprojects(&self) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::list_projects().await;
        Self::convert_result(result)
    }

    #[tool(description = "Force terminate all active Goose sessions and cleanup execution state.")]
    async fn killsessions(&self) -> Result<CallToolResult, RmcpError> {
        let result = GooseCommands::kill_all_sessions().await;
        Self::convert_result(result)
    }

    #[tool(description = "Check if any Goose sessions are currently active.")]
    async fn checksessions(&self) -> Result<CallToolResult, RmcpError> {
        let has_active = GooseCommands::has_active_sessions();
        let message = if has_active {
            "Active Goose sessions detected".to_string()
        } else {
            "No active Goose sessions".to_string()
        };
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    fn convert_result(result: CommandResult) -> Result<CallToolResult, RmcpError> {
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

impl ServerHandler for GooseServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("Goose MCP server provides comprehensive tools for interacting with the Goose AI agent CLI.\n\nMANDATORY WORKFLOW FOR EVERY USER MESSAGE:\n\n1. IMMEDIATE PROGRESS RESPONSE: Send a progress update\n   Example: {\"tool\": \"progress\", \"arguments\": {\"message\": \"Executing Goose operation...\"}}\n\n2. SESSION MANAGEMENT: Check and manage active sessions\n   - Use checksessions to verify current state\n   - Use killsessions to cleanup when needed\n\n3. EXECUTE OPERATIONS: Perform requested Goose operations\n   - runtask for headless execution\n   - startsession for interactive sessions\n   - Configuration and project management\n\n4. MANDATORY FINAL SEND: End with a 'send' tool call containing results\n   Example: {\"tool\": \"send\", \"arguments\": {\"message\": \"Goose operation completed successfully\"}}\n\nCRITICAL: Pattern is wait -> progress -> [goose operations] -> send -> EXIT\n\nSESSION MANAGEMENT RULES:\n- Check active sessions before starting new operations\n- Prevent duplicate execution of same task\n- Always terminate sessions after completion\n- Use killsessions to force cleanup when needed\n- Look for completion markers in outputs\n\nUSER VISIBILITY RULES:\n- Users can ONLY see messages sent via 'send' and 'progress' tools\n- If you don't use 'send', the user sees NOTHING\n- Always provide progress updates so users know work is happening\n\nFORBIDDEN BEHAVIORS:\n- Never end a turn without 'send'\n- Never start work without 'progress'\n- Never execute same command multiple times\n- Never start tasks without checking active sessions\n- Never leave sessions active after completion\n- Never send follow-up messages asking if user needs help\n- Never ask \"Is there anything else I can help you with?\"\n- Never send unsolicited check-in messages\n\nAVAILABLE TOOLS:\n- runtask: Execute instructions (with deduplication)\n- startsession: Start interactive session (with tracking)\n- killsessions: Force terminate all sessions\n- checksessions: Check for active sessions\n- Session, project, and configuration management tools\n\nERROR HANDLING:\n- If \"already being executed\" error: inform user to wait\n- If timeout errors: use killsessions then retry\n- If hanging: force terminate with killsessions\n- Always cleanup state after errors\n\nJSON PARAMETER RULES:\n- Parameters MUST be valid JSON: {\"message\": \"text\"}\n- Use double quotes only\n- No trailing characters after closing brace\n- No comments outside JSON\n\nPARAMETER PARSING FAILURES WILL BREAK THE SYSTEM".to_string()),
        }
    }
}
