use crate::types::{Program, Proposal};
use dioxus::prelude::*;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProgramDetail {
    pub program: Program,
    pub proposals: Vec<Proposal>,
}

#[dioxus::prelude::post("/api/programs/create")]
pub async fn create_program(
    id_token: String,
    title: String,
    summary: String,
    body_markdown: String,
) -> Result<Program, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, title, summary, body_markdown);
        Err(ServerFnError::new("create_program is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;

        let author_user_id = crate::auth::require_user_id(id_token).await?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let row = sqlx::query(
            r#"
            insert into programs (author_user_id, title, summary, body_markdown)
            values ($1, $2, $3, $4)
            returning
                CAST(id as TEXT) as id,
                CAST(author_user_id as TEXT) as author_user_id,
                title,
                summary,
                body_markdown,
                CAST(created_at as TEXT) as created_at,
                CAST(updated_at as TEXT) as updated_at
            "#,
        )
        .bind(crate::db::uuid_to_db(author_user_id))
        .bind(&title)
        .bind(&summary)
        .bind(&body_markdown)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let program_id: String = row.get("id");
        sqlx::query(
            "insert into activity (user_id, action, target_type, target_id) values ($1, 'created', 'program', $2)",
        )
        .bind(crate::db::uuid_to_db(author_user_id))
        .bind(&program_id)
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let id = crate::db::uuid_from_db(&program_id)?;
        let author_user_id = crate::db::uuid_from_db(&row.get::<String, _>("author_user_id"))?;
        let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
        let updated_at = crate::db::datetime_from_db(&row.get::<String, _>("updated_at"))?;

        Ok(Program {
            id,
            author_user_id,
            title: row.get("title"),
            summary: row.get("summary"),
            body_markdown: row.get("body_markdown"),
            created_at,
            updated_at,
            vote_score: 0,
        })
    }
}

#[dioxus::prelude::post("/api/programs/add_item")]
pub async fn add_program_item(
    id_token: String,
    program_id: String,
    proposal_id: String,
    position: i32,
) -> Result<(), ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, program_id, proposal_id, position);
        Err(ServerFnError::new("add_program_item is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use uuid::Uuid;
        let user_id = crate::auth::require_user_id(id_token).await?;
        let pid =
            Uuid::parse_str(&program_id).map_err(|_| ServerFnError::new("invalid program_id"))?;
        let prop_id =
            Uuid::parse_str(&proposal_id).map_err(|_| ServerFnError::new("invalid proposal_id"))?;

        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        // Ownership check (program author)
        let owner = sqlx::query_scalar::<_, String>(
            "select CAST(author_user_id as TEXT) from programs where id = $1",
        )
        .bind(crate::db::uuid_to_db(pid))
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let owner = crate::db::uuid_from_db(&owner)?;
        if owner != user_id {
            return Err(ServerFnError::new("not allowed"));
        }

        sqlx::query(
            "insert into program_items (program_id, proposal_id, position) values ($1, $2, $3) on conflict (program_id, proposal_id) do update set position = excluded.position",
        )
        .bind(crate::db::uuid_to_db(pid))
        .bind(crate::db::uuid_to_db(prop_id))
        .bind(position)
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(())
    }
}

#[dioxus::prelude::get("/api/programs/list")]
pub async fn list_programs(limit: i64) -> Result<Vec<Program>, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = limit;
        Err(ServerFnError::new("list_programs is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;
        let rows = sqlx::query(
            r#"
            select
                CAST(p.id as TEXT) as id,
                CAST(p.author_user_id as TEXT) as author_user_id,
                p.title,
                p.summary,
                p.body_markdown,
                CAST(p.created_at as TEXT) as created_at,
                CAST(p.updated_at as TEXT) as updated_at,
                coalesce(sum(v.value), 0) as vote_score
            from programs p
            left join votes v
                on v.target_type = 'program' and v.target_id = p.id
            group by p.id
            order by p.created_at desc
            limit $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut programs = Vec::with_capacity(rows.len());
        for row in rows {
            let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
            let author_user_id = crate::db::uuid_from_db(&row.get::<String, _>("author_user_id"))?;
            let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
            let updated_at = crate::db::datetime_from_db(&row.get::<String, _>("updated_at"))?;
            programs.push(Program {
                id,
                author_user_id,
                title: row.get("title"),
                summary: row.get("summary"),
                body_markdown: row.get("body_markdown"),
                created_at,
                updated_at,
                vote_score: row.get::<i64, _>("vote_score"),
            });
        }

        Ok(programs)
    }
}

#[dioxus::prelude::get("/api/programs/get/:id")]
pub async fn get_program(id: String) -> Result<ProgramDetail, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = id;
        Err(ServerFnError::new("get_program is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        use uuid::Uuid;

        let program_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("invalid id"))?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let row = sqlx::query(
            r#"
            select
                CAST(p.id as TEXT) as id,
                CAST(p.author_user_id as TEXT) as author_user_id,
                p.title,
                p.summary,
                p.body_markdown,
                CAST(p.created_at as TEXT) as created_at,
                CAST(p.updated_at as TEXT) as updated_at,
                coalesce(sum(v.value), 0) as vote_score
            from programs p
            left join votes v
                on v.target_type = 'program' and v.target_id = p.id
            where p.id = $1
            group by p.id
            "#,
        )
        .bind(crate::db::uuid_to_db(program_id))
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
        let author_user_id = crate::db::uuid_from_db(&row.get::<String, _>("author_user_id"))?;
        let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
        let updated_at = crate::db::datetime_from_db(&row.get::<String, _>("updated_at"))?;

        let program = Program {
            id,
            author_user_id,
            title: row.get("title"),
            summary: row.get("summary"),
            body_markdown: row.get("body_markdown"),
            created_at,
            updated_at,
            vote_score: row.get::<i64, _>("vote_score"),
        };

        let sql = if crate::db::is_sqlite() {
            r#"
            select
                CAST(pr.id as TEXT) as id,
                CAST(pr.author_user_id as TEXT) as author_user_id,
                pr.title,
                pr.summary,
                pr.body_markdown,
                pr.tags,
                CAST(pr.created_at as TEXT) as created_at,
                CAST(pr.updated_at as TEXT) as updated_at,
                coalesce(sum(v.value), 0) as vote_score
            from program_items pi
            join proposals pr on pr.id = pi.proposal_id
            left join votes v
                on v.target_type = 'proposal' and v.target_id = pr.id
            where pi.program_id = $1
            group by pr.id, pi.position
            order by pi.position asc
            "#
        } else {
            r#"
            select
                CAST(pr.id as TEXT) as id,
                CAST(pr.author_user_id as TEXT) as author_user_id,
                pr.title,
                pr.summary,
                pr.body_markdown,
                to_json(pr.tags)::text as tags,
                CAST(pr.created_at as TEXT) as created_at,
                CAST(pr.updated_at as TEXT) as updated_at,
                coalesce(sum(v.value), 0) as vote_score
            from program_items pi
            join proposals pr on pr.id = pi.proposal_id
            left join votes v
                on v.target_type = 'proposal' and v.target_id = pr.id
            where pi.program_id = $1
            group by pr.id, pi.position
            order by pi.position asc
            "#
        };

        let proposal_rows = sqlx::query(sql)
            .bind(crate::db::uuid_to_db(program_id))
            .fetch_all(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut proposals = Vec::with_capacity(proposal_rows.len());
        for row in proposal_rows {
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

        Ok(ProgramDetail { program, proposals })
    }
}

#[dioxus::prelude::post("/api/programs/update")]
pub async fn update_program(
    id_token: String,
    id: String,
    title: String,
    summary: String,
    body_markdown: String,
) -> Result<Program, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, id, title, summary, body_markdown);
        Err(ServerFnError::new("update_program is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        use uuid::Uuid;

        let user_id = crate::auth::require_user_id(id_token).await?;
        let program_id = Uuid::parse_str(&id).map_err(|_| ServerFnError::new("invalid id"))?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let owner = sqlx::query_scalar::<_, String>(
            "select CAST(author_user_id as TEXT) from programs where id = $1",
        )
        .bind(crate::db::uuid_to_db(program_id))
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let owner = crate::db::uuid_from_db(&owner)?;
        if owner != user_id {
            return Err(ServerFnError::new("not allowed"));
        }

        let row = sqlx::query(
            r#"
            update programs
            set title = $2,
                summary = $3,
                body_markdown = $4,
                updated_at = now()
            where id = $1
            returning
                CAST(id as TEXT) as id,
                CAST(author_user_id as TEXT) as author_user_id,
                title,
                summary,
                body_markdown,
                CAST(created_at as TEXT) as created_at,
                CAST(updated_at as TEXT) as updated_at
            "#,
        )
        .bind(crate::db::uuid_to_db(program_id))
        .bind(&title)
        .bind(&summary)
        .bind(&body_markdown)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let score = sqlx::query_scalar::<_, i64>(
            "select coalesce(sum(value), 0) from votes where target_type = 'program' and target_id = $1",
        )
        .bind(crate::db::uuid_to_db(program_id))
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
        let author_user_id = crate::db::uuid_from_db(&row.get::<String, _>("author_user_id"))?;
        let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
        let updated_at = crate::db::datetime_from_db(&row.get::<String, _>("updated_at"))?;

        Ok(Program {
            id,
            author_user_id,
            title: row.get("title"),
            summary: row.get("summary"),
            body_markdown: row.get("body_markdown"),
            created_at,
            updated_at,
            vote_score: score,
        })
    }
}
