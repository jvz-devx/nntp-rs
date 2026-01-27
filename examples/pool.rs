//! Connection pool example
//!
//! Demonstrates high-throughput parallel fetching using the connection pool.
//!
//! Run with: cargo run --example pool

use nntp_rs::{NntpPool, RetryConfig, ServerConfig};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Configure the NNTP server
    let config = ServerConfig {
        host: std::env::var("NNTP_HOST").unwrap_or_else(|_| "news.example.com".to_string()),
        port: std::env::var("NNTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(563),
        tls: true,
        allow_insecure_tls: false,
        username: std::env::var("NNTP_USER").unwrap_or_else(|_| "user".to_string()),
        password: std::env::var("NNTP_PASS").unwrap_or_else(|_| "pass".to_string()),
    };

    // Create a connection pool with custom retry config
    let retry_config = RetryConfig {
        max_retries: 3,
        initial_backoff_ms: 100,
        max_backoff_ms: 5000,
        backoff_multiplier: 2.0,
        jitter: true,
    };

    let pool_size = 5;
    println!(
        "Creating connection pool with {} connections to {}:{}...",
        pool_size, config.host, config.port
    );

    let pool = Arc::new(NntpPool::with_retry_config(config, pool_size, retry_config).await?);

    // Show pool state
    let state = pool.state();
    println!(
        "Pool created: {} connections, {} idle",
        state.connections, state.idle_connections
    );

    // Demonstrate parallel fetching
    let group = std::env::var("NNTP_GROUP").unwrap_or_else(|_| "alt.test".to_string());

    // First, get group info
    let group_info = {
        let mut conn = pool.get().await?;
        conn.select_group(&group).await?
    };

    println!(
        "\nGroup '{}': {} articles ({}-{})",
        group, group_info.count, group_info.first, group_info.last
    );

    if group_info.count == 0 {
        println!("No articles to fetch.");
        return Ok(());
    }

    // Fetch articles in parallel using multiple pool connections
    let num_fetches = 10.min(group_info.count as usize);
    let articles_to_fetch: Vec<u64> = (0..num_fetches as u64)
        .map(|i| group_info.last - i)
        .filter(|&n| n >= group_info.first)
        .collect();

    println!(
        "\nFetching {} articles in parallel...",
        articles_to_fetch.len()
    );
    let start = Instant::now();

    // Spawn parallel fetch tasks
    let mut handles = Vec::new();
    for article_num in articles_to_fetch {
        let pool = pool.clone();
        let group = group.clone();

        handles.push(tokio::spawn(async move {
            let mut conn = pool.get().await?;
            conn.select_group(&group).await?;

            let entries = conn.fetch_xover(&article_num.to_string()).await?;
            Ok::<_, nntp_rs::NntpError>(entries.into_iter().next())
        }));
    }

    // Collect results
    let mut success_count = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok(Some(entry))) => {
                success_count += 1;
                println!("  #{}: {}", entry.article_number, entry.subject);
            }
            Ok(Ok(None)) => println!("  (no entry)"),
            Ok(Err(e)) => println!("  Error: {}", e),
            Err(e) => println!("  Task error: {}", e),
        }
    }

    let elapsed = start.elapsed();
    println!(
        "\nFetched {} articles in {:?} ({:.1} articles/sec)",
        success_count,
        elapsed,
        success_count as f64 / elapsed.as_secs_f64()
    );

    // Show final pool state
    let state = pool.state();
    println!(
        "\nFinal pool state: {} connections, {} idle, {} in use",
        state.connections,
        state.idle_connections,
        pool.connections_in_use()
    );

    Ok(())
}
