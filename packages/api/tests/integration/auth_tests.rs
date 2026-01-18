use api::test_utils::TestContext;
use sqlx::Row;

#[tokio::test]
async fn test_signup_creates_user() {
    let ctx = TestContext::new().await;
    ctx.set_global();

    // Call the signup function
    let result = api::signup(
        "newuser@test.com".to_string(),
        "Password123".to_string(),
    )
    .await;

    assert!(result.is_ok(), "Signup should succeed");

    // Verify user exists in database
    let user = sqlx::query("SELECT email FROM users WHERE email = $1")
        .bind("newuser@test.com")
        .fetch_optional(&ctx.pool)
        .await
        .expect("Query should succeed");

    assert!(user.is_some(), "User should exist in database");

    // Verify the email matches
    if let Some(row) = user {
        let email: String = row.get("email");
        assert_eq!(email, "newuser@test.com");
    }
}
