use super::types::*;
use std::error::Error;

#[derive(Debug, Clone)]
pub struct SearXNGClient {
    client: reqwest::Client,
    config: SearXNGConfig,
}

impl SearXNGClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            config: SearXNGConfig {
                base_url,
                default_count: 20,
                max_count: 100,
            },
        }
    }

    #[allow(dead_code)] // Future configuration support
    pub fn with_config(config: SearXNGConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    pub async fn search(
        &self,
        request: SearXNGWebSearchRequest,
    ) -> Result<SearchResponse, Box<dyn Error + Send + Sync>> {
        if request.query.trim().is_empty() {
            return Err("Search query cannot be empty".into());
        }

        let count = request
            .count
            .unwrap_or(self.config.default_count)
            .min(self.config.max_count)
            .max(1);
        let offset = request.offset.unwrap_or(0);
        let page = (offset / count) + 1;

        let url = format!("{}/search", self.config.base_url.trim_end_matches('/'));
        let params = vec![
            ("q", request.query.clone()),
            ("format", "json".to_string()),
            ("pageno", page.to_string()),
        ];

        let response = self
            .client
            .get(&url)
            .query(&params)
            .header("Accept", "application/json")
            .header("User-Agent", "Mozilla/5.0 (compatible; SearXNG-MCP/1.0)")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());
            return Err(format!("SearXNG API error {}: {}", status, error_body).into());
        }

        let json_response: serde_json::Value = response.json().await?;

        let results: Vec<SearchResult> = json_response
            .get("results")
            .and_then(|r| r.as_array())
            .unwrap_or(&vec![])
            .iter()
            .skip(offset as usize % count as usize)
            .take(count as usize)
            .filter_map(|result| {
                Some(SearchResult {
                    title: result.get("title")?.as_str()?.to_string(),
                    url: result.get("url")?.as_str()?.to_string(),
                    content: result
                        .get("content")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_string()),
                    engine: result
                        .get("engine")
                        .and_then(|e| e.as_str())
                        .map(|s| s.to_string()),
                    score: result.get("score").and_then(|s| s.as_f64()),
                    category: result
                        .get("category")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_string()),
                })
            })
            .collect();

        let answers = json_response
            .get("answers")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            });

        let suggestions = json_response
            .get("suggestions")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            });

        let corrections = json_response
            .get("corrections")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            });

        let total_results = json_response
            .get("number_of_results")
            .and_then(|n| n.as_u64())
            .unwrap_or(results.len() as u64) as usize;

        Ok(SearchResponse {
            query: request.query,
            results,
            total_results,
            page,
            per_page: count,
            answers,
            suggestions,
            corrections,
        })
    }
}
