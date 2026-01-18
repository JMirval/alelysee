use crate::types::ActivityItem;
use dioxus::prelude::*;
#[cfg(feature = "server")]
use tracing::debug;

#[dioxus::prelude::post("/api/activity/me")]
pub async fn list_my_activity(
    id_token: String,
    limit: i64,
) -> Result<Vec<ActivityItem>, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, limit);
        Err(ServerFnError::new("list_my_activity is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use crate::types::{ActivityAction, ContentTargetType};
        use sqlx::Row;
        debug!("activity.list_my_activity: limit={}", limit);
        let user_id = crate::auth::require_user_id(id_token).await?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let title_expr = if crate::db::is_sqlite() {
            "substr(body_markdown, 1, 80)"
        } else {
            "left(body_markdown, 80)"
        };
        let sql = format!(
            r#"
            select
                CAST(a.id as TEXT) as id,
                CAST(a.user_id as TEXT) as user_id,
                a.action,
                a.target_type,
                CAST(a.target_id as TEXT) as target_id,
                CAST(a.created_at as TEXT) as created_at,
                case
                    when a.target_type = 'proposal' then (select title from proposals where id = a.target_id)
                    when a.target_type = 'program' then (select title from programs where id = a.target_id)
                    when a.target_type = 'comment' then (select {} from comments where id = a.target_id)
                    when a.target_type = 'video' then (select storage_key from videos where id = a.target_id)
                    else null
                end as title
            from activity a
            where a.user_id = $1
            order by a.created_at desc
            limit $2
            "#,
            title_expr
        );
        let rows = sqlx::query(&sql)
        .bind(crate::db::uuid_to_db(user_id))
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
            let user_id = crate::db::uuid_from_db(&row.get::<String, _>("user_id"))?;
            let target_id = crate::db::uuid_from_db(&row.get::<String, _>("target_id"))?;
            let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
            items.push(ActivityItem {
                id,
                user_id,
                action: match row.get::<String, _>("action").as_str() {
                    "created" => ActivityAction::Created,
                    "voted_up" => ActivityAction::VotedUp,
                    "voted_down" => ActivityAction::VotedDown,
                    "commented" => ActivityAction::Commented,
                    _ => ActivityAction::Created,
                },
                target_type: match row.get::<String, _>("target_type").as_str() {
                    "proposal" => ContentTargetType::Proposal,
                    "program" => ContentTargetType::Program,
                    "video" => ContentTargetType::Video,
                    "comment" => ContentTargetType::Comment,
                    _ => ContentTargetType::Proposal,
                },
                target_id,
                created_at,
                title: row.get("title"),
            });
        }

        debug!("activity.list_my_activity: count={}", items.len());
        Ok(items)
    }
}
