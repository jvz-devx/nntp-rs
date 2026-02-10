//! Extended Listing & Metadata Live Integration Tests
//!
//! Tests for listing commands that are not covered by the rfc_commands suite:
//! `list_active_times`, `list_counts`, `list_distributions`, `list_moderators`,
//! `list_motd`, `list_subscriptions`, and `fetch_xover`.
//!
//! Many servers don't support all LIST variants — tests gracefully handle
//! 503 (not supported) responses by printing a skip message.

#![cfg(feature = "live-tests")]

use nntp_rs::{NntpClient, NntpError};
use std::sync::Arc;

use super::{get_test_config, get_test_group};

/// Helper to check if an error is a "not supported" 503 response
fn is_not_supported(err: &NntpError) -> bool {
    matches!(err, NntpError::Protocol { code: 503, .. })
}

#[tokio::test]
async fn test_list_active_times() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    match client.list_active_times("*").await {
        Ok(times) => {
            println!("LIST ACTIVE.TIMES returned {} entries", times.len());
        }
        Err(ref e) if is_not_supported(e) => {
            println!("LIST ACTIVE.TIMES not supported by server (503), skipping");
        }
        Err(e) => panic!("Unexpected error from LIST ACTIVE.TIMES: {:?}", e),
    }
}

#[tokio::test]
async fn test_list_counts() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    match client.list_counts("*").await {
        Ok(counts) => {
            println!("LIST COUNTS returned {} entries", counts.len());
            if let Some(first) = counts.first() {
                println!(
                    "  First entry: name={}, count={}, low={}, high={}, status={}",
                    first.name, first.count, first.low, first.high, first.status
                );
            }
        }
        Err(ref e) if is_not_supported(e) => {
            println!("LIST COUNTS not supported by server (503), skipping");
        }
        Err(e) => panic!("Unexpected error from LIST COUNTS: {:?}", e),
    }
}

#[tokio::test]
async fn test_list_distributions() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    match client.list_distributions().await {
        Ok(dists) => {
            println!("LIST DISTRIBUTIONS returned {} entries", dists.len());
        }
        Err(ref e) if is_not_supported(e) => {
            println!("LIST DISTRIBUTIONS not supported by server (503), skipping");
        }
        Err(e) => panic!("Unexpected error from LIST DISTRIBUTIONS: {:?}", e),
    }
}

#[tokio::test]
async fn test_list_moderators() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    match client.list_moderators().await {
        Ok(mods) => {
            println!("LIST MODERATORS returned {} entries", mods.len());
        }
        Err(ref e) if is_not_supported(e) => {
            println!("LIST MODERATORS not supported by server (503), skipping");
        }
        Err(e) => panic!("Unexpected error from LIST MODERATORS: {:?}", e),
    }
}

#[tokio::test]
async fn test_list_motd() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    match client.list_motd().await {
        Ok(motd) => {
            println!("LIST MOTD returned {} lines", motd.len());
            for line in &motd {
                println!("  MOTD: {}", line);
            }
        }
        Err(ref e) if is_not_supported(e) => {
            println!("LIST MOTD not supported by server (503), skipping");
        }
        Err(e) => panic!("Unexpected error from LIST MOTD: {:?}", e),
    }
}

#[tokio::test]
async fn test_list_subscriptions() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    match client.list_subscriptions().await {
        Ok(subs) => {
            println!("LIST SUBSCRIPTIONS returned {} entries", subs.len());
        }
        Err(ref e) if is_not_supported(e) => {
            println!("LIST SUBSCRIPTIONS not supported by server (503), skipping");
        }
        Err(e) => panic!("Unexpected error from LIST SUBSCRIPTIONS: {:?}", e),
    }
}

#[tokio::test]
async fn test_fetch_xover() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let info = client.select_group(&group).await.unwrap();

    if info.count == 0 {
        println!("Group {} is empty, skipping XOVER test", group);
        return;
    }

    let start = if info.last > 10 {
        info.last - 10
    } else {
        info.first
    };
    let range = format!("{}-{}", start, info.last);

    match client.fetch_xover(&range).await {
        Ok(entries) => {
            println!("XOVER {} returned {} entries", range, entries.len());

            if let Some(first) = entries.first() {
                println!(
                    "  First XOVER entry: article_num={}, subject={}, author={}",
                    first.article_number, first.subject, first.author
                );
                assert!(first.article_number >= start);
                assert!(!first.subject.is_empty());
                assert!(!first.message_id.is_empty());
            }

            // Verify XOVER returns data consistent with OVER
            let over_entries = client.over(&range).await.unwrap();
            println!(
                "  OVER returned {} entries for same range",
                over_entries.len()
            );

            // Both should return the same number of entries
            assert_eq!(
                entries.len(),
                over_entries.len(),
                "XOVER and OVER should return same number of entries"
            );
        }
        Err(ref e) if is_not_supported(e) => {
            println!("XOVER not supported by server (503), skipping");
        }
        Err(e) => panic!("Unexpected error from XOVER: {:?}", e),
    }
}
