//! Data structures for OpenAI usage metrics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Aggregated usage metrics for display in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
    /// Cost in cents for the last 30 days
    pub cost_30d_cents: u64,
    /// Cost in cents for the last 7 days
    pub cost_7d_cents: u64,
    /// Cost in cents for the last 24 hours
    pub cost_24h_cents: u64,

    /// Audio seconds transcribed in the last 30 days
    pub seconds_30d: u64,
    /// Audio seconds transcribed in the last 7 days
    pub seconds_7d: u64,
    /// Audio seconds transcribed in the last 24 hours
    pub seconds_24h: u64,

    /// Number of API requests in the last 30 days
    pub requests_30d: u64,
    /// Number of API requests in the last 7 days
    pub requests_7d: u64,
    /// Number of API requests in the last 24 hours
    pub requests_24h: u64,

    /// When these metrics were last fetched
    pub last_updated: DateTime<Utc>,
}

impl Default for UsageMetrics {
    fn default() -> Self {
        Self {
            cost_30d_cents: 0,
            cost_7d_cents: 0,
            cost_24h_cents: 0,
            seconds_30d: 0,
            seconds_7d: 0,
            seconds_24h: 0,
            requests_30d: 0,
            requests_7d: 0,
            requests_24h: 0,
            last_updated: Utc::now(),
        }
    }
}

// ============================================================================
// OpenAI API Response Types
// ============================================================================

/// Response from /v1/organization/costs endpoint
#[derive(Debug, Deserialize)]
pub struct CostsResponse {
    pub object: String,
    pub data: Vec<CostBucket>,
}

#[derive(Debug, Deserialize)]
pub struct CostBucket {
    pub object: String,
    pub amount: CostAmount,
    pub line_item: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CostAmount {
    /// Cost in cents
    pub value: f64,
    pub currency: String,
}

/// Response from /v1/organization/usage/audio_transcriptions endpoint
#[derive(Debug, Deserialize)]
pub struct AudioUsageResponse {
    pub object: String,
    pub data: Vec<AudioUsageBucket>,
}

#[derive(Debug, Deserialize)]
pub struct AudioUsageBucket {
    pub object: String,
    /// Unix timestamp for start of bucket
    pub start_time: i64,
    /// Unix timestamp for end of bucket
    pub end_time: i64,
    pub results: Vec<AudioUsageResult>,
}

#[derive(Debug, Deserialize)]
pub struct AudioUsageResult {
    pub object: String,
    /// Number of seconds of audio transcribed
    pub seconds: u64,
    /// Number of requests
    pub num_model_requests: u64,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
    pub api_key_id: Option<String>,
    pub model: Option<String>,
}
