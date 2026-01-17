use crate::types::{Comment, ContentTargetType};
use dioxus::prelude::*;

#[dioxus::prelude::post("/api/comments/create")]
pub async fn create_comment(
    id_token: String,
    target_type: ContentTargetType,
    target_id: String,
    parent_comment_id: Option<String>,
    body_markdown: String,
) -> Result<Comment, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (
            id_token,
            target_type,
            target_id,
            parent_comment_id,
            body_markdown,
        );
        Err(ServerFnError::new("create_comment is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        use uuid::Uuid;

        let author_user_id = crate::auth::require_user_id(id_token).await?;
        let tid =
            Uuid::parse_str(&target_id).map_err(|_| ServerFnError::new("invalid target_id"))?;
        let parent_id = match parent_comment_id {
            None => None,
            Some(s) if s.trim().is_empty() => None,
            Some(s) => Some(
                Uuid::parse_str(&s).map_err(|_| ServerFnError::new("invalid parent_comment_id"))?,
            ),
        };

        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let parent_id_db = parent_id.map(crate::db::uuid_to_db);
        let row = sqlx::query(
            r#"
            insert into comments (author_user_id, target_type, target_id, parent_comment_id, body_markdown)
            values ($1, $2, $3, $4, $5)
            returning
                CAST(id as TEXT) as id,
                CAST(author_user_id as TEXT) as author_user_id,
                target_type,
                CAST(target_id as TEXT) as target_id,
                CAST(parent_comment_id as TEXT) as parent_comment_id,
                body_markdown,
                CAST(created_at as TEXT) as created_at
            "#,
        )
        .bind(crate::db::uuid_to_db(author_user_id))
        .bind(target_type.as_db())
        .bind(crate::db::uuid_to_db(tid))
        .bind(parent_id_db)
        .bind(&body_markdown)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let cid = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;

        let _ = sqlx::query(
            "insert into activity (user_id, action, target_type, target_id) values ($1, 'commented', $2, $3)",
        )
        .bind(crate::db::uuid_to_db(author_user_id))
        .bind(target_type.as_db())
        .bind(crate::db::uuid_to_db(tid))
        .execute(pool)
        .await;

        let author_user_id = crate::db::uuid_from_db(&row.get::<String, _>("author_user_id"))?;
        let parent_comment_id = match row.get::<Option<String>, _>("parent_comment_id") {
            Some(value) => Some(crate::db::uuid_from_db(&value)?),
            None => None,
        };
        let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;

        Ok(Comment {
            id: cid,
            author_user_id,
            target_type,
            target_id: tid,
            parent_comment_id,
            body_markdown: row.get("body_markdown"),
            created_at,
            vote_score: 0,
        })
    }
}

#[dioxus::prelude::get("/api/comments/list")]
pub async fn list_comments(
    target_type: ContentTargetType,
    target_id: String,
    limit: i64,
) -> Result<Vec<Comment>, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (target_type, target_id, limit);
        Err(ServerFnError::new("list_comments is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        use uuid::Uuid;

        let tid =
            Uuid::parse_str(&target_id).map_err(|_| ServerFnError::new("invalid target_id"))?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let rows = sqlx::query(
            r#"
            select
                CAST(c.id as TEXT) as id,
                CAST(c.author_user_id as TEXT) as author_user_id,
                CAST(c.parent_comment_id as TEXT) as parent_comment_id,
                c.body_markdown,
                CAST(c.created_at as TEXT) as created_at,
                coalesce(sum(v.value), 0) as vote_score
            from comments c
            left join votes v
                on v.target_type = 'comment' and v.target_id = c.id
            where c.target_type = $1 and c.target_id = $2
            group by c.id
            order by c.created_at asc
            limit $3
            "#,
        )
        .bind(target_type.as_db())
        .bind(crate::db::uuid_to_db(tid))
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut comments = Vec::with_capacity(rows.len());
        for row in rows {
            let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
            let author_user_id = crate::db::uuid_from_db(&row.get::<String, _>("author_user_id"))?;
            let parent_comment_id = match row.get::<Option<String>, _>("parent_comment_id") {
                Some(value) => Some(crate::db::uuid_from_db(&value)?),
                None => None,
            };
            let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
            comments.push(Comment {
                id,
                author_user_id,
                target_type,
                target_id: tid,
                parent_comment_id,
                body_markdown: row.get("body_markdown"),
                created_at,
                vote_score: row.get::<i64, _>("vote_score"),
            });
        }

        Ok(comments)
    }
}
