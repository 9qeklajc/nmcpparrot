// Remove unused imports

#[derive(Debug, Clone)]
pub struct TaskAnalysis {
    pub primary_intent: String,
    pub sub_tasks: Vec<SubTask>,
    pub agent_requirements: Vec<AgentRequirement>,
    pub execution_strategy: ExecutionStrategy,
}

#[derive(Debug, Clone)]
pub struct SubTask {
    pub id: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub agent_type: String,
    pub priority: u8,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AgentRequirement {
    pub agent_type: String,
    pub task_description: String,
    pub reason: String,
    pub urgency: TaskUrgency,
}

#[derive(Debug, Clone)]
pub enum ExecutionStrategy {
    Sequential,
    Parallel,
    Hybrid,
}

#[derive(Debug, Clone)]
pub enum TaskUrgency {
    Critical,
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone)]
pub struct IntelligentOrchestrator {
    // Keyword mappings for automatic agent type detection
    search_keywords: Vec<&'static str>,
    development_keywords: Vec<&'static str>,
    project_keywords: Vec<&'static str>,
    communication_keywords: Vec<&'static str>,
    multi_tool_keywords: Vec<&'static str>,
}

impl IntelligentOrchestrator {
    pub fn new() -> Self {
        Self {
            search_keywords: vec![
                // EXPLICIT WEB SEARCH COMMANDS
                "web search",
                "search the web",
                "search online",
                "search the internet",
                "internet search",
                "online search",
                "web lookup",
                "web research",
                "internet research",
                "online research",
                "search web",
                "web query",
                "internet query",
                "online query",
                // SEARCH ENGINE REFERENCES
                "google",
                "google for",
                "google search",
                "bing",
                "bing search",
                "search engine",
                "use search engine",
                "search engines",
                // EXPLICIT ONLINE LOOKUP PHRASES
                "find online",
                "look up online",
                "find on the web",
                "look up on the web",
                "check online",
                "verify online",
                "browse for",
                "browse online",
                "surf for",
                "hunt online",
                "discover online",
                "explore online",
                "investigate online",
                // CURRENT/LIVE DATA REQUESTS
                "current price",
                "live price",
                "real-time price",
                "latest price",
                "price check online",
                "check current price",
                "get current price",
                "find current price",
                "search current price",
                "live data",
                "real-time data",
                "current data",
                "fresh data",
                "updated data",
                "up-to-date",
                "most recent",
                // NEWS AND INFORMATION SEARCHES
                "latest news",
                "recent news",
                "breaking news",
                "current news",
                "news search",
                "search news",
                "find news",
                "get news",
                "check news",
                "news online",
                "web news",
                "internet news",
                "online news",
                "headlines",
                "breaking headlines",
                "current headlines",
                // MARKET AND FINANCIAL DATA
                "market data",
                "stock price",
                "crypto price",
                "bitcoin price",
                "cryptocurrency price",
                "exchange rate",
                "currency rate",
                "trading price",
                "market price",
                "financial data",
                "stock market",
                "crypto market",
                "market search",
                "price search",
                // VERIFICATION AND FACT-CHECKING
                "verify this online",
                "fact check",
                "confirm online",
                "validate online",
                "check this online",
                "research this online",
                "investigate this online",
                // WEATHER AND STATUS CHECKS
                "current weather",
                "weather search",
                "check weather",
                "weather online",
                "status online",
                "check status online",
                // EXPLICIT INFORMATION GATHERING
                "gather information online",
                "collect data online",
                "find information online",
                "search for information",
                "online information",
                "web information",
                "internet information",
                "search for data",
                "find data online",
                "get data online",
                // COMPARISON SEARCHES
                "compare online",
                "search and compare",
                "find alternatives online",
                "research options online",
                "compare prices online",
                "search competitors",
                // DISCOVERY AND EXPLORATION
                "discover online",
                "explore web",
                "find websites",
                "search websites",
                "web discovery",
                "online discovery",
                "internet discovery",
            ],
            development_keywords: vec![
                "code",
                "develop",
                "build",
                "create",
                "implement",
                "fix",
                "bug",
                "error",
                "debug",
                "test",
                "deploy",
                "compile",
                "function",
                "class",
                "method",
                "api",
                "database",
                "server",
                "authentication",
                "auth",
                "login",
                "security",
                "system",
                "feature",
                "module",
                "component",
                "library",
                "framework",
                "application",
                "program",
                "script",
                "software",
            ],
            project_keywords: vec![
                "project",
                "manage",
                "organize",
                "plan",
                "schedule",
                "track",
                "document",
                "note",
                "event",
                "milestone",
                "task",
                "todo",
                "workflow",
                "process",
                "coordination",
                "team",
                "progress",
                "report",
                "status",
                "update",
                "meeting",
                "deadline",
            ],
            communication_keywords: vec![
                "message",
                "send",
                "notify",
                "alert",
                "communicate",
                "chat",
                "email",
                "call",
                "contact",
                "reach",
                "inform",
                "tell",
                "announce",
                "broadcast",
                "share",
                "discuss",
                "talk",
            ],
            multi_tool_keywords: vec![
                "complex",
                "multiple",
                "comprehensive",
                "full",
                "complete",
                "end-to-end",
                "integrated",
                "combined",
                "multi-step",
                "workflow",
                "pipeline",
                "orchestrate",
                "coordinate",
                "various aspects",
                "different components",
                "step by step",
            ],
        }
    }

    pub fn analyze_request(&self, request: &str) -> TaskAnalysis {
        let request_lower = request.to_lowercase();
        let words: Vec<&str> = request_lower.split_whitespace().collect();

        // Detect primary intent and complexity
        let complexity = self.assess_complexity(&request_lower, &words);
        let primary_intent = self.determine_primary_intent(&request_lower);

        // Break down into sub-tasks if complex
        let sub_tasks = if complexity > 3 {
            self.decompose_complex_request(&request_lower, &words)
        } else {
            self.create_simple_task(&request_lower)
        };

        // Determine agent requirements
        let agent_requirements = self.determine_agent_requirements(&sub_tasks, &request_lower);

        // Choose execution strategy
        let execution_strategy = self.choose_execution_strategy(&sub_tasks, &agent_requirements);

        TaskAnalysis {
            primary_intent,
            sub_tasks,
            agent_requirements,
            execution_strategy,
        }
    }

    fn assess_complexity(&self, request: &str, words: &[&str]) -> u8 {
        let mut complexity = 0;

        // Multiple action words increase complexity
        let action_words = ["and", "then", "also", "plus", "additionally", "furthermore"];
        complexity += action_words
            .iter()
            .filter(|&word| request.contains(word))
            .count() as u8;

        // Multiple domain keywords increase complexity
        let mut domain_count = 0;
        if self.contains_keywords(request, &self.search_keywords) {
            domain_count += 1;
        }
        if self.contains_keywords(request, &self.development_keywords) {
            domain_count += 1;
        }
        if self.contains_keywords(request, &self.project_keywords) {
            domain_count += 1;
        }
        if self.contains_keywords(request, &self.communication_keywords) {
            domain_count += 1;
        }

        complexity += if domain_count > 1 {
            domain_count * 2
        } else {
            0
        };

        // Long requests are often complex
        if words.len() > 15 {
            complexity += 2;
        }
        if words.len() > 25 {
            complexity += 2;
        }

        // Specific complexity indicators
        if request.contains("step") || request.contains("phase") {
            complexity += 2;
        }
        if request.contains("first") && (request.contains("then") || request.contains("next")) {
            complexity += 3;
        }

        complexity.min(10)
    }

    fn determine_primary_intent(&self, request: &str) -> String {
        if self.contains_keywords(request, &self.search_keywords) {
            "Information Gathering".to_string()
        } else if self.contains_keywords(request, &self.development_keywords) {
            "Development & Implementation".to_string()
        } else if self.contains_keywords(request, &self.project_keywords) {
            "Project Management".to_string()
        } else if self.contains_keywords(request, &self.communication_keywords) {
            "Communication & Coordination".to_string()
        } else if self.contains_keywords(request, &self.multi_tool_keywords) {
            "Multi-Domain Operation".to_string()
        } else {
            "General Task Execution".to_string()
        }
    }

    fn decompose_complex_request(&self, request: &str, _words: &[&str]) -> Vec<SubTask> {
        let mut sub_tasks = Vec::new();
        let mut task_id = 1;

        // Split by common connectors
        let parts = self.split_request_into_parts(request);

        for part in parts {
            let part_trimmed = part.trim();
            if part_trimmed.is_empty() {
                continue;
            }

            let keywords = self.extract_keywords(part_trimmed);
            let agent_type = self.determine_agent_type_for_part(part_trimmed);
            let priority = self.assess_priority(part_trimmed);

            sub_tasks.push(SubTask {
                id: format!("task_{}", task_id),
                description: part_trimmed.to_string(),
                keywords,
                agent_type,
                priority,
                dependencies: if task_id > 1 {
                    vec![format!("task_{}", task_id - 1)]
                } else {
                    vec![]
                },
            });

            task_id += 1;
        }

        // If no clear parts, create domain-based tasks
        if sub_tasks.len() <= 1 {
            sub_tasks = self.create_domain_based_tasks(request);
        }

        sub_tasks
    }

    fn create_simple_task(&self, request: &str) -> Vec<SubTask> {
        let keywords = self.extract_keywords(request);
        let agent_type = self.determine_agent_type_for_part(request);

        vec![SubTask {
            id: "task_1".to_string(),
            description: request.to_string(),
            keywords,
            agent_type,
            priority: 5,
            dependencies: vec![],
        }]
    }

    fn split_request_into_parts(&self, request: &str) -> Vec<String> {
        let delimiters = [
            " and ",
            " then ",
            " also ",
            " plus ",
            " additionally ",
            " furthermore ",
            ", ",
            "; ",
        ];
        let mut parts = vec![request.to_string()];

        for delimiter in &delimiters {
            let mut new_parts = Vec::new();
            for part in parts {
                let split_parts: Vec<&str> = part.split(delimiter).collect();
                if split_parts.len() > 1 {
                    new_parts.extend(split_parts.iter().map(|s| s.to_string()));
                } else {
                    new_parts.push(part);
                }
            }
            parts = new_parts;
        }

        parts
    }

    fn create_domain_based_tasks(&self, request: &str) -> Vec<SubTask> {
        let mut tasks = Vec::new();
        let mut task_id = 1;

        // Check each domain and create tasks accordingly
        if self.contains_keywords(request, &self.search_keywords) {
            tasks.push(SubTask {
                id: format!("task_{}", task_id),
                description: format!("Research and gather information: {}", request),
                keywords: self.extract_keywords_from_domain(&self.search_keywords, request),
                agent_type: "search".to_string(),
                priority: 7,
                dependencies: vec![],
            });
            task_id += 1;
        }

        if self.contains_keywords(request, &self.development_keywords) {
            tasks.push(SubTask {
                id: format!("task_{}", task_id),
                description: format!("Development implementation: {}", request),
                keywords: self.extract_keywords_from_domain(&self.development_keywords, request),
                agent_type: "goose".to_string(),
                priority: 8,
                dependencies: if tasks.is_empty() {
                    vec![]
                } else {
                    vec![tasks.last().unwrap().id.clone()]
                },
            });
            task_id += 1;
        }

        if self.contains_keywords(request, &self.project_keywords) {
            tasks.push(SubTask {
                id: format!("task_{}", task_id),
                description: format!("Project management: {}", request),
                keywords: self.extract_keywords_from_domain(&self.project_keywords, request),
                agent_type: "enhanced".to_string(),
                priority: 6,
                dependencies: vec![],
            });
            let _ = task_id; // Task ID tracked for future use
        }

        if tasks.is_empty() {
            // Fallback to combined agent for complex requests
            tasks.push(SubTask {
                id: "task_1".to_string(),
                description: request.to_string(),
                keywords: self.extract_keywords(request),
                agent_type: "combined".to_string(),
                priority: 5,
                dependencies: vec![],
            });
        }

        tasks
    }

    fn determine_agent_requirements(
        &self,
        sub_tasks: &[SubTask],
        _original_request: &str,
    ) -> Vec<AgentRequirement> {
        let mut requirements = Vec::new();
        let mut agent_types_used = std::collections::HashSet::new();

        for task in sub_tasks {
            if !agent_types_used.contains(&task.agent_type) {
                let urgency = match task.priority {
                    9..=10 => TaskUrgency::Critical,
                    7..=8 => TaskUrgency::High,
                    4..=6 => TaskUrgency::Normal,
                    _ => TaskUrgency::Low,
                };

                let reason = match task.agent_type.as_str() {
                    "search" => "Information gathering and research required",
                    "goose" => "Development and implementation tasks detected",
                    "enhanced" => "Project management and documentation needed",
                    "combined" => "Multi-domain operation requiring various tools",
                    "chat" => "Communication and coordination required",
                    _ => "General task execution needed",
                };

                requirements.push(AgentRequirement {
                    agent_type: task.agent_type.clone(),
                    task_description: task.description.clone(),
                    reason: reason.to_string(),
                    urgency,
                });

                agent_types_used.insert(task.agent_type.clone());
            }
        }

        requirements
    }

    fn choose_execution_strategy(
        &self,
        sub_tasks: &[SubTask],
        agent_requirements: &[AgentRequirement],
    ) -> ExecutionStrategy {
        // Check for explicit parallel keywords in tasks
        let parallel_keywords = [
            "parallel",
            "simultaneously",
            "concurrent",
            "at the same time",
            "together",
            "in parallel",
            "concurrently",
            "multiple agents",
        ];

        let has_parallel_intent = sub_tasks.iter().any(|task| {
            parallel_keywords
                .iter()
                .any(|&keyword| task.description.to_lowercase().contains(keyword))
        });

        if has_parallel_intent {
            return ExecutionStrategy::Parallel;
        }

        // PRIORITIZE PARALLEL EXECUTION for efficiency
        if sub_tasks.len() <= 1 && agent_requirements.len() <= 1 {
            return ExecutionStrategy::Sequential;
        }

        // Check for dependencies
        let has_dependencies = sub_tasks.iter().any(|task| !task.dependencies.is_empty());

        if has_dependencies {
            ExecutionStrategy::Hybrid
        } else {
            // DEFAULT TO PARALLEL for multiple agents (was > 2, now >= 2)
            ExecutionStrategy::Parallel
        }
    }

    fn contains_keywords(&self, text: &str, keywords: &[&str]) -> bool {
        keywords.iter().any(|&keyword| text.contains(keyword))
    }

    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let mut keywords = Vec::new();

        for &keyword in self
            .search_keywords
            .iter()
            .chain(self.development_keywords.iter())
            .chain(self.project_keywords.iter())
            .chain(self.communication_keywords.iter())
            .chain(self.multi_tool_keywords.iter())
        {
            if text.contains(keyword) {
                keywords.push(keyword.to_string());
            }
        }

        keywords
    }

    fn extract_keywords_from_domain(&self, domain_keywords: &[&str], text: &str) -> Vec<String> {
        domain_keywords
            .iter()
            .filter(|&&keyword| text.contains(keyword))
            .map(|&keyword| keyword.to_string())
            .collect()
    }

    fn determine_agent_type_for_part(&self, part: &str) -> String {
        if self.contains_keywords(part, &self.search_keywords) {
            "search".to_string()
        } else if self.contains_keywords(part, &self.development_keywords) {
            "goose".to_string()
        } else if self.contains_keywords(part, &self.project_keywords) {
            "enhanced".to_string()
        } else if self.contains_keywords(part, &self.communication_keywords) {
            "chat".to_string()
        } else {
            // For multi-tool keywords or unrecognized patterns, default to combined
            "combined".to_string()
        }
    }

    fn assess_priority(&self, part: &str) -> u8 {
        let mut priority = 5; // Default priority

        // Urgency indicators
        if part.contains("urgent") || part.contains("critical") || part.contains("asap") {
            priority += 3;
        }
        if part.contains("important") || part.contains("priority") {
            priority += 2;
        }
        if part.contains("first") || part.contains("immediately") {
            priority += 2;
        }

        // Time sensitivity
        if part.contains("now") || part.contains("today") {
            priority += 1;
        }

        priority.min(10)
    }

    pub fn generate_orchestration_plan(&self, analysis: &TaskAnalysis) -> String {
        let mut plan = "🎯 **Intelligent Task Orchestration Plan**\n\n".to_string();
        plan.push_str(&format!(
            "**Primary Intent**: {}\n",
            analysis.primary_intent
        ));
        plan.push_str(&format!(
            "**Execution Strategy**: {:?}\n",
            analysis.execution_strategy
        ));
        plan.push_str(&format!(
            "**Sub-tasks Identified**: {}\n\n",
            analysis.sub_tasks.len()
        ));

        plan.push_str("**📋 Task Breakdown:**\n");
        for (i, task) in analysis.sub_tasks.iter().enumerate() {
            plan.push_str(&format!(
                "{}. **{}** ({})\n   - {}\n   - Keywords: {}\n   - Priority: {}/10\n\n",
                i + 1,
                task.id,
                task.agent_type,
                task.description,
                task.keywords.join(", "),
                task.priority
            ));
        }

        plan.push_str("**🤖 Agent Requirements:**\n");
        for req in &analysis.agent_requirements {
            plan.push_str(&format!(
                "- **{}** Agent: {} (Urgency: {:?})\n",
                req.agent_type.to_uppercase(),
                req.reason,
                req.urgency
            ));
        }

        plan
    }
}
