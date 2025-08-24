use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub content: Option<String>,
    pub engine: Option<String>,
    pub score: Option<f64>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total_results: usize,
    pub page: u32,
    pub per_page: u32,
    pub answers: Option<Vec<String>>,
    pub suggestions: Option<Vec<String>>,
    pub corrections: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearXNGWebSearchRequest {
    #[schemars(description = "Search terms")]
    pub query: String,
    #[schemars(description = "Results per page (default 20)")]
    pub count: Option<u32>,
    #[schemars(description = "Pagination offset (default 0)")]
    pub offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SearXNGConfig {
    pub base_url: String,
    pub default_count: u32,
    pub max_count: u32,
}
