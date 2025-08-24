use super::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{Duration, Instant};

#[derive(Debug)]
pub struct HealthMonitor {
    agent_health: Arc<RwLock<HashMap<String, AgentHealth>>>,
    timeout_sender: mpsc::UnboundedSender<String>,
    config: AgentConfig,
}

#[derive(Debug, Clone)]
struct AgentHealth {
    #[allow(dead_code)] // Future health tracking
    agent_id: String,
    last_heartbeat: Instant,
    timeout_duration: Duration,
    status: AgentStatus,
    message_count: u64,
}

impl HealthMonitor {
    pub fn new(config: AgentConfig) -> (Self, mpsc::UnboundedReceiver<String>) {
        let (timeout_sender, timeout_receiver) = mpsc::unbounded_channel();

        (
            Self {
                agent_health: Arc::new(RwLock::new(HashMap::new())),
                timeout_sender,
                config,
            },
            timeout_receiver,
        )
    }

    pub async fn register_agent(&self, agent_id: String, timeout_duration: Option<Duration>) {
        let timeout =
            timeout_duration.unwrap_or(Duration::from_secs(self.config.default_timeout_seconds));
        let health = AgentHealth {
            agent_id: agent_id.clone(),
            last_heartbeat: Instant::now(),
            timeout_duration: timeout,
            status: AgentStatus::Starting,
            message_count: 0,
        };

        let mut health_map = self.agent_health.write().await;
        health_map.insert(agent_id, health);
    }

    pub async fn unregister_agent(&self, agent_id: &str) {
        let mut health_map = self.agent_health.write().await;
        health_map.remove(agent_id);
    }

    pub async fn update_heartbeat(&self, agent_id: &str, status: AgentStatus) {
        let mut health_map = self.agent_health.write().await;
        if let Some(health) = health_map.get_mut(agent_id) {
            health.last_heartbeat = Instant::now();
            health.status = status;
            health.message_count += 1;
        }
    }

    #[allow(dead_code)]
    pub async fn get_agent_status(&self, agent_id: &str) -> Option<AgentStatus> {
        let health_map = self.agent_health.read().await;
        health_map.get(agent_id).map(|h| h.status.clone())
    }

    #[allow(dead_code)]
    pub async fn get_all_agent_statuses(&self) -> HashMap<String, AgentStatus> {
        let health_map = self.agent_health.read().await;
        health_map
            .iter()
            .map(|(id, health)| (id.clone(), health.status.clone()))
            .collect()
    }

    pub async fn start_monitoring(&self) {
        let health_monitor = Arc::new(self.clone());
        let check_interval = Duration::from_secs(self.config.health_check_interval_seconds);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            loop {
                interval.tick().await;
                health_monitor.check_timeouts().await;
            }
        });
    }

    async fn check_timeouts(&self) {
        let now = Instant::now();
        let mut timed_out_agents = Vec::new();

        {
            let health_map = self.agent_health.read().await;
            for (agent_id, health) in health_map.iter() {
                if now.duration_since(health.last_heartbeat) > health.timeout_duration {
                    timed_out_agents.push(agent_id.clone());
                }
            }
        }

        for agent_id in timed_out_agents {
            log::warn!("Agent {} timed out", agent_id);

            {
                let mut health_map = self.agent_health.write().await;
                if let Some(health) = health_map.get_mut(&agent_id) {
                    health.status = AgentStatus::Error("Timeout".to_string());
                }
            }

            if let Err(e) = self.timeout_sender.send(agent_id.clone()) {
                log::error!(
                    "Failed to send timeout notification for agent {}: {}",
                    agent_id,
                    e
                );
            }
        }
    }

    #[allow(dead_code)]
    pub async fn get_health_summary(&self) -> HealthSummary {
        let health_map = self.agent_health.read().await;
        let mut summary = HealthSummary {
            total_agents: health_map.len(),
            healthy_agents: 0,
            unhealthy_agents: 0,
            timed_out_agents: 0,
            total_messages: 0,
        };

        for health in health_map.values() {
            summary.total_messages += health.message_count;

            match &health.status {
                AgentStatus::Running | AgentStatus::Idle | AgentStatus::Busy => {
                    summary.healthy_agents += 1;
                }
                AgentStatus::Error(msg) if msg == "Timeout" => {
                    summary.timed_out_agents += 1;
                }
                _ => {
                    summary.unhealthy_agents += 1;
                }
            }
        }

        summary
    }
}

impl Clone for HealthMonitor {
    fn clone(&self) -> Self {
        Self {
            agent_health: self.agent_health.clone(),
            timeout_sender: self.timeout_sender.clone(),
            config: self.config.clone(),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Future health reporting
pub struct HealthSummary {
    pub total_agents: usize,
    pub healthy_agents: usize,
    pub unhealthy_agents: usize,
    pub timed_out_agents: usize,
    pub total_messages: u64,
}
