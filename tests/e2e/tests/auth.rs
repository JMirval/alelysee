use e2e::{browser::Browser, test_server::TestServer};

#[tokio::test]
async fn test_signin_page_loads() {
    let server = TestServer::start()
        .await
        .expect("Failed to start test server");

    let browser = Browser::launch().expect("Failed to launch browser");
    let page = browser.new_page().expect("Failed to create page");

    // Navigate to signin page
    page.goto(&format!("{}/auth/signin", server.url()))
        .expect("Failed to navigate");

    // Check that signin form exists
    let result = page.find_element("input[name='email']");
    assert!(result.is_ok(), "Email input should exist");

    let result = page.find_element("input[name='password']");
    assert!(result.is_ok(), "Password input should exist");

    let result = page.find_element("button[type='submit']");
    assert!(result.is_ok(), "Submit button should exist");
}
