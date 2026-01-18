use crate::types::Proposal;
use dioxus::prelude::*;
#[cfg(feature = "server")]
use tracing::{debug, info};

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

        info!(
            "proposals.create_proposal: title_len={} tags_len={}",
            title.len(),
            tags_csv.len()
        );
        let author_user_id = crate::auth::require_user_id(id_token).await?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let tags: Vec<String> = tags_csv
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        let tags_json = crate::db::tags_to_db(&tags)?;

        let sql = if crate::db::is_sqlite() {
            r#"
            insert into proposals (author_user_id, title, summary, body_markdown, tags)
            values ($1, $2, $3, $4, $5)
            returning
                CAST(id as TEXT) as id,
                CAST(author_user_id as TEXT) as author_user_id,
                title,
                summary,
                body_markdown,
                tags,
                CAST(created_at as TEXT) as created_at,
                CAST(updated_at as TEXT) as updated_at
            "#
        } else {
            r#"
            insert into proposals (author_user_id, title, summary, body_markdown, tags)
            values ($1, $2, $3, $4, ARRAY(SELECT jsonb_array_elements_text($5::jsonb)))
            returning
                CAST(id as TEXT) as id,
                CAST(author_user_id as TEXT) as author_user_id,
                title,
                summary,
                body_markdown,
                to_json(tags)::text as tags,
                CAST(created_at as TEXT) as created_at,
                CAST(updated_at as TEXT) as updated_at
            "#
        };

        let row = sqlx::query(sql)
            .bind(crate::db::uuid_to_db(author_user_id))
            .bind(&title)
            .bind(&summary)
            .bind(&body_markdown)
            .bind(&tags_json)
            .fetch_one(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        // activity: created proposal
        let proposal_id: String = row.get("id");
        info!("proposals.create_proposal: proposal_id={}", proposal_id);
        sqlx::query(
            "insert into activity (user_id, action, target_type, target_id) values ($1, 'created', 'proposal', $2)",
        )
        .bind(crate::db::uuid_to_db(author_user_id))
        .bind(&proposal_id)
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let id = crate::db::uuid_from_db(&proposal_id)?;
        let author_user_id = crate::db::uuid_from_db(&row.get::<String, _>("author_user_id"))?;
        let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
        let updated_at = crate::db::datetime_from_db(&row.get::<String, _>("updated_at"))?;

        Ok(Proposal {
            id,
            author_user_id,
            title: row.get("title"),
            summary: row.get("summary"),
            body_markdown: row.get("body_markdown"),
            tags: crate::db::tags_from_db(&row.get::<String, _>("tags"))?,
            created_at,
            updated_at,
            vote_score: 0,
        })
    }
}

#[dioxus::prelude::post("/api/proposals/list")]
pub async fn list_proposals(limit: i64) -> Result<Vec<Proposal>, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = limit;
        Err(ServerFnError::new("list_proposals is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;

        debug!("proposals.list_proposals: limit={}", limit);
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;
        let sql = if crate::db::is_sqlite() {
            r#"
            select
                CAST(p.id as TEXT) as id,
                CAST(p.author_user_id as TEXT) as author_user_id,
                p.title,
                p.summary,
                p.body_markdown,
                p.tags,
                CAST(p.created_at as TEXT) as created_at,
                CAST(p.updated_at as TEXT) as updated_at,
                coalesce(sum(v.value), 0) as vote_score
            from proposals p
            left join votes v
                on v.target_type = 'proposal' and v.target_id = p.id
            group by p.id
            order by p.created_at desc
            limit $1
            "#
        } else {
            r#"
            select
                CAST(p.id as TEXT) as id,
                CAST(p.author_user_id as TEXT) as author_user_id,
                p.title,
                p.summary,
                p.body_markdown,
                to_json(p.tags)::text as tags,
                CAST(p.created_at as TEXT) as created_at,
                CAST(p.updated_at as TEXT) as updated_at,
                coalesce(sum(v.value), 0) as vote_score
            from proposals p
            left join votes v
                on v.target_type = 'proposal' and v.target_id = p.id
            group by p.id
            order by p.created_at desc
            limit $1
            "#
        };

        let rows = sqlx::query(sql)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut proposals = Vec::with_capacity(rows.len());
        for row in rows {
            let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
            let author_user_id = crate::db::uuid_from_db(&row.get::<String, _>("author_user_id"))?;
            let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
            let updated_at = crate::db::datetime_from_db(&row.get::<String, _>("updated_at"))?;
            proposals.push(Proposal {
                id,
                author_user_id,
                title: row.get("title"),
                summary: row.get("summary"),
                body_markdown: row.get("body_markdown"),
                tags: crate::db::tags_from_db(&row.get::<String, _>("tags"))?,
                created_at,
                updated_at,
                vote_score: row.get::<i64, _>("vote_score"),
            });
        }

        debug!("proposals.list_proposals: count={}", proposals.len());
        Ok(proposals)
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
        use uuid::Uuid;

        debug!("proposals.get_proposal: id={}", id);
        let pid = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("invalid id"))?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let sql = if crate::db::is_sqlite() {
            r#"
            select
                CAST(p.id as TEXT) as id,
                CAST(p.author_user_id as TEXT) as author_user_id,
                p.title,
                p.summary,
                p.body_markdown,
                p.tags,
                CAST(p.created_at as TEXT) as created_at,
                CAST(p.updated_at as TEXT) as updated_at,
                coalesce(sum(v.value), 0) as vote_score
            from proposals p
            left join votes v
                on v.target_type = 'proposal' and v.target_id = p.id
            where p.id = $1
            group by p.id
            "#
        } else {
            r#"
            select
                CAST(p.id as TEXT) as id,
                CAST(p.author_user_id as TEXT) as author_user_id,
                p.title,
                p.summary,
                p.body_markdown,
                to_json(p.tags)::text as tags,
                CAST(p.created_at as TEXT) as created_at,
                CAST(p.updated_at as TEXT) as updated_at,
                coalesce(sum(v.value), 0) as vote_score
            from proposals p
            left join votes v
                on v.target_type = 'proposal' and v.target_id = p.id
            where p.id = $1
            group by p.id
            "#
        };

        let row = sqlx::query(sql)
            .bind(crate::db::uuid_to_db(pid))
            .fetch_one(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
        let author_user_id = crate::db::uuid_from_db(&row.get::<String, _>("author_user_id"))?;
        let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
        let updated_at = crate::db::datetime_from_db(&row.get::<String, _>("updated_at"))?;

        Ok(Proposal {
            id,
            author_user_id,
            title: row.get("title"),
            summary: row.get("summary"),
            body_markdown: row.get("body_markdown"),
            tags: crate::db::tags_from_db(&row.get::<String, _>("tags"))?,
            created_at,
            updated_at,
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
        use uuid::Uuid;

        info!("proposals.update_proposal: id={}", id);
        let user_id = crate::auth::require_user_id(id_token).await?;
        let pid = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("invalid id"))?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let owner = sqlx::query_scalar::<_, String>(
            "select CAST(author_user_id as TEXT) from proposals where id = $1",
        )
        .bind(crate::db::uuid_to_db(pid))
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let owner = crate::db::uuid_from_db(&owner)?;
        if owner != user_id {
            info!("proposals.update_proposal: forbidden user_id={}", user_id);
            return Err(ServerFnError::new("not allowed"));
        }

        let tags: Vec<String> = tags_csv
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        let tags_json = crate::db::tags_to_db(&tags)?;

        let sql = if crate::db::is_sqlite() {
            r#"
            update proposals
            set title = $2,
                summary = $3,
                body_markdown = $4,
                tags = $5,
                updated_at = now()
            where id = $1
            returning
                CAST(id as TEXT) as id,
                CAST(author_user_id as TEXT) as author_user_id,
                title,
                summary,
                body_markdown,
                tags,
                CAST(created_at as TEXT) as created_at,
                CAST(updated_at as TEXT) as updated_at
            "#
        } else {
            r#"
            update proposals
            set title = $2,
                summary = $3,
                body_markdown = $4,
                tags = ARRAY(SELECT jsonb_array_elements_text($5::jsonb)),
                updated_at = now()
            where id = $1
            returning
                CAST(id as TEXT) as id,
                CAST(author_user_id as TEXT) as author_user_id,
                title,
                summary,
                body_markdown,
                to_json(tags)::text as tags,
                CAST(created_at as TEXT) as created_at,
                CAST(updated_at as TEXT) as updated_at
            "#
        };

        let row = sqlx::query(sql)
            .bind(crate::db::uuid_to_db(pid))
            .bind(&title)
            .bind(&summary)
            .bind(&body_markdown)
            .bind(&tags_json)
            .fetch_one(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let score = sqlx::query_scalar::<_, i64>(
            "select coalesce(sum(value), 0) from votes where target_type = 'proposal' and target_id = $1",
        )
        .bind(crate::db::uuid_to_db(pid))
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
        let author_user_id = crate::db::uuid_from_db(&row.get::<String, _>("author_user_id"))?;
        let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
        let updated_at = crate::db::datetime_from_db(&row.get::<String, _>("updated_at"))?;

        Ok(Proposal {
            id,
            author_user_id,
            title: row.get("title"),
            summary: row.get("summary"),
            body_markdown: row.get("body_markdown"),
            tags: crate::db::tags_from_db(&row.get::<String, _>("tags"))?,
            created_at,
            updated_at,
            vote_score: score,
        })
    }
}
