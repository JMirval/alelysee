use crate::types::Proposal;
use dioxus::prelude::*;

#[dioxus::prelude::post("/api/proposals/create")]
pub async fn create_proposal(
    id_token: String,
    title: String,
    summary: String,
    body_markdown: String,
    tags_csv: String,
) -> Result<Proposal, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, title, summary, body_markdown, tags_csv);
        Err(ServerFnError::new("create_proposal is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        use time::OffsetDateTime;
        use uuid::Uuid;

        let author_user_id = crate::auth::require_user_id(id_token).await?;
        let pool = crate::pool()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let tags: Vec<String> = tags_csv
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let row = sqlx::query(
            r#"
            insert into proposals (author_user_id, title, summary, body_markdown, tags)
            values ($1, $2, $3, $4, $5)
            returning id, author_user_id, title, summary, body_markdown, tags, created_at, updated_at
            "#,
        )
        .bind(author_user_id)
        .bind(&title)
        .bind(&summary)
        .bind(&body_markdown)
        .bind(&tags)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        // activity: created proposal
        sqlx::query(
            "insert into activity (user_id, action, target_type, target_id) values ($1, 'created', 'proposal', $2)",
        )
        .bind(author_user_id)
        .bind::<Uuid>(row.get("id"))
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(Proposal {
            id: row.get("id"),
            author_user_id: row.get("author_user_id"),
            title: row.get("title"),
            summary: row.get("summary"),
            body_markdown: row.get("body_markdown"),
            tags: row.get("tags"),
            created_at: row.get::<OffsetDateTime, _>("created_at"),
            updated_at: row.get::<OffsetDateTime, _>("updated_at"),
            vote_score: 0,
        })
    }
}

#[dioxus::prelude::get("/api/proposals/list")]
pub async fn list_proposals(limit: i64) -> Result<Vec<Proposal>, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = limit;
        Err(ServerFnError::new("list_proposals is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        use time::OffsetDateTime;

        let pool = crate::pool()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let rows = sqlx::query(
            r#"
            select
                p.id,
                p.author_user_id,
                p.title,
                p.summary,
                p.body_markdown,
                p.tags,
                p.created_at,
                p.updated_at,
                coalesce(sum(v.value), 0) as vote_score
            from proposals p
            left join votes v
                on v.target_type = 'proposal' and v.target_id = p.id
            group by p.id
            order by p.created_at desc
            limit $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| Proposal {
                id: row.get("id"),
                author_user_id: row.get("author_user_id"),
                title: row.get("title"),
                summary: row.get("summary"),
                body_markdown: row.get("body_markdown"),
                tags: row.get("tags"),
                created_at: row.get::<OffsetDateTime, _>("created_at"),
                updated_at: row.get::<OffsetDateTime, _>("updated_at"),
                vote_score: row.get::<i64, _>("vote_score"),
            })
            .collect())
    }
}

#[dioxus::prelude::get("/api/proposals/get/:id")]
pub async fn get_proposal(id: String) -> Result<Proposal, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = id;
        Err(ServerFnError::new("get_proposal is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        use time::OffsetDateTime;
        use uuid::Uuid;

        let pid = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("invalid id"))?;
        let pool = crate::pool()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let row = sqlx::query(
            r#"
            select
                p.id,
                p.author_user_id,
                p.title,
                p.summary,
                p.body_markdown,
                p.tags,
                p.created_at,
                p.updated_at,
                coalesce(sum(v.value), 0) as vote_score
            from proposals p
            left join votes v
                on v.target_type = 'proposal' and v.target_id = p.id
            where p.id = $1
            group by p.id
            "#,
        )
        .bind(pid)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(Proposal {
            id: row.get("id"),
            author_user_id: row.get("author_user_id"),
            title: row.get("title"),
            summary: row.get("summary"),
            body_markdown: row.get("body_markdown"),
            tags: row.get("tags"),
            created_at: row.get::<OffsetDateTime, _>("created_at"),
            updated_at: row.get::<OffsetDateTime, _>("updated_at"),
            vote_score: row.get::<i64, _>("vote_score"),
        })
    }
}

#[dioxus::prelude::post("/api/proposals/update")]
pub async fn update_proposal(
    id_token: String,
    id: String,
    title: String,
    summary: String,
    body_markdown: String,
    tags_csv: String,
) -> Result<Proposal, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, id, title, summary, body_markdown, tags_csv);
        Err(ServerFnError::new("update_proposal is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        use time::OffsetDateTime;
        use uuid::Uuid;

        let user_id = crate::auth::require_user_id(id_token).await?;
        let pid = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("invalid id"))?;
        let pool = crate::pool()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let owner =
            sqlx::query_scalar::<_, Uuid>("select author_user_id from proposals where id = $1")
                .bind(pid)
                .fetch_one(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
        if owner != user_id {
            return Err(ServerFnError::new("not allowed"));
        }

        let tags: Vec<String> = tags_csv
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let row = sqlx::query(
            r#"
            update proposals
            set title = $2,
                summary = $3,
                body_markdown = $4,
                tags = $5,
                updated_at = now()
            where id = $1
            returning id, author_user_id, title, summary, body_markdown, tags, created_at, updated_at
            "#,
        )
        .bind(pid)
        .bind(&title)
        .bind(&summary)
        .bind(&body_markdown)
        .bind(&tags)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let score = sqlx::query_scalar::<_, i64>(
            "select coalesce(sum(value), 0) from votes where target_type = 'proposal' and target_id = $1",
        )
        .bind(pid)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(Proposal {
            id: row.get("id"),
            author_user_id: row.get("author_user_id"),
            title: row.get("title"),
            summary: row.get("summary"),
            body_markdown: row.get("body_markdown"),
            tags: row.get("tags"),
            created_at: row.get::<OffsetDateTime, _>("created_at"),
            updated_at: row.get::<OffsetDateTime, _>("updated_at"),
            vote_score: score,
        })
    }
}
