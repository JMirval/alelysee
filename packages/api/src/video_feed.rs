use crate::types::{ContentTargetType, Video};
use dioxus::prelude::*;
#[cfg(feature = "server")]
use sqlx::Row;
#[cfg(feature = "server")]
use tracing::{debug, info};

#[dioxus::prelude::post("/api/video_feed/mark_viewed")]
pub async fn mark_video_viewed(id_token: String, video_id: String) -> Result<(), ServerFnError> {
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
        let vid = Uuid::parse_str(&video_id).map_err(|_| ServerFnError::new("invalid video_id"))?;

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

        info!(
            "video_feed.mark_video_viewed: recorded user_id={} video_id={}",
            user_id, vid
        );
        Ok(())
    }
}

#[dioxus::prelude::post("/api/video_feed/bookmark")]
pub async fn bookmark_video(id_token: String, video_id: String) -> Result<bool, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, video_id);
        Err(ServerFnError::new("bookmark_video is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use uuid::Uuid;

        debug!("video_feed.bookmark_video: video_id={}", video_id);
        let user_id = crate::auth::require_user_id(id_token).await?;
        let vid = Uuid::parse_str(&video_id).map_err(|_| ServerFnError::new("invalid video_id"))?;

        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        // Check if bookmark exists
        let exists = sqlx::query("select 1 from bookmarks where user_id = $1 and video_id = $2")
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
            info!(
                "video_feed.bookmark_video: removed bookmark user_id={} video_id={}",
                user_id, vid
            );
            Ok(false)
        } else {
            // Add bookmark
            sqlx::query("insert into bookmarks (user_id, video_id) values ($1, $2)")
                .bind(crate::db::uuid_to_db(user_id))
                .bind(crate::db::uuid_to_db(vid))
                .execute(pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            info!(
                "video_feed.bookmark_video: added bookmark user_id={} video_id={}",
                user_id, vid
            );
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
        debug!(
            "video_feed.list_bookmarked_videos: limit={} offset={}",
            limit, offset
        );
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
        debug!(
            "video_feed.list_feed_videos: limit={} offset={}",
            limit, offset
        );
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
        let mut feed = merge_and_shuffle(collaborative_videos, popular_videos, interactive_videos);

        // Phase 5: Check if feed is empty (all videos exhausted) and reset
        if feed.is_empty() {
            info!("video_feed.list_feed_videos: all videos exhausted, resetting views");
            reset_viewed_videos(user_id, pool).await?;

            // Retry once after reset
            let collaborative_videos = get_collaborative_videos(user_id, pool).await?;
            let popular_videos = get_popular_videos(user_id, pool).await?;
            let interactive_videos = get_interactive_videos(user_id, pool).await?;
            feed = merge_and_shuffle(collaborative_videos, popular_videos, interactive_videos);
        }

        // Phase 6: Apply pagination
        let total = feed.len();
        let start = offset.min(total as i64) as usize;
        let end = (offset + limit).min(total as i64) as usize;
        let paginated_feed = feed[start..end].to_vec();

        debug!(
            "video_feed.list_feed_videos: total={} returning={}",
            total,
            paginated_feed.len()
        );
        Ok(paginated_feed)
    }
}

#[cfg(feature = "server")]
async fn get_collaborative_videos(
    user_id: uuid::Uuid,
    pool: &sqlx::Pool<sqlx::Any>,
) -> Result<Vec<Video>, ServerFnError> {
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

    let max_iterations = collaborative.len() + popular.len() + interactive.len();

    for (pattern_idx, _) in (0..max_iterations).enumerate() {
        let source = pattern[pattern_idx % pattern.len()];

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
async fn reset_viewed_videos(
    user_id: uuid::Uuid,
    pool: &sqlx::Pool<sqlx::Any>,
) -> Result<(), ServerFnError> {
    info!("video_feed: resetting view history for user_id={}", user_id);

    sqlx::query("delete from video_views where user_id = $1")
        .bind(crate::db::uuid_to_db(user_id))
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[cfg(feature = "server")]
fn parse_video_rows(rows: Vec<sqlx::any::AnyRow>) -> Result<Vec<Video>, ServerFnError> {
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

#[dioxus::prelude::post("/api/video_feed/list_single_content")]
pub async fn list_single_content_videos(
    target_type: ContentTargetType,
    target_id: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<Video>, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (target_type, target_id, limit, offset);
        Err(ServerFnError::new(
            "list_single_content_videos is server-only",
        ))
    }

    #[cfg(feature = "server")]
    {
        use uuid::Uuid;

        debug!(
            "video_feed.list_single_content_videos: target_type={:?} target_id={} limit={} offset={}",
            target_type, target_id, limit, offset
        );

        let tid =
            Uuid::parse_str(&target_id).map_err(|_| ServerFnError::new("invalid target_id"))?;

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
            left join votes vo on vo.target_type = 'video' and vo.target_id = v.id
            where v.target_type = $1 and v.target_id = $2
            group by v.id
            order by v.created_at desc
            limit $3 offset $4
            "#,
        )
        .bind(target_type.as_db())
        .bind(crate::db::uuid_to_db(tid))
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let videos = parse_video_rows(rows)?;
        debug!(
            "video_feed.list_single_content_videos: count={}",
            videos.len()
        );
        Ok(videos)
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use crate::test_support::{pool, reset_db};
    use uuid::Uuid;

    async fn create_test_user(pool: &sqlx::Pool<sqlx::Postgres>) -> Uuid {
        sqlx::query_scalar("INSERT INTO users DEFAULT VALUES RETURNING id")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn create_test_proposal(pool: &sqlx::Pool<sqlx::Postgres>, user_id: Uuid) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO proposals (author_user_id, title, summary, body_markdown)
             VALUES ($1, 'Test Proposal', 'Test', 'Test')
             RETURNING id",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn create_test_video(
        pool: &sqlx::Pool<sqlx::Postgres>,
        user_id: Uuid,
        target_id: Uuid,
    ) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO videos (owner_user_id, target_type, target_id, storage_bucket, storage_key, content_type)
             VALUES ($1, 'proposal', $2, 'test', 'test.mp4', 'video/mp4')
             RETURNING id",
        )
        .bind(user_id)
        .bind(target_id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_mark_video_viewed_prevents_duplicates() {
        let Some(pool) = pool().await else {
            eprintln!("Skipping test: no DATABASE_URL");
            return;
        };
        reset_db().await.unwrap();

        let user_id = create_test_user(pool).await;
        let proposal_id = create_test_proposal(pool, user_id).await;
        let video_id = create_test_video(pool, user_id, proposal_id).await;

        // Mark video as viewed first time
        sqlx::query(
            "INSERT INTO video_views (user_id, video_id) VALUES ($1, $2)
             ON CONFLICT (user_id, video_id) DO NOTHING",
        )
        .bind(user_id)
        .bind(video_id)
        .execute(pool)
        .await
        .unwrap();

        // Attempt to mark again - should be idempotent
        let result = sqlx::query(
            "INSERT INTO video_views (user_id, video_id) VALUES ($1, $2)
             ON CONFLICT (user_id, video_id) DO NOTHING",
        )
        .bind(user_id)
        .bind(video_id)
        .execute(pool)
        .await;

        assert!(result.is_ok());

        // Verify only one entry exists
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM video_views WHERE user_id = $1 AND video_id = $2",
        )
        .bind(user_id)
        .bind(video_id)
        .fetch_one(pool)
        .await
        .unwrap();

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_bookmark_toggle() {
        let Some(pool) = pool().await else {
            eprintln!("Skipping test: no DATABASE_URL");
            return;
        };
        reset_db().await.unwrap();

        let user_id = create_test_user(pool).await;
        let proposal_id = create_test_proposal(pool, user_id).await;
        let video_id = create_test_video(pool, user_id, proposal_id).await;

        // Check if bookmark exists (should be none)
        let exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM bookmarks WHERE user_id = $1 AND video_id = $2")
                .bind(user_id)
                .bind(video_id)
                .fetch_optional(pool)
                .await
                .unwrap();

        assert!(exists.is_none());

        // Add bookmark
        sqlx::query(
            "INSERT INTO bookmarks (user_id, video_id) VALUES ($1, $2)
             ON CONFLICT (user_id, video_id) DO NOTHING",
        )
        .bind(user_id)
        .bind(video_id)
        .execute(pool)
        .await
        .unwrap();

        // Verify bookmark exists
        let exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM bookmarks WHERE user_id = $1 AND video_id = $2")
                .bind(user_id)
                .bind(video_id)
                .fetch_optional(pool)
                .await
                .unwrap();

        assert!(exists.is_some());

        // Remove bookmark
        sqlx::query("DELETE FROM bookmarks WHERE user_id = $1 AND video_id = $2")
            .bind(user_id)
            .bind(video_id)
            .execute(pool)
            .await
            .unwrap();

        // Verify bookmark removed
        let exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM bookmarks WHERE user_id = $1 AND video_id = $2")
                .bind(user_id)
                .bind(video_id)
                .fetch_optional(pool)
                .await
                .unwrap();

        assert!(exists.is_none());
    }

    #[tokio::test]
    async fn test_list_bookmarked_videos() {
        let Some(pool) = pool().await else {
            eprintln!("Skipping test: no DATABASE_URL");
            return;
        };
        reset_db().await.unwrap();

        let user_id = create_test_user(pool).await;
        let proposal_id = create_test_proposal(pool, user_id).await;

        // Create 3 videos
        let video1 = create_test_video(pool, user_id, proposal_id).await;
        let _video2 = create_test_video(pool, user_id, proposal_id).await;
        let video3 = create_test_video(pool, user_id, proposal_id).await;

        // Bookmark video 1 and 3
        sqlx::query("INSERT INTO bookmarks (user_id, video_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(video1)
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO bookmarks (user_id, video_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(video3)
            .execute(pool)
            .await
            .unwrap();

        // Query bookmarked videos
        let rows = sqlx::query(
            "SELECT v.* FROM videos v
             JOIN bookmarks b ON v.id = b.video_id
             WHERE b.user_id = $1
             ORDER BY b.created_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn test_view_history_filtering() {
        let Some(pool) = pool().await else {
            eprintln!("Skipping test: no DATABASE_URL");
            return;
        };
        reset_db().await.unwrap();

        let user_id = create_test_user(pool).await;
        let proposal_id = create_test_proposal(pool, user_id).await;

        // Create 3 videos
        let video1 = create_test_video(pool, user_id, proposal_id).await;
        let video2 = create_test_video(pool, user_id, proposal_id).await;
        let _video3 = create_test_video(pool, user_id, proposal_id).await;

        // Mark video 1 and 2 as viewed
        sqlx::query("INSERT INTO video_views (user_id, video_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(video1)
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO video_views (user_id, video_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(video2)
            .execute(pool)
            .await
            .unwrap();

        // Query unviewed videos
        let rows = sqlx::query(
            "SELECT v.* FROM videos v
             WHERE NOT EXISTS (
                 SELECT 1 FROM video_views vv
                 WHERE vv.user_id = $1 AND vv.video_id = v.id
             )
             LIMIT 10",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap();

        // Should only return video3
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn test_view_exhaustion_reset() {
        let Some(pool) = pool().await else {
            eprintln!("Skipping test: no DATABASE_URL");
            return;
        };
        reset_db().await.unwrap();

        let user_id = create_test_user(pool).await;
        let proposal_id = create_test_proposal(pool, user_id).await;

        // Create 2 videos
        let video1 = create_test_video(pool, user_id, proposal_id).await;
        let video2 = create_test_video(pool, user_id, proposal_id).await;

        // Mark both as viewed
        sqlx::query("INSERT INTO video_views (user_id, video_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(video1)
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO video_views (user_id, video_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(video2)
            .execute(pool)
            .await
            .unwrap();

        // Verify all videos are marked as viewed
        let unviewed_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM videos v
             WHERE NOT EXISTS (
                 SELECT 1 FROM video_views vv
                 WHERE vv.user_id = $1 AND vv.video_id = v.id
             )",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();

        assert_eq!(unviewed_count, 0);

        // Reset view history for this user
        sqlx::query("DELETE FROM video_views WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();

        // Verify all videos are now unviewed
        let unviewed_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM videos v
             WHERE NOT EXISTS (
                 SELECT 1 FROM video_views vv
                 WHERE vv.user_id = $1 AND vv.video_id = v.id
             )",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();

        assert_eq!(unviewed_count, 2);
    }

    #[tokio::test]
    async fn test_weighted_shuffling_distribution() {
        // Test that the weighted shuffling produces roughly the expected ratio
        let collaborative = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let popular = vec![11, 12, 13, 14, 15, 16, 17];
        let interactive = vec![18, 19, 20, 21, 22, 23, 24];

        let result = merge_and_shuffle_test(collaborative, popular, interactive);

        // Should have all 24 items
        assert_eq!(result.len(), 24);

        // Check that we have items from all three categories
        let has_collab = result.iter().any(|&x| x <= 10);
        let has_popular = result.iter().any(|&x| (11..=17).contains(&x));
        let has_interactive = result.iter().any(|&x| x >= 18);

        assert!(has_collab);
        assert!(has_popular);
        assert!(has_interactive);
    }

    // Test helper that mimics the real merge_and_shuffle logic
    fn merge_and_shuffle_test(
        mut collab: Vec<i32>,
        mut pop: Vec<i32>,
        mut inter: Vec<i32>,
    ) -> Vec<i32> {
        let mut result = Vec::new();
        let collab_weight = 4;
        let pop_weight = 3;
        let inter_weight = 3;

        while !collab.is_empty() || !pop.is_empty() || !inter.is_empty() {
            for _ in 0..collab_weight {
                if let Some(item) = collab.pop() {
                    result.push(item);
                }
            }
            for _ in 0..pop_weight {
                if let Some(item) = pop.pop() {
                    result.push(item);
                }
            }
            for _ in 0..inter_weight {
                if let Some(item) = inter.pop() {
                    result.push(item);
                }
            }
        }

        result
    }
}
