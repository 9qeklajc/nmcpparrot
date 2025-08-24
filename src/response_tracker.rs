use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone)]
pub struct ResponseTracker {
    has_sent_response: Arc<AtomicBool>,
    conversation_active: Arc<AtomicBool>,
}

impl ResponseTracker {
    pub fn new() -> Self {
        Self {
            has_sent_response: Arc::new(AtomicBool::new(false)),
            conversation_active: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start_conversation(&self) {
        self.has_sent_response.store(false, Ordering::Relaxed);
        self.conversation_active.store(true, Ordering::Relaxed);
    }

    pub fn mark_response_sent(&self) {
        self.has_sent_response.store(true, Ordering::Relaxed);
    }

    pub fn mark_progress_sent(&self) {
        // Progress messages don't count as final responses
        // This is just for tracking activity
    }

    #[allow(dead_code)]
    pub fn end_conversation(&self) {
        self.conversation_active.store(false, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn has_sent_final_response(&self) -> bool {
        self.has_sent_response.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn is_conversation_active(&self) -> bool {
        self.conversation_active.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub async fn ensure_response_sent<F, Fut>(&self, send_fallback: F) -> Result<(), String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        // Wait a bit to see if the agent sends a response naturally
        let wait_result = timeout(Duration::from_secs(2), async {
            while self.is_conversation_active() && !self.has_sent_final_response() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await;

        // If timeout expired and no response was sent, send fallback
        if wait_result.is_err() && self.is_conversation_active() && !self.has_sent_final_response()
        {
            log::warn!("Agent did not send final response - sending fallback message");
            send_fallback().await?;
            self.mark_response_sent();
        }

        Ok(())
    }
}

pub fn create_response_reminder() -> String {
    "🚨 CRITICAL MANDATORY WORKFLOW - NO EXCEPTIONS:\n\
    \n\
    1️⃣ IMMEDIATE ACTION REQUIRED: Send progress update NOW\n\
       {\"tool\": \"progress\", \"arguments\": {\"message\": \"I'm working on your request...\"}}\n\
    \n\
    2️⃣ PERFORM OPERATIONS: Execute the user's request\n\
    \n\
    3️⃣ MANDATORY FINAL RESPONSE: You MUST end with 'send' tool call\n\
       {\"tool\": \"send\", \"arguments\": {\"message\": \"[Your final response here]\"}}\n\
    \n\
    🔴 CRITICAL: The user can ONLY see messages sent via 'send' and 'progress' tools\n\
    🔴 CRITICAL: If you don't use 'send', the user sees NOTHING\n\
    🔴 CRITICAL: If you don't use 'progress', the user thinks you're not working\n\
    \n\
    💀 FAILURE TO FOLLOW THIS PATTERN WILL BREAK THE USER EXPERIENCE\n\
    \n\
    ⚠️ This applies to EVERY response: simple answers, complex operations, errors, confirmations - ALL must follow this pattern.".to_string()
}
