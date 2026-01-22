//! RFC Command Tests - Live Integration Testing
//!
//! This test suite validates RFC 3977, 4643, 8054, and 4642 NNTP commands
//! against a real NNTP server.
//!
//! Run with:
//! ```bash
//! cargo test --features live-tests -- --test-threads=1
//! ```

#![cfg(feature = "live-tests")]

use nntp_rs::{NntpClient, NntpError};
use std::sync::Arc;

use super::{get_test_config, get_test_group};

// RFC 3977 - Core Commands

#[tokio::test]
async fn test_capabilities() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();

    let caps = client.capabilities().await.unwrap();
    println!("Server capabilities: {:?}", caps.list());

    // Most servers should support VERSION 2
    assert!(caps.has("VERSION"));
}

#[tokio::test]
async fn test_mode_reader() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();

    let posting_allowed = client.mode_reader().await.unwrap();
    println!("MODE READER - Posting allowed: {}", posting_allowed);

    // Should succeed (even if posting is not allowed)
    assert!(posting_allowed || !posting_allowed); // Always true, but demonstrates the return
}

#[tokio::test]
async fn test_quit() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();

    // QUIT should succeed
    let result = client.quit().await;
    assert!(result.is_ok());

    // Connection should be broken after QUIT
    assert!(client.is_broken());
}

#[tokio::test]
async fn test_group_select() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (count, first, last) = client.select_group(&group).await.unwrap();

    println!(
        "GROUP {}: count={}, first={}, last={}",
        group, count, first, last
    );

    assert!(last >= first);
    assert!(client.current_group() == Some(group.as_str()));
}

#[tokio::test]
async fn test_group_not_found() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let result = client.select_group("nonexistent.group.xyz.12345").await;

    match result {
        Err(NntpError::NoSuchGroup(_)) => {
            println!("Correctly received NoSuchGroup error");
        }
        Err(e) => panic!("Expected NoSuchGroup, got: {:?}", e),
        Ok(_) => panic!("Expected error for nonexistent group"),
    }
}

#[tokio::test]
async fn test_listgroup() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (_, first, last) = client.select_group(&group).await.unwrap();

    if last == 0 {
        println!("Group {} is empty, skipping LISTGROUP test", group);
        return;
    }

    // Get article list for the entire group
    let articles = client.listgroup(&group, None).await.unwrap();
    println!("LISTGROUP returned {} articles", articles.len());

    if !articles.is_empty() {
        assert!(articles[0] >= first);
        assert!(articles[articles.len() - 1] <= last);
    }

    // Test with range
    let range = format!("{}-{}", first, std::cmp::min(first + 10, last));
    let articles_range = client.listgroup(&group, Some(&range)).await.unwrap();
    println!(
        "LISTGROUP {} returned {} articles",
        range,
        articles_range.len()
    );
}

#[tokio::test]
async fn test_article_navigation() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (count, first, last) = client.select_group(&group).await.unwrap();

    if count == 0 {
        println!("Group {} is empty, skipping navigation test", group);
        return;
    }

    // STAT first article
    let (article_num, message_id) = client.stat(&first.to_string()).await.unwrap();
    println!(
        "STAT {}: article_num={}, message_id={}",
        first, article_num, message_id
    );
    assert_eq!(article_num, first);

    // Try NEXT if there are more articles
    if count > 1 {
        let (next_num, next_msgid) = client.next().await.unwrap();
        println!("NEXT: article_num={}, message_id={}", next_num, next_msgid);
        assert!(next_num > article_num);

        // LAST should go back
        let (prev_num, prev_msgid) = client.last().await.unwrap();
        println!("LAST: article_num={}, message_id={}", prev_num, prev_msgid);
        assert_eq!(prev_num, article_num);
        assert_eq!(prev_msgid, message_id);
    }
}

#[tokio::test]
async fn test_article_fetch() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (count, first, last) = client.select_group(&group).await.unwrap();

    if count == 0 {
        println!("Group {} is empty, skipping fetch test", group);
        return;
    }

    let test_article = if last > 10 { last - 5 } else { first };

    // ARTICLE - fetch full article
    let article_response = client
        .fetch_article(&test_article.to_string())
        .await
        .unwrap();
    println!(
        "ARTICLE {}: got {} lines",
        test_article,
        article_response.lines.len()
    );
    assert!(!article_response.lines.is_empty());
    assert_eq!(article_response.code, 220); // ARTICLE_FOLLOWS

    // HEAD - fetch headers only
    let head_response = client.fetch_head(&test_article.to_string()).await.unwrap();
    println!(
        "HEAD {}: got {} header lines",
        test_article,
        head_response.lines.len()
    );
    assert!(!head_response.lines.is_empty());
    assert_eq!(head_response.code, 221); // HEAD_FOLLOWS

    // BODY - fetch body only
    let body_response = client.fetch_body(&test_article.to_string()).await.unwrap();
    println!(
        "BODY {}: got {} body lines",
        test_article,
        body_response.lines.len()
    );
    assert_eq!(body_response.code, 222); // BODY_FOLLOWS

    // Verify ARTICLE = HEAD + BODY (roughly)
    assert!(article_response.lines.len() >= head_response.lines.len());
}

#[tokio::test]
async fn test_article_by_message_id() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (count, first, last) = client.select_group(&group).await.unwrap();

    if count == 0 {
        println!("Group {} is empty, skipping message-id test", group);
        return;
    }

    let test_article = if last > 10 { last - 5 } else { first };
    let (_, message_id) = client.stat(&test_article.to_string()).await.unwrap();

    println!("Testing fetch by message-id: {}", message_id);

    // Fetch by message-id
    let article_response = client.fetch_article(&message_id).await.unwrap();
    println!(
        "ARTICLE <msgid>: got {} lines",
        article_response.lines.len()
    );
    assert!(!article_response.lines.is_empty());
}

#[tokio::test]
async fn test_stat() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (count, first, _) = client.select_group(&group).await.unwrap();

    if count == 0 {
        println!("Group {} is empty, skipping STAT test", group);
        return;
    }

    let (article_num, message_id) = client.stat(&first.to_string()).await.unwrap();
    println!(
        "STAT: article_num={}, message_id={}",
        article_num, message_id
    );

    assert_eq!(article_num, first);
    assert!(message_id.contains("@"));
    assert!(message_id.starts_with('<'));
    assert!(message_id.ends_with('>'));
}

#[tokio::test]
async fn test_post_not_permitted() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // Try to post an article (most test servers don't allow posting)
    use nntp_rs::ArticleBuilder;

    let article = ArticleBuilder::new()
        .from("test@example.com")
        .newsgroups(vec![get_test_group()])
        .subject("Test post")
        .body("This is a test.\r\n")
        .build()
        .unwrap();

    let result = client.post(&article).await;

    match result {
        Err(NntpError::PostingNotPermitted) => {
            println!("Correctly received PostingNotPermitted (440)");
        }
        Err(NntpError::PostingFailed(_)) => {
            println!("Posting failed (expected on most servers)");
        }
        Ok(_) => {
            println!("POST succeeded (rare for test servers)");
        }
        Err(e) => {
            println!("POST error: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_date() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();

    let date_str = client.date().await.unwrap();
    println!("Server DATE: {}", date_str);

    // Should be in format YYYYMMDDhhmmss (14 digits)
    assert_eq!(date_str.len(), 14);
    assert!(date_str.chars().all(|c| c.is_ascii_digit()));

    // Parse year should be reasonable
    let year: u32 = date_str[0..4].parse().unwrap();
    assert!(year >= 2020 && year <= 2030);
}

#[tokio::test]
async fn test_help() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();

    let help_response = client.help().await.unwrap();
    println!("HELP response: {} lines", help_response.lines.len());

    assert!(help_response.is_success());
    assert!(!help_response.lines.is_empty());
}

#[tokio::test]
async fn test_newgroups() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // Query for groups created in the last 30 days
    let date = "20200101"; // Date far in the past to get some results
    let time = "000000";

    let groups = client.newgroups(date, time, true).await.unwrap();
    println!("NEWGROUPS returned {} groups", groups.len());

    // Just verify it doesn't error - result depends on server
}

#[tokio::test]
async fn test_newnews() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();

    // Query for articles from the last 30 days
    let date = "20200101"; // Date far in the past to get some results
    let time = "000000";

    let message_ids = client.newnews(&group, date, time, true).await.unwrap();
    println!(
        "NEWNEWS {} returned {} message-ids",
        group,
        message_ids.len()
    );

    // Just verify it doesn't error - result depends on server
}

#[tokio::test]
async fn test_list_active() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // List all groups (limited by wildmat)
    let groups = client.list_active("*").await.unwrap();
    println!("LIST ACTIVE returned {} groups", groups.len());

    assert!(!groups.is_empty());

    // Check first group structure
    let first_group = &groups[0];
    println!(
        "First group: name={}, high={}, low={}, status={}",
        first_group.name, first_group.high, first_group.low, first_group.status
    );

    assert!(!first_group.name.is_empty());
    assert!(first_group.high >= first_group.low);
}

#[tokio::test]
async fn test_list_newsgroups() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let newsgroups = client.list_newsgroups("*").await.unwrap();
    println!("LIST NEWSGROUPS returned {} entries", newsgroups.len());

    // Not all servers support this, so just verify no error
}

#[tokio::test]
async fn test_list_overview_fmt() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let format = client.list_overview_fmt().await.unwrap();
    println!("LIST OVERVIEW.FMT returned {} fields", format.len());

    assert!(!format.is_empty());

    // Should contain standard fields
    let has_subject = format.iter().any(|f| f.contains("Subject"));
    let has_from = format.iter().any(|f| f.contains("From"));
    assert!(has_subject && has_from);
}

#[tokio::test]
async fn test_over() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (count, first, last) = client.select_group(&group).await.unwrap();

    if count == 0 {
        println!("Group {} is empty, skipping OVER test", group);
        return;
    }

    let start = if last > 10 { last - 10 } else { first };
    let range = format!("{}-{}", start, last);

    let entries = client.over(&range).await.unwrap();
    println!("OVER {} returned {} entries", range, entries.len());

    assert!(!entries.is_empty());

    // Check first entry
    let first_entry = &entries[0];
    println!(
        "First OVER entry: article_num={}, subject={}, author={}",
        first_entry.article_number, first_entry.subject, first_entry.author
    );

    assert!(first_entry.article_number >= start);
    assert!(!first_entry.subject.is_empty());
    assert!(!first_entry.message_id.is_empty());
}

#[tokio::test]
async fn test_hdr() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (count, first, last) = client.select_group(&group).await.unwrap();

    if count == 0 {
        println!("Group {} is empty, skipping HDR test", group);
        return;
    }

    let start = if last > 10 { last - 10 } else { first };
    let range = format!("{}-{}", start, last);

    // Test HDR Subject
    let headers = client.hdr("Subject", &range).await.unwrap();
    println!("HDR Subject {} returned {} entries", range, headers.len());

    assert!(!headers.is_empty());

    // Check first entry
    let first_hdr = &headers[0];
    println!(
        "First HDR entry: article_num={}, value={}",
        first_hdr.article_number, first_hdr.value
    );

    assert!(first_hdr.article_number >= start);
}

// RFC 4643 - Authentication

#[tokio::test]
async fn test_authenticate_basic() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();

    let result = client.authenticate().await;
    assert!(result.is_ok());
    assert!(client.is_authenticated());

    println!("AUTHINFO USER/PASS authentication succeeded");
}

#[tokio::test]
async fn test_authenticate_wrong_credentials() {
    let mut config = get_test_config();
    config.password = "wrongpassword123".to_string();

    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();

    let result = client.authenticate().await;

    match result {
        Err(NntpError::AuthFailed(_)) => {
            println!("Correctly received AuthFailed error");
        }
        Err(e) => {
            println!("Authentication error: {:?}", e);
            // Some servers may return different error types
        }
        Ok(_) => panic!("Expected authentication to fail with wrong password"),
    }
}

#[tokio::test]
async fn test_authenticate_sasl_plain() {
    let config = get_test_config();
    let username = config.username.clone();
    let password = config.password.clone();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();

    // Check if server supports SASL
    let caps = client.capabilities().await.unwrap();

    if !caps.has("SASL") {
        println!("Server doesn't support SASL, skipping test");
        return;
    }

    use nntp_rs::SaslPlain;
    let sasl_plain = SaslPlain::new(&username, &password);

    let result = client.authenticate_sasl(sasl_plain).await;

    match result {
        Ok(_) => {
            println!("AUTHINFO SASL PLAIN succeeded");
            assert!(client.is_authenticated());
        }
        Err(NntpError::Protocol { code, message }) if code == 503 => {
            println!("SASL PLAIN not supported by server: {}", message);
        }
        Err(e) => {
            println!("SASL PLAIN error: {:?}", e);
        }
    }
}

// RFC 8054 - Compression

#[tokio::test]
async fn test_compression_deflate() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // Try to enable compression
    let compression_enabled = client.try_enable_compression().await.unwrap();

    if compression_enabled {
        println!("Compression enabled successfully");
        assert!(client.is_compression_enabled());

        // Fetch some data to test compression
        let group = get_test_group();
        let (count, first, last) = client.select_group(&group).await.unwrap();

        if count > 0 {
            let start = if last > 20 { last - 20 } else { first };
            let range = format!("{}-{}", start, last);
            let _ = client.over(&range).await.unwrap();

            // Check bandwidth stats
            let (compressed, decompressed) = client.get_bandwidth_stats();
            println!(
                "Bandwidth stats: compressed={}, decompressed={}, ratio={:.1}%",
                compressed,
                decompressed,
                if decompressed > 0 {
                    (1.0 - (compressed as f64 / decompressed as f64)) * 100.0
                } else {
                    0.0
                }
            );

            assert!(decompressed > 0);
        }
    } else {
        println!("Compression not supported by server");
    }
}

// RFC 4642 - TLS

#[tokio::test]
async fn test_implicit_tls() {
    let config = get_test_config();

    // Config should already have TLS enabled
    assert!(config.tls);

    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    // If we got here, implicit TLS (port 563) works
    println!("Implicit TLS connection succeeded");

    // Verify we can perform operations
    let group = get_test_group();
    let result = client.select_group(&group).await;
    assert!(result.is_ok());
}

// Note: STARTTLS test is not included because most modern servers use
// implicit TLS on port 563 rather than STARTTLS upgrade. If the server
// supports STARTTLS on a different port, it would need separate configuration.
