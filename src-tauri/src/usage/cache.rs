//! Caching layer for usage metrics to avoid API spam.

use std::time::{Duration, Instant};

use super::types::UsageMetrics;

/// Cache duration (5 minutes)
const CACHE_DURATION: Duration = Duration::from_secs(5 * 60);

/// Cache for usage metrics.
pub struct UsageCache {
    metrics: Option<UsageMetrics>,
    cached_at: Option<Instant>,
}

impl UsageCache {
    pub fn new() -> Self {
        Self {
            metrics: None,
            cached_at: None,
        }
    }

    /// Get cached metrics if still valid.
    pub fn get(&self) -> Option<&UsageMetrics> {
        match (&self.metrics, self.cached_at) {
            (Some(metrics), Some(cached_at)) => {
                if cached_at.elapsed() < CACHE_DURATION {
                    Some(metrics)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Get cached metrics regardless of freshness.
    pub fn get_stale(&self) -> Option<&UsageMetrics> {
        self.metrics.as_ref()
    }

    /// Update cached metrics.
    pub fn set(&mut self, metrics: UsageMetrics) {
        self.metrics = Some(metrics);
        self.cached_at = Some(Instant::now());
    }

    /// Check if cache is valid (not expired).
    pub fn is_valid(&self) -> bool {
        self.get().is_some()
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.metrics = None;
        self.cached_at = None;
    }
}

impl Default for UsageCache {
    fn default() -> Self {
        Self::new()
    }
}
