use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub task: String,
    pub status: AgentStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_active: chrono::DateTime<chrono::Utc>,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Starting,
    Running,
    Idle,
    Busy,
    Error(String),
    Stopping,
    Stopped,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Starting => write!(f, "Starting"),
            AgentStatus::Running => write!(f, "Running"),
            AgentStatus::Idle => write!(f, "Idle"),
            AgentStatus::Busy => write!(f, "Busy"),
            AgentStatus::Error(e) => write!(f, "Error: {}", e),
            AgentStatus::Stopping => write!(f, "Stopping"),
            AgentStatus::Stopped => write!(f, "Stopped"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub id: String,
    #[allow(dead_code)] // Future message routing
    pub from_agent: Option<String>,
    #[allow(dead_code)] // Future message routing
    pub to_agent: Option<String>,
    pub message_type: MessageType,
    pub content: String,
    #[allow(dead_code)] // Future timestamp tracking
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub response_channel: Option<mpsc::UnboundedSender<String>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Future message type expansion
pub enum MessageType {
    Task,
    Response,
    Progress,
    Error,
    Status,
    Heartbeat,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // System monitoring data structure
pub struct SystemStatus {
    pub active_agents: usize,
    pub max_agents: usize,
    pub memory_usage_percent: f64,
    pub cpu_usage_percent: f64,
    pub uptime_seconds: u64,
    pub messages_processed: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CreateAgentRequest {
    #[schemars(description = "Type of agent to create (chat, goose, search, combined)")]
    pub agent_type: String,
    #[schemars(description = "Initial task description for the agent")]
    pub task: String,
    #[schemars(description = "Optional specific capabilities to enable")]
    pub capabilities: Option<Vec<String>>,
    #[schemars(description = "Optional timeout in seconds")]
    pub timeout_seconds: Option<u64>,
    #[schemars(description = "Optional priority level (1-10, higher is more priority)")]
    #[allow(dead_code)] // Future priority support
    pub priority: Option<u8>,
    #[schemars(description = "Optional metadata key-value pairs")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateMultipleAgentsRequest {
    #[schemars(description = "List of agents to create in parallel")]
    pub agents: Vec<CreateAgentRequest>,
    #[schemars(description = "Execution strategy (parallel is default for this function)")]
    #[allow(dead_code)] // Future use for execution strategy options
    pub execution_strategy: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StopAgentRequest {
    #[schemars(description = "ID of the agent to stop")]
    pub agent_id: String,
    #[schemars(description = "Whether to force stop (true) or graceful shutdown (false)")]
    #[allow(dead_code)] // Future force stop support
    pub force: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MessageAgentRequest {
    #[schemars(description = "ID of the agent to send message to")]
    pub agent_id: String,
    #[schemars(description = "Message content to send")]
    pub message: String,
    #[schemars(description = "Whether to wait for response")]
    #[allow(dead_code)] // Future response waiting
    pub wait_for_response: Option<bool>,
    #[schemars(description = "Timeout for response in seconds")]
    #[allow(dead_code)] // Future timeout support
    pub timeout_seconds: Option<u64>,
}

#[derive(schemars::JsonSchema, serde::Deserialize, Debug)]
pub struct AnalyzeRequestArgs {
    #[schemars(description = "The user request to analyze and break down into sub-tasks")]
    pub request: String,
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_agents: usize,
    pub default_timeout_seconds: u64,
    pub health_check_interval_seconds: u64,
    #[allow(dead_code)] // Future queue management
    pub message_queue_size: usize,
    pub memory_limit_percent: f64,
    pub cpu_limit_percent: f64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_agents: 10,
            default_timeout_seconds: 300, // 5 minutes (reduced for faster feedback)
            health_check_interval_seconds: 60, // Check every minute (increased from 30s)
            message_queue_size: 1000,
            memory_limit_percent: 80.0,
            cpu_limit_percent: 80.0,
        }
    }
}

#[derive(Debug)]
pub struct AgentHandle {
    #[allow(dead_code)] // Future handle management
    pub id: String,
    pub sender: mpsc::UnboundedSender<AgentMessage>,
    pub join_handle: tokio::task::JoinHandle<()>,
}

pub type AgentError = Box<dyn std::error::Error + Send + Sync>;
pub type AgentResult<T> = Result<T, AgentError>;
