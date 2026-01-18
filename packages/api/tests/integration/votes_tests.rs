use api::test_utils::TestContext;
use api::types::ContentTargetType;

async fn create_user_with_token(ctx: &TestContext, email: &str) -> String {
    api::signup(email.to_string(), "Password123".to_string())
        .await
        .expect("Signup should succeed");

    sqlx::query("UPDATE users SET email_verified = true WHERE email = $1")
        .bind(email)
        .execute(&ctx.pool)
        .await
        .expect("Should verify user");

    api::signin(email.to_string(), "Password123".to_string())
        .await
        .expect("Signin should succeed")
}

async fn create_proposal(ctx: &TestContext, author_user_id: &str) -> String {
    sqlx::query_scalar(
        "insert into proposals (author_user_id, title, summary, body_markdown, tags) values ($1, 'T', '', '', '[]') returning id",
    )
    .bind(author_user_id)
    .fetch_one(&ctx.pool)
    .await
    .expect("Should create proposal")
}

#[tokio::test]
async fn vote_state_roundtrip() {
    let ctx = TestContext::new().await;
    ctx.set_global();

    let token = create_user_with_token(&ctx, "voter@test.com").await;
    let author_id: String = sqlx::query_scalar("select id from users where email = $1")
        .bind("voter@test.com")
        .fetch_one(&ctx.pool)
        .await
        .expect("Should fetch user id");

    let proposal_id = create_proposal(&ctx, &author_id).await;

    let state = api::set_vote(
        token.clone(),
        ContentTargetType::Proposal,
        proposal_id.clone(),
        1,
    )
    .await
    .expect("Should upvote");
    assert_eq!(state.score, 1);
    assert_eq!(state.my_vote, Some(1));

    let state = api::get_vote_state(
        token.clone(),
        ContentTargetType::Proposal,
        proposal_id.clone(),
    )
    .await
    .expect("Should fetch vote state");
    assert_eq!(state.score, 1);
    assert_eq!(state.my_vote, Some(1));

    let state = api::set_vote(
        token.clone(),
        ContentTargetType::Proposal,
        proposal_id.clone(),
        -1,
    )
    .await
    .expect("Should flip vote");
    assert_eq!(state.score, -1);
    assert_eq!(state.my_vote, Some(-1));

    let state = api::set_vote(
        token.clone(),
        ContentTargetType::Proposal,
        proposal_id.clone(),
        0,
    )
    .await
    .expect("Should clear vote");
    assert_eq!(state.score, 0);
    assert_eq!(state.my_vote, None);

    let state =
        api::get_vote_state(token, ContentTargetType::Proposal, proposal_id).await;
    let state = state.expect("Should fetch cleared vote state");
    assert_eq!(state.score, 0);
    assert_eq!(state.my_vote, None);
}
