use crate::goose_mcp::types::*;
use log;
use std::collections::HashMap;
use std::io::Write;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tempfile::NamedTempFile;
use tokio::time::timeout;

// Global execution tracking to prevent duplicate commands
lazy_static::lazy_static! {
    static ref EXECUTION_TRACKER: Arc<Mutex<HashMap<String, Instant>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref ACTIVE_SESSIONS: Arc<Mutex<HashMap<String, bool>>> = Arc::new(Mutex::new(HashMap::new()));
}

pub struct GooseCommands;

impl GooseCommands {
    pub async fn run_task(request: RunTaskRequest) -> CommandResult {
        // Create unique execution key for deduplication
        let execution_key = format!(
            "runtask_{}",
            request
                .instructions
                .chars()
                .take(50)
                .collect::<String>()
                .replace(" ", "_")
        );

        // Check if this exact command is already being executed
        if let Ok(mut tracker) = EXECUTION_TRACKER.lock() {
            if let Some(last_execution) = tracker.get(&execution_key) {
                if last_execution.elapsed() < Duration::from_secs(10) {
                    return CommandResult::error(
                        "Same task is already being executed. Please wait.".to_string(),
                        -1,
                    );
                }
            }
            tracker.insert(execution_key.clone(), Instant::now());
        }

        let mut cmd = Command::new("goose");
        cmd.arg("run");

        if let Some(file_path) = &request.instruction_file {
            cmd.arg("-i").arg(file_path);
        } else {
            if request.instructions.trim().is_empty() {
                // Clean up tracker
                if let Ok(mut tracker) = EXECUTION_TRACKER.lock() {
                    tracker.remove(&execution_key);
                }
                return CommandResult::error("Instructions cannot be empty".to_string(), 1);
            }

            match Self::create_temp_file(&request.instructions) {
                Ok(temp_file) => {
                    cmd.arg("-i").arg(temp_file.path());
                    let result = Self::execute_command_with_cleanup(cmd, execution_key).await;
                    return result;
                }
                Err(e) => {
                    // Clean up tracker
                    if let Ok(mut tracker) = EXECUTION_TRACKER.lock() {
                        tracker.remove(&execution_key);
                    }
                    return CommandResult::error(format!("Failed to create temp file: {}", e), 1);
                }
            }
        }

        if let Some(max_turns) = request.max_turns {
            cmd.arg("--max-turns").arg(max_turns.to_string());
        }

        if request.debug.unwrap_or(false) {
            cmd.arg("--debug");
        }

        Self::execute_command_with_cleanup(cmd, execution_key).await
    }

    pub async fn start_session(request: SessionRequest) -> CommandResult {
        let session_id = request
            .id
            .clone()
            .unwrap_or_else(|| format!("session_{}", chrono::Utc::now().timestamp()));

        // Check if session is already active
        if let Ok(mut sessions) = ACTIVE_SESSIONS.lock() {
            if sessions.get(&session_id).unwrap_or(&false) == &true {
                return CommandResult::error(
                    format!("Session {} is already active", session_id),
                    -1,
                );
            }
            sessions.insert(session_id.clone(), true);
        }

        let mut cmd = Command::new("goose");
        cmd.arg("session");

        if let Some(name) = &request.name {
            cmd.arg("--name").arg(name);
        }

        if request.resume.unwrap_or(false) {
            cmd.arg("--resume");
            if let Some(id) = &request.id {
                cmd.arg("--id").arg(id);
            }
        }

        if let Some(extension) = &request.with_extension {
            cmd.arg("--with-extension").arg(extension);
        }

        if let Some(builtin) = &request.with_builtin {
            cmd.arg("--with-builtin").arg(builtin);
        }

        if request.debug.unwrap_or(false) {
            cmd.arg("--debug");
        }

        if let Some(max_turns) = request.max_turns {
            cmd.arg("--max-turns").arg(max_turns.to_string());
        }

        let result = Self::execute_command(cmd).await;

        // Mark session as inactive after completion
        if let Ok(mut sessions) = ACTIVE_SESSIONS.lock() {
            sessions.insert(session_id, false);
        }

        result
    }

    pub async fn list_sessions(request: SessionListRequest) -> CommandResult {
        let mut cmd = Command::new("goose");
        cmd.arg("session").arg("list");

        if request.verbose.unwrap_or(false) {
            cmd.arg("--verbose");
        }

        if let Some(format) = &request.format {
            cmd.arg("--format").arg(format);
        }

        if request.ascending.unwrap_or(false) {
            cmd.arg("--ascending");
        }

        Self::execute_command(cmd).await
    }

    pub async fn remove_session(request: SessionRemoveRequest) -> CommandResult {
        let session_key = request
            .id
            .clone()
            .or_else(|| request.name.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let mut cmd = Command::new("goose");
        cmd.arg("session").arg("remove");

        if let Some(id) = &request.id {
            cmd.arg("-i").arg(id);
            // Force terminate the session if it's active
            if let Ok(mut sessions) = ACTIVE_SESSIONS.lock() {
                sessions.insert(id.clone(), false);
            }
        } else if let Some(name) = &request.name {
            cmd.arg("-n").arg(name);
        } else if let Some(regex) = &request.regex {
            cmd.arg("-r").arg(regex);
        } else {
            return CommandResult::error("Must specify id, name, or regex pattern".to_string(), 1);
        }

        let result = Self::execute_command(cmd).await;

        // Ensure session is marked as terminated
        if let Ok(mut sessions) = ACTIVE_SESSIONS.lock() {
            sessions.insert(session_key, false);
        }

        result
    }

    pub async fn export_session(request: SessionExportRequest) -> CommandResult {
        let mut cmd = Command::new("goose");
        cmd.arg("session").arg("export");

        if let Some(id) = &request.id {
            cmd.arg("-i").arg(id);
        } else if let Some(name) = &request.name {
            cmd.arg("-n").arg(name);
        } else if let Some(path) = &request.path {
            cmd.arg("-p").arg(path);
        }

        if let Some(output) = &request.output {
            cmd.arg("-o").arg(output);
        }

        Self::execute_command(cmd).await
    }

    pub async fn configure(request: ConfigureRequest) -> CommandResult {
        let mut cmd = Command::new("goose");
        cmd.arg("configure");

        if request.reconfigure.unwrap_or(false) {
            cmd.arg("--reconfigure");
        }

        Self::execute_command(cmd).await
    }

    pub async fn update(request: UpdateRequest) -> CommandResult {
        let mut cmd = Command::new("goose");
        cmd.arg("update");

        if request.canary.unwrap_or(false) {
            cmd.arg("--canary");
        }

        if request.reconfigure.unwrap_or(false) {
            cmd.arg("--reconfigure");
        }

        Self::execute_command(cmd).await
    }

    pub async fn info(request: InfoRequest) -> CommandResult {
        let mut cmd = Command::new("goose");
        cmd.arg("info");

        if request.verbose.unwrap_or(false) {
            cmd.arg("--verbose");
        }

        Self::execute_command(cmd).await
    }

    pub async fn version() -> CommandResult {
        let cmd = Command::new("goose");
        let mut cmd = cmd;
        cmd.arg("--version");

        Self::execute_command(cmd).await
    }

    pub async fn help() -> CommandResult {
        let cmd = Command::new("goose");
        let mut cmd = cmd;
        cmd.arg("--help");

        Self::execute_command(cmd).await
    }

    pub async fn mcp_list(request: McpListRequest) -> CommandResult {
        let mut cmd = Command::new("goose");
        cmd.arg("mcp").arg("list");

        if request.available.unwrap_or(false) {
            cmd.arg("--available");
        }

        if request.installed.unwrap_or(false) {
            cmd.arg("--installed");
        }

        Self::execute_command(cmd).await
    }

    pub async fn mcp_install(request: McpInstallRequest) -> CommandResult {
        let mut cmd = Command::new("goose");
        cmd.arg("mcp").arg("install").arg(&request.server);

        if request.force.unwrap_or(false) {
            cmd.arg("--force");
        }

        Self::execute_command(cmd).await
    }

    pub async fn project_management(request: ProjectRequest) -> CommandResult {
        let mut cmd = Command::new("goose");

        if request.new.unwrap_or(false) {
            cmd.arg("projects");
        } else {
            cmd.arg("project");
        }

        if let Some(project) = &request.project {
            cmd.arg(project);
        }

        Self::execute_command(cmd).await
    }

    pub async fn list_projects() -> CommandResult {
        let mut cmd = Command::new("goose");
        cmd.arg("projects");

        Self::execute_command(cmd).await
    }

    // Add a new method to force kill all active sessions
    pub async fn kill_all_sessions() -> CommandResult {
        log::info!("Killing all active Goose sessions...");

        // Mark all sessions as inactive
        if let Ok(mut sessions) = ACTIVE_SESSIONS.lock() {
            for (_, active) in sessions.iter_mut() {
                *active = false;
            }
            sessions.clear();
        }

        // Clear execution tracker
        if let Ok(mut tracker) = EXECUTION_TRACKER.lock() {
            tracker.clear();
        }

        // Force kill any goose processes
        let kill_result = tokio::process::Command::new("pkill")
            .arg("-f")
            .arg("goose")
            .output()
            .await;

        match kill_result {
            Ok(output) => {
                if output.status.success() {
                    CommandResult::success("All Goose sessions terminated".to_string())
                } else {
                    CommandResult::success(
                        "Session cleanup completed (no active processes found)".to_string(),
                    )
                }
            }
            Err(e) => {
                log::warn!("Failed to kill processes: {}", e);
                CommandResult::success("Session state cleared (process kill failed)".to_string())
            }
        }
    }

    // New method to check if any sessions are active
    pub fn has_active_sessions() -> bool {
        if let Ok(sessions) = ACTIVE_SESSIONS.lock() {
            sessions.values().any(|&active| active)
        } else {
            false
        }
    }

    async fn execute_command_with_cleanup(cmd: Command, execution_key: String) -> CommandResult {
        let result = Self::execute_command(cmd).await;

        // Clean up execution tracker regardless of success/failure
        if let Ok(mut tracker) = EXECUTION_TRACKER.lock() {
            tracker.remove(&execution_key);
        }

        result
    }

    async fn execute_command(cmd: Command) -> CommandResult {
        const MAX_RETRIES: u32 = 3;
        const COMMAND_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes
        const RETRY_DELAY: Duration = Duration::from_secs(5);

        let program = cmd.get_program().to_os_string();
        let args: Vec<_> = cmd.get_args().map(|s| s.to_os_string()).collect();
        let envs: Vec<_> = cmd
            .get_envs()
            .map(|(k, v)| (k.to_os_string(), v.unwrap_or_default().to_os_string()))
            .collect();

        log::debug!("Executing command: {:?} with args: {:?}", program, args);

        for attempt in 1..=MAX_RETRIES {
            log::debug!("Command attempt {} of {}", attempt, MAX_RETRIES);

            let cmd_future = tokio::task::spawn_blocking({
                let program = program.clone();
                let args = args.clone();
                let envs = envs.clone();

                move || {
                    let mut cmd = Command::new(program);
                    cmd.args(args);
                    cmd.envs(envs);
                    cmd.output()
                }
            });

            match timeout(COMMAND_TIMEOUT, cmd_future).await {
                Ok(Ok(Ok(output))) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let exit_code = output.status.code().unwrap_or(-1);

                    if output.status.success() {
                        log::debug!("Command succeeded on attempt {}", attempt);

                        // Add session completion marker to output
                        let enhanced_output = format!(
                            "{}\n🔚 EXECUTION COMPLETED - SESSION READY FOR TERMINATION",
                            stdout
                        );
                        return CommandResult::success(enhanced_output);
                    } else {
                        let error_msg = if stderr.is_empty() { stdout } else { stderr };

                        // Check for specific errors that indicate hanging or timeout
                        if Self::is_recoverable_error(&error_msg, exit_code)
                            && attempt < MAX_RETRIES
                        {
                            log::warn!(
                                "Recoverable error on attempt {}: {} (exit code: {})",
                                attempt,
                                error_msg,
                                exit_code
                            );
                            log::info!("Retrying in {} seconds...", RETRY_DELAY.as_secs());
                            tokio::time::sleep(RETRY_DELAY).await;
                            continue;
                        }

                        return CommandResult::error(error_msg, exit_code);
                    }
                }
                Ok(Ok(Err(e))) => {
                    let error_msg = format!("Command execution failed: {}", e);
                    log::error!("Attempt {} failed: {}", attempt, error_msg);

                    if attempt < MAX_RETRIES {
                        log::info!("Retrying in {} seconds...", RETRY_DELAY.as_secs());
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }

                    return CommandResult::error(error_msg, -1);
                }
                Ok(Err(e)) => {
                    let error_msg = format!("Task execution failed: {}", e);
                    log::error!("Attempt {} failed: {}", attempt, error_msg);

                    if attempt < MAX_RETRIES {
                        log::info!("Retrying in {} seconds...", RETRY_DELAY.as_secs());
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }

                    return CommandResult::error(error_msg, -1);
                }
                Err(_) => {
                    let error_msg = format!(
                        "Command timed out after {} seconds",
                        COMMAND_TIMEOUT.as_secs()
                    );
                    log::error!("Attempt {} timed out", attempt);

                    if attempt < MAX_RETRIES {
                        log::info!("Retrying in {} seconds...", RETRY_DELAY.as_secs());
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }

                    return CommandResult::error(error_msg, -2);
                }
            }
        }

        CommandResult::error(format!("Failed after {} attempts", MAX_RETRIES), -1)
    }

    fn is_recoverable_error(error_msg: &str, exit_code: i32) -> bool {
        // Check for common recoverable errors
        let recoverable_patterns = [
            "connection refused",
            "network error",
            "timeout",
            "temporarily unavailable",
            "rate limit",
            "service unavailable",
            "502 bad gateway",
            "503 service unavailable",
            "504 gateway timeout",
            "INVALID_ARGUMENT", // The specific error you mentioned
        ];

        let error_lower = error_msg.to_lowercase();
        let has_recoverable_pattern = recoverable_patterns
            .iter()
            .any(|pattern| error_lower.contains(pattern));

        // Consider some exit codes as recoverable
        let recoverable_exit_codes = [1, 2, 124, 137, 143]; // Common timeout/interrupt codes
        let has_recoverable_exit_code = recoverable_exit_codes.contains(&exit_code);

        has_recoverable_pattern || has_recoverable_exit_code
    }

    fn create_temp_file(content: &str) -> Result<NamedTempFile, std::io::Error> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(content.as_bytes())?;
        temp_file.flush()?;
        Ok(temp_file)
    }
}
