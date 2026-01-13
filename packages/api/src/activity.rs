use crate::types::ActivityItem;
use dioxus::prelude::*;

#[dioxus::prelude::get("/api/activity/me")]
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
        use time::OffsetDateTime;
        use uuid::Uuid;

        let user_id = crate::auth::require_user_id(id_token).await?;
        let pool = crate::pool()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let rows = sqlx::query(
            r#"
            select
                a.id,
                a.user_id,
                a.action,
                a.target_type,
                a.target_id,
                a.created_at,
                case
                    when a.target_type = 'proposal' then (select title from proposals where id = a.target_id)
                    when a.target_type = 'program' then (select title from programs where id = a.target_id)
                    when a.target_type = 'comment' then (select left(body_markdown, 80) from comments where id = a.target_id)
                    when a.target_type = 'video' then (select storage_key from videos where id = a.target_id)
                    else null
                end as title
            from activity a
            where a.user_id = $1
            order by a.created_at desc
            limit $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| ActivityItem {
                id: row.get("id"),
                user_id: row.get("user_id"),
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
                target_id: row.get::<Uuid, _>("target_id"),
                created_at: row.get::<OffsetDateTime, _>("created_at"),
                title: row.get("title"),
            })
            .collect())
    }
}
