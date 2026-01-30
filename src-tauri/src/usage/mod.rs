//! OpenAI Usage API integration for fetching usage metrics.
//!
//! This module provides:
//! - API client for OpenAI organization costs and usage endpoints
//! - Data structures for usage metrics
//! - Caching to avoid API spam
//!
//! Requires an OpenAI Admin API key with `api.usage.read` permission.

mod cache;
mod client;
mod types;

pub use cache::UsageCache;
pub use client::fetch_usage_metrics;
pub use types::UsageMetrics;
