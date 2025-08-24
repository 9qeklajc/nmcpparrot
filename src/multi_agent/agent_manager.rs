use super::agent_pool::AgentPool;
use super::health_monitor::HealthMonitor;
use super::message_bus::MessageBus;
use super::resource_scheduler::ResourceScheduler;
use super::types::*;
use nostr_sdk::prelude::*;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::Duration;

#[derive(Debug)]
pub struct AgentManager {
    agent_pool: Arc<AgentPool>,
    health_monitor: Arc<HealthMonitor>,
    message_bus: Arc<MessageBus>,
    resource_scheduler: Arc<ResourceScheduler>,
    #[allow(dead_code)] // Future configuration management
    config: AgentConfig,
    _timeout_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<String>>>>,
    _broadcast_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<AgentMessage>>>>,
}

impl AgentManager {
    pub fn new(
        client: Client,
        progress_client: Option<Client>,
        keys: Keys,
        our_pubkey: PublicKey,
        target_pubkey: PublicKey,
    ) -> Self {
        let config = AgentConfig::default();

        // Create NostrMemoryServer for agents to use
        let nostr_memory = crate::nostr_mcp::NostrMemoryServer::new(
            client.clone(),
            progress_client.clone(),
            keys,
            our_pubkey,
            target_pubkey,
        );

        let agent_pool = Arc::new(AgentPool::new(
            client,
            progress_client,
            our_pubkey,
            target_pubkey,
            nostr_memory,
        ));

        let (health_monitor, timeout_receiver) = HealthMonitor::new(config.clone());
        let health_monitor = Arc::new(health_monitor);

        let (message_bus, broadcast_receiver) = MessageBus::new();
        let message_bus = Arc::new(message_bus);

        let resource_scheduler = Arc::new(ResourceScheduler::new(config.clone()));

        let mut manager = Self {
            agent_pool,
            health_monitor: health_monitor.clone(),
            message_bus: message_bus.clone(),
            resource_scheduler: resource_scheduler.clone(),
            config,
            _timeout_receiver: Arc::new(RwLock::new(Some(timeout_receiver))),
            _broadcast_receiver: Arc::new(RwLock::new(Some(broadcast_receiver))),
        };

        manager.start_background_tasks();
        manager
    }

    pub async fn create_agent(&mut self, request: CreateAgentRequest) -> AgentResult<String> {
        self.resource_scheduler.reserve_agent_slot().await?;

        match self.agent_pool.create_agent(request.clone()).await {
            Ok(agent_id) => {
                // Register agent with message bus for routing
                if let Some(sender) = self.agent_pool.get_agent_sender(&agent_id).await {
                    self.message_bus
                        .register_agent(agent_id.clone(), sender)
                        .await;
                }

                // Register with health monitor
                let timeout_duration = request.timeout_seconds.map(Duration::from_secs);
                self.health_monitor
                    .register_agent(agent_id.clone(), timeout_duration)
                    .await;

                self.health_monitor
                    .update_heartbeat(&agent_id, AgentStatus::Running)
                    .await;

                log::info!("Successfully created agent: {}", agent_id);
                Ok(agent_id)
            }
            Err(e) => {
                self.resource_scheduler.release_agent_slot().await;
                Err(e)
            }
        }
    }

    pub async fn stop_agent(&mut self, agent_id: &str) -> AgentResult<bool> {
        let result = self.agent_pool.stop_agent(agent_id).await?;

        if result {
            // Cleanup all registrations
            self.health_monitor.unregister_agent(agent_id).await;
            self.message_bus.unregister_agent(agent_id).await;
            self.resource_scheduler.release_agent_slot().await;
            log::info!("Successfully stopped agent: {}", agent_id);
        }

        Ok(result)
    }

    pub async fn send_message_to_agent(
        &self,
        agent_id: &str,
        message: &str,
    ) -> AgentResult<String> {
        // Send message directly through agent pool (which handles response channels)
        let response = self
            .agent_pool
            .send_message_to_agent(agent_id, message)
            .await?;

        // Update health status
        self.health_monitor
            .update_heartbeat(agent_id, AgentStatus::Busy)
            .await;

        Ok(response)
    }

    pub async fn list_agents(&self) -> Vec<Agent> {
        self.agent_pool.list_agents().await
    }

    /// Check for and mark completed agents as stopped
    pub async fn detect_and_mark_completed_agents(&self) -> AgentResult<usize> {
        let agents = self.agent_pool.list_agents().await;
        let mut completed_count = 0;

        for agent in &agents {
            // Check if agent is idle for more than 10 seconds (indicating task completion)
            if matches!(agent.status, AgentStatus::Running | AgentStatus::Busy) {
                let time_since_active = chrono::Utc::now()
                    .signed_duration_since(agent.last_active)
                    .num_seconds();

                if time_since_active > 10 {
                    log::info!("Agent {} appears to have completed its task (idle for {}s), marking as stopped", 
                              agent.name, time_since_active);

                    self.agent_pool
                        .update_agent_status(&agent.id, AgentStatus::Stopped)
                        .await;
                    completed_count += 1;
                }
            }
        }

        Ok(completed_count)
    }

    /// Clean up stopped agents and return count of cleaned agents
    pub async fn cleanup_stopped_agents(&self) -> usize {
        self.agent_pool.cleanup_stopped_agents().await
    }

    #[allow(dead_code)] // System monitoring functionality
    pub async fn get_system_status(&self) -> SystemStatus {
        let message_count = self.message_bus.get_message_count().await;
        self.resource_scheduler
            .get_system_status(message_count)
            .await
    }

    #[allow(dead_code)] // Future broadcasting functionality
    pub async fn broadcast_message(&self, message: &str) -> AgentResult<()> {
        let agent_message = AgentMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from_agent: None,
            to_agent: None,
            message_type: MessageType::Task,
            content: message.to_string(),
            timestamp: chrono::Utc::now(),
            response_channel: None,
        };

        self.message_bus.send_to_all_agents(agent_message).await
    }

    #[allow(dead_code)]
    pub async fn get_agent_health_summary(&self) -> super::health_monitor::HealthSummary {
        self.health_monitor.get_health_summary().await
    }

    #[allow(dead_code)]
    pub async fn force_cleanup_timed_out_agents(&mut self) -> AgentResult<Vec<String>> {
        let statuses = self.health_monitor.get_all_agent_statuses().await;
        let mut cleaned_up = Vec::new();

        for (agent_id, status) in statuses {
            if let AgentStatus::Error(ref msg) = status {
                if msg == "Timeout" && self.stop_agent(&agent_id).await? {
                    cleaned_up.push(agent_id);
                }
            }
        }

        Ok(cleaned_up)
    }

    fn start_background_tasks(&mut self) {
        let health_monitor = self.health_monitor.clone();
        tokio::spawn(async move {
            health_monitor.start_monitoring().await;
        });

        let resource_scheduler = self.resource_scheduler.clone();
        tokio::spawn(async move {
            resource_scheduler.start_monitoring().await;
        });

        let health_monitor = self.health_monitor.clone();
        let agent_pool = self.agent_pool.clone();
        let resource_scheduler = self.resource_scheduler.clone();

        let timeout_receiver = self._timeout_receiver.clone();
        let message_bus = self.message_bus.clone();
        tokio::spawn(async move {
            let receiver = timeout_receiver.write().await.take();
            if let Some(mut rx) = receiver {
                while let Some(timed_out_agent_id) = rx.recv().await {
                    log::warn!("Agent {} timed out, attempting cleanup", timed_out_agent_id);

                    if let Ok(stopped) = agent_pool.stop_agent(&timed_out_agent_id).await {
                        if stopped {
                            health_monitor.unregister_agent(&timed_out_agent_id).await;
                            message_bus.unregister_agent(&timed_out_agent_id).await;
                            resource_scheduler.release_agent_slot().await;
                            log::info!("Cleaned up timed out agent: {}", timed_out_agent_id);
                        }
                    }
                }
            }
        });

        let message_bus = self.message_bus.clone();
        let broadcast_receiver = self._broadcast_receiver.clone();
        tokio::spawn(async move {
            let receiver = broadcast_receiver.write().await.take();
            if let Some(mut rx) = receiver {
                while let Some(broadcast_message) = rx.recv().await {
                    log::debug!("Processing broadcast message: {:?}", broadcast_message);
                    let _ = message_bus.send_to_all_agents(broadcast_message).await;
                }
            }
        });
    }

    #[allow(dead_code)]
    pub fn get_config(&self) -> &AgentConfig {
        &self.config
    }

    #[allow(dead_code)]
    pub async fn get_active_agent_count(&self) -> usize {
        self.resource_scheduler.get_active_agent_count().await
    }

    #[allow(dead_code)]
    pub async fn can_create_agent(&self) -> bool {
        self.resource_scheduler.can_create_agent().await
    }
}
