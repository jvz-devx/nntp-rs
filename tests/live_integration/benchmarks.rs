//! Performance Benchmark Tests - Live Integration Testing
//!
//! This test suite measures performance characteristics against a real NNTP server:
//! - Connection establishment time
//! - Authentication time
//! - Article retrieval latency
//! - Throughput (articles/second)
//!
//! Run with:
//! ```bash
//! cargo test --features live-tests benchmarks -- --test-threads=1 --nocapture
//! ```

#![cfg(feature = "live-tests")]

use nntp_rs::NntpClient;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{get_test_config, get_test_group};

// Connection and Authentication Benchmarks

#[tokio::test]
async fn bench_connection_establishment() {
    let config = get_test_config();

    // Measure single connection
    let start = Instant::now();
    let mut client = NntpClient::connect(Arc::new(config.clone())).await.unwrap();
    let elapsed = start.elapsed();

    println!("Connection establishment: {:?}", elapsed);
    println!(
        "Connection establishment: {:.2} ms",
        elapsed.as_secs_f64() * 1000.0
    );

    // Reasonable bounds: should connect within 5 seconds (typically much faster)
    assert!(
        elapsed < Duration::from_secs(5),
        "Connection took too long: {:?}",
        elapsed
    );

    client.quit().await.ok();
}

#[tokio::test]
async fn bench_authentication() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();

    // Measure authentication time
    let start = Instant::now();
    client.authenticate().await.unwrap();
    let elapsed = start.elapsed();

    println!("Authentication: {:?}", elapsed);
    println!("Authentication: {:.2} ms", elapsed.as_secs_f64() * 1000.0);

    // Should authenticate within 3 seconds
    assert!(
        elapsed < Duration::from_secs(3),
        "Authentication took too long: {:?}",
        elapsed
    );

    client.quit().await.ok();
}

#[tokio::test]
async fn bench_full_connection_cycle() {
    let config = get_test_config();

    // Measure complete connection + auth + disconnect cycle
    let start = Instant::now();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();
    client.quit().await.unwrap();
    let elapsed = start.elapsed();

    println!(
        "Full connection cycle (connect + auth + quit): {:?}",
        elapsed
    );
    println!(
        "Full connection cycle: {:.2} ms",
        elapsed.as_secs_f64() * 1000.0
    );

    // Should complete within 10 seconds
    assert!(
        elapsed < Duration::from_secs(10),
        "Full cycle took too long: {:?}",
        elapsed
    );
}

// Article Retrieval Latency Benchmarks

#[tokio::test]
async fn bench_article_retrieval_latency() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (_, first, last) = client.select_group(&group).await.unwrap();

    if last == 0 {
        println!("Group {} is empty, skipping latency benchmark", group);
        client.quit().await.ok();
        return;
    }

    // Pick an article number in the middle range
    let article_num = (first + last) / 2;

    // Get message-id for the article
    let result = client.stat(&article_num.to_string()).await;

    match result {
        Ok((_, message_id)) => {
            // Measure article retrieval latency
            let start = Instant::now();
            let article = client.fetch_article(&message_id).await;
            let elapsed = start.elapsed();

            match article {
                Ok(_) => {
                    println!(
                        "Article retrieval latency (article {}): {:?}",
                        article_num, elapsed
                    );
                    println!(
                        "Article retrieval latency: {:.2} ms",
                        elapsed.as_secs_f64() * 1000.0
                    );

                    // Should retrieve within 5 seconds (network dependent)
                    assert!(
                        elapsed < Duration::from_secs(5),
                        "Article retrieval took too long: {:?}",
                        elapsed
                    );
                }
                Err(e) => {
                    println!("Could not retrieve article {}: {:?}", article_num, e);
                }
            }
        }
        Err(e) => {
            println!("Could not STAT article {}: {:?}", article_num, e);
        }
    }

    client.quit().await.ok();
}

#[tokio::test]
async fn bench_head_retrieval_latency() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (_, first, last) = client.select_group(&group).await.unwrap();

    if last == 0 {
        println!("Group {} is empty, skipping HEAD latency benchmark", group);
        client.quit().await.ok();
        return;
    }

    let article_num = (first + last) / 2;

    // Get message-id for the article
    let result = client.stat(&article_num.to_string()).await;

    match result {
        Ok((_, message_id)) => {
            // Measure HEAD retrieval latency (headers only, should be faster)
            let start = Instant::now();
            let headers = client.fetch_head(&message_id).await;
            let elapsed = start.elapsed();

            match headers {
                Ok(_) => {
                    println!(
                        "HEAD retrieval latency (article {}): {:?}",
                        article_num, elapsed
                    );
                    println!(
                        "HEAD retrieval latency: {:.2} ms",
                        elapsed.as_secs_f64() * 1000.0
                    );

                    // HEAD should be faster than full article
                    assert!(
                        elapsed < Duration::from_secs(3),
                        "HEAD retrieval took too long: {:?}",
                        elapsed
                    );
                }
                Err(e) => {
                    println!(
                        "Could not retrieve HEAD for article {}: {:?}",
                        article_num, e
                    );
                }
            }
        }
        Err(e) => {
            println!("Could not STAT article {}: {:?}", article_num, e);
        }
    }

    client.quit().await.ok();
}

#[tokio::test]
async fn bench_stat_command_latency() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (_, first, last) = client.select_group(&group).await.unwrap();

    if last == 0 {
        println!("Group {} is empty, skipping STAT latency benchmark", group);
        client.quit().await.ok();
        return;
    }

    let article_num = (first + last) / 2;

    // Measure STAT command latency (no data transfer, just status check)
    let start = Instant::now();
    let result = client.stat(&article_num.to_string()).await;
    let elapsed = start.elapsed();

    match result {
        Ok(_) => {
            println!(
                "STAT command latency (article {}): {:?}",
                article_num, elapsed
            );
            println!(
                "STAT command latency: {:.2} ms",
                elapsed.as_secs_f64() * 1000.0
            );

            // STAT should be very fast (no data transfer)
            assert!(
                elapsed < Duration::from_secs(2),
                "STAT took too long: {:?}",
                elapsed
            );
        }
        Err(e) => {
            println!("Could not STAT article {}: {:?}", article_num, e);
        }
    }

    client.quit().await.ok();
}

// Throughput Benchmarks

#[tokio::test]
async fn bench_article_throughput() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (_, first, last) = client.select_group(&group).await.unwrap();

    if last == 0 {
        println!("Group {} is empty, skipping throughput benchmark", group);
        client.quit().await.ok();
        return;
    }

    // Retrieve up to 10 articles to measure throughput
    let num_articles = std::cmp::min(10, last - first + 1);
    let test_range = first..first + num_articles;

    println!("Measuring throughput for {} articles...", num_articles);

    let start = Instant::now();
    let mut success_count = 0;
    let mut total_bytes = 0u64;

    for article_num in test_range {
        // Get message-id first
        if let Ok((_, message_id)) = client.stat(&article_num.to_string()).await {
            match client.fetch_article(&message_id).await {
                Ok(response) => {
                    success_count += 1;
                    // Count total lines as a proxy for bytes
                    total_bytes += response.lines.iter().map(|l| l.len() as u64).sum::<u64>();
                }
                Err(_) => {
                    // Skip missing articles
                }
            }
        }
    }

    let elapsed = start.elapsed();

    if success_count > 0 {
        let articles_per_sec = success_count as f64 / elapsed.as_secs_f64();
        let bytes_per_sec = total_bytes as f64 / elapsed.as_secs_f64();
        let kb_per_sec = bytes_per_sec / 1024.0;

        println!("Retrieved {} articles in {:?}", success_count, elapsed);
        println!("Throughput: {:.2} articles/sec", articles_per_sec);
        println!(
            "Throughput: {:.2} KB/sec ({} bytes total)",
            kb_per_sec, total_bytes
        );

        // Should retrieve at least 1 article per 5 seconds
        assert!(
            articles_per_sec >= 0.2,
            "Throughput too low: {:.2} articles/sec",
            articles_per_sec
        );
    } else {
        println!("No articles retrieved successfully");
    }

    client.quit().await.ok();
}

#[tokio::test]
async fn bench_header_throughput() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();
    let (_, first, last) = client.select_group(&group).await.unwrap();

    if last == 0 {
        println!(
            "Group {} is empty, skipping header throughput benchmark",
            group
        );
        client.quit().await.ok();
        return;
    }

    // Retrieve headers for up to 20 articles (headers only, should be faster)
    let num_articles = std::cmp::min(20, last - first + 1);
    let test_range = first..first + num_articles;

    println!(
        "Measuring header throughput for {} articles...",
        num_articles
    );

    let start = Instant::now();
    let mut success_count = 0;

    for article_num in test_range {
        // Get message-id first
        if let Ok((_, message_id)) = client.stat(&article_num.to_string()).await {
            match client.fetch_head(&message_id).await {
                Ok(_) => success_count += 1,
                Err(_) => {}
            }
        }
    }

    let elapsed = start.elapsed();

    if success_count > 0 {
        let headers_per_sec = success_count as f64 / elapsed.as_secs_f64();

        println!("Retrieved {} headers in {:?}", success_count, elapsed);
        println!("Header throughput: {:.2} headers/sec", headers_per_sec);

        // Headers should be retrieved faster than full articles
        // Should retrieve at least 2 headers per second
        assert!(
            headers_per_sec >= 2.0,
            "Header throughput too low: {:.2} headers/sec",
            headers_per_sec
        );
    } else {
        println!("No headers retrieved successfully");
    }

    client.quit().await.ok();
}

#[tokio::test]
async fn bench_group_selection_throughput() {
    let config = get_test_config();
    let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
    client.authenticate().await.unwrap();

    let group = get_test_group();

    // Measure how many group selections we can do per second
    let num_iterations = 10;

    println!(
        "Measuring GROUP selection throughput ({} iterations)...",
        num_iterations
    );

    let start = Instant::now();

    for _ in 0..num_iterations {
        client.select_group(&group).await.unwrap();
    }

    let elapsed = start.elapsed();
    let selections_per_sec = num_iterations as f64 / elapsed.as_secs_f64();

    println!(
        "Completed {} GROUP selections in {:?}",
        num_iterations, elapsed
    );
    println!(
        "GROUP selection throughput: {:.2} selections/sec",
        selections_per_sec
    );

    // Should complete at least 1 group selection per second
    assert!(
        selections_per_sec >= 1.0,
        "GROUP selection throughput too low: {:.2} selections/sec",
        selections_per_sec
    );

    client.quit().await.ok();
}
