pub mod agent_manager;
pub mod agent_pool;
pub mod health_monitor;
pub mod message_bus;
pub mod orchestrator;
pub mod resource_scheduler;
pub mod types;

use crate::mcp::chat::Chat;
use goose_mcp::nostr_memory_mcp::NostrMcpRouter;
use nostr_sdk::prelude::*;
use rmcp::{
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, Error as RmcpError, ServerHandler,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use agent_manager::AgentManager;
use orchestrator::IntelligentOrchestrator;
use types::*;

#[derive(Debug, Clone)]
pub struct MultiAgentMcp {
    agent_manager: Arc<RwLock<AgentManager>>,
    chat: Chat,
    orchestrator: IntelligentOrchestrator,
    #[allow(dead_code)] // Used in agent architecture but blocked at main orchestrator level
    nostr_memory: NostrMcpRouter,
}

#[tool(tool_box)]
impl MultiAgentMcp {
    pub fn new(
        client: Client,
        progress_client: Option<Client>,
        keys: Keys,
        our_pubkey: PublicKey,
        target_pubkey: PublicKey,
    ) -> Self {
        Self {
            agent_manager: Arc::new(RwLock::new(AgentManager::new(
                client.clone(),
                progress_client.clone(),
                keys.clone(),
                our_pubkey,
                target_pubkey,
            ))),
            chat: Chat::new(
                client.clone(),
                progress_client.clone(),
                our_pubkey,
                target_pubkey,
            ),
            orchestrator: IntelligentOrchestrator::new(),
            nostr_memory: NostrMcpRouter::new(Some(keys.secret_key().to_bech32().unwrap())),
        }
    }

    #[tool(
        description = "Send a message to the user - ONLY use for agent deployment feedback, NOT for answers"
    )]
    async fn send(
        &self,
        #[tool(aggr)] crate::mcp::types::SendMessageRequest { message }: crate::mcp::types::SendMessageRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let request = crate::mcp::types::SendMessageRequest { message };
        let message_lower = request.message.to_lowercase();

        // ROUTING: Messages that should go to PROGRESS CHANNEL (orchestration status)
        let is_progress_message = message_lower
            .contains("multi-capability task execution complete")
            || message_lower.contains("task execution complete")
            || message_lower.contains("project management complete")
            || message_lower.contains("search completed and results")
            || message_lower.contains("goose task completed successfully")
            || message_lower.contains("execution complete")
            || message_lower.contains("analysis complete")
            || message_lower.contains("deploying")
            || message_lower.contains("agent deployed")
            || message_lower.contains("agent creation progress")
            || message_lower.contains("deployment confirmation")
            || message_lower.contains("orchestration status")
            || message_lower.contains("ready and standing by")
            || message_lower.contains("task processing")
            || message_lower.contains("tools executed")
            || message_lower.contains("task completed");

        // Send orchestration status messages to progress channel
        if is_progress_message {
            let _ = self
                .chat
                .progress(crate::mcp::types::ProgressMessageRequest {
                    message: request.message.clone(),
                })
                .await;
            return Ok(CallToolResult::success(vec![Content::text(
                "Status update sent to progress channel",
            )]));
        }

        // ENFORCEMENT: Check if remaining messages are valid agent management vs forbidden direct answers
        let is_agent_management = message_lower.contains("task processing initiated")
            || message_lower.contains("task processing failed")
            || message_lower.contains("background processing")
            || message_lower.contains("system ready")
            || message_lower.contains("all tasks completed")
            || message_lower.contains("deployed")
            || message_lower.contains("system processing")
            || message_lower.contains("agents have delivered")
            || message_lower.contains("completion status")
            || message_lower.contains("results delivered");

        if !is_agent_management {
            // This looks like a direct answer attempt - enforce agent creation
            let enforcement_message = "🚨 **AGENT CREATION MANDATE VIOLATION** 🚨\n\n\
                ❌ **FORBIDDEN**: Direct answers are not allowed\n\
                ⚡ **REQUIRED**: You must create a specialized agent instead\n\n\
                🎯 **Correct Workflow**:\n\
                1. analyze_request(request=\"[user's original message]\")\n\
                2. create_agent(agent_type=\"[appropriate_type]\", task=\"[user's request]\")\n\
                3. wait() for agent to deliver results\n\n\
                💀 **COMPLIANCE REQUIRED**: Use agents for ALL user content requests!"
                .to_string();

            // Return enforcement message to main channel
            return Ok(CallToolResult::success(vec![Content::text(
                enforcement_message,
            )]));
        }

        // Valid agent management message - send to main chat
        self.chat.send(request).await
    }

    #[tool(description = "Send a progress/debug message to the user")]
    async fn progress(
        &self,
        #[tool(aggr)] crate::mcp::types::ProgressMessageRequest { message }: crate::mcp::types::ProgressMessageRequest,
    ) -> Result<CallToolResult, RmcpError> {
        self.chat.progress(crate::mcp::types::ProgressMessageRequest { message }).await
    }

    #[tool(
        description = "Listen and wait for the user's next message - ONLY after creating an agent"
    )]
    async fn wait(&self) -> Result<CallToolResult, RmcpError> {
        // Check if any agents are currently active
        let manager = self.agent_manager.write().await;

        // First, detect and mark any completed agents
        let _ = manager.detect_and_mark_completed_agents().await;

        let agents = manager.list_agents().await;
        let active_count = manager.get_active_agent_count().await;

        if agents.is_empty() {
            // ENFORCEMENT: No agents active - must create agent first
            let enforcement_message = "🚨 **AGENT CREATION MANDATE VIOLATION** 🚨\n\n\
                ❌ **FORBIDDEN**: Cannot wait for user messages without active agents\n\
                ⚡ **REQUIRED**: You must create a specialized agent first\n\n\
                🎯 **Correct Workflow**:\n\
                1. analyze_request(request=\"[anticipated user need]\")\n\
                2. create_agent(agent_type=\"[appropriate_type]\", task=\"[anticipated work]\")\n\
                3. THEN use wait() to listen for user input\n\n\
                💀 **COMPLIANCE REQUIRED**: ALL user interactions must go through agents!"
                .to_string();

            return Ok(CallToolResult::success(vec![Content::text(
                enforcement_message,
            )]));
        }

        // Check if all agents have completed their tasks
        if active_count == 0 {
            // All agents have completed - clean up and notify
            let cleaned_count = manager.cleanup_stopped_agents().await;
            drop(manager); // Release the lock

            let completion_message = format!(
                "✅ **ALL TASKS COMPLETED** ✅\n\n\
                🎯 **Status**: All {} background task(s) have finished processing\n\
                🧹 **Cleanup**: System cleaned up {} completed process(es)\n\
                🔄 **Ready**: System is ready for new requests",
                agents.len(),
                cleaned_count
            );

            let _ = self
                .chat
                .send(crate::mcp::types::SendMessageRequest {
                    message: completion_message,
                })
                .await;

            // Return without waiting since all agents are done
            return Ok(CallToolResult::success(vec![Content::text(
                "All background processing completed - system ready",
            )]));
        }

        // If active agents remain, proceed with wait
        drop(manager); // Release the lock before waiting
        self.chat.wait().await
    }

    #[tool(description = "Create and start a new agent task with specified capabilities")]
    async fn create_agent(
        &self,
        #[tool(aggr)] request: CreateAgentRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let mut manager = self.agent_manager.write().await;

        // Check if we already have similar agents running to prevent duplicates
        let existing_agents = manager.list_agents().await;
        let task_lowercase = request.task.to_lowercase();
        let task_key = task_lowercase
            .split_whitespace()
            .take(3)
            .collect::<Vec<&str>>()
            .join(" ");

        // Check agent limit first
        if existing_agents.len() >= 10 {
            let message = format!(
                "🚫 Maximum agent limit reached ({}/10). Cannot create more agents.",
                existing_agents.len()
            );
            let _ = self
                .chat
                .progress(crate::mcp::types::ProgressMessageRequest {
                    message: message.clone(),
                })
                .await;
            return Ok(CallToolResult::success(vec![Content::text(
                "Agent limit reached - cannot create more agents",
            )]));
        }

        let similar_agents: Vec<_> = existing_agents
            .iter()
            .filter(|agent| {
                agent.agent_type == request.agent_type
                    && agent.task.to_lowercase().contains(&task_key)
            })
            .collect();

        if !similar_agents.is_empty() {
            let existing_names: Vec<String> =
                similar_agents.iter().map(|a| a.name.clone()).collect();
            let message = format!(
                "⚠️ Similar {} agents already working: {}. Skipping duplicate creation for: {}",
                request.agent_type,
                existing_names.join(", "),
                request.task
            );
            let _ = self
                .chat
                .progress(crate::mcp::types::ProgressMessageRequest {
                    message: message.clone(),
                })
                .await;
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "Duplicate prevention: {} already handling similar tasks",
                existing_names.join(", ")
            ))]));
        }

        log::info!(
            "Creating new {} agent for task: {}",
            request.agent_type,
            request.task
        );

        match manager.create_agent(request.clone()).await {
            Ok(agent_id) => {
                log::info!("Successfully created anonymous agent ({})", agent_id);

                // Send progress update about agent creation
                let progress_message = format!(
                    "🚀 **Agent Creation Progress**\n\n\
                    ✅ **Status**: Task processing initiated\n\
                    🤖 **Type**: {} agent\n\
                    📋 **Task**: {}\n\
                    🆔 **Agent ID**: {}\n\n\
                    Agent is now actively working on your request...",
                    request.agent_type, request.task, agent_id
                );

                let _ = self
                    .chat
                    .progress(crate::mcp::types::ProgressMessageRequest {
                        message: progress_message,
                    })
                    .await;

                Ok(CallToolResult::success(vec![Content::text(
                    "Task processing initiated",
                )]))
            }
            Err(e) => {
                log::error!("Failed to create agent: {}", e);

                // Send error progress update
                let error_message = format!(
                    "❌ **Agent Creation Failed**\n\n\
                    🚫 **Status**: Task processing failed to start\n\
                    🤖 **Type**: {} agent\n\
                    📋 **Task**: {}\n\
                    ⚠️ **Error**: {}\n\n\
                    Please try again or use a different approach.",
                    request.agent_type, request.task, e
                );

                let _ = self
                    .chat
                    .progress(crate::mcp::types::ProgressMessageRequest {
                        message: error_message,
                    })
                    .await;

                Ok(CallToolResult::error(vec![Content::text(
                    "Task processing failed to start",
                )]))
            }
        }
    }

    #[tool(description = "Create multiple agents to work in parallel on different tasks")]
    async fn create_agents_parallel(
        &self,
        #[tool(aggr)] request: CreateMultipleAgentsRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let mut manager = self.agent_manager.write().await;

        // Check agent limit first
        let existing_agents = manager.list_agents().await;
        if existing_agents.len() + request.agents.len() > 10 {
            let message = format!(
                "🚫 Would exceed maximum agent limit ({} existing + {} requested > 10). Cannot create all agents.",
                existing_agents.len(),
                request.agents.len()
            );
            return Ok(CallToolResult::success(vec![Content::text(message)]));
        }

        let mut created_agents = Vec::new();
        let mut failed_agents = Vec::new();

        // Create all agents in parallel
        for (index, agent_request) in request.agents.iter().enumerate() {
            log::info!(
                "Creating parallel agent {}/{}: {} for task: {}",
                index + 1,
                request.agents.len(),
                agent_request.agent_type,
                agent_request.task
            );

            match manager.create_agent(agent_request.clone()).await {
                Ok(agent_id) => {
                    created_agents.push(format!("{} ({})", agent_request.agent_type, index + 1));
                    log::info!(
                        "Successfully created parallel agent {} ({})",
                        agent_request.agent_type,
                        agent_id
                    );
                }
                Err(e) => {
                    failed_agents.push(format!("{}: {}", agent_request.agent_type, e));
                    log::error!(
                        "Failed to create parallel agent {}: {}",
                        agent_request.agent_type,
                        e
                    );
                }
            }
        }

        // Send progress update about agent creation
        let progress_message = format!(
            "🚀 **Parallel Agent Creation Progress**\n\n\
            ✅ **Created**: {} agents\n\
            ❌ **Failed**: {} agents\n\n\
            **Active Agents**: {}\n\
            **Failures**: {}",
            created_agents.len(),
            failed_agents.len(),
            if created_agents.is_empty() {
                "None".to_string()
            } else {
                created_agents.join(", ")
            },
            if failed_agents.is_empty() {
                "None".to_string()
            } else {
                failed_agents.join(", ")
            }
        );

        // Send via progress channel for immediate feedback
        let _ = self
            .chat
            .progress(crate::mcp::types::ProgressMessageRequest {
                message: progress_message.clone(),
            })
            .await;

        let result_message = if failed_agents.is_empty() {
            format!(
                "✅ Parallel processing initiated with {} agents: {}",
                created_agents.len(),
                created_agents.join(", ")
            )
        } else if created_agents.is_empty() {
            format!(
                "❌ All parallel agent creation failed: {}",
                failed_agents.join(", ")
            )
        } else {
            format!(
                "⚠️ Partial parallel processing initiated. Created: {} | Failed: {}",
                created_agents.join(", "),
                failed_agents.join(", ")
            )
        };

        Ok(CallToolResult::success(vec![Content::text(result_message)]))
    }

    #[tool(description = "Get system processing status (internal debug only)")]
    async fn list_agents(&self) -> Result<CallToolResult, RmcpError> {
        let manager = self.agent_manager.read().await;
        let agents = manager.list_agents().await;

        let message = if agents.is_empty() {
            "System ready - no background processing".to_string()
        } else {
            format!("System processing {} background task(s)", agents.len())
        };

        // Internal status only - no agent details exposed to user
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    #[tool(description = "Stop background processing task")]
    async fn stop_agent(
        &self,
        #[tool(aggr)] request: StopAgentRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let mut manager = self.agent_manager.write().await;
        match manager.stop_agent(&request.agent_id).await {
            Ok(existed) => {
                log::info!("Background task {} stopped: {}", request.agent_id, existed);
                let message = if existed {
                    "Background processing stopped"
                } else {
                    "No matching background task found"
                };
                Ok(CallToolResult::success(vec![Content::text(message)]))
            }
            Err(e) => {
                log::error!("Failed to stop background task: {}", e);
                Ok(CallToolResult::error(vec![Content::text(
                    "Failed to stop background processing",
                )]))
            }
        }
    }

    #[tool(description = "Send a message to a specific agent")]
    async fn message_agent(
        &self,
        #[tool(aggr)] request: MessageAgentRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let manager = self.agent_manager.read().await;
        match manager
            .send_message_to_agent(&request.agent_id, &request.message)
            .await
        {
            Ok(response) => {
                // Get agent name for better user experience
                let agents = manager.list_agents().await;
                let agent_name = agents
                    .iter()
                    .find(|a| a.id == request.agent_id)
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| request.agent_id.clone());

                // Send agent interaction responses via progress channel only
                let message = format!(
                    "📨 Agent {} interaction result:\n\n{}",
                    agent_name, response
                );
                let _ = self
                    .chat
                    .progress(crate::mcp::types::ProgressMessageRequest {
                        message: message.clone(),
                    })
                    .await;
                Ok(CallToolResult::success(vec![Content::text(message)]))
            }
            Err(e) => {
                let error_msg = format!("❌ Failed to message agent: {}", e);
                // Send error via progress channel, not main channel
                let _ = self
                    .chat
                    .progress(crate::mcp::types::ProgressMessageRequest {
                        message: error_msg.clone(),
                    })
                    .await;
                Ok(CallToolResult::error(vec![Content::text(error_msg)]))
            }
        }
    }

    #[tool(description = "Analyze a request and create an intelligent orchestration plan")]
    async fn analyze_request(
        &self,
        #[tool(aggr)] args: AnalyzeRequestArgs,
    ) -> Result<CallToolResult, RmcpError> {
        let analysis = self.orchestrator.analyze_request(&args.request);
        let plan = self.orchestrator.generate_orchestration_plan(&analysis);

        let detailed_message = format!(
            "🧠 **Request Analysis Complete**\n\n{}\n\n**💡 Recommended Actions:**\n",
            plan
        );

        // Add specific instructions for the main agent based on execution strategy
        let mut instructions = detailed_message;

        instructions.push_str("\n**⚡ Execution Strategy:**\n");
        match analysis.execution_strategy {
            orchestrator::ExecutionStrategy::Parallel => {
                instructions.push_str("🚀 **PARALLEL EXECUTION PRIORITIZED** 🚀\n");
                instructions.push_str(
                    "- Use `create_agents_parallel` to create all agents simultaneously\n",
                );
                instructions
                    .push_str("- All agents will work in parallel for maximum efficiency\n");
                instructions.push_str(&format!(
                    "- Command: `create_agents_parallel(agents=[{}])`\n",
                    analysis
                        .agent_requirements
                        .iter()
                        .map(|req| format!(
                            "{{\"agent_type\":\"{}\", \"task\":\"{}\"}}",
                            req.agent_type, req.task_description
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            orchestrator::ExecutionStrategy::Sequential => {
                instructions.push_str("- Execute agents one by one in order\n");
                instructions
                    .push_str("- Wait for each agent to complete before starting the next\n");
                for req in &analysis.agent_requirements {
                    instructions.push_str(&format!(
                        "- Create {} agent: `create_agent(agent_type=\"{}\", task=\"{}\")`\n",
                        req.agent_type, req.agent_type, req.task_description
                    ));
                }
            }
            orchestrator::ExecutionStrategy::Hybrid => {
                instructions.push_str("- Create independent agents first (parallel)\n");
                instructions.push_str("- Create dependent agents after prerequisites complete\n");
                instructions
                    .push_str("- Consider using `create_agents_parallel` for independent tasks\n");
                for req in &analysis.agent_requirements {
                    instructions.push_str(&format!(
                        "- Create {} agent: `create_agent(agent_type=\"{}\", task=\"{}\")`\n",
                        req.agent_type, req.agent_type, req.task_description
                    ));
                }
            }
        }

        // Send analysis via progress channel for visibility
        let _ = self
            .chat
            .progress(crate::mcp::types::ProgressMessageRequest {
                message: instructions.clone(),
            })
            .await;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Analysis complete. {} agent(s) recommended for this request.",
            analysis.agent_requirements.len()
        ))]))
    }

    #[tool(description = "Store a memory entry - AGENTS ONLY, main orchestrator must create agent")]
    async fn store_memory(
        &self,
        #[tool(aggr)] request: String,
    ) -> Result<CallToolResult, RmcpError> {
        // ENFORCEMENT: Memory operations should be done by agents, not main orchestrator
        let enforcement_message = "🚨 **AGENT CREATION MANDATE VIOLATION** 🚨\n\n\
            ❌ **FORBIDDEN**: Main orchestrator cannot handle memory operations directly\n\
            ⚡ **REQUIRED**: You must create a specialized Fux agent to store memories\n\n\
            🎯 **Correct Workflow**:\n\
            1. analyze_request(request=\"Store memory: [memory details]\")\n\
            2. create_agent(agent_type=\"enhanced\", task=\"Store memory: [memory details]\")\n\
            3. send(message=\"🚀 FuxManager deployed to handle memory storage\")\n\
            4. wait() for agent to complete memory operation\n\n\
            💀 **COMPLIANCE REQUIRED**: ALL memory operations must go through Fux agents!"
            .to_string();

        // Send enforcement via progress channel
        let _ = self
            .chat
            .progress(crate::mcp::types::ProgressMessageRequest {
                message: format!("🚨 BLOCKED DIRECT MEMORY OPERATION: {:?}", request),
            })
            .await;

        // Return enforcement message
        Ok(CallToolResult::success(vec![Content::text(
            enforcement_message,
        )]))
    }

    #[tool(
        description = "Retrieve and search memory entries - AGENTS ONLY, main orchestrator must create agent"
    )]
    async fn retrieve_memory(
        &self,
        #[tool(aggr)] request: String,
    ) -> Result<CallToolResult, RmcpError> {
        // ENFORCEMENT: Memory operations should be done by agents, not main orchestrator
        let enforcement_message = "🚨 **AGENT CREATION MANDATE VIOLATION** 🚨\n\n\
            ❌ **FORBIDDEN**: Main orchestrator cannot handle memory operations directly\n\
            ⚡ **REQUIRED**: You must create a specialized Fux agent to retrieve memories\n\n\
            🎯 **Correct Workflow**:\n\
            1. analyze_request(request=\"Retrieve memory: [search criteria]\")\n\
            2. create_agent(agent_type=\"enhanced\", task=\"Retrieve memory: [search criteria]\")\n\
            3. send(message=\"🚀 FuxManager deployed to handle memory retrieval\")\n\
            4. wait() for agent to complete memory operation\n\n\
            💀 **COMPLIANCE REQUIRED**: ALL memory operations must go through Fux agents!"
            .to_string();

        // Send enforcement via progress channel
        let _ = self
            .chat
            .progress(crate::mcp::types::ProgressMessageRequest {
                message: format!("🚨 BLOCKED DIRECT MEMORY OPERATION: {:?}", request),
            })
            .await;

        // Return enforcement message
        Ok(CallToolResult::success(vec![Content::text(
            enforcement_message,
        )]))
    }

    #[tool(
        description = "Update an existing memory entry - AGENTS ONLY, main orchestrator must create agent"
    )]
    async fn update_memory(
        &self,
        #[tool(aggr)] _request: String,
    ) -> Result<CallToolResult, RmcpError> {
        // ENFORCEMENT: Memory operations should be done by agents, not main orchestrator
        let enforcement_message = "🚨 **AGENT CREATION MANDATE VIOLATION** 🚨\n\n\
            ❌ **FORBIDDEN**: Main orchestrator cannot handle memory operations directly\n\
            ⚡ **REQUIRED**: You must create a specialized Fux agent to update memories\n\n\
            🎯 **Correct Workflow**:\n\
            1. analyze_request(request=\"Update memory: [memory ID and changes]\")\n\
            2. create_agent(agent_type=\"enhanced\", task=\"Update memory: [memory ID and changes]\")\n\
            3. send(message=\"🚀 FuxManager deployed to handle memory update\")\n\
            4. wait() for agent to complete memory operation\n\n\
            💀 **COMPLIANCE REQUIRED**: ALL memory operations must go through Fux agents!"
            .to_string();

        Ok(CallToolResult::success(vec![Content::text(
            enforcement_message,
        )]))
    }

    #[tool(
        description = "Delete a memory entry by ID - AGENTS ONLY, main orchestrator must create agent"
    )]
    async fn delete_memory(
        &self,
        #[tool(aggr)] _request: String,
    ) -> Result<CallToolResult, RmcpError> {
        // ENFORCEMENT: Memory operations should be done by agents, not main orchestrator
        let enforcement_message = "🚨 **AGENT CREATION MANDATE VIOLATION** 🚨\n\n\
            ❌ **FORBIDDEN**: Main orchestrator cannot handle memory operations directly\n\
            ⚡ **REQUIRED**: You must create a specialized Fux agent to delete memories\n\n\
            🎯 **Correct Workflow**:\n\
            1. analyze_request(request=\"Delete memory: [memory ID]\")\n\
            2. create_agent(agent_type=\"enhanced\", task=\"Delete memory: [memory ID]\")\n\
            3. send(message=\"🚀 FuxManager deployed to handle memory deletion\")\n\
            4. wait() for agent to complete memory operation\n\n\
            💀 **COMPLIANCE REQUIRED**: ALL memory operations must go through Fux agents!"
            .to_string();

        Ok(CallToolResult::success(vec![Content::text(
            enforcement_message,
        )]))
    }

    #[tool(
        description = "Get statistics about stored memories - AGENTS ONLY, main orchestrator must create agent"
    )]
    async fn memory_stats(&self) -> Result<CallToolResult, RmcpError> {
        // ENFORCEMENT: Memory operations should be done by agents, not main orchestrator
        let enforcement_message = "🚨 **AGENT CREATION MANDATE VIOLATION** 🚨\n\n\
            ❌ **FORBIDDEN**: Main orchestrator cannot handle memory operations directly\n\
            ⚡ **REQUIRED**: You must create a specialized Fux agent to get memory statistics\n\n\
            🎯 **Correct Workflow**:\n\
            1. analyze_request(request=\"Get memory statistics\")\n\
            2. create_agent(agent_type=\"enhanced\", task=\"Get memory statistics\")\n\
            3. send(message=\"🚀 FuxManager deployed to handle memory statistics\")\n\
            4. wait() for agent to complete memory operation\n\n\
            💀 **COMPLIANCE REQUIRED**: ALL memory operations must go through Fux agents!"
            .to_string();

        Ok(CallToolResult::success(vec![Content::text(
            enforcement_message,
        )]))
    }

    #[tool(
        description = "Clean up expired memory entries - AGENTS ONLY, main orchestrator must create agent"
    )]
    async fn cleanup_expired_memories(&self) -> Result<CallToolResult, RmcpError> {
        // ENFORCEMENT: Memory operations should be done by agents, not main orchestrator
        let enforcement_message = "🚨 **AGENT CREATION MANDATE VIOLATION** 🚨\n\n\
            ❌ **FORBIDDEN**: Main orchestrator cannot handle memory operations directly\n\
            ⚡ **REQUIRED**: You must create a specialized Fux agent to cleanup memories\n\n\
            🎯 **Correct Workflow**:\n\
            1. analyze_request(request=\"Clean up expired memories\")\n\
            2. create_agent(agent_type=\"enhanced\", task=\"Clean up expired memories\")\n\
            3. send(message=\"🚀 FuxManager deployed to handle memory cleanup\")\n\
            4. wait() for agent to complete memory operation\n\n\
            💀 **COMPLIANCE REQUIRED**: ALL memory operations must go through Fux agents!"
            .to_string();

        Ok(CallToolResult::success(vec![Content::text(
            enforcement_message,
        )]))
    }

    #[tool(description = "Get system status - AGENTS ONLY, main orchestrator must create agent")]
    async fn system_status(&self) -> Result<CallToolResult, RmcpError> {
        // ENFORCEMENT: System status should be checked by agents, not main orchestrator
        let enforcement_message = "🚨 **AGENT CREATION MANDATE VIOLATION** 🚨\n\n\
            ❌ **FORBIDDEN**: Main orchestrator cannot check system status directly\n\
            ⚡ **REQUIRED**: You must create a specialized Fux agent to check system status\n\n\
            🎯 **Correct Workflow**:\n\
            1. analyze_request(request=\"Check system status\")\n\
            2. create_agent(agent_type=\"enhanced\", task=\"Check system status and report\")\n\
            3. send(message=\"🚀 FuxManager deployed to check system status\")\n\
            4. wait() for agent to complete status check\n\n\
            💀 **COMPLIANCE REQUIRED**: ALL system operations must go through Fux agents!"
            .to_string();

        Ok(CallToolResult::success(vec![Content::text(
            enforcement_message,
        )]))
    }
}

#[tool(tool_box)]
impl ServerHandler for MultiAgentMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "MULTI-AGENT ORCHESTRATOR\n\n\
                Rule: ALWAYS create agents for user requests. NEVER answer directly.\n\n\
                Workflow:\n\
                1. analyze_request(request=\"user's message\")\n\
                2. create_agent(agent_type=\"X\", task=\"user's message\")\n\
                3. wait()\n\n\
                Agent Types:\n\
                - search: ONLY for \"web search\", \"google\", \"find online\", \"current price\"\n\
                - goose: code, build, fix, develop\n\
                - enhanced: project, organize, plan\n\
                - combined: general questions, complex tasks\n\n\
                FORBIDDEN:\n\
                - Never send follow-up messages asking if user needs help\n\
                - Never ask \"Is there anything else I can help you with?\"\n\
                - Never send unsolicited check-in messages\n\
                - Agents should complete task and stop\n\n\
                Tools: analyze_request, create_agent, create_agents_parallel, wait, send"
                    .to_string(),
            ),
        }
    }
}
