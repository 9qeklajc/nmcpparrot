use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunTaskRequest {
    pub instructions: String,
    pub instruction_file: Option<String>,
    pub max_turns: Option<u32>,
    pub debug: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionRequest {
    pub name: Option<String>,
    pub id: Option<String>,
    pub resume: Option<bool>,
    pub with_extension: Option<String>,
    pub with_builtin: Option<String>,
    pub debug: Option<bool>,
    pub max_turns: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionListRequest {
    pub verbose: Option<bool>,
    pub format: Option<String>,
    pub ascending: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionRemoveRequest {
    pub id: Option<String>,
    pub name: Option<String>,
    pub regex: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionExportRequest {
    pub id: Option<String>,
    pub name: Option<String>,
    pub path: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConfigureRequest {
    pub reconfigure: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateRequest {
    pub canary: Option<bool>,
    pub reconfigure: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InfoRequest {
    pub verbose: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct McpListRequest {
    pub available: Option<bool>,
    pub installed: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct McpInstallRequest {
    pub server: String,
    pub force: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectRequest {
    pub project: Option<String>,
    pub new: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub exit_code: i32,
}

impl CommandResult {
    pub fn success(output: String) -> Self {
        Self {
            success: true,
            output,
            error: None,
            exit_code: 0,
        }
    }

    pub fn error(error: String, exit_code: i32) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error),
            exit_code,
        }
    }
}
