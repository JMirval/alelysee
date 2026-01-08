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
        use time::OffsetDateTime;
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

        let pool = crate::pool()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let row = sqlx::query(
            r#"
            insert into comments (author_user_id, target_type, target_id, parent_comment_id, body_markdown)
            values ($1, $2, $3, $4, $5)
            returning id, author_user_id, target_type, target_id, parent_comment_id, body_markdown, created_at
            "#,
        )
        .bind(author_user_id)
        .bind(target_type.as_db())
        .bind(tid)
        .bind(parent_id)
        .bind(&body_markdown)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let cid: Uuid = row.get("id");

        let _ = sqlx::query(
            "insert into activity (user_id, action, target_type, target_id) values ($1, 'commented', $2, $3)",
        )
        .bind(author_user_id)
        .bind(target_type.as_db())
        .bind(tid)
        .execute(pool)
        .await;

        Ok(Comment {
            id: cid,
            author_user_id: row.get("author_user_id"),
            target_type,
            target_id: tid,
            parent_comment_id: row.get("parent_comment_id"),
            body_markdown: row.get("body_markdown"),
            created_at: row.get::<OffsetDateTime, _>("created_at"),
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
        use time::OffsetDateTime;
        use uuid::Uuid;

        let tid =
            Uuid::parse_str(&target_id).map_err(|_| ServerFnError::new("invalid target_id"))?;
        let pool = crate::pool()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let rows = sqlx::query(
            r#"
            select
                c.id,
                c.author_user_id,
                c.parent_comment_id,
                c.body_markdown,
                c.created_at,
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
        .bind(tid)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| Comment {
                id: row.get("id"),
                author_user_id: row.get("author_user_id"),
                target_type,
                target_id: tid,
                parent_comment_id: row.get("parent_comment_id"),
                body_markdown: row.get("body_markdown"),
                created_at: row.get::<OffsetDateTime, _>("created_at"),
                vote_score: row.get::<i64, _>("vote_score"),
            })
            .collect())
    }
}
