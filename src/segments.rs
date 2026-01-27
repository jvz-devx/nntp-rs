//! Segment fetcher for Usenet binary downloads
//!
//! This module provides functionality to fetch NZB segments with retry logic,
//! progress tracking, and priority queue support.

use crate::error::{NntpError, Result};
use crate::nzb::NzbSegment;
use crate::NntpClient;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Status of a segment fetch operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentStatus {
    /// Segment is waiting to be fetched
    Pending,
    /// Segment is currently being fetched
    InProgress,
    /// Segment was successfully fetched
    Completed,
    /// Segment failed to fetch after all retries
    Failed,
    /// Segment was not found on the server (430 error)
    NotFound,
}

/// Information about a segment fetch result
#[derive(Debug, Clone)]
pub struct SegmentFetchResult {
    /// Index of the segment in the original segments slice
    pub segment_index: usize,
    /// Status of the fetch operation
    pub status: SegmentStatus,
    /// The article content (if successful)
    pub content: Option<Vec<String>>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Progress information for segment fetching
#[derive(Debug, Clone)]
pub struct FetchProgress {
    /// Total number of segments
    pub total_segments: usize,
    /// Number of segments completed
    pub completed_segments: usize,
    /// Number of segments failed
    pub failed_segments: usize,
    /// Number of segments not found
    pub not_found_segments: usize,
    /// Total bytes expected
    pub total_bytes: u64,
    /// Bytes downloaded so far
    pub downloaded_bytes: u64,
}

impl FetchProgress {
    /// Create a new progress tracker
    pub fn new(total_segments: usize, total_bytes: u64) -> Self {
        Self {
            total_segments,
            completed_segments: 0,
            failed_segments: 0,
            not_found_segments: 0,
            total_bytes,
            downloaded_bytes: 0,
        }
    }

    /// Check if all segments are processed (completed, failed, or not found)
    pub fn is_complete(&self) -> bool {
        self.completed_segments + self.failed_segments + self.not_found_segments
            >= self.total_segments
    }

    /// Get the percentage of bytes downloaded (0-100)
    #[must_use]
    pub fn percent_complete(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.downloaded_bytes as f64 / self.total_bytes as f64) * 100.0
    }

    /// Get the percentage of segments completed (0-100)
    pub fn segment_percent_complete(&self) -> f64 {
        if self.total_segments == 0 {
            return 0.0;
        }
        (self.completed_segments as f64 / self.total_segments as f64) * 100.0
    }
}

/// Configuration for segment fetching
#[derive(Debug, Clone)]
pub struct FetchConfig {
    /// Maximum number of retry attempts for failed fetches
    pub max_retries: usize,
    /// Whether to skip segments that are not found (430 error)
    /// If true, NotFound segments will not cause an error
    pub skip_not_found: bool,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            skip_not_found: false,
        }
    }
}

/// Segment fetcher for downloading NZB segments
///
/// # Example
///
/// ```no_run
/// use nntp_rs::{NntpClient, ServerConfig, SegmentFetcher, FetchConfig};
/// use nntp_rs::nzb::NzbSegment;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ServerConfig::tls("news.example.com", "user", "pass");
/// let mut client = NntpClient::connect(Arc::new(config)).await?;
/// client.authenticate().await?;
///
/// let segments = vec![
///     NzbSegment {
///         bytes: 768000,
///         number: 1,
///         message_id: "<part1@example.com>".to_string(),
///     },
/// ];
///
/// let fetcher = SegmentFetcher::new(client, FetchConfig::default());
/// let results = fetcher.fetch_segments(&segments).await?;
/// # Ok(())
/// # }
/// ```
pub struct SegmentFetcher {
    client: Arc<Mutex<NntpClient>>,
    config: FetchConfig,
    progress: Arc<Mutex<FetchProgress>>,
}

impl SegmentFetcher {
    /// Create a new segment fetcher with the given client and configuration
    pub fn new(client: NntpClient, config: FetchConfig) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            config,
            progress: Arc::new(Mutex::new(FetchProgress::new(0, 0))),
        }
    }

    /// Get the current progress
    pub async fn progress(&self) -> FetchProgress {
        self.progress.lock().await.clone()
    }

    /// Fetch a single segment with retry logic
    ///
    /// Returns a `SegmentFetchResult` with the status and content (if successful).
    /// The `segment_index` parameter is stored in the result for mapping back to
    /// the original segments slice.
    pub async fn fetch_segment(
        &self,
        segment: &NzbSegment,
        segment_index: usize,
    ) -> SegmentFetchResult {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                debug!(
                    "Retry attempt {} for segment {} ({})",
                    attempt, segment.number, segment.message_id
                );
            }

            let result = {
                let mut client = self.client.lock().await;
                client.fetch_article(&segment.message_id).await
            };
            match result {
                Ok(response) => {
                    debug!(
                        "Successfully fetched segment {} ({} bytes)",
                        segment.number, segment.bytes
                    );

                    // Update progress
                    let mut progress = self.progress.lock().await;
                    progress.completed_segments += 1;
                    progress.downloaded_bytes += segment.bytes;
                    drop(progress);

                    return SegmentFetchResult {
                        segment_index,
                        status: SegmentStatus::Completed,
                        content: Some(response.lines),
                        error: None,
                    };
                }
                Err(NntpError::NoSuchArticle(_)) => {
                    warn!(
                        "Segment {} not found: {}",
                        segment.number, segment.message_id
                    );

                    // Update progress
                    let mut progress = self.progress.lock().await;
                    progress.not_found_segments += 1;
                    drop(progress);

                    return SegmentFetchResult {
                        segment_index,
                        status: SegmentStatus::NotFound,
                        content: None,
                        error: Some(format!("Article not found: {}", segment.message_id)),
                    };
                }
                Err(e) => {
                    warn!(
                        "Error fetching segment {} (attempt {}): {}",
                        segment.number,
                        attempt + 1,
                        e
                    );
                    last_error = Some(e);
                }
            }

            // Don't sleep after the last attempt
            if attempt < self.config.max_retries {
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    100 * (attempt as u64 + 1),
                ))
                .await;
            }
        }

        // All retries failed
        let error_msg = last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Unknown error".to_string());

        warn!(
            "Segment {} failed after {} retries: {}",
            segment.number,
            self.config.max_retries + 1,
            error_msg
        );

        // Update progress
        let mut progress = self.progress.lock().await;
        progress.failed_segments += 1;
        drop(progress);

        SegmentFetchResult {
            segment_index,
            status: SegmentStatus::Failed,
            content: None,
            error: Some(error_msg),
        }
    }

    /// Fetch multiple segments in order
    ///
    /// Returns a vector of `SegmentFetchResult` for each segment.
    ///
    /// # Errors
    ///
    /// Returns an error if any segment fails and `skip_not_found` is false,
    /// or if required segments cannot be fetched.
    pub async fn fetch_segments(&self, segments: &[NzbSegment]) -> Result<Vec<SegmentFetchResult>> {
        // Initialize progress
        let total_bytes: u64 = segments.iter().map(|s| s.bytes).sum();
        {
            let mut progress = self.progress.lock().await;
            *progress = FetchProgress::new(segments.len(), total_bytes);
        }

        let mut results = Vec::with_capacity(segments.len());

        for (idx, segment) in segments.iter().enumerate() {
            let result = self.fetch_segment(segment, idx).await;

            // Check if we should fail early
            if !self.config.skip_not_found && result.status == SegmentStatus::NotFound {
                return Err(NntpError::NoSuchArticle(segment.message_id.clone()));
            }

            if result.status == SegmentStatus::Failed {
                return Err(NntpError::Other(format!(
                    "Failed to fetch segment {}: {}",
                    segment.number,
                    result.error.as_deref().unwrap_or("unknown error")
                )));
            }

            results.push(result);
        }

        Ok(results)
    }

    /// Fetch segments with a priority order
    ///
    /// Segments are fetched in the order specified by `priority_indices`.
    /// This is useful for fetching PAR2 headers first, or prioritizing certain files.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nntp_rs::{SegmentFetcher, FetchConfig};
    /// # use nntp_rs::nzb::NzbSegment;
    /// # async fn example(fetcher: SegmentFetcher, segments: Vec<NzbSegment>) -> Result<(), Box<dyn std::error::Error>> {
    /// // Fetch segments 0, 2, 1 in that order
    /// let results = fetcher.fetch_segments_prioritized(&segments, &[0, 2, 1]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_segments_prioritized(
        &self,
        segments: &[NzbSegment],
        priority_indices: &[usize],
    ) -> Result<Vec<SegmentFetchResult>> {
        // Initialize progress
        let total_bytes: u64 = segments.iter().map(|s| s.bytes).sum();
        {
            let mut progress = self.progress.lock().await;
            *progress = FetchProgress::new(segments.len(), total_bytes);
        }

        // Create a result vector with the same size, initially empty
        let mut results: Vec<Option<SegmentFetchResult>> = vec![None; segments.len()];

        // Fetch in priority order
        for &idx in priority_indices {
            if idx >= segments.len() {
                return Err(NntpError::Other(format!(
                    "Invalid priority index: {} (segments length: {})",
                    idx,
                    segments.len()
                )));
            }

            let segment = &segments[idx];
            let result = self.fetch_segment(segment, idx).await;

            // Check if we should fail early
            if !self.config.skip_not_found && result.status == SegmentStatus::NotFound {
                return Err(NntpError::NoSuchArticle(segment.message_id.clone()));
            }

            if result.status == SegmentStatus::Failed {
                return Err(NntpError::Other(format!(
                    "Failed to fetch segment {}: {}",
                    segment.number,
                    result.error.as_deref().unwrap_or("unknown error")
                )));
            }

            results[idx] = Some(result);
        }

        // Fetch remaining segments in order
        for (idx, segment) in segments.iter().enumerate() {
            if results[idx].is_some() {
                continue; // Already fetched
            }

            let result = self.fetch_segment(segment, idx).await;

            // Check if we should fail early
            if !self.config.skip_not_found && result.status == SegmentStatus::NotFound {
                return Err(NntpError::NoSuchArticle(segment.message_id.clone()));
            }

            if result.status == SegmentStatus::Failed {
                return Err(NntpError::Other(format!(
                    "Failed to fetch segment {}: {}",
                    segment.number,
                    result.error.as_deref().unwrap_or("unknown error")
                )));
            }

            results[idx] = Some(result);
        }

        // All results must be Some: every index 0..segments.len() is visited exactly once
        // in the two loops above (priority order, then remaining segments)
        #[allow(clippy::expect_used)]
        {
            Ok(results
                .into_iter()
                .map(|r| {
                    r.expect("BUG: result slot not filled - every segment index should be visited exactly once")
                })
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_progress_new() {
        let progress = FetchProgress::new(10, 1000);
        assert_eq!(progress.total_segments, 10);
        assert_eq!(progress.total_bytes, 1000);
        assert_eq!(progress.completed_segments, 0);
        assert_eq!(progress.failed_segments, 0);
        assert_eq!(progress.not_found_segments, 0);
        assert_eq!(progress.downloaded_bytes, 0);
    }

    #[test]
    fn test_fetch_progress_is_complete() {
        let mut progress = FetchProgress::new(10, 1000);
        assert!(!progress.is_complete());

        progress.completed_segments = 8;
        progress.failed_segments = 1;
        progress.not_found_segments = 1;
        assert!(progress.is_complete());

        progress.completed_segments = 10;
        assert!(progress.is_complete());
    }

    #[test]
    fn test_fetch_progress_percent_complete() {
        let mut progress = FetchProgress::new(10, 1000);
        assert_eq!(progress.percent_complete(), 0.0);

        progress.downloaded_bytes = 500;
        assert_eq!(progress.percent_complete(), 50.0);

        progress.downloaded_bytes = 1000;
        assert_eq!(progress.percent_complete(), 100.0);
    }

    #[test]
    fn test_fetch_progress_segment_percent_complete() {
        let mut progress = FetchProgress::new(10, 1000);
        assert_eq!(progress.segment_percent_complete(), 0.0);

        progress.completed_segments = 5;
        assert_eq!(progress.segment_percent_complete(), 50.0);

        progress.completed_segments = 10;
        assert_eq!(progress.segment_percent_complete(), 100.0);
    }

    #[test]
    fn test_fetch_progress_zero_total() {
        let progress = FetchProgress::new(0, 0);
        assert_eq!(progress.percent_complete(), 0.0);
        assert_eq!(progress.segment_percent_complete(), 0.0);
        assert!(progress.is_complete());
    }

    #[test]
    fn test_fetch_config_default() {
        let config = FetchConfig::default();
        assert_eq!(config.max_retries, 3);
        assert!(!config.skip_not_found);
    }

    #[test]
    fn test_segment_status_equality() {
        assert_eq!(SegmentStatus::Pending, SegmentStatus::Pending);
        assert_eq!(SegmentStatus::Completed, SegmentStatus::Completed);
        assert_ne!(SegmentStatus::Pending, SegmentStatus::Completed);
    }

    #[test]
    fn test_segment_fetch_result_clone() {
        let result = SegmentFetchResult {
            segment_index: 0,
            status: SegmentStatus::Completed,
            content: Some(vec!["line1".to_string(), "line2".to_string()]),
            error: None,
        };

        let cloned = result.clone();
        assert_eq!(cloned.segment_index, result.segment_index);
        assert_eq!(cloned.status, result.status);
        assert_eq!(cloned.content, result.content);
    }
}
