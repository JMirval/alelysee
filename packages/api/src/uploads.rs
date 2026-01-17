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
        use aws_credential_types::Credentials;
        use aws_sdk_s3::presigning::PresigningConfig;
        use aws_sdk_s3::types::ObjectCannedAcl;
        use aws_sdk_s3::{config::Builder as S3ConfigBuilder, config::Region};
        use std::time::Duration;
        use uuid::Uuid;

        const MAX_BYTES: i64 = 200 * 1024 * 1024; // 200MB MVP limit
        if byte_size <= 0 || byte_size > MAX_BYTES {
            return Err(ServerFnError::new("invalid file size"));
        }

        // Ensure authenticated user exists (and we record ownership at finalize time).
        let _user_id = crate::auth::require_user_id(id_token).await?;

        let bucket = std::env::var("STORAGE_BUCKET")
            .map_err(|_| ServerFnError::new("STORAGE_BUCKET not set"))?;
        let endpoint = std::env::var("STORAGE_ENDPOINT")
            .map_err(|_| ServerFnError::new("STORAGE_ENDPOINT not set"))?;
        let access_key = std::env::var("STORAGE_ACCESS_KEY")
            .map_err(|_| ServerFnError::new("STORAGE_ACCESS_KEY not set"))?;
        let secret_key = std::env::var("STORAGE_SECRET_KEY")
            .map_err(|_| ServerFnError::new("STORAGE_SECRET_KEY not set"))?;
        let region = std::env::var("STORAGE_REGION").unwrap_or_else(|_| "auto".to_string());

        let key = format!(
            "videos/{}/{}/{}",
            target_type.as_db(),
            target_id,
            Uuid::new_v4()
        );

        let creds = Credentials::new(access_key, secret_key, None, None, "railway");
        let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(Region::new(region))
            .credentials_provider(creds)
            .load()
            .await;

        let s3_config = S3ConfigBuilder::from(&sdk_config)
            .endpoint_url(endpoint)
            .force_path_style(true)
            .build();
        let client = aws_sdk_s3::Client::from_conf(s3_config);

        let presigned = client
            .put_object()
            .bucket(&bucket)
            .key(&key)
            .content_type(content_type)
            .acl(ObjectCannedAcl::Private)
            .presigned(
                PresigningConfig::expires_in(Duration::from_secs(60 * 10))
                    .map_err(|_| ServerFnError::new("presign config error"))?,
            )
            .await
            .map_err(|e| ServerFnError::new(format!("presign error: {e}")))?;

        Ok(UploadIntent {
            presigned_put_url: presigned.uri().to_string(),
            storage_key: key,
            bucket,
        })
    }
}

#[dioxus::prelude::post("/api/uploads/finalize_video")]
pub async fn finalize_video_upload(
    id_token: String,
    target_type: ContentTargetType,
    target_id: String,
    storage_key: String,
    content_type: String,
) -> Result<Video, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, target_type, target_id, storage_key, content_type);
        Err(ServerFnError::new("finalize_video_upload is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use aws_credential_types::Credentials;
        use aws_sdk_s3::{config::Builder as S3ConfigBuilder, config::Region};
        use sqlx::Row;
        use uuid::Uuid;

        let owner_user_id = crate::auth::require_user_id(id_token).await?;
        let tid =
            Uuid::parse_str(&target_id).map_err(|_| ServerFnError::new("invalid target_id"))?;

        let bucket = std::env::var("STORAGE_BUCKET")
            .map_err(|_| ServerFnError::new("STORAGE_BUCKET not set"))?;
        let endpoint = std::env::var("STORAGE_ENDPOINT")
            .map_err(|_| ServerFnError::new("STORAGE_ENDPOINT not set"))?;
        let access_key = std::env::var("STORAGE_ACCESS_KEY")
            .map_err(|_| ServerFnError::new("STORAGE_ACCESS_KEY not set"))?;
        let secret_key = std::env::var("STORAGE_SECRET_KEY")
            .map_err(|_| ServerFnError::new("STORAGE_SECRET_KEY not set"))?;
        let region = std::env::var("STORAGE_REGION").unwrap_or_else(|_| "auto".to_string());

        let creds = Credentials::new(access_key, secret_key, None, None, "railway");
        let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(Region::new(region))
            .credentials_provider(creds)
            .load()
            .await;

        let s3_config = S3ConfigBuilder::from(&sdk_config)
            .endpoint_url(endpoint)
            .force_path_style(true)
            .build();
        let client = aws_sdk_s3::Client::from_conf(s3_config);

        client
            .head_object()
            .bucket(&bucket)
            .key(&storage_key)
            .send()
            .await
            .map_err(|e| ServerFnError::new(format!("head_object failed: {e}")))?;

        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let row = sqlx::query(
            r#"
            insert into videos (owner_user_id, target_type, target_id, storage_bucket, storage_key, content_type)
            values ($1, $2, $3, $4, $5, $6)
            returning
                CAST(id as TEXT) as id,
                CAST(owner_user_id as TEXT) as owner_user_id,
                target_type,
                CAST(target_id as TEXT) as target_id,
                storage_bucket,
                storage_key,
                content_type,
                duration_seconds,
                CAST(created_at as TEXT) as created_at
            "#,
        )
        .bind(crate::db::uuid_to_db(owner_user_id))
        .bind(target_type.as_db())
        .bind(crate::db::uuid_to_db(tid))
        .bind(&bucket)
        .bind(&storage_key)
        .bind(&content_type)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let vid = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
        let _ = sqlx::query(
            "insert into activity (user_id, action, target_type, target_id) values ($1, 'created', 'video', $2)",
        )
        .bind(crate::db::uuid_to_db(owner_user_id))
        .bind(crate::db::uuid_to_db(vid))
        .execute(pool)
        .await;

        let owner_user_id = crate::db::uuid_from_db(&row.get::<String, _>("owner_user_id"))?;
        let target_id = crate::db::uuid_from_db(&row.get::<String, _>("target_id"))?;
        let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;

        Ok(Video {
            id: vid,
            owner_user_id,
            target_type,
            target_id,
            storage_bucket: row.get("storage_bucket"),
            storage_key: row.get("storage_key"),
            content_type: row.get("content_type"),
            duration_seconds: row.get("duration_seconds"),
            created_at,
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
        use uuid::Uuid;

        let tid =
            Uuid::parse_str(&target_id).map_err(|_| ServerFnError::new("invalid target_id"))?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let rows = sqlx::query(
            r#"
            select
                CAST(v.id as TEXT) as id,
                CAST(v.owner_user_id as TEXT) as owner_user_id,
                CAST(v.target_id as TEXT) as target_id,
                v.storage_bucket,
                v.storage_key,
                v.content_type,
                v.duration_seconds,
                CAST(v.created_at as TEXT) as created_at,
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
        .bind(crate::db::uuid_to_db(tid))
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        let mut videos = Vec::with_capacity(rows.len());
        for row in rows {
            let id = crate::db::uuid_from_db(&row.get::<String, _>("id"))?;
            let owner_user_id = crate::db::uuid_from_db(&row.get::<String, _>("owner_user_id"))?;
            let created_at = crate::db::datetime_from_db(&row.get::<String, _>("created_at"))?;
            videos.push(Video {
                id,
                owner_user_id,
                target_type,
                target_id: tid,
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
}
