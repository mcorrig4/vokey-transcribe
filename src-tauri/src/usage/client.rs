//! OpenAI Usage API client.

use chrono::{Duration, Utc};

use super::types::{AudioUsageResponse, CostsResponse, UsageMetrics};

const OPENAI_BASE_URL: &str = "https://api.openai.com/v1/organization";

/// Fetch usage metrics from OpenAI API.
///
/// Returns aggregated metrics for 30d, 7d, and 24h periods.
/// Requires a valid Admin API key with usage read permissions.
pub async fn fetch_usage_metrics(admin_key: &str) -> Result<UsageMetrics, String> {
    let client = reqwest::Client::new();
    let now = Utc::now();

    // Calculate time boundaries
    let now_ts = now.timestamp();
    let day_ago = (now - Duration::days(1)).timestamp();
    let week_ago = (now - Duration::days(7)).timestamp();
    let month_ago = (now - Duration::days(30)).timestamp();

    // Fetch costs for all three periods in parallel
    let (cost_30d, cost_7d, cost_24h) = tokio::try_join!(
        fetch_costs(&client, admin_key, month_ago, now_ts),
        fetch_costs(&client, admin_key, week_ago, now_ts),
        fetch_costs(&client, admin_key, day_ago, now_ts),
    )?;

    // Fetch audio usage for all three periods in parallel
    let (audio_30d, audio_7d, audio_24h) = tokio::try_join!(
        fetch_audio_usage(&client, admin_key, month_ago, now_ts),
        fetch_audio_usage(&client, admin_key, week_ago, now_ts),
        fetch_audio_usage(&client, admin_key, day_ago, now_ts),
    )?;

    Ok(UsageMetrics {
        cost_30d_cents: cost_30d,
        cost_7d_cents: cost_7d,
        cost_24h_cents: cost_24h,
        seconds_30d: audio_30d.0,
        seconds_7d: audio_7d.0,
        seconds_24h: audio_24h.0,
        requests_30d: audio_30d.1,
        requests_7d: audio_7d.1,
        requests_24h: audio_24h.1,
        last_updated: now,
    })
}

/// Fetch costs from OpenAI API for a given time range.
/// Returns total cost in cents.
async fn fetch_costs(
    client: &reqwest::Client,
    admin_key: &str,
    start_time: i64,
    end_time: i64,
) -> Result<u64, String> {
    let url = format!(
        "{}/costs?start_time={}&end_time={}",
        OPENAI_BASE_URL, start_time, end_time
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", admin_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Network error fetching costs: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(match status.as_u16() {
            401 => "Invalid API key".to_string(),
            403 => "API key lacks usage read permission".to_string(),
            429 => "Rate limited - try again later".to_string(),
            _ => format!("API error {}: {}", status, body),
        });
    }

    let costs: CostsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse costs response: {}", e))?;

    // Sum all cost buckets and convert to cents
    let total_cents: u64 = costs
        .data
        .iter()
        .map(|bucket| (bucket.amount.value * 100.0).round() as u64)
        .sum();

    Ok(total_cents)
}

/// Fetch audio transcription usage from OpenAI API for a given time range.
/// Returns (total_seconds, total_requests).
async fn fetch_audio_usage(
    client: &reqwest::Client,
    admin_key: &str,
    start_time: i64,
    end_time: i64,
) -> Result<(u64, u64), String> {
    let url = format!(
        "{}/usage/audio_transcriptions?start_time={}&end_time={}",
        OPENAI_BASE_URL, start_time, end_time
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", admin_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Network error fetching audio usage: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(match status.as_u16() {
            401 => "Invalid API key".to_string(),
            403 => "API key lacks usage read permission".to_string(),
            429 => "Rate limited - try again later".to_string(),
            _ => format!("API error {}: {}", status, body),
        });
    }

    let usage: AudioUsageResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse audio usage response: {}", e))?;

    // Sum all buckets
    let mut total_seconds: u64 = 0;
    let mut total_requests: u64 = 0;

    for bucket in usage.data {
        for result in bucket.results {
            total_seconds += result.seconds;
            total_requests += result.num_model_requests;
        }
    }

    Ok((total_seconds, total_requests))
}
