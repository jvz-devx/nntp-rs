//! High-Throughput Binary Fetch & Pipelining Live Integration Tests
//!
//! Tests for `fetch_article_binary`, `fetch_body_binary`, and
//! `fetch_articles_pipelined` methods.

#![cfg(feature = "live-tests")]

use nntp_rs::NntpClient;
use std::sync::Arc;

use super::{get_test_config, get_test_group};

#[tokio::test]
async fn test_fetch_article_binary() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let info = client.select_group(&group).await.unwrap();

    if info.count == 0 {
        println!("Group {} is empty, skipping binary article test", group);
        return;
    }

    let article_num = if info.last > 10 {
        info.last - 5
    } else {
        info.first
    };

    let response = client
        .fetch_article_binary(&article_num.to_string())
        .await
        .unwrap();

    println!(
        "fetch_article_binary: code={}, data_len={}",
        response.code,
        response.data.len()
    );

    assert_eq!(response.code, 220);
    assert!(!response.data.is_empty());
}

#[tokio::test]
async fn test_fetch_body_binary() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let info = client.select_group(&group).await.unwrap();

    if info.count == 0 {
        println!("Group {} is empty, skipping binary body test", group);
        return;
    }

    let article_num = if info.last > 10 {
        info.last - 5
    } else {
        info.first
    };

    let response = client
        .fetch_body_binary(&article_num.to_string())
        .await
        .unwrap();

    println!(
        "fetch_body_binary: code={}, data_len={}",
        response.code,
        response.data.len()
    );

    assert_eq!(response.code, 222);
    assert!(!response.data.is_empty());
}

#[tokio::test]
async fn test_binary_vs_text_consistency() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let info = client.select_group(&group).await.unwrap();

    if info.count == 0 {
        println!("Group {} is empty, skipping consistency test", group);
        return;
    }

    let article_num = if info.last > 10 {
        info.last - 5
    } else {
        info.first
    };
    let id = article_num.to_string();

    let text_response = client.fetch_article(&id).await.unwrap();
    let binary_response = client.fetch_article_binary(&id).await.unwrap();

    let text_total_bytes: usize = text_response.lines.iter().map(|l| l.len()).sum();
    let binary_bytes = binary_response.data.len();

    println!(
        "Text fetch: {} lines, ~{} bytes; Binary fetch: {} bytes",
        text_response.lines.len(),
        text_total_bytes,
        binary_bytes
    );

    // Both should have content
    assert!(!text_response.lines.is_empty());
    assert!(!binary_response.data.is_empty());

    // Binary should be at least as large as the text content
    // (text strips line endings, binary preserves raw data)
    assert!(binary_bytes >= text_total_bytes);
}

#[tokio::test]
async fn test_fetch_articles_pipelined() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let info = client.select_group(&group).await.unwrap();

    if info.count < 3 {
        println!(
            "Group {} has fewer than 3 articles, skipping pipeline test",
            group
        );
        return;
    }

    // Collect message IDs using stat + next
    let mut message_ids = Vec::new();
    let start = if info.last > 10 {
        info.last - 5
    } else {
        info.first
    };

    let stat_info = client.stat(&start.to_string()).await.unwrap();
    message_ids.push(stat_info.message_id);

    for _ in 0..2 {
        match client.next().await {
            Ok(next_info) => message_ids.push(next_info.message_id),
            Err(_) => break,
        }
    }

    if message_ids.len() < 2 {
        println!("Could not find enough articles for pipelining test");
        return;
    }

    println!("Pipelining {} articles", message_ids.len());

    let id_refs: Vec<&str> = message_ids.iter().map(String::as_str).collect();
    let responses = client.fetch_articles_pipelined(&id_refs, 10).await.unwrap();

    assert_eq!(responses.len(), message_ids.len());
    for (i, response) in responses.iter().enumerate() {
        println!(
            "  Pipeline result {}: code={}, data_len={}",
            i,
            response.code,
            response.data.len()
        );
        assert!(response.is_success());
        assert!(!response.data.is_empty());
    }
}

#[tokio::test]
async fn test_pipeline_empty_ids() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let responses = client.fetch_articles_pipelined(&[], 10).await.unwrap();
    assert!(responses.is_empty());
    println!("Pipeline with empty IDs returned empty vec");
}

#[tokio::test]
async fn test_pipeline_single_article() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let info = client.select_group(&group).await.unwrap();

    if info.count == 0 {
        println!("Group {} is empty, skipping single pipeline test", group);
        return;
    }

    let article_num = if info.last > 10 {
        info.last - 5
    } else {
        info.first
    };

    let stat_info = client.stat(&article_num.to_string()).await.unwrap();
    let ids = [stat_info.message_id.as_str()];

    let responses = client.fetch_articles_pipelined(&ids, 10).await.unwrap();
    assert_eq!(responses.len(), 1);
    assert!(responses[0].is_success());
    assert!(!responses[0].data.is_empty());

    println!(
        "Single pipeline fetch: code={}, data_len={}",
        responses[0].code,
        responses[0].data.len()
    );
}
