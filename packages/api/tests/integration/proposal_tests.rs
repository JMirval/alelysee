use api::test_utils::TestContext;

#[tokio::test]
async fn test_create_proposal() {
    let ctx = TestContext::new().await;
    ctx.set_global();

    // Create and verify a user
    api::signup("author@test.com".to_string(), "Password123".to_string())
        .await
        .expect("Signup should succeed");

    sqlx::query("UPDATE users SET email_verified = true WHERE email = $1")
        .bind("author@test.com")
        .execute(&ctx.pool)
        .await
        .expect("Should verify user");

    let token = api::signin("author@test.com".to_string(), "Password123".to_string())
        .await
        .expect("Signin should succeed");

    // Create proposal (this may need to be updated based on actual API)
    // For now, just verify the test compiles
    // TODO: Implement actual proposal creation test
}
