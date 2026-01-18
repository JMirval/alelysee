use crate::types::{ContentTargetType, VoteState};
use dioxus::prelude::*;
#[cfg(feature = "server")]
use tracing::{debug, info};

/// Set a vote on any content.
///
/// - `value = 1` upvote
/// - `value = -1` downvote
/// - `value = 0` clears vote
#[dioxus::prelude::post("/api/votes/set")]
pub async fn set_vote(
    id_token: String,
    target_type: ContentTargetType,
    target_id: String,
    value: i16,
) -> Result<VoteState, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, target_type, target_id, value);
        Err(ServerFnError::new("set_vote is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use uuid::Uuid;

        debug!(
            "votes.set_vote: target_type={:?} target_id={} value={}",
            target_type, target_id, value
        );
        let user_id = crate::auth::require_user_id(id_token).await?;
        let tid =
            Uuid::parse_str(&target_id).map_err(|_| ServerFnError::new("invalid target_id"))?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        if value == 0 {
            info!("votes.set_vote: clear user_id={}", user_id);
            sqlx::query(
                "delete from votes where user_id = $1 and target_type = $2 and target_id = $3",
            )
            .bind(crate::db::uuid_to_db(user_id))
            .bind(target_type.as_db())
            .bind(crate::db::uuid_to_db(tid))
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        } else if value == 1 || value == -1 {
            info!("votes.set_vote: set user_id={} value={}", user_id, value);
            let sql = if crate::db::is_sqlite() {
                r#"
                insert into votes (user_id, target_type, target_id, value)
                values ($1, $2, $3, $4)
                on conflict (user_id, target_type, target_id)
                do update set value = excluded.value, updated_at = CURRENT_TIMESTAMP
                "#
            } else {
                r#"
                insert into votes (user_id, target_type, target_id, value)
                values ($1, $2, $3, $4)
                on conflict (user_id, target_type, target_id)
                do update set value = excluded.value, updated_at = now()
                "#
            };
            sqlx::query(sql)
                .bind(crate::db::uuid_to_db(user_id))
                .bind(target_type.as_db())
                .bind(crate::db::uuid_to_db(tid))
                .bind(value)
                .execute(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;

            // Activity log (best-effort)
            let action = if value == 1 { "voted_up" } else { "voted_down" };
            let _ = sqlx::query(
                "insert into activity (user_id, action, target_type, target_id) values ($1, $2, $3, $4)",
            )
            .bind(crate::db::uuid_to_db(user_id))
            .bind(action)
            .bind(target_type.as_db())
            .bind(crate::db::uuid_to_db(tid))
            .execute(pool)
            .await;
        } else {
            return Err(ServerFnError::new("value must be -1, 0, or 1"));
        }

        let score: i64 = sqlx::query_scalar(
            "select coalesce(sum(value), 0) from votes where target_type = $1 and target_id = $2",
        )
        .bind(target_type.as_db())
        .bind(crate::db::uuid_to_db(tid))
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let my_vote: Option<i16> = sqlx::query_scalar(
            "select value from votes where user_id = $1 and target_type = $2 and target_id = $3",
        )
        .bind(crate::db::uuid_to_db(user_id))
        .bind(target_type.as_db())
        .bind(crate::db::uuid_to_db(tid))
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        debug!("votes.set_vote: score={} my_vote={:?}", score, my_vote);
        Ok(VoteState {
            target_type,
            target_id: tid,
            score,
            my_vote,
        })
    }
}

/// Get the current vote state for a user + target.
#[dioxus::prelude::post("/api/votes/state")]
pub async fn get_vote_state(
    id_token: String,
    target_type: ContentTargetType,
    target_id: String,
) -> Result<VoteState, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, target_type, target_id);
        Err(ServerFnError::new("get_vote_state is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use uuid::Uuid;

        debug!(
            "votes.get_vote_state: target_type={:?} target_id={}",
            target_type, target_id
        );
        let user_id = crate::auth::require_user_id(id_token).await?;
        let tid =
            Uuid::parse_str(&target_id).map_err(|_| ServerFnError::new("invalid target_id"))?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let score: i64 = sqlx::query_scalar(
            "select coalesce(sum(value), 0) from votes where target_type = $1 and target_id = $2",
        )
        .bind(target_type.as_db())
        .bind(crate::db::uuid_to_db(tid))
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let my_vote: Option<i16> = sqlx::query_scalar(
            "select value from votes where user_id = $1 and target_type = $2 and target_id = $3",
        )
        .bind(crate::db::uuid_to_db(user_id))
        .bind(target_type.as_db())
        .bind(crate::db::uuid_to_db(tid))
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        debug!(
            "votes.get_vote_state: user_id={} score={} my_vote={:?}",
            user_id, score, my_vote
        );
        Ok(VoteState {
            target_type,
            target_id: tid,
            score,
            my_vote,
        })
    }
}
