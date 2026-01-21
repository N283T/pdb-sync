//! RCSB Search API client for finding new PDB entries.
//!
//! Uses the RCSB Search API v2 to query for entries released after a given date
//! with optional filters for experimental method, resolution, and organism.

use crate::cli::args::ExperimentalMethod;
use crate::error::{PdbSyncError, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const SEARCH_API_URL: &str = "https://search.rcsb.org/rcsbsearch/v2/query";
const USER_AGENT: &str = concat!("pdb-sync/", env!("CARGO_PKG_VERSION"));

/// Filters for the RCSB search query
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    /// Filter by experimental method
    pub method: Option<ExperimentalMethod>,
    /// Filter by maximum resolution (Å)
    pub resolution: Option<f64>,
    /// Filter by source organism (scientific name)
    pub organism: Option<String>,
}

/// RCSB Search API client
pub struct RcsbSearchClient {
    client: reqwest::Client,
}

/// Search result from RCSB API
#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub result_set: Vec<ResultEntry>,
    #[allow(dead_code)]
    pub total_count: u32,
}

#[derive(Debug, Deserialize)]
pub struct ResultEntry {
    pub identifier: String,
}

/// Query structure for RCSB Search API
#[derive(Debug, Serialize)]
struct SearchQuery {
    query: QueryNode,
    return_type: String,
    request_options: RequestOptions,
}

#[derive(Debug, Serialize)]
struct RequestOptions {
    results_content_type: Vec<String>,
    return_all_hits: bool,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[allow(clippy::enum_variant_names)]
enum QueryNode {
    #[serde(rename = "group")]
    GroupNode {
        logical_operator: String,
        nodes: Vec<QueryNode>,
    },
    #[serde(rename = "terminal")]
    TerminalNode {
        service: String,
        parameters: TerminalParameters,
    },
}

#[derive(Debug, Serialize)]
struct TerminalParameters {
    attribute: String,
    operator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    negation: Option<bool>,
}

impl RcsbSearchClient {
    /// Create a new RCSB Search API client
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| PdbSyncError::SearchApi(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self { client })
    }

    /// Search for entries released since the given date
    pub async fn search_new_entries(
        &self,
        since: NaiveDate,
        filters: &SearchFilters,
    ) -> Result<Vec<String>> {
        let query = self.build_query(since, filters);
        let query_json = serde_json::to_string(&query)
            .map_err(|e| PdbSyncError::SearchApi(format!("Failed to serialize query: {}", e)))?;

        tracing::debug!("RCSB Search query: {}", query_json);

        let response = self
            .client
            .post(SEARCH_API_URL)
            .header("Content-Type", "application/json")
            .body(query_json)
            .send()
            .await
            .map_err(|e| PdbSyncError::SearchApi(format!("Request failed: {}", e)))?;

        let status = response.status();

        // Handle "no results" response (204 No Content or empty result)
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(Vec::new());
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(PdbSyncError::SearchApi(format!(
                "API returned {}: {}",
                status, body
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| PdbSyncError::SearchApi(format!("Failed to read response: {}", e)))?;

        // Handle empty response body
        if body.is_empty() {
            return Ok(Vec::new());
        }

        let result: SearchResult = serde_json::from_str(&body).map_err(|e| {
            PdbSyncError::SearchApi(format!("Failed to parse response: {} - body: {}", e, body))
        })?;

        let pdb_ids: Vec<String> = result
            .result_set
            .into_iter()
            .map(|entry| entry.identifier.to_lowercase())
            .collect();

        tracing::debug!("Found {} entries since {}", pdb_ids.len(), since);

        Ok(pdb_ids)
    }

    /// Build the search query with date filter and optional additional filters
    fn build_query(&self, since: NaiveDate, filters: &SearchFilters) -> SearchQuery {
        let mut nodes = Vec::new();

        // Date filter: entries released since the given date
        nodes.push(QueryNode::TerminalNode {
            service: "text".to_string(),
            parameters: TerminalParameters {
                attribute: "rcsb_accession_info.initial_release_date".to_string(),
                operator: "greater".to_string(),
                value: Some(serde_json::json!(since.format("%Y-%m-%d").to_string())),
                negation: None,
            },
        });

        // Experimental method filter
        if let Some(method) = &filters.method {
            nodes.push(QueryNode::TerminalNode {
                service: "text".to_string(),
                parameters: TerminalParameters {
                    attribute: "exptl.method".to_string(),
                    operator: "exact_match".to_string(),
                    value: Some(serde_json::json!(method.as_rcsb_value())),
                    negation: None,
                },
            });
        }

        // Resolution filter (only applicable for X-ray and EM)
        if let Some(resolution) = filters.resolution {
            nodes.push(QueryNode::TerminalNode {
                service: "text".to_string(),
                parameters: TerminalParameters {
                    attribute: "rcsb_entry_info.resolution_combined".to_string(),
                    operator: "less_or_equal".to_string(),
                    value: Some(serde_json::json!(resolution)),
                    negation: None,
                },
            });
        }

        // Organism filter
        if let Some(organism) = &filters.organism {
            nodes.push(QueryNode::TerminalNode {
                service: "text".to_string(),
                parameters: TerminalParameters {
                    attribute: "rcsb_entity_source_organism.scientific_name".to_string(),
                    operator: "contains_words".to_string(),
                    value: Some(serde_json::json!(organism)),
                    negation: None,
                },
            });
        }

        // Combine all nodes with AND
        let query = if nodes.len() == 1 {
            nodes.remove(0)
        } else {
            QueryNode::GroupNode {
                logical_operator: "and".to_string(),
                nodes,
            }
        };

        SearchQuery {
            query,
            return_type: "entry".to_string(),
            request_options: RequestOptions {
                results_content_type: vec!["experimental".to_string()],
                return_all_hits: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_query_date_only() {
        let client = RcsbSearchClient::new().unwrap();
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let filters = SearchFilters::default();

        let query = client.build_query(since, &filters);
        let json = serde_json::to_string_pretty(&query).unwrap();

        assert!(json.contains("rcsb_accession_info.initial_release_date"));
        assert!(json.contains("2025-01-01"));
        assert!(json.contains("greater"));
    }

    #[test]
    fn test_build_query_with_method_filter() {
        let client = RcsbSearchClient::new().unwrap();
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let filters = SearchFilters {
            method: Some(ExperimentalMethod::Xray),
            ..Default::default()
        };

        let query = client.build_query(since, &filters);
        let json = serde_json::to_string_pretty(&query).unwrap();

        assert!(json.contains("exptl.method"));
        assert!(json.contains("X-RAY DIFFRACTION"));
    }

    #[test]
    fn test_build_query_with_resolution_filter() {
        let client = RcsbSearchClient::new().unwrap();
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let filters = SearchFilters {
            resolution: Some(2.0),
            ..Default::default()
        };

        let query = client.build_query(since, &filters);
        let json = serde_json::to_string_pretty(&query).unwrap();

        assert!(json.contains("resolution_combined"));
        assert!(json.contains("less_or_equal"));
        assert!(json.contains("2.0"));
    }

    #[test]
    fn test_build_query_with_organism_filter() {
        let client = RcsbSearchClient::new().unwrap();
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let filters = SearchFilters {
            organism: Some("Homo sapiens".to_string()),
            ..Default::default()
        };

        let query = client.build_query(since, &filters);
        let json = serde_json::to_string_pretty(&query).unwrap();

        assert!(json.contains("rcsb_entity_source_organism.scientific_name"));
        assert!(json.contains("Homo sapiens"));
    }

    #[test]
    fn test_build_query_combined_filters() {
        let client = RcsbSearchClient::new().unwrap();
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let filters = SearchFilters {
            method: Some(ExperimentalMethod::Em),
            resolution: Some(3.5),
            organism: Some("SARS-CoV-2".to_string()),
        };

        let query = client.build_query(since, &filters);
        let json = serde_json::to_string_pretty(&query).unwrap();

        // Should be a group node with AND
        assert!(json.contains("\"logical_operator\": \"and\""));
        assert!(json.contains("ELECTRON MICROSCOPY"));
        assert!(json.contains("3.5"));
        assert!(json.contains("SARS-CoV-2"));
    }

    // Network test - ignored by default
    #[tokio::test]
    #[ignore]
    async fn test_search_api_call() {
        let client = RcsbSearchClient::new().unwrap();
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let filters = SearchFilters::default();

        let result = client.search_new_entries(since, &filters).await;
        assert!(result.is_ok());

        let ids = result.unwrap();
        println!("Found {} entries since 2025-01-01", ids.len());
    }
}
