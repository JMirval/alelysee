#![cfg(all(test, feature = "server"))]

use crate::types::ContentTargetType;
use uuid::Uuid;

#[tokio::test]
async fn db_boots_and_resets() {
    // Skip if no DB available
    if crate::test_support::pool().await.is_none() {
        return;
    }
    crate::test_support::reset_db().await.expect("reset db");
}

#[tokio::test]
async fn votes_aggregate_for_proposal() {
    let pool = match crate::test_support::pool().await {
        Some(p) => p,
        None => return,
    };
    crate::test_support::reset_db().await.expect("reset db");

    // Create two users
    let sub1 = format!("test-sub-{}", Uuid::new_v4());
    let sub2 = format!("test-sub-{}", Uuid::new_v4());
    let user1: Uuid = sqlx::query_scalar("insert into users (cognito_sub) values ($1) returning id")
        .bind(sub1)
        .fetch_one(pool)
        .await
        .unwrap();
    let user2: Uuid = sqlx::query_scalar("insert into users (cognito_sub) values ($1) returning id")
        .bind(sub2)
        .fetch_one(pool)
        .await
        .unwrap();

    // Create proposal
    let proposal_id: Uuid = sqlx::query_scalar(
        "insert into proposals (author_user_id, title, summary, body_markdown, tags) values ($1, 'T', '', '', '{}'::text[]) returning id",
    )
    .bind(user1)
    .fetch_one(pool)
    .await
    .unwrap();

    // Vote +1 and -1
    sqlx::query("insert into votes (user_id, target_type, target_id, value) values ($1, 'proposal', $2, 1)")
        .bind(user1)
        .bind(proposal_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("insert into votes (user_id, target_type, target_id, value) values ($1, 'proposal', $2, -1)")
        .bind(user2)
        .bind(proposal_id)
        .execute(pool)
        .await
        .unwrap();

    // Verify aggregation
    let score: i64 = sqlx::query_scalar(
        "select coalesce(sum(value), 0) from votes where target_type = 'proposal' and target_id = $1",
    )
    .bind(proposal_id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(score, 0);

    // Ensure our serverfn vote endpoint can update (bypassing auth isn't possible here),
    // but we can still validate type mapping is stable.
    assert_eq!(ContentTargetType::Proposal.as_db(), "proposal");
}

#[tokio::test]
async fn comments_and_activity_insert() {
    let pool = match crate::test_support::pool().await {
        Some(p) => p,
        None => return,
    };
    crate::test_support::reset_db().await.expect("reset db");

    // user + proposal
    let sub = format!("test-sub-{}", Uuid::new_v4());
    let user_id: Uuid = sqlx::query_scalar("insert into users (cognito_sub) values ($1) returning id")
        .bind(sub)
        .fetch_one(pool)
        .await
        .unwrap();

    let proposal_id: Uuid = sqlx::query_scalar(
        "insert into proposals (author_user_id, title, summary, body_markdown, tags) values ($1, 'T', '', '', '{}'::text[]) returning id",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap();

    // comment
    let _comment_id: Uuid = sqlx::query_scalar(
        "insert into comments (author_user_id, target_type, target_id, parent_comment_id, body_markdown) values ($1, 'proposal', $2, null, 'hello') returning id",
    )
    .bind(user_id)
    .bind(proposal_id)
    .fetch_one(pool)
    .await
    .unwrap();

    // activity row
    sqlx::query(
        "insert into activity (user_id, action, target_type, target_id) values ($1, 'commented', 'proposal', $2)",
    )
    .bind(user_id)
    .bind(proposal_id)
    .execute(pool)
    .await
    .unwrap();

    let count: i64 = sqlx::query_scalar("select count(*) from activity where user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();

    assert_eq!(count, 1);
}


