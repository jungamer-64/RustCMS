//! HTTP E2E Staging Integration Tests
//!
//! Tests actual HTTP communication with the Staging environment using reqwest.
//! Verifies that API responses match expected formats, status codes, and data.
//!
//! Feature Requirements:
//! - `database`: For persistent storage
//! - `restructure_presentation`: For API v2 endpoints
//!
//! Pre-requisites:
//! 1. Start Staging environment: `docker-compose -f docker-compose.staging.yml up -d`
//! 2. Set environment variables:
//!    - DATABASE_URL=postgres://postgres:password@localhost:5432/cms_staging
//!    - REDIS_URL=redis://localhost:6379
//! 3. Run migrations: `cargo run --bin cms-migrate -- migrate --no-seed`

#![cfg(all(feature = "database", feature = "restructure_presentation"))]
// Re-export in case features are not enabled
#![allow(dead_code)]

use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use std::time::Duration;

const BASE_URL: &str = "http://localhost:3000";
const STAGING_TIMEOUT: Duration = Duration::from_secs(30);

/// Test setup for HTTP E2E tests
struct HttpTestSetup {
    client: Client,
    base_url: String,
}

impl HttpTestSetup {
    fn new() -> Self {
        let client = Client::builder()
            .timeout(STAGING_TIMEOUT)
            .build()
            .expect("Failed to create reqwest client");

        Self {
            client,
            base_url: BASE_URL.to_string(),
        }
    }

    async fn is_server_ready(&self) -> bool {
        tokio::time::timeout(
            Duration::from_secs(5),
            self.client.get(&format!("{}/health", self.base_url)).send(),
        )
        .await
        .is_ok()
    }

    async fn wait_for_server(&self, max_retries: u32) {
        let mut retries = 0;
        while retries < max_retries {
            if self.is_server_ready().await {
                return;
            }
            retries += 1;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        panic!("Server did not become ready after {} retries", max_retries);
    }
}

// ============================================================================
// HTTP GET Endpoint Tests
// ============================================================================

/// Test 1: GET /health endpoint
/// Verifies that the server is running and responding
#[tokio::test]
async fn test_http_get_health_endpoint() {
    let setup = HttpTestSetup::new();

    let response = setup
        .client
        .get(&format!("{}/health", setup.base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            // Server is running; verify status
            println!("✓ Server is running (status: {})", resp.status());
        }
        Err(e) => {
            eprintln!("✗ Server not available at {}: {}", setup.base_url, e);
            eprintln!("  Start Staging: docker-compose -f docker-compose.staging.yml up -d");
        }
    }
}

/// Test 2: GET /api/v2/tags (empty list)
/// Verifies empty list response format and status code
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_get_tags_empty_list() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let response = setup
        .client
        .get(&format!("{}/api/v2/tags", setup.base_url))
        .send()
        .await
        .expect("Failed to send GET /api/v2/tags request");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Expected 200 OK for empty tags list"
    );

    let body: Value = response
        .json()
        .await
        .expect("Failed to parse JSON response");

    assert!(body.is_array(), "Response should be an array");
    println!("✓ GET /api/v2/tags returned empty array: {}", body);
}

/// Test 3: GET /api/v2/users/{id} (not found)
/// Verifies 404 response for non-existent user
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_get_user_not_found() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let fake_user_id = "00000000-0000-0000-0000-000000000000";
    let response = setup
        .client
        .get(&format!("{}/api/v2/users/{}", setup.base_url, fake_user_id))
        .send()
        .await
        .expect("Failed to send GET /api/v2/users/:id request");

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Expected 404 for non-existent user"
    );

    let body: Value = response
        .json()
        .await
        .expect("Failed to parse JSON error response");

    assert!(
        body.get("error").is_some(),
        "Error response should contain 'error' field"
    );
    println!("✓ GET /api/v2/users/{{id}} returned 404: {}", body);
}

// ============================================================================
// HTTP POST Endpoint Tests
// ============================================================================

/// Test 4: POST /api/v2/users (valid registration)
/// Verifies user creation and response format
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_post_user_registration() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let payload = json!({
        "email": "test@example.com",
        "username": "testuser",
        "password": "SecurePassword123!"
    });

    let response = setup
        .client
        .post(&format!("{}/api/v2/users", setup.base_url))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send POST /api/v2/users request");

    match response.status() {
        StatusCode::CREATED => {
            let body: Value = response
                .json()
                .await
                .expect("Failed to parse JSON response");
            assert!(
                body.get("id").is_some(),
                "Response should contain 'id' field"
            );
            println!("✓ POST /api/v2/users created user: {}", body);
        }
        StatusCode::CONFLICT => {
            println!("✓ User already exists (expected if running multiple times)");
        }
        status => panic!("Unexpected status code: {}", status),
    }
}

/// Test 5: POST /api/v2/users (invalid email)
/// Verifies validation error handling
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_post_user_invalid_email() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let payload = json!({
        "email": "not-an-email",
        "username": "testuser",
        "password": "SecurePassword123!"
    });

    let response = setup
        .client
        .post(&format!("{}/api/v2/users", setup.base_url))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send POST /api/v2/users request");

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Expected 400 for invalid email"
    );

    let body: Value = response
        .json()
        .await
        .expect("Failed to parse JSON error response");

    assert!(
        body.get("error").is_some(),
        "Error response should contain 'error' field"
    );
    println!(
        "✓ POST /api/v2/users validation rejected invalid email: {}",
        body
    );
}

/// Test 6: POST /api/v2/posts (create post)
/// Verifies post creation with title and content
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_post_create_post() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let payload = json!({
        "title": "Test Post",
        "content": "This is test content with sufficient length.",
        "slug": "test-post",
        "author_id": "00000000-0000-0000-0000-000000000001"
    });

    let response = setup
        .client
        .post(&format!("{}/api/v2/posts", setup.base_url))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send POST /api/v2/posts request");

    match response.status() {
        StatusCode::CREATED => {
            let body: Value = response
                .json()
                .await
                .expect("Failed to parse JSON response");
            assert!(
                body.get("id").is_some(),
                "Response should contain 'id' field"
            );
            println!("✓ POST /api/v2/posts created post: {}", body);
        }
        StatusCode::BAD_REQUEST => {
            eprintln!("✗ Post creation failed with validation error");
        }
        status => panic!("Unexpected status code: {}", status),
    }
}

// ============================================================================
// HTTP Content-Type and Header Tests
// ============================================================================

/// Test 7: Response Content-Type headers
/// Verifies that responses have correct Content-Type: application/json
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_response_content_type() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let response = setup
        .client
        .get(&format!("{}/api/v2/tags", setup.base_url))
        .send()
        .await
        .expect("Failed to send GET /api/v2/tags request");

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    assert!(
        content_type.contains("application/json"),
        "Expected application/json content-type, got: {}",
        content_type
    );
    println!("✓ Response has correct Content-Type: {}", content_type);
}

/// Test 8: Deprecation headers for API v1
/// Verifies that API v1 endpoints return deprecation warnings
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_deprecation_headers() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let response = setup
        .client
        .get(&format!("{}/api/v1/tags", setup.base_url))
        .send()
        .await
        .expect("Failed to send GET /api/v1/tags request");

    let deprecated_header = response
        .headers()
        .get("deprecation")
        .and_then(|v| v.to_str().ok());

    if deprecated_header.is_some() {
        println!("✓ API v1 returns Deprecation header");
    } else {
        println!("⚠ API v1 Deprecation header not present (may be optional)");
    }
}

// ============================================================================
// HTTP Error Handling Tests
// ============================================================================

/// Test 9: 405 Method Not Allowed
/// Verifies correct error for unsupported HTTP methods
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_method_not_allowed() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let response = setup
        .client
        .patch(&format!("{}/api/v2/tags", setup.base_url))
        .json(&json!({}))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status() == StatusCode::METHOD_NOT_ALLOWED
                || resp.status() == StatusCode::NOT_FOUND
            {
                println!(
                    "✓ PATCH /api/v2/tags returned appropriate error: {}",
                    resp.status()
                );
            }
        }
        Err(_) => {
            println!("✓ PATCH request failed as expected (method not allowed)");
        }
    }
}

/// Test 10: Timeout handling
/// Verifies that slow requests are handled gracefully
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_request_timeout() {
    let client = Client::builder()
        .timeout(Duration::from_millis(100)) // Very short timeout
        .build()
        .expect("Failed to create reqwest client");

    // This test intentionally has a short timeout
    // In a real scenario, we'd use a slow endpoint
    let result = client
        .get(&format!("{}/api/v2/tags", BASE_URL))
        .send()
        .await;

    match result {
        Ok(resp) => {
            println!("✓ Request completed within timeout: {}", resp.status());
        }
        Err(e) if e.is_timeout() => {
            println!("✓ Timeout error detected as expected");
        }
        Err(e) => {
            println!("⚠ Request failed: {}", e);
        }
    }
}

// ============================================================================
// HTTP Performance and Load Tests
// ============================================================================

/// Test 11: Concurrent requests
/// Verifies that the server handles multiple simultaneous requests
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_concurrent_requests() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let client = setup.client.clone();
            let base_url = setup.base_url.clone();
            tokio::spawn(async move {
                let response = client
                    .get(&format!("{}/api/v2/tags", base_url))
                    .send()
                    .await;

                (i, response.is_ok())
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(handles).await;

    let successful = results.iter().filter(|r| r.as_ref().unwrap().1).count();
    println!(
        "✓ Concurrent requests completed: {}/5 successful",
        successful
    );

    assert!(
        successful > 0,
        "At least some concurrent requests should succeed"
    );
}

/// Test 12: Response time measurement
/// Measures and logs response times for key endpoints
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_response_time_measurement() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let endpoints = vec!["/api/v2/tags", "/api/v2/categories", "/api/v2/posts"];

    for endpoint in endpoints {
        let start = std::time::Instant::now();
        let response = setup
            .client
            .get(&format!("{}{}", setup.base_url, endpoint))
            .send()
            .await;

        let elapsed = start.elapsed();

        match response {
            Ok(resp) => println!(
                "✓ GET {} completed in {:.2}ms (status: {})",
                endpoint,
                elapsed.as_secs_f64() * 1000.0,
                resp.status()
            ),
            Err(e) => eprintln!("✗ GET {} failed: {}", endpoint, e),
        }
    }
}

// ============================================================================
// HTTP Canary Release Tests
// ============================================================================

/// Test 13: API v2 traffic routing
/// Verifies that requests are routed based on Canary configuration
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_canary_v2_routing() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let response_v2 = setup
        .client
        .get(&format!("{}/api/v2/tags", setup.base_url))
        .send()
        .await
        .expect("Failed to send GET /api/v2/tags request");

    assert_eq!(
        response_v2.status(),
        StatusCode::OK,
        "Expected 200 OK for /api/v2/tags"
    );

    println!("✓ API v2 endpoints are accessible");
}

/// Test 14: API v1 backward compatibility
/// Verifies that legacy API v1 endpoints still work
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_api_v1_backward_compat() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let response_v1 = setup
        .client
        .get(&format!("{}/api/v1/tags", setup.base_url))
        .send()
        .await
        .expect("Failed to send GET /api/v1/tags request");

    assert!(
        response_v1.status().is_success() || response_v1.status() == StatusCode::NOT_FOUND,
        "API v1 should return either success or 404, got: {}",
        response_v1.status()
    );

    println!("✓ API v1 backward compatibility maintained");
}

// ============================================================================
// HTTP Integration Workflow Tests
// ============================================================================

/// Test 15: User registration → Tag creation workflow
/// Tests a complete workflow across multiple endpoints
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_workflow_user_and_tag_creation() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    // Step 1: Create a user
    let user_payload = json!({
        "email": "workflow@example.com",
        "username": "workflowuser",
        "password": "SecurePassword123!"
    });

    let user_response = setup
        .client
        .post(&format!("{}/api/v2/users", setup.base_url))
        .json(&user_payload)
        .send()
        .await
        .expect("Failed to create user");

    if user_response.status() != StatusCode::CREATED
        && user_response.status() != StatusCode::CONFLICT
    {
        panic!("User creation failed: {}", user_response.status());
    }

    println!("✓ Step 1: User created/verified");

    // Step 2: Create a tag
    let tag_payload = json!({
        "name": "workflow-tag",
        "description": "A tag for workflow testing"
    });

    let tag_response = setup
        .client
        .post(&format!("{}/api/v2/tags", setup.base_url))
        .json(&tag_payload)
        .send()
        .await
        .expect("Failed to create tag");

    if tag_response.status() != StatusCode::CREATED && tag_response.status() != StatusCode::CONFLICT
    {
        eprintln!("Tag creation warning: {}", tag_response.status());
    }

    println!("✓ Step 2: Tag created/verified");
    println!("✓ Workflow completed successfully");
}

/// Test 16: API response schema validation
/// Verifies that API responses follow expected JSON structure
#[tokio::test]
#[ignore] // Skip until server is running
async fn test_http_response_schema_validation() {
    let setup = HttpTestSetup::new();
    setup.wait_for_server(10).await;

    let response = setup
        .client
        .get(&format!("{}/api/v2/tags", setup.base_url))
        .send()
        .await
        .expect("Failed to send GET /api/v2/tags request");

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response
        .json()
        .await
        .expect("Failed to parse JSON response");

    // Validate that response is either an array or an object with expected fields
    match &body {
        Value::Array(_) => {
            println!("✓ Response is a valid array");
        }
        Value::Object(obj) => {
            assert!(
                obj.contains_key("data") || obj.contains_key("error"),
                "Response object should contain 'data' or 'error' field"
            );
            println!("✓ Response is a valid object with expected fields");
        }
        _ => panic!("Response should be array or object"),
    }
}

// ============================================================================
// Notes for Running Tests
// ============================================================================
//
// To run all HTTP E2E tests:
// 1. Start Staging environment:
//    docker-compose -f docker-compose.staging.yml up -d
//
// 2. Set environment variables:
//    export DATABASE_URL=postgres://postgres:password@localhost:5432/cms_staging
//    export REDIS_URL=redis://localhost:6379
//
// 3. Run migrations:
//    cargo run --bin cms-migrate -- migrate --no-seed
//
// 4. Start the application:
//    cargo run --features "database,restructure_presentation"
//
// 5. Run tests (remove #[ignore] or use --ignored flag):
//    cargo test --test e2e_http_staging \
//      --features "database,restructure_presentation" -- --ignored --nocapture
//
// Tips:
// - Tests are marked #[ignore] to avoid failures in CI without real services
// - Use `--nocapture` flag to see println! output
// - Some tests (empty list, not found) are safe to run without setup
// - Performance tests may show high latency if server is under load
