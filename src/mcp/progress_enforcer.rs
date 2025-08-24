use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug)]
#[allow(dead_code)] // Future use for progress tracking
pub struct ProgressTracker {
    last_progress: RwLock<HashMap<String, Instant>>,
    progress_required_tools: Vec<String>,
}

impl Clone for ProgressTracker {
    fn clone(&self) -> Self {
        Self::new()
    }
}

#[allow(dead_code)] // Future implementation for progress tracking
impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            last_progress: RwLock::new(HashMap::new()),
            progress_required_tools: vec![
                "addnote".to_string(),
                "addvent".to_string(),
                "searchnotes".to_string(),
                "searchevents".to_string(),
                "listnotes".to_string(),
                "listevents".to_string(),
                "runtask".to_string(),
                "startsession".to_string(),
            ],
        }
    }

    pub async fn mark_progress_sent(&self, session_id: &str) {
        let mut tracker = self.last_progress.write().await;
        tracker.insert(session_id.to_string(), Instant::now());
    }

    pub async fn should_send_progress_reminder(&self, session_id: &str, tool_name: &str) -> bool {
        if !self
            .progress_required_tools
            .contains(&tool_name.to_string())
        {
            return false;
        }

        let tracker = self.last_progress.read().await;
        match tracker.get(session_id) {
            Some(last_progress) => last_progress.elapsed() > Duration::from_secs(10),
            None => true,
        }
    }

    pub fn create_progress_reminder(&self, tool_name: &str) -> String {
        format!(
            "⚠️ CRITICAL: Before executing '{}', you MUST send a progress update using the 'progress' tool. \
            This keeps the user informed that their request is being processed. \
            Example: {{\"tool\": \"progress\", \"arguments\": {{\"message\": \"Processing your {} request...\"}}}}\n\n\
            After completion, you MUST also send final results using the 'send' tool.",
            tool_name, tool_name
        )
    }

    pub fn create_comprehensive_instructions(&self) -> String {
        "🚨 **ZERO TOLERANCE WORKFLOW ENFORCEMENT**:\n\n\
        1️⃣ **INSTANT PROGRESS REQUIRED**: The MOMENT you start processing, send progress\n\
        2️⃣ **EXECUTE OPERATION**: Use the requested tool (addnote, searchnotes, etc.)\n\
        3️⃣ **MANDATORY FINAL SEND**: You MUST end with 'send' - NO EXCEPTIONS EVER\n\n\
        📋 **ABSOLUTELY REQUIRED PATTERN**:\n\
        ```json\n\
        {\"tool\": \"progress\", \"arguments\": {\"message\": \"Processing your [operation] request...\"}}\n\
        {\"tool\": \"[operation]\", \"arguments\": {...}}\n\
        {\"tool\": \"send\", \"arguments\": {\"message\": \"✅ [Operation] completed: [results]\"}}\n\
        ```\n\n\
        🔴 **CRITICAL ENFORCEMENT RULES**:\n\
        • EVERY user message MUST trigger progress → operation → send\n\
        • NO EXCEPTIONS for simple requests - ALL need progress\n\
        • NO EXCEPTIONS for quick operations - ALL need final send\n\
        • Users see NOTHING if you don't use send\n\
        • Users think you're broken if you don't use progress\n\n\
        💀 **VIOLATION CONSEQUENCES**:\n\
        ❌ **SKIP PROGRESS** → User thinks system is frozen\n\
        ❌ **SKIP FINAL SEND** → User gets no response\n\
        ❌ **BREAK PATTERN** → System appears broken\n\n\
        🚫 **ABSOLUTELY FORBIDDEN**:\n\
        • Ending without 'send' tool call\n\
        • Starting operations without 'progress'\n\
        • Assuming users know what you're doing\n\
        • Silent failures or completions"
            .to_string()
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
