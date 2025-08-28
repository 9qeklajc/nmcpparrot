use crate::goose_mcp::{commands::GooseCommands, types::*};
use rmcp::{
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, Error as RmcpError, ServerHandler,
};

#[derive(Debug, Clone)]
pub struct GooseServer;

#[tool(tool_box)]
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
            instructions: Some("This server provides comprehensive tools for interacting with the Goose AI agent CLI. You can execute tasks, manage sessions, configure settings, handle projects, and perform all major Goose operations.\n\n🚨 CRITICAL SESSION MANAGEMENT & DUPLICATE PREVENTION:\n\n🔄 **EXECUTION CONTROL**:\n• Each request is tracked to prevent duplicate execution\n• If same task is already running, you'll get an error message\n• Use 'checksessions' to verify current execution state\n• Use 'killsessions' to force terminate all active sessions\n\n⚠️ **DUPLICATE RESPONSE PREVENTION**:\n• NEVER execute the same command multiple times for one request\n• If you get \"already being executed\" error, STOP and inform user\n• Wait for current execution to complete before new requests\n• Check execution status before starting new operations\n\n🔚 **MANDATORY SESSION TERMINATION**:\n• After completing ANY task, check for active sessions\n• Use 'killsessions' to cleanup when task is done\n• Look for \"🔚 EXECUTION COMPLETED\" marker in outputs\n• ALWAYS terminate sessions after successful completion\n\n📋 **REQUIRED WORKFLOW**:\n1. Check if sessions active (checksessions)\n2. Execute requested operation (runtask/startsession/etc)\n3. Wait for completion marker in output\n4. Terminate sessions (killsessions)\n5. Confirm cleanup completed\n\n🛡️ **ERROR HANDLING**:\n• If \"already being executed\" error: inform user to wait\n• If timeout errors: use killsessions then retry\n• If hanging: force terminate with killsessions\n• Always cleanup state after errors\n\n🚫 **STRICTLY FORBIDDEN**:\n• Multiple executions of same command\n• Starting new tasks without checking active sessions\n• Leaving sessions active after completion\n• Ignoring duplicate execution warnings\n• Sending follow-up messages asking if user needs help\n• Asking \"Is there anything else I can help you with?\"\n• Unsolicited check-in messages after task completion\n\n⚡ **TOOLS AVAILABLE**:\n• 'runtask' - Execute instructions (with deduplication)\n• 'startsession' - Start interactive session (with tracking)\n• 'killsessions' - Force terminate all sessions\n• 'checksessions' - Check for active sessions\n• All standard Goose operations with session management\n\n💀 **FAILURE TO FOLLOW SESSION MANAGEMENT WILL CAUSE**:\n❌ Duplicate responses to users\n❌ Multiple agents responding to same request\n❌ System resource exhaustion\n❌ Hanging/zombie processes\n❌ Broken user experience\n\nUse 'run_task' for headless execution of instructions, 'start_session' for interactive sessions, and various management tools for sessions, projects, and configuration. All commands support the full range of Goose CLI options and return structured results with success/failure status and detailed output.".to_string()),
        }
    }
}
