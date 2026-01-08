use crate::types::{ContentTargetType, UploadIntent, Video};
use dioxus::prelude::*;

#[dioxus::prelude::post("/api/uploads/video_intent")]
pub async fn create_video_upload_intent(
    id_token: String,
    target_type: ContentTargetType,
    target_id: String,
    content_type: String,
    byte_size: i64,
) -> Result<UploadIntent, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, target_type, target_id, content_type, byte_size);
        Err(ServerFnError::new(
            "create_video_upload_intent is server-only",
        ))
    }

    #[cfg(feature = "server")]
    {
        use aws_sdk_s3::presigning::PresigningConfig;
        use std::time::Duration;
        use uuid::Uuid;

        const MAX_BYTES: i64 = 200 * 1024 * 1024; // 200MB MVP limit
        if byte_size <= 0 || byte_size > MAX_BYTES {
            return Err(ServerFnError::new("invalid file size"));
        }

        // Ensure authenticated user exists (and we record ownership at finalize time).
        let _user_id = crate::auth::require_user_id(id_token).await?;

        let bucket =
            std::env::var("S3_BUCKET").map_err(|_| ServerFnError::new("S3_BUCKET not set"))?;

        let key = format!(
            "videos/{}/{}/{}",
            target_type.as_db(),
            target_id,
            Uuid::new_v4()
        );

        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = aws_sdk_s3::Client::new(&config);

        let presigned = client
            .put_object()
            .bucket(&bucket)
            .key(&key)
            .content_type(content_type)
            .presigned(
                PresigningConfig::expires_in(Duration::from_secs(60 * 10))
                    .map_err(|_| ServerFnError::new("presign config error"))?,
            )
            .await
            .map_err(|e| ServerFnError::new(format!("presign error: {e}")))?;

        Ok(UploadIntent {
            presigned_put_url: presigned.uri().to_string(),
            s3_key: key,
            bucket,
        })
    }
}

#[dioxus::prelude::post("/api/uploads/finalize_video")]
pub async fn finalize_video_upload(
    id_token: String,
    target_type: ContentTargetType,
    target_id: String,
    s3_key: String,
    content_type: String,
) -> Result<Video, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, target_type, target_id, s3_key, content_type);
        Err(ServerFnError::new("finalize_video_upload is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        use time::OffsetDateTime;
        use uuid::Uuid;

        let owner_user_id = crate::auth::require_user_id(id_token).await?;
        let tid =
            Uuid::parse_str(&target_id).map_err(|_| ServerFnError::new("invalid target_id"))?;
        let bucket =
            std::env::var("S3_BUCKET").map_err(|_| ServerFnError::new("S3_BUCKET not set"))?;

        // Best-effort: ensure object exists.
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = aws_sdk_s3::Client::new(&config);
        client
            .head_object()
            .bucket(&bucket)
            .key(&s3_key)
            .send()
            .await
            .map_err(|e| ServerFnError::new(format!("head_object failed: {e}")))?;

        let pool = crate::pool()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

        let row = sqlx::query(
            r#"
            insert into videos (owner_user_id, target_type, target_id, s3_bucket, s3_key, content_type)
            values ($1, $2, $3, $4, $5, $6)
            returning id, owner_user_id, target_type, target_id, s3_bucket, s3_key, content_type, duration_seconds, created_at
            "#,
        )
        .bind(owner_user_id)
        .bind(target_type.as_db())
        .bind(tid)
        .bind(&bucket)
        .bind(&s3_key)
        .bind(&content_type)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let vid: Uuid = row.get("id");
        let _ = sqlx::query(
            "insert into activity (user_id, action, target_type, target_id) values ($1, 'created', 'video', $2)",
        )
        .bind(owner_user_id)
        .bind(vid)
        .execute(pool)
        .await;

        Ok(Video {
            id: row.get("id"),
            owner_user_id: row.get("owner_user_id"),
            target_type,
            target_id: row.get("target_id"),
            s3_bucket: row.get("s3_bucket"),
            s3_key: row.get("s3_key"),
            content_type: row.get("content_type"),
            duration_seconds: row.get("duration_seconds"),
            created_at: row.get::<OffsetDateTime, _>("created_at"),
            vote_score: 0,
        })
    }
}

#[dioxus::prelude::get("/api/videos/list")]
pub async fn list_videos(
    target_type: ContentTargetType,
    target_id: String,
    limit: i64,
) -> Result<Vec<Video>, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (target_type, target_id, limit);
        Err(ServerFnError::new("list_videos is server-only"))
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
                v.id,
                v.owner_user_id,
                v.target_id,
                v.s3_bucket,
                v.s3_key,
                v.content_type,
                v.duration_seconds,
                v.created_at,
                coalesce(sum(vo.value), 0) as vote_score
            from videos v
            left join votes vo
                on vo.target_type = 'video' and vo.target_id = v.id
            where v.target_type = $1 and v.target_id = $2
            group by v.id
            order by v.created_at desc
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
            .map(|row| Video {
                id: row.get("id"),
                owner_user_id: row.get("owner_user_id"),
                target_type,
                target_id: tid,
                s3_bucket: row.get("s3_bucket"),
                s3_key: row.get("s3_key"),
                content_type: row.get("content_type"),
                duration_seconds: row.get("duration_seconds"),
                created_at: row.get::<OffsetDateTime, _>("created_at"),
                vote_score: row.get::<i64, _>("vote_score"),
            })
            .collect())
    }
}
