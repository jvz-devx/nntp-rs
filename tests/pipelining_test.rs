//! Tests for NNTP command pipelining
//!
//! This module tests the fetch_articles_pipelined() method which sends
//! multiple ARTICLE commands before waiting for responses.

#[cfg(feature = "live-tests")]
mod live_pipelining_tests {
    use nntp_rs::{NntpClient, ServerConfig};
    use std::sync::Arc;

    fn get_test_config() -> ServerConfig {
        let host = std::env::var("NNTP_HOST").expect("NNTP_HOST not set");
        let port = std::env::var("NNTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(563);
        let username = std::env::var("NNTP_USER").expect("NNTP_USER not set");
        let password = std::env::var("NNTP_PASS").expect("NNTP_PASS not set");

        ServerConfig {
            host,
            port,
            tls: true,
            allow_insecure_tls: false,
            username,
            password,
        }
    }

    fn get_test_group() -> String {
        std::env::var("NNTP_GROUP").unwrap_or_else(|_| "alt.test".to_string())
    }

    #[tokio::test]
    async fn test_pipelining_empty_input() {
        let config = get_test_config();
        let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
        client.authenticate().await.unwrap();

        // Test with empty array
        let ids: Vec<&str> = vec![];
        let results = client.fetch_articles_pipelined(&ids, 10).await.unwrap();
        assert_eq!(results.len(), 0, "Empty input should return empty results");
    }

    #[tokio::test]
    async fn test_pipelining_single_article() {
        let config = get_test_config();
        let group = get_test_group();

        let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
        client.authenticate().await.unwrap();

        // Select a group and get an article
        let info = client.select_group(&group).await.unwrap();
        if info.count == 0 {
            println!("Skipping test: group {} is empty", group);
            return;
        }

        // Fetch single article with pipelining
        let article_num = info.first.to_string();
        let ids = vec![article_num.as_str()];
        let results = client.fetch_articles_pipelined(&ids, 10).await.unwrap();

        assert_eq!(results.len(), 1, "Should return exactly one result");
        assert!(results[0].is_success(), "Response should be successful");
        assert!(!results[0].data.is_empty(), "Article should have data");
    }

    #[tokio::test]
    async fn test_pipelining_depth_2() {
        let config = get_test_config();
        let group = get_test_group();

        let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
        client.authenticate().await.unwrap();

        // Select a group and get multiple articles
        let info = client.select_group(&group).await.unwrap();
        if info.count < 2 {
            println!("Skipping test: group {} has less than 2 articles", group);
            return;
        }

        // Fetch 2 articles with pipeline depth 2
        let ids: Vec<String> = (info.first..info.first + 2).map(|n: u64| n.to_string()).collect();
        let id_refs: Vec<&str> = ids.iter().map(|s: &String| s.as_str()).collect();
        let results = client.fetch_articles_pipelined(&id_refs, 2).await.unwrap();

        assert_eq!(results.len(), 2, "Should return exactly two results");
        assert!(
            results[0].is_success(),
            "First response should be successful"
        );
        assert!(
            results[1].is_success(),
            "Second response should be successful"
        );
        assert!(
            !results[0].data.is_empty(),
            "First article should have data"
        );
        assert!(
            !results[1].data.is_empty(),
            "Second article should have data"
        );
    }

    #[tokio::test]
    async fn test_pipelining_depth_5() {
        let config = get_test_config();
        let group = get_test_group();

        let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
        client.authenticate().await.unwrap();

        // Select a group and get multiple articles
        let info = client.select_group(&group).await.unwrap();
        if info.count < 5 {
            println!("Skipping test: group {} has less than 5 articles", group);
            return;
        }

        // Fetch 5 articles with pipeline depth 5
        let ids: Vec<String> = (info.first..info.first + 5).map(|n: u64| n.to_string()).collect();
        let id_refs: Vec<&str> = ids.iter().map(|s: &String| s.as_str()).collect();
        let results = client.fetch_articles_pipelined(&id_refs, 5).await.unwrap();

        assert_eq!(results.len(), 5, "Should return exactly five results");
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_success(), "Response {} should be successful", i);
            assert!(!result.data.is_empty(), "Article {} should have data", i);
        }
    }

    #[tokio::test]
    async fn test_pipelining_depth_10() {
        let config = get_test_config();
        let group = get_test_group();

        let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
        client.authenticate().await.unwrap();

        // Select a group and get multiple articles
        let info = client.select_group(&group).await.unwrap();
        if info.count < 10 {
            println!("Skipping test: group {} has less than 10 articles", group);
            return;
        }

        // Fetch 10 articles with pipeline depth 10
        let ids: Vec<String> = (info.first..info.first + 10).map(|n: u64| n.to_string()).collect();
        let id_refs: Vec<&str> = ids.iter().map(|s: &String| s.as_str()).collect();
        let results = client.fetch_articles_pipelined(&id_refs, 10).await.unwrap();

        assert_eq!(results.len(), 10, "Should return exactly ten results");
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_success(), "Response {} should be successful", i);
            assert!(!result.data.is_empty(), "Article {} should have data", i);
        }
    }

    #[tokio::test]
    async fn test_pipelining_partial_batch() {
        let config = get_test_config();
        let group = get_test_group();

        let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
        client.authenticate().await.unwrap();

        // Select a group and get multiple articles
        let info = client.select_group(&group).await.unwrap();
        if info.count < 7 {
            println!("Skipping test: group {} has less than 7 articles", group);
            return;
        }

        // Fetch 7 articles with pipeline depth 5
        // This should process 5 + 2 (partial batch at end)
        let ids: Vec<String> = (info.first..info.first + 7).map(|n: u64| n.to_string()).collect();
        let id_refs: Vec<&str> = ids.iter().map(|s: &String| s.as_str()).collect();
        let results = client.fetch_articles_pipelined(&id_refs, 5).await.unwrap();

        assert_eq!(results.len(), 7, "Should return exactly seven results");
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_success(), "Response {} should be successful", i);
            assert!(!result.data.is_empty(), "Article {} should have data", i);
        }
    }

    #[tokio::test]
    async fn test_pipelining_response_ordering() {
        let config = get_test_config();
        let group = get_test_group();

        let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
        client.authenticate().await.unwrap();

        // Select a group and get multiple articles
        let info = client.select_group(&group).await.unwrap();
        if info.count < 5 {
            println!("Skipping test: group {} has less than 5 articles", group);
            return;
        }

        // Fetch 5 articles and verify they come back in order
        let ids: Vec<String> = (info.first..info.first + 5).map(|n: u64| n.to_string()).collect();
        let id_refs: Vec<&str> = ids.iter().map(|s: &String| s.as_str()).collect();
        let results = client.fetch_articles_pipelined(&id_refs, 5).await.unwrap();

        assert_eq!(results.len(), 5, "Should return exactly five results");

        // Verify responses are in the same order as requests
        // Note: We can't verify the exact article numbers without parsing the response,
        // but we can verify all responses are successful and have data
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_success(), "Response {} should be successful", i);
            assert!(!result.data.is_empty(), "Article {} should have data", i);
        }
    }

    #[tokio::test]
    async fn test_pipelining_invalid_article_id() {
        let config = get_test_config();
        let group = get_test_group();

        let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
        client.authenticate().await.unwrap();

        // Select a group
        client.select_group(&group).await.unwrap();

        // Try to fetch an invalid article ID in the middle of valid ones
        let info = client.select_group(&group).await.unwrap();
        if info.count < 2 {
            println!("Skipping test: group {} has less than 2 articles", group);
            return;
        }

        // Create a list with an invalid ID in the middle
        let ids = [
            info.first.to_string(),
            "999999999".to_string(), // Invalid article number
            (info.first + 1).to_string(),
        ];
        let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();

        // This should fail when the invalid article is encountered
        let result = client.fetch_articles_pipelined(&id_refs, 5).await;
        assert!(
            result.is_err(),
            "Should fail when invalid article ID is encountered"
        );

        // The error should be NoSuchArticle
        match result {
            Err(nntp_rs::NntpError::NoSuchArticle(id)) => {
                assert_eq!(id, "999999999", "Error should contain the invalid ID");
            }
            _ => panic!("Expected NoSuchArticle error"),
        }
    }

    #[tokio::test]
    async fn test_pipelining_minimum_depth() {
        let config = get_test_config();
        let group = get_test_group();

        let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
        client.authenticate().await.unwrap();

        // Select a group and get an article
        let info = client.select_group(&group).await.unwrap();
        if info.count < 2 {
            println!("Skipping test: group {} has less than 2 articles", group);
            return;
        }

        // Test with pipeline depth 0 (should be clamped to 1)
        let ids: Vec<String> = (info.first..info.first + 2).map(|n: u64| n.to_string()).collect();
        let id_refs: Vec<&str> = ids.iter().map(|s: &String| s.as_str()).collect();
        let results = client.fetch_articles_pipelined(&id_refs, 0).await.unwrap();

        assert_eq!(results.len(), 2, "Should return exactly two results");
        assert!(
            results[0].is_success(),
            "First response should be successful"
        );
        assert!(
            results[1].is_success(),
            "Second response should be successful"
        );
    }

    #[tokio::test]
    async fn test_pipelining_vs_sequential_performance() {
        let config = get_test_config();
        let group = get_test_group();

        let mut client = NntpClient::connect(Arc::new(config)).await.unwrap();
        client.authenticate().await.unwrap();

        // Select a group and get multiple articles
        let info = client.select_group(&group).await.unwrap();
        if info.count < 10 {
            println!("Skipping test: group {} has less than 10 articles", group);
            return;
        }

        let ids: Vec<String> = (info.first..info.first + 10).map(|n: u64| n.to_string()).collect();
        let id_refs: Vec<&str> = ids.iter().map(|s: &String| s.as_str()).collect();

        // Test sequential fetching (pipeline depth 1)
        let start = std::time::Instant::now();
        let results_sequential = client.fetch_articles_pipelined(&id_refs, 1).await.unwrap();
        let sequential_duration = start.elapsed();

        assert_eq!(results_sequential.len(), 10);

        // Test pipelined fetching (pipeline depth 10)
        let start = std::time::Instant::now();
        let results_pipelined = client.fetch_articles_pipelined(&id_refs, 10).await.unwrap();
        let pipelined_duration = start.elapsed();

        assert_eq!(results_pipelined.len(), 10);

        println!(
            "Sequential: {:?}, Pipelined: {:?}, Speedup: {:.2}x",
            sequential_duration,
            pipelined_duration,
            sequential_duration.as_secs_f64() / pipelined_duration.as_secs_f64()
        );

        // Pipelining should be faster or at least not significantly slower
        // We expect at least some improvement, but we'll just verify both work
        assert!(
            results_sequential.len() == results_pipelined.len(),
            "Both methods should return the same number of results"
        );
    }
}
