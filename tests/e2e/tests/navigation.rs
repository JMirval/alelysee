use e2e::test_server::TestServer;

#[tokio::test]
async fn test_homepage_loads() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    // Make HTTP request to homepage
    let response = reqwest::get(server.url())
        .await
        .expect("Failed to fetch homepage");

    assert_eq!(response.status(), 200, "Homepage should return 200 OK");

    let body = response.text().await.expect("Failed to read body");
    assert!(
        body.contains("Alelysee") || body.contains("DOCTYPE"),
        "Should contain HTML"
    );
}
