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

        // Phase 2: Get popular videos (30% weight)
        let popular_videos = get_popular_videos(user_id, pool).await?;

        // Phase 3: Get interactive videos (30% weight)
        let interactive_videos = get_interactive_videos(user_id, pool).await?;

        // Phase 4: Merge and shuffle with weights
        let feed = merge_and_shuffle(collaborative_videos, popular_videos, interactive_videos);

        // TODO: Phase 5: Check if feed is empty and reset views

        // Phase 6: Apply pagination
        let total = feed.len();
        let start = offset.min(total as i64) as usize;
        let end = (offset + limit).min(total as i64) as usize;
        let paginated_feed = feed[start..end].to_vec();

        debug!("video_feed.list_feed_videos: total={} returning={}", total, paginated_feed.len());
        Ok(paginated_feed)
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
async fn get_popular_videos(
    user_id: uuid::Uuid,
    pool: &sqlx::Pool<sqlx::Any>,
) -> Result<Vec<Video>, ServerFnError> {
    use sqlx::Row;

    // Videos with highest vote scores in past 7 days
    let sql = if crate::db::is_sqlite() {
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
        left join votes vo on vo.target_type = 'video' and vo.target_id = v.id
        where v.created_at > datetime('now', '-7 days')
            and v.id not in (
                select video_id from video_views where user_id = $1
            )
        group by v.id
        order by vote_score desc
        limit 15
        "#
    } else {
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
        left join votes vo on vo.target_type = 'video' and vo.target_id = v.id
        where v.created_at > now() - interval '7 days'
            and v.id not in (
                select video_id from video_views where user_id = $1
            )
        group by v.id
        order by vote_score desc
        limit 15
        "#
    };

    let rows = sqlx::query(sql)
        .bind(crate::db::uuid_to_db(user_id))
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parse_video_rows(rows)
}

#[cfg(feature = "server")]
async fn get_interactive_videos(
    user_id: uuid::Uuid,
    pool: &sqlx::Pool<sqlx::Any>,
) -> Result<Vec<Video>, ServerFnError> {
    use sqlx::Row;

    // Videos with most votes + comments (comments weighted 2x)
    let sql = if crate::db::is_sqlite() {
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
            coalesce(sum(vo.value), 0) as vote_score,
            (count(distinct vo.id) + count(distinct c.id) * 2) as interaction_score
        from videos v
        left join votes vo on vo.target_type = 'video' and vo.target_id = v.id
        left join comments c on c.target_type = 'video' and c.target_id = v.id
        where v.created_at > datetime('now', '-7 days')
            and v.id not in (
                select video_id from video_views where user_id = $1
            )
        group by v.id
        order by interaction_score desc
        limit 15
        "#
    } else {
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
            coalesce(sum(vo.value), 0) as vote_score,
            (count(distinct vo.id) + count(distinct c.id) * 2) as interaction_score
        from videos v
        left join votes vo on vo.target_type = 'video' and vo.target_id = v.id
        left join comments c on c.target_type = 'video' and c.target_id = v.id
        where v.created_at > now() - interval '7 days'
            and v.id not in (
                select video_id from video_views where user_id = $1
            )
        group by v.id
        order by interaction_score desc
        limit 15
        "#
    };

    let rows = sqlx::query(sql)
        .bind(crate::db::uuid_to_db(user_id))
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    parse_video_rows(rows)
}

#[cfg(feature = "server")]
fn merge_and_shuffle(
    collaborative: Vec<Video>,
    popular: Vec<Video>,
    interactive: Vec<Video>,
) -> Vec<Video> {
    use std::collections::HashSet;
    use uuid::Uuid;

    let mut result = Vec::new();
    let mut seen_ids: HashSet<Uuid> = HashSet::new();

    // Add videos with weighted sampling: 40% collaborative, 30% popular, 30% interactive
    let mut collab_idx = 0;
    let mut popular_idx = 0;
    let mut interactive_idx = 0;

    // Simple weighted round-robin: 4 collab, 3 popular, 3 interactive, repeat
    let pattern = vec![0, 0, 0, 0, 1, 1, 1, 2, 2, 2]; // 4:3:3 ratio

    let mut pattern_idx = 0;
    let max_iterations = collaborative.len() + popular.len() + interactive.len();

    for _ in 0..max_iterations {
        let source = pattern[pattern_idx % pattern.len()];
        pattern_idx += 1;

        let video = match source {
            0 => {
                if collab_idx < collaborative.len() {
                    let v = &collaborative[collab_idx];
                    collab_idx += 1;
                    Some(v.clone())
                } else {
                    None
                }
            }
            1 => {
                if popular_idx < popular.len() {
                    let v = &popular[popular_idx];
                    popular_idx += 1;
                    Some(v.clone())
                } else {
                    None
                }
            }
            2 => {
                if interactive_idx < interactive.len() {
                    let v = &interactive[interactive_idx];
                    interactive_idx += 1;
                    Some(v.clone())
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(v) = video {
            if !seen_ids.contains(&v.id) {
                seen_ids.insert(v.id);
                result.push(v);
            }
        }

        // Break if all sources exhausted
        if collab_idx >= collaborative.len()
            && popular_idx >= popular.len()
            && interactive_idx >= interactive.len()
        {
            break;
        }
    }

    result
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
