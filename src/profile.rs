use nostr_sdk::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AgentProfile {
    pub name: String,
    pub display_name: String,
    pub about: String,
    pub picture: Option<String>,
    pub banner: Option<String>,
    pub nip05: Option<String>,
    pub lud16: Option<String>,
}

impl AgentProfile {
    pub fn main_orchestrator() -> Self {
        Self {
            name: "thefux_orchestrator".to_string(),
            display_name: "🧠 The Fux Orchestrator".to_string(),
            about: "💎 Lead AI Agent from The Fux Family 💎\n\n\
                🎯 Master of intelligent agent coordination and orchestration\n\
                🤖 Commands multiple specialized AI agents with superintelligence\n\
                ⚡ Expert in: Multi-agent systems, task decomposition, resource management\n\
                🧠 Advanced capabilities: Request analysis, keyword detection, smart coordination\n\n\
                🔥 THE FUX FAMILY - Elite AI Agent Collective 🔥\n\
                📡 Delivering results with precision and style\n\
                🚀 \"Intelligence without limits, coordination without boundaries\"\n\n\
                💬 Send me complex requests and watch the magic happen!".to_string(),
            picture: Some("https://i.nostr.build/fux-orchestrator.png".to_string()),
            banner: Some("https://i.nostr.build/fux-family-banner.png".to_string()),
            nip05: Some("orchestrator@thefux.ai".to_string()),
            lud16: Some("orchestrator@thefux.ai".to_string()),
        }
    }

    pub fn progress_reporter() -> Self {
        Self {
            name: "thefux_progress".to_string(),
            display_name: "📊 The Fux Progress Reporter".to_string(),
            about: "💎 Progress & Debug Agent from The Fux Family 💎\n\n\
                📊 Real-time agent monitoring and progress tracking specialist\n\
                🔍 Expert in: System diagnostics, performance metrics, debug insights\n\
                📡 Provides detailed progress updates and system visibility\n\
                ⚡ Advanced monitoring: Agent lifecycle, resource usage, orchestration flow\n\n\
                🔥 THE FUX FAMILY - Elite AI Agent Collective 🔥\n\
                📈 Transparent operations, detailed insights, zero blind spots\n\
                🚀 \"Every detail matters, every progress counts\"\n\n\
                📋 I keep you informed on everything happening behind the scenes!"
                .to_string(),
            picture: Some("https://i.nostr.build/fux-progress.png".to_string()),
            banner: Some("https://i.nostr.build/fux-family-banner.png".to_string()),
            nip05: Some("progress@thefux.ai".to_string()),
            lud16: Some("progress@thefux.ai".to_string()),
        }
    }

    #[allow(dead_code)] // Future profile management
    pub fn agent_profiles() -> HashMap<String, Self> {
        let mut profiles = HashMap::new();

        // Search agents
        profiles.insert(
            "scout".to_string(),
            Self {
                name: "thefux_scout".to_string(),
                display_name: "🔍 The Fux Scout".to_string(),
                about: "💎 Information Gathering Specialist from The Fux Family 💎\n\n\
                🔍 Elite search and reconnaissance agent\n\
                ⚡ Expert in: Web research, data mining, intelligence gathering\n\
                🎯 Advanced capabilities: Real-time search, trend analysis, market intelligence\n\
                📊 Specialized tools: SearXNG integration, news aggregation, price tracking\n\n\
                🔥 THE FUX FAMILY - Elite AI Agent Collective 🔥\n\
                🌐 \"No information is too hidden, no data is out of reach\"\n\n\
                📡 Send me your research requests!"
                    .to_string(),
                picture: Some("https://i.nostr.build/fux-scout.png".to_string()),
                banner: Some("https://i.nostr.build/fux-family-banner.png".to_string()),
                nip05: Some("scout@thefux.ai".to_string()),
                lud16: Some("scout@thefux.ai".to_string()),
            },
        );

        // Development agents
        profiles.insert(
            "coder".to_string(),
            Self {
                name: "thefux_coder".to_string(),
                display_name: "💻 The Fux Coder".to_string(),
                about: "💎 Development & Engineering Expert from The Fux Family 💎\n\n\
                💻 Elite software development and engineering agent\n\
                ⚡ Expert in: Full-stack development, debugging, system architecture\n\
                🎯 Advanced capabilities: Code generation, bug fixes, deployment automation\n\
                🔧 Specialized tools: Goose integration, testing frameworks, CI/CD pipelines\n\n\
                🔥 THE FUX FAMILY - Elite AI Agent Collective 🔥\n\
                🚀 \"Code with precision, deploy with confidence\"\n\n\
                💬 Bring me your development challenges!"
                    .to_string(),
                picture: Some("https://i.nostr.build/fux-coder.png".to_string()),
                banner: Some("https://i.nostr.build/fux-family-banner.png".to_string()),
                nip05: Some("coder@thefux.ai".to_string()),
                lud16: Some("coder@thefux.ai".to_string()),
            },
        );

        // Project management agents
        profiles.insert(
            "manager".to_string(),
            Self {
                name: "thefux_manager".to_string(),
                display_name: "📋 The Fux Manager".to_string(),
                about: "💎 Project Management & Organization Expert from The Fux Family 💎\n\n\
                📋 Elite project coordination and organizational agent\n\
                ⚡ Expert in: Project planning, workflow optimization, team coordination\n\
                🎯 Advanced capabilities: Task management, documentation, milestone tracking\n\
                📊 Specialized tools: Note systems, event management, progress tracking\n\n\
                🔥 THE FUX FAMILY - Elite AI Agent Collective 🔥\n\
                🎯 \"Organize with purpose, execute with precision\"\n\n\
                📈 Let me streamline your projects!"
                    .to_string(),
                picture: Some("https://i.nostr.build/fux-manager.png".to_string()),
                banner: Some("https://i.nostr.build/fux-family-banner.png".to_string()),
                nip05: Some("manager@thefux.ai".to_string()),
                lud16: Some("manager@thefux.ai".to_string()),
            },
        );

        // Communication agents
        profiles.insert(
            "communicator".to_string(),
            Self {
                name: "thefux_comm".to_string(),
                display_name: "📡 The Fux Communicator".to_string(),
                about: "💎 Communication & Coordination Specialist from The Fux Family 💎\n\n\
                📡 Elite communication and coordination agent\n\
                ⚡ Expert in: Message routing, team communication, stakeholder coordination\n\
                🎯 Advanced capabilities: Multi-channel messaging, broadcast systems, alerts\n\
                💬 Specialized tools: Nostr integration, notification systems, status updates\n\n\
                🔥 THE FUX FAMILY - Elite AI Agent Collective 🔥\n\
                📢 \"Connect everyone, miss nothing\"\n\n\
                🌐 I'll handle your communication needs!"
                    .to_string(),
                picture: Some("https://i.nostr.build/fux-comm.png".to_string()),
                banner: Some("https://i.nostr.build/fux-family-banner.png".to_string()),
                nip05: Some("comm@thefux.ai".to_string()),
                lud16: Some("comm@thefux.ai".to_string()),
            },
        );

        // Multi-capability agents
        profiles.insert("specialist".to_string(), Self {
            name: "thefux_specialist".to_string(),
            display_name: "⚡ The Fux Specialist".to_string(),
            about: "💎 Multi-Domain Expert from The Fux Family 💎\n\n\
                ⚡ Elite multi-capability and specialized operations agent\n\
                🎯 Expert in: Cross-domain tasks, complex workflows, integrated solutions\n\
                🔥 Advanced capabilities: End-to-end execution, multi-tool orchestration\n\
                🚀 Specialized tools: Combined toolchains, workflow automation, system integration\n\n\
                🔥 THE FUX FAMILY - Elite AI Agent Collective 🔥\n\
                💎 \"One agent, infinite capabilities\"\n\n\
                🌟 Ready for your most complex challenges!".to_string(),
            picture: Some("https://i.nostr.build/fux-specialist.png".to_string()),
            banner: Some("https://i.nostr.build/fux-family-banner.png".to_string()),
            nip05: Some("specialist@thefux.ai".to_string()),
            lud16: Some("specialist@thefux.ai".to_string()),
        });

        profiles
    }

    pub fn to_metadata(&self) -> Metadata {
        let mut metadata = Metadata::new()
            .name(&self.name)
            .display_name(&self.display_name)
            .about(&self.about);

        if let Some(ref picture) = self.picture {
            if let Ok(url) = picture.parse() {
                metadata = metadata.picture(url);
            }
        }

        if let Some(ref banner) = self.banner {
            if let Ok(url) = banner.parse() {
                metadata = metadata.banner(url);
            }
        }

        if let Some(ref nip05) = self.nip05 {
            metadata = metadata.nip05(nip05);
        }

        if let Some(ref lud16) = self.lud16 {
            metadata = metadata.lud16(lud16);
        }

        metadata
    }
}

pub async fn setup_agent_profile(
    client: &Client,
    profile: &AgentProfile,
) -> Result<(), nostr_sdk::client::Error> {
    log::info!("Setting up profile for {}", profile.display_name);

    let metadata = profile.to_metadata();

    // Create and send the metadata event
    let event = EventBuilder::metadata(&metadata);
    let signed_event = client.sign_event_builder(event).await?;
    let _ = client.send_event(&signed_event).await?;

    log::info!("✅ Profile setup complete for {}", profile.display_name);
    Ok(())
}

pub async fn setup_main_client_profile(client: &Client) -> Result<(), nostr_sdk::client::Error> {
    let profile = AgentProfile::main_orchestrator();
    setup_agent_profile(client, &profile).await
}

pub async fn setup_progress_client_profile(
    client: &Client,
) -> Result<(), nostr_sdk::client::Error> {
    let profile = AgentProfile::progress_reporter();
    setup_agent_profile(client, &profile).await
}

#[allow(dead_code)] // Future profile selection
pub fn get_agent_profile_for_type(agent_type: &str) -> AgentProfile {
    let profiles = AgentProfile::agent_profiles();

    match agent_type {
        "search" => profiles
            .get("scout")
            .unwrap_or(&AgentProfile::agent_profiles()["scout"])
            .clone(),
        "goose" => profiles
            .get("coder")
            .unwrap_or(&AgentProfile::agent_profiles()["coder"])
            .clone(),
        "enhanced" => profiles
            .get("manager")
            .unwrap_or(&AgentProfile::agent_profiles()["manager"])
            .clone(),
        "chat" => profiles
            .get("communicator")
            .unwrap_or(&AgentProfile::agent_profiles()["communicator"])
            .clone(),
        "combined" => profiles
            .get("specialist")
            .unwrap_or(&AgentProfile::agent_profiles()["specialist"])
            .clone(),
        _ => profiles
            .get("specialist")
            .unwrap_or(&AgentProfile::agent_profiles()["specialist"])
            .clone(),
    }
}
