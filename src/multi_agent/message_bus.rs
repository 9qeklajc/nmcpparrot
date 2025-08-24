use super::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug)]
pub struct MessageBus {
    agents: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<AgentMessage>>>>,
    #[allow(dead_code)] // Future broadcasting functionality
    broadcast_sender: mpsc::UnboundedSender<AgentMessage>,
    message_count: Arc<RwLock<u64>>,
}

impl MessageBus {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<AgentMessage>) {
        let (broadcast_sender, broadcast_receiver) = mpsc::unbounded_channel();

        (
            Self {
                agents: Arc::new(RwLock::new(HashMap::new())),
                broadcast_sender,
                message_count: Arc::new(RwLock::new(0)),
            },
            broadcast_receiver,
        )
    }

    pub async fn register_agent(
        &self,
        agent_id: String,
        sender: mpsc::UnboundedSender<AgentMessage>,
    ) {
        let mut agents = self.agents.write().await;
        agents.insert(agent_id, sender);
    }

    pub async fn unregister_agent(&self, agent_id: &str) {
        let mut agents = self.agents.write().await;
        agents.remove(agent_id);
    }

    #[allow(dead_code)]
    pub async fn send_to_agent(&self, agent_id: &str, message: AgentMessage) -> AgentResult<()> {
        let agents = self.agents.read().await;
        if let Some(sender) = agents.get(agent_id) {
            sender.send(message).map_err(|e| -> AgentError {
                format!("Failed to send message to agent {}: {}", agent_id, e).into()
            })?;
            self.increment_message_count().await;
            Ok(())
        } else {
            Err(format!("Agent {} not found", agent_id).into())
        }
    }

    #[allow(dead_code)]
    pub async fn broadcast(&self, message: AgentMessage) -> AgentResult<()> {
        self.broadcast_sender
            .send(message)
            .map_err(|e| -> AgentError { format!("Failed to broadcast message: {}", e).into() })?;
        self.increment_message_count().await;
        Ok(())
    }

    pub async fn send_to_all_agents(&self, message: AgentMessage) -> AgentResult<()> {
        let agents = self.agents.read().await;
        let mut errors = Vec::new();

        for (agent_id, sender) in agents.iter() {
            let msg = AgentMessage {
                id: format!("{}-{}", message.id, agent_id),
                ..message.clone()
            };

            if let Err(e) = sender.send(msg) {
                errors.push(format!("Failed to send to {}: {}", agent_id, e));
            }
        }

        if !errors.is_empty() {
            return Err(format!("Failed to send to some agents: {}", errors.join(", ")).into());
        }

        self.increment_message_count().await;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_active_agents(&self) -> Vec<String> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }

    #[allow(dead_code)] // Message count monitoring
    pub async fn get_message_count(&self) -> u64 {
        *self.message_count.read().await
    }

    async fn increment_message_count(&self) {
        let mut count = self.message_count.write().await;
        *count += 1;
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        let (bus, _) = Self::new();
        bus
    }
}
