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

#[tokio::test]
async fn test_signup_rejects_weak_password() {
    let ctx = TestContext::new().await;
    ctx.set_global();

    let result = api::signup(
        "test@example.com".to_string(),
        "weak".to_string(),
    )
    .await;

    assert!(result.is_err(), "Should reject weak password");
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("Password must be"),
        "Error should mention password requirements"
    );
}

#[tokio::test]
async fn test_signup_rejects_duplicate_email() {
    let ctx = TestContext::new().await;
    ctx.set_global();

    // First signup should succeed
    api::signup(
        "duplicate@test.com".to_string(),
        "Password123".to_string(),
    )
    .await
    .expect("First signup should succeed");

    // Second signup with same email should fail
    let result = api::signup(
        "duplicate@test.com".to_string(),
        "Password456".to_string(),
    )
    .await;

    assert!(result.is_err(), "Should reject duplicate email");
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("already registered") || error.contains("already exists"),
        "Error should mention email already exists: {}",
        error
    );
}

#[tokio::test]
async fn test_signin_with_valid_credentials() {
    let ctx = TestContext::new().await;
    ctx.set_global();

    // Create user
    api::signup(
        "signin@test.com".to_string(),
        "Password123".to_string(),
    )
    .await
    .expect("Signup should succeed");

    // Verify email manually (bypass email verification for test)
    sqlx::query("UPDATE users SET email_verified = 1 WHERE email = $1")
        .bind("signin@test.com")
        .execute(&ctx.pool)
        .await
        .expect("Should update user");

    // Signin should succeed
    let token = api::signin(
        "signin@test.com".to_string(),
        "Password123".to_string(),
    )
    .await
    .expect("Signin should succeed");

    assert!(!token.is_empty(), "Should return JWT token");
}

#[tokio::test]
async fn test_signin_rejects_wrong_password() {
    let ctx = TestContext::new().await;
    ctx.set_global();

    // Create user
    api::signup(
        "wrongpass@test.com".to_string(),
        "Password123".to_string(),
    )
    .await
    .expect("Signup should succeed");

    // Verify email
    sqlx::query("UPDATE users SET email_verified = 1 WHERE email = $1")
        .bind("wrongpass@test.com")
        .execute(&ctx.pool)
        .await
        .expect("Should update user");

    // Signin with wrong password should fail
    let result = api::signin(
        "wrongpass@test.com".to_string(),
        "WrongPassword".to_string(),
    )
    .await;

    assert!(result.is_err(), "Should reject wrong password");
}

#[tokio::test]
async fn test_signin_rejects_unverified_email() {
    let ctx = TestContext::new().await;
    ctx.set_global();

    // Create user (email not verified)
    api::signup(
        "unverified@test.com".to_string(),
        "Password123".to_string(),
    )
    .await
    .expect("Signup should succeed");

    // Signin should fail for unverified email
    let result = api::signin(
        "unverified@test.com".to_string(),
        "Password123".to_string(),
    )
    .await;

    assert!(result.is_err(), "Should reject unverified email");
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("verify your email"),
        "Error should mention email verification"
    );
}
