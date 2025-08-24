use super::types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ResourceScheduler {
    config: AgentConfig,
    active_agents: Arc<RwLock<usize>>,
    system_stats: Arc<RwLock<SystemStats>>,
    start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct SystemStats {
    memory_usage_percent: f64,
    cpu_usage_percent: f64,
}

impl ResourceScheduler {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            active_agents: Arc::new(RwLock::new(0)),
            system_stats: Arc::new(RwLock::new(SystemStats {
                memory_usage_percent: 0.0,
                cpu_usage_percent: 0.0,
            })),
            start_time: chrono::Utc::now(),
        }
    }

    pub async fn can_create_agent(&self) -> bool {
        let active = *self.active_agents.read().await;
        if active >= self.config.max_agents {
            return false;
        }

        let stats = self.system_stats.read().await;
        stats.memory_usage_percent < self.config.memory_limit_percent
            && stats.cpu_usage_percent < self.config.cpu_limit_percent
    }

    pub async fn reserve_agent_slot(&self) -> AgentResult<()> {
        if !self.can_create_agent().await {
            return Err("Resource limits exceeded, cannot create new agent".into());
        }

        let mut active = self.active_agents.write().await;
        *active += 1;
        Ok(())
    }

    pub async fn release_agent_slot(&self) {
        let mut active = self.active_agents.write().await;
        if *active > 0 {
            *active -= 1;
        }
    }

    #[allow(dead_code)]
    pub async fn get_active_agent_count(&self) -> usize {
        *self.active_agents.read().await
    }

    pub async fn update_system_stats(&self) {
        let mut stats = self.system_stats.write().await;

        stats.memory_usage_percent = self.get_memory_usage().await;
        stats.cpu_usage_percent = self.get_cpu_usage().await;
    }

    #[allow(dead_code)] // System status monitoring
    pub async fn get_system_status(&self, message_count: u64) -> SystemStatus {
        let active_agents = *self.active_agents.read().await;
        let stats = self.system_stats.read().await;
        let uptime = chrono::Utc::now()
            .signed_duration_since(self.start_time)
            .num_seconds() as u64;

        SystemStatus {
            active_agents,
            max_agents: self.config.max_agents,
            memory_usage_percent: stats.memory_usage_percent,
            cpu_usage_percent: stats.cpu_usage_percent,
            uptime_seconds: uptime,
            messages_processed: message_count,
        }
    }

    #[allow(dead_code)]
    pub fn get_config(&self) -> &AgentConfig {
        &self.config
    }

    async fn get_memory_usage(&self) -> f64 {
        #[cfg(target_os = "linux")]
        {
            match std::fs::read_to_string("/proc/meminfo") {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().collect();
                    let mut total_kb = 0u64;
                    let mut available_kb = 0u64;

                    for line in lines {
                        if line.starts_with("MemTotal:") {
                            if let Some(value) = line.split_whitespace().nth(1) {
                                total_kb = value.parse().unwrap_or(0);
                            }
                        } else if line.starts_with("MemAvailable:") {
                            if let Some(value) = line.split_whitespace().nth(1) {
                                available_kb = value.parse().unwrap_or(0);
                            }
                        }
                    }

                    if total_kb > 0 {
                        let used_kb = total_kb.saturating_sub(available_kb);
                        return (used_kb as f64 / total_kb as f64) * 100.0;
                    }
                }
                Err(_) => {}
            }
        }

        50.0
    }

    async fn get_cpu_usage(&self) -> f64 {
        #[cfg(target_os = "linux")]
        {
            match std::fs::read_to_string("/proc/loadavg") {
                Ok(content) => {
                    if let Some(load_str) = content.split_whitespace().next() {
                        if let Ok(load) = load_str.parse::<f64>() {
                            let cpu_count = num_cpus::get() as f64;
                            return (load / cpu_count) * 100.0;
                        }
                    }
                }
                Err(_) => {}
            }
        }

        25.0
    }

    pub async fn start_monitoring(&self) {
        let scheduler = Arc::new(self.clone());
        let interval = self.config.health_check_interval_seconds;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval));
            loop {
                interval.tick().await;
                scheduler.update_system_stats().await;
            }
        });
    }
}

impl Clone for ResourceScheduler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            active_agents: self.active_agents.clone(),
            system_stats: self.system_stats.clone(),
            start_time: self.start_time,
        }
    }
}
