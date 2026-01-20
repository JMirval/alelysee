use crate::types::{ContentTargetType, Video};
use dioxus::prelude::*;
#[cfg(feature = "server")]
use tracing::{debug, info};

#[dioxus::prelude::post("/api/video_feed/mark_viewed")]
pub async fn mark_video_viewed(
    id_token: String,
    video_id: String,
) -> Result<(), ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, video_id);
        Err(ServerFnError::new("mark_video_viewed is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use uuid::Uuid;

        debug!("video_feed.mark_video_viewed: video_id={}", video_id);
        let user_id = crate::auth::require_user_id(id_token).await?;
        let vid = Uuid::parse_str(&video_id)
            .map_err(|_| ServerFnError::new("invalid video_id"))?;

        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        // Insert view record (ignore if duplicate due to unique constraint)
        let sql = if crate::db::is_sqlite() {
            r#"
            insert or ignore into video_views (user_id, video_id)
            values ($1, $2)
            "#
        } else {
            r#"
            insert into video_views (user_id, video_id)
            values ($1, $2)
            on conflict (user_id, video_id) do nothing
            "#
        };

        sqlx::query(sql)
            .bind(crate::db::uuid_to_db(user_id))
            .bind(crate::db::uuid_to_db(vid))
            .execute(pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        info!("video_feed.mark_video_viewed: recorded user_id={} video_id={}", user_id, vid);
        Ok(())
    }
}

#[dioxus::prelude::post("/api/video_feed/bookmark")]
pub async fn bookmark_video(
    id_token: String,
    video_id: String,
) -> Result<bool, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, video_id);
        Err(ServerFnError::new("bookmark_video is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        use uuid::Uuid;

        debug!("video_feed.bookmark_video: video_id={}", video_id);
        let user_id = crate::auth::require_user_id(id_token).await?;
        let vid = Uuid::parse_str(&video_id)
            .map_err(|_| ServerFnError::new("invalid video_id"))?;

        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        // Check if bookmark exists
        let exists = sqlx::query(
            "select 1 from bookmarks where user_id = $1 and video_id = $2"
        )
        .bind(crate::db::uuid_to_db(user_id))
        .bind(crate::db::uuid_to_db(vid))
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .is_some();

        if exists {
            // Remove bookmark
            sqlx::query("delete from bookmarks where user_id = $1 and video_id = $2")
                .bind(crate::db::uuid_to_db(user_id))
                .bind(crate::db::uuid_to_db(vid))
                .execute(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            info!("video_feed.bookmark_video: removed bookmark user_id={} video_id={}", user_id, vid);
            Ok(false)
        } else {
            // Add bookmark
            sqlx::query("insert into bookmarks (user_id, video_id) values ($1, $2)")
                .bind(crate::db::uuid_to_db(user_id))
                .bind(crate::db::uuid_to_db(vid))
                .execute(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            info!("video_feed.bookmark_video: added bookmark user_id={} video_id={}", user_id, vid);
            Ok(true)
        }
    }
}

#[dioxus::prelude::post("/api/video_feed/list_bookmarks")]
pub async fn list_bookmarked_videos(
    id_token: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<Video>, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, limit, offset);
        Err(ServerFnError::new("list_bookmarked_videos is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;

        debug!("video_feed.list_bookmarked_videos: limit={} offset={}", limit, offset);
        let user_id = crate::auth::require_user_id(id_token).await?;

        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let rows = sqlx::query(
            r#"
            select
                CAST(v.id as TEXT) as id,
                CAST(v.owner_user_id as TEXT) as owner_user_id,
                v.target_type,
                CAST(v.target_id as TEXT) as target_id,
                v.storage_bucket,
                v.storage_key,
                v.content_type,
                v.duration_seconds,
                CAST(v.created_at as TEXT) as created_at,
                coalesce(sum(vo.value), 0) as vote_score
            from videos v
            join bookmarks b on b.video_id = v.id
            left join votes vo on vo.target_type = 'video' and vo.target_id = v.id
            where b.user_id = $1
            group by v.id
            order by b.created_at desc
            limit $2 offset $3
            "#,
        )
        .bind(crate::db::uuid_to_db(user_id))
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut videos = Vec::with_capacity(rows.len());
        for row in rows {
            let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
            let owner_user_id = crate::db::uuid_from_db(&row.get::<String, _>("owner_user_id"))?;
            let target_id = crate::db::uuid_from_db(&row.get::<String, _>("target_id"))?;
            let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
            let target_type = match row.get::<String, _>("target_type").as_str() {
                "proposal" => ContentTargetType::Proposal,
                "program" => ContentTargetType::Program,
                "video" => ContentTargetType::Video,
                "comment" => ContentTargetType::Comment,
                _ => return Err(ServerFnError::new("invalid target_type")),
            };

            videos.push(Video {
                id,
                owner_user_id,
                target_type,
                target_id,
                storage_bucket: row.get("storage_bucket"),
                storage_key: row.get("storage_key"),
                content_type: row.get("content_type"),
                duration_seconds: row.get("duration_seconds"),
                created_at,
                vote_score: row.get::<i64, _>("vote_score"),
            });
        }

        debug!("video_feed.list_bookmarked_videos: count={}", videos.len());
        Ok(videos)
    }
}

#[dioxus::prelude::post("/api/video_feed/list_feed")]
pub async fn list_feed_videos(
    id_token: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<Video>, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, limit, offset);
        Err(ServerFnError::new("list_feed_videos is server-only"))
    }

    #[cfg(feature = "server")]
    {
        debug!("video_feed.list_feed_videos: limit={} offset={}", limit, offset);
        let user_id = crate::auth::require_user_id(id_token).await?;

        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        // Phase 1: Get collaborative filtering videos (40% weight)
        let collaborative_videos = get_collaborative_videos(user_id, pool).await?;

        // TODO: Phase 2: Get popular videos (30% weight)
        // TODO: Phase 3: Get interactive videos (30% weight)
        // TODO: Phase 4: Merge and shuffle with weights
        // TODO: Phase 5: Apply pagination

        // For now, just return collaborative videos
        Ok(collaborative_videos)
    }
}

#[cfg(feature = "server")]
async fn get_collaborative_videos(
    user_id: uuid::Uuid,
    pool: &sqlx::Pool<sqlx::Any>,
) -> Result<Vec<Video>, ServerFnError> {
    use sqlx::Row;

    // Find videos liked by users who liked videos you liked
    let rows = sqlx::query(
        r#"
        select distinct
            CAST(v.id as TEXT) as id,
            CAST(v.owner_user_id as TEXT) as owner_user_id,
            v.target_type,
            CAST(v.target_id as TEXT) as target_id,
            v.storage_bucket,
            v.storage_key,
            v.content_type,
            v.duration_seconds,
            CAST(v.created_at as TEXT) as created_at,
            coalesce(sum(vo.value), 0) as vote_score
        from videos v
        join votes vo on vo.target_type = 'video' and vo.target_id = v.id and vo.value = 1
        where vo.user_id in (
            select distinct vo2.user_id
            from votes vo2
            join votes vo3 on vo3.target_type = 'video' and vo3.value = 1 and vo3.user_id = $1
            where vo2.target_type = 'video'
                and vo2.value = 1
                and vo2.target_id = vo3.target_id
                and vo2.user_id != $1
        )
        and v.id not in (
            select video_id from video_views where user_id = $1
        )
        group by v.id
        limit 20
        "#,
    )
    .bind(crate::db::uuid_to_db(user_id))
    .fetch_all(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    parse_video_rows(rows)
}

#[cfg(feature = "server")]
fn parse_video_rows(rows: Vec<sqlx::any::AnyRow>) -> Result<Vec<Video>, ServerFnError> {
    use sqlx::Row;
    let mut videos = Vec::with_capacity(rows.len());

    for row in rows {
        let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
        let owner_user_id = crate::db::uuid_from_db(&row.get::<String, _>("owner_user_id"))?;
        let target_id = crate::db::uuid_from_db(&row.get::<String, _>("target_id"))?;
        let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
        let target_type = match row.get::<String, _>("target_type").as_str() {
            "proposal" => ContentTargetType::Proposal,
            "program" => ContentTargetType::Program,
            "video" => ContentTargetType::Video,
            "comment" => ContentTargetType::Comment,
            _ => return Err(ServerFnError::new("invalid target_type")),
        };

        videos.push(Video {
            id,
            owner_user_id,
            target_type,
            target_id,
            storage_bucket: row.get("storage_bucket"),
            storage_key: row.get("storage_key"),
            content_type: row.get("content_type"),
            duration_seconds: row.get("duration_seconds"),
            created_at,
            vote_score: row.get::<i64, _>("vote_score"),
        });
    }

    Ok(videos)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mark_video_viewed_prevents_duplicates() {
        // This will be implemented after we have test infrastructure
        // For now, just verify compilation
    }
}
