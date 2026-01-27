//! Header caching for NNTP client
//!
//! This module provides in-memory caching for article metadata to reduce redundant network requests.
//! The cache stores XOVER entries (article metadata) indexed by article number within a newsgroup context.
//!
//! # Cache Strategy
//!
//! - **Scope**: Per-newsgroup (cleared when changing groups)
//! - **Index**: Article number (u64)
//! - **Eviction**: LRU (Least Recently Used)
//! - **Size Limit**: Configurable max entries
//!
//! # Example
//!
//! ```no_run
//! use nntp_rs::cache::{HeaderCache, LruHeaderCache};
//! use nntp_rs::XoverEntry;
//!
//! let mut cache = LruHeaderCache::new(1000); // Cache up to 1000 entries
//!
//! // Store article metadata
//! let entry = XoverEntry {
//!     article_number: 12345,
//!     subject: "Test Article".to_string(),
//!     author: "user@example.com".to_string(),
//!     date: "2024-01-01".to_string(),
//!     message_id: "<test@example.com>".to_string(),
//!     references: "".to_string(),
//!     bytes: 1024,
//!     lines: 50,
//! };
//! cache.put(12345, entry.clone());
//!
//! // Retrieve from cache
//! if let Some(cached) = cache.get(&12345) {
//!     println!("Found in cache: {}", cached.subject);
//! }
//!
//! // Clear cache when changing groups
//! cache.clear();
//! ```

use crate::XoverEntry;
use std::collections::HashMap;

/// Trait for header caching implementations
pub trait HeaderCache {
    /// Store an article's overview data
    fn put(&mut self, article_number: u64, entry: XoverEntry);

    /// Retrieve an article's overview data
    fn get(&mut self, article_number: &u64) -> Option<&XoverEntry>;

    /// Check if an article exists in cache
    fn contains(&self, article_number: &u64) -> bool;

    /// Remove an article from cache
    fn remove(&mut self, article_number: &u64) -> Option<XoverEntry>;

    /// Clear all cached entries
    fn clear(&mut self);

    /// Get the number of cached entries
    fn len(&self) -> usize;

    /// Check if cache is empty
    #[must_use]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the maximum cache size
    fn capacity(&self) -> usize;
}

/// LRU (Least Recently Used) cache for article metadata
///
/// Implements a simple LRU cache using a HashMap and access ordering.
/// When the cache is full, the least recently accessed entry is evicted.
///
/// # Example
///
/// ```
/// use nntp_rs::cache::{HeaderCache, LruHeaderCache};
/// use nntp_rs::XoverEntry;
///
/// let mut cache = LruHeaderCache::new(2); // Max 2 entries
///
/// let entry1 = XoverEntry {
///     article_number: 1,
///     subject: "First".to_string(),
///     author: "author1@example.com".to_string(),
///     date: "2024-01-01".to_string(),
///     message_id: "<1@example.com>".to_string(),
///     references: "".to_string(),
///     bytes: 100,
///     lines: 10,
/// };
///
/// let entry2 = XoverEntry {
///     article_number: 2,
///     subject: "Second".to_string(),
///     author: "author2@example.com".to_string(),
///     date: "2024-01-02".to_string(),
///     message_id: "<2@example.com>".to_string(),
///     references: "".to_string(),
///     bytes: 200,
///     lines: 20,
///     };
///
/// cache.put(1, entry1);
/// cache.put(2, entry2);
/// assert_eq!(cache.len(), 2);
///
/// // Access entry 1 to make it recently used
/// cache.get(&1);
///
/// // Adding a third entry will evict entry 2 (least recently used)
/// let entry3 = XoverEntry {
///     article_number: 3,
///     subject: "Third".to_string(),
///     author: "author3@example.com".to_string(),
///     date: "2024-01-03".to_string(),
///     message_id: "<3@example.com>".to_string(),
///     references: "".to_string(),
///     bytes: 300,
///     lines: 30,
/// };
/// cache.put(3, entry3);
///
/// assert_eq!(cache.len(), 2);
/// assert!(cache.contains(&1)); // Still cached
/// assert!(!cache.contains(&2)); // Evicted
/// assert!(cache.contains(&3)); // Newly added
/// ```
#[derive(Debug, Clone)]
pub struct LruHeaderCache {
    /// Maximum number of entries
    max_size: usize,
    /// Storage for cached entries
    entries: HashMap<u64, XoverEntry>,
    /// Access order tracking (article_number -> access_count)
    /// Higher access_count means more recently used
    access_order: HashMap<u64, u64>,
    /// Current access counter
    access_counter: u64,
}

impl LruHeaderCache {
    /// Create a new LRU cache with the specified maximum size
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum number of entries to cache (must be > 0)
    ///
    /// # Panics
    ///
    /// Panics if `max_size` is 0
    ///
    /// # Example
    ///
    /// ```
    /// use nntp_rs::cache::{LruHeaderCache, HeaderCache};
    ///
    /// let cache = LruHeaderCache::new(1000);
    /// assert_eq!(cache.capacity(), 1000);
    /// assert!(cache.is_empty());
    /// ```
    pub fn new(max_size: usize) -> Self {
        assert!(max_size > 0, "Cache size must be greater than 0");
        Self {
            max_size,
            entries: HashMap::new(),
            access_order: HashMap::new(),
            access_counter: 0,
        }
    }

    /// Evict the least recently used entry
    ///
    /// Returns the article number that was evicted, or None if cache is empty
    fn evict_lru(&mut self) -> Option<u64> {
        if self.entries.is_empty() {
            return None;
        }

        // Find the entry with the lowest access counter
        let lru_article = self
            .access_order
            .iter()
            .min_by_key(|(_, &access_count)| access_count)
            .map(|(&article_number, _)| article_number)?;

        self.entries.remove(&lru_article);
        self.access_order.remove(&lru_article);

        Some(lru_article)
    }

    /// Update access time for an entry
    fn touch(&mut self, article_number: &u64) {
        self.access_counter += 1;
        self.access_order
            .insert(*article_number, self.access_counter);
    }
}

impl HeaderCache for LruHeaderCache {
    fn put(&mut self, article_number: u64, entry: XoverEntry) {
        // If we're at capacity and this is a new entry, evict LRU
        if self.entries.len() >= self.max_size && !self.entries.contains_key(&article_number) {
            self.evict_lru();
        }

        // Insert or update the entry
        self.entries.insert(article_number, entry);
        self.touch(&article_number);
    }

    fn get(&mut self, article_number: &u64) -> Option<&XoverEntry> {
        if self.entries.contains_key(article_number) {
            self.touch(article_number);
            self.entries.get(article_number)
        } else {
            None
        }
    }

    fn contains(&self, article_number: &u64) -> bool {
        self.entries.contains_key(article_number)
    }

    fn remove(&mut self, article_number: &u64) -> Option<XoverEntry> {
        self.access_order.remove(article_number);
        self.entries.remove(article_number)
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
        self.access_counter = 0;
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn capacity(&self) -> usize {
        self.max_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(article_number: u64, subject: &str) -> XoverEntry {
        XoverEntry {
            article_number,
            subject: subject.to_string(),
            author: format!("author{}@example.com", article_number),
            date: format!("2024-01-{:02}", article_number % 31 + 1),
            message_id: format!("<{}@example.com>", article_number),
            references: String::new(),
            bytes: (article_number * 100) as usize,
            lines: (article_number * 10) as usize,
        }
    }

    #[test]
    fn test_cache_creation() {
        let cache = LruHeaderCache::new(100);
        assert_eq!(cache.capacity(), 100);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    #[should_panic(expected = "Cache size must be greater than 0")]
    fn test_cache_zero_size_panics() {
        LruHeaderCache::new(0);
    }

    #[test]
    fn test_put_and_get() {
        let mut cache = LruHeaderCache::new(10);
        let entry = create_test_entry(12345, "Test Article");

        cache.put(12345, entry.clone());
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());

        let retrieved = cache.get(&12345).unwrap();
        assert_eq!(retrieved.article_number, 12345);
        assert_eq!(retrieved.subject, "Test Article");
    }

    #[test]
    fn test_contains() {
        let mut cache = LruHeaderCache::new(10);
        let entry = create_test_entry(100, "Test");

        assert!(!cache.contains(&100));
        cache.put(100, entry);
        assert!(cache.contains(&100));
    }

    #[test]
    fn test_remove() {
        let mut cache = LruHeaderCache::new(10);
        let entry = create_test_entry(200, "Remove Me");

        cache.put(200, entry.clone());
        assert!(cache.contains(&200));

        let removed = cache.remove(&200).unwrap();
        assert_eq!(removed.article_number, 200);
        assert!(!cache.contains(&200));
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_clear() {
        let mut cache = LruHeaderCache::new(10);
        for i in 1..=5 {
            cache.put(i, create_test_entry(i, &format!("Article {}", i)));
        }
        assert_eq!(cache.len(), 5);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        for i in 1..=5 {
            assert!(!cache.contains(&i));
        }
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = LruHeaderCache::new(3);

        // Fill cache to capacity
        cache.put(1, create_test_entry(1, "First"));
        cache.put(2, create_test_entry(2, "Second"));
        cache.put(3, create_test_entry(3, "Third"));
        assert_eq!(cache.len(), 3);

        // Access entry 1 to make it recently used
        cache.get(&1);

        // Add a fourth entry - should evict entry 2 (least recently used)
        cache.put(4, create_test_entry(4, "Fourth"));
        assert_eq!(cache.len(), 3);
        assert!(cache.contains(&1)); // Still there (recently accessed)
        assert!(!cache.contains(&2)); // Evicted (least recently used)
        assert!(cache.contains(&3)); // Still there
        assert!(cache.contains(&4)); // Newly added
    }

    #[test]
    fn test_lru_eviction_order() {
        let mut cache = LruHeaderCache::new(2);

        cache.put(1, create_test_entry(1, "First"));
        cache.put(2, create_test_entry(2, "Second"));

        // Access 1 to make it more recent
        cache.get(&1);

        // Add 3, should evict 2
        cache.put(3, create_test_entry(3, "Third"));
        assert!(cache.contains(&1));
        assert!(!cache.contains(&2));
        assert!(cache.contains(&3));

        // Access 3
        cache.get(&3);

        // Add 4, should evict 1
        cache.put(4, create_test_entry(4, "Fourth"));
        assert!(!cache.contains(&1));
        assert!(cache.contains(&3));
        assert!(cache.contains(&4));
    }

    #[test]
    fn test_update_existing_entry() {
        let mut cache = LruHeaderCache::new(3);

        cache.put(1, create_test_entry(1, "Original"));
        cache.put(2, create_test_entry(2, "Second"));
        cache.put(3, create_test_entry(3, "Third"));

        // Update entry 1 (should not cause eviction)
        cache.put(1, create_test_entry(1, "Updated"));
        assert_eq!(cache.len(), 3);

        let entry = cache.get(&1).unwrap();
        assert_eq!(entry.subject, "Updated");
    }

    #[test]
    fn test_get_nonexistent() {
        let mut cache = LruHeaderCache::new(10);
        assert!(cache.get(&999).is_none());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut cache = LruHeaderCache::new(10);
        assert!(cache.remove(&999).is_none());
    }

    #[test]
    fn test_large_cache() {
        let mut cache = LruHeaderCache::new(1000);

        // Add 500 entries
        for i in 1..=500 {
            cache.put(i, create_test_entry(i, &format!("Article {}", i)));
        }
        assert_eq!(cache.len(), 500);

        // Verify all entries are retrievable
        for i in 1..=500 {
            assert!(cache.contains(&i));
            let entry = cache.get(&i).unwrap();
            assert_eq!(entry.article_number, i);
        }
    }

    #[test]
    fn test_single_entry_cache() {
        let mut cache = LruHeaderCache::new(1);

        cache.put(1, create_test_entry(1, "First"));
        assert_eq!(cache.len(), 1);

        // Adding second entry should evict first
        cache.put(2, create_test_entry(2, "Second"));
        assert_eq!(cache.len(), 1);
        assert!(!cache.contains(&1));
        assert!(cache.contains(&2));
    }

    #[test]
    fn test_high_churn() {
        let mut cache = LruHeaderCache::new(5);

        // Add 100 entries with only 5 slots
        for i in 1..=100 {
            cache.put(i, create_test_entry(i, &format!("Article {}", i)));
            assert!(cache.len() <= 5);
        }

        // Only the last 5 should remain (96-100)
        assert_eq!(cache.len(), 5);
        for i in 96..=100 {
            assert!(cache.contains(&i));
        }
        for i in 1..=95 {
            assert!(!cache.contains(&i));
        }
    }

    #[test]
    fn test_access_pattern_preservation() {
        let mut cache = LruHeaderCache::new(3);

        cache.put(1, create_test_entry(1, "First"));
        cache.put(2, create_test_entry(2, "Second"));
        cache.put(3, create_test_entry(3, "Third"));

        // Keep accessing 1 and 2
        for _ in 0..5 {
            cache.get(&1);
            cache.get(&2);
        }

        // Add new entries - 3 should be evicted first since it's LRU
        cache.put(4, create_test_entry(4, "Fourth"));
        assert!(cache.contains(&1));
        assert!(cache.contains(&2));
        assert!(!cache.contains(&3));
        assert!(cache.contains(&4));

        cache.put(5, create_test_entry(5, "Fifth"));
        assert!(cache.contains(&1) || cache.contains(&2)); // At least one is still there
        assert!(!cache.contains(&3));
    }
}
