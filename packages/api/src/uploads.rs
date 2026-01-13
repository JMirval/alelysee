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
        let _ = (id_token, target_type, target_id, content_type, byte_size);
        // TODO(railway): wire uploads to a Railway-hosted service or external object storage
        // and return a presigned_put_url + storage_key here.
        Err(ServerFnError::new("video uploads are not configured"))
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
        let _ = (id_token, target_type, target_id, storage_key, content_type);
        // TODO(railway): persist uploaded video metadata after verifying storage upload.
        Err(ServerFnError::new("video uploads are not configured"))
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
                v.storage_bucket,
                v.storage_key,
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
                storage_bucket: row.get("storage_bucket"),
                storage_key: row.get("storage_key"),
                content_type: row.get("content_type"),
                duration_seconds: row.get("duration_seconds"),
                created_at: row.get::<OffsetDateTime, _>("created_at"),
                vote_score: row.get::<i64, _>("vote_score"),
            })
            .collect())
    }
}
