//! Basic NNTP client example
//!
//! Run with: cargo run --example basic

use nntp_rs::{NntpClient, ServerConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Configure the NNTP server
    // Replace with your actual server credentials
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

    println!("Connecting to {}:{}...", config.host, config.port);

    // Connect to the server
    let mut client = NntpClient::connect(Arc::new(config)).await?;
    println!("Connected!");

    // Authenticate
    client.authenticate().await?;
    println!("Authenticated!");

    // Try to enable compression
    let compression_enabled = client.try_enable_compression().await?;
    println!(
        "Compression: {}",
        if compression_enabled {
            "enabled"
        } else {
            "not available"
        }
    );

    // Select a newsgroup
    let group = std::env::var("NNTP_GROUP").unwrap_or_else(|_| "alt.test".to_string());
    let group_info = client.select_group(&group).await?;
    println!(
        "Selected group '{}': {} articles ({}-{})",
        group, group_info.count, group_info.first, group_info.last
    );

    // Fetch some article overview data
    if group_info.count > 0 {
        let start = if group_info.last > 10 {
            group_info.last - 10
        } else {
            group_info.first
        };
        let range = format!("{}-{}", start, group_info.last);
        println!("\nFetching XOVER {}...", range);

        let entries = client.fetch_xover(&range).await?;
        println!("Got {} entries:\n", entries.len());

        for entry in entries.iter().take(5) {
            println!(
                "  #{}: {} (by {}, {} bytes)",
                entry.article_number, entry.subject, entry.author, entry.bytes
            );
        }

        if entries.len() > 5 {
            println!("  ... and {} more", entries.len() - 5);
        }
    }

    // Show bandwidth stats if compression was used
    if compression_enabled {
        let (compressed, decompressed) = client.get_bandwidth_stats();
        if decompressed > 0 {
            let ratio = (1.0 - (compressed as f64 / decompressed as f64)) * 100.0;
            println!(
                "\nBandwidth: {} bytes compressed, {} bytes original ({:.1}% savings)",
                compressed, decompressed, ratio
            );
        }
    }

    // Close gracefully
    client.quit().await?;
    println!("\nConnection closed.");

    Ok(())
}
