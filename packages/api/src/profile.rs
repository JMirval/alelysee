use crate::types::Profile;
use dioxus::prelude::*;
#[cfg(feature = "server")]
use tracing::info;

#[dioxus::prelude::post("/api/profile/upsert")]
pub async fn upsert_profile(
    id_token: String,
    display_name: String,
    bio: String,
    avatar_url: Option<String>,
    location: Option<String>,
) -> Result<Profile, ServerFnError> {
    #[cfg(not(feature = "server"))]
    {
        let _ = (id_token, display_name, bio, avatar_url, location);
        Err(ServerFnError::new("upsert_profile is server-only"))
    }

    #[cfg(feature = "server")]
    {
        use sqlx::Row;
        info!(
            "profile.upsert_profile: display_name_len={} bio_len={}",
            display_name.len(),
            bio.len()
        );
        let user_id = crate::auth::require_user_id(id_token).await?;
        let state = crate::state::AppState::global();
        let pool = state.db.pool().await;

        let row = sqlx::query(
            r#"
            insert into profiles (user_id, display_name, bio, avatar_url, location, updated_at)
            values ($1, $2, $3, $4, $5, now())
            on conflict (user_id)
            do update set
                display_name = excluded.display_name,
                bio = excluded.bio,
                avatar_url = excluded.avatar_url,
                location = excluded.location,
                updated_at = now()
            returning
                CAST(user_id as TEXT) as user_id,
                display_name,
                bio,
                avatar_url,
                location,
                CAST(updated_at as TEXT) as updated_at
            "#,
        )
        .bind(crate::db::uuid_to_db(user_id))
        .bind(&display_name)
        .bind(&bio)
        .bind(&avatar_url)
        .bind(&location)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        info!("profile.upsert_profile: user_id={}", user_id);
        Ok(Profile {
            user_id: crate::db::uuid_from_db(&row.get::<String, _>("user_id"))?,
            display_name: row.get("display_name"),
            bio: row.get("bio"),
            avatar_url: row.get("avatar_url"),
            location: row.get("location"),
            updated_at: crate::db::datetime_from_db(&row.get::<String, _>("updated_at"))?,
        })
    }
}
