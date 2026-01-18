use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use sqlx::{Any, Pool};
use uuid::Uuid;

pub async fn seed_database(pool: &Pool<Any>) -> Result<()> {
    tracing::info!("Starting database seeding...");

    // Create users with hashed passwords
    let argon2 = Argon2::default();
    let password = "Password123";
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
        .to_string();

    let users = vec![
        ("user1@local.dev", "Alice Dupont"),
        ("user2@local.dev", "Bob Martin"),
        ("user3@local.dev", "Claire Lefebvre"),
    ];

    let mut user_ids = Vec::with_capacity(users.len());
    for (email, display_name) in users {
        let user_id = Uuid::new_v4().to_string();
        let auth_subject = user_id.clone();
        sqlx::query(
            r#"
            INSERT INTO users (id, auth_subject, email, password_hash, email_verified)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(&user_id)
        .bind(&auth_subject)
        .bind(email)
        .bind(&password_hash)
        .bind(true)
        .execute(pool)
        .await
        .with_context(|| format!("Failed to create user {email}"))?;

        sqlx::query(
            r#"
            INSERT INTO profiles (user_id, display_name)
            VALUES ($1, $2)
            "#,
        )
        .bind(&user_id)
        .bind(display_name)
        .execute(pool)
        .await
        .with_context(|| format!("Failed to create profile for {email}"))?;

        user_ids.push(user_id);
    }

    let user1_id = user_ids[0].clone();
    let user2_id = user_ids[1].clone();
    let user3_id = user_ids[2].clone();

    tracing::info!("Created 3 users");

    // Create proposals
    let proposal_ids = create_proposals(pool, &user1_id, &user2_id, &user3_id).await?;
    tracing::info!("Created {} proposals", proposal_ids.len());

    // Create programs
    create_programs(pool, &user1_id, &proposal_ids).await?;
    tracing::info!("Created programs");

    // Create comments
    create_comments(pool, &user1_id, &user2_id, &user3_id, &proposal_ids).await?;
    tracing::info!("Created comments");

    // Create votes
    create_votes(pool, &user1_id, &user2_id, &user3_id, &proposal_ids).await?;
    tracing::info!("Created votes");

    tracing::info!("Database seeding completed successfully");
    Ok(())
}

async fn create_proposals(
    pool: &Pool<Any>,
    user1_id: &str,
    user2_id: &str,
    user3_id: &str,
) -> Result<Vec<String>> {
    let mut ids = Vec::new();

    let proposals = vec![
        (
            user1_id,
            "Instaurer une semaine de travail de 4 jours",
            "Réduire le temps de travail hebdomadaire à 32 heures sur 4 jours, sans perte de salaire, pour améliorer la qualité de vie et la productivité.",
            "travail,qualite-de-vie",
        ),
        (
            user1_id,
            "Créer un revenu de base universel",
            "Mettre en place un revenu minimum garanti pour tous les citoyens majeurs, financé par une refonte de la fiscalité et des aides sociales.",
            "social,economie",
        ),
        (
            user2_id,
            "Interdire les pesticides néonicotinoïdes",
            "Bannir définitivement l'usage des pesticides néonicotinoïdes pour protéger les abeilles et la biodiversité.",
            "environnement,agriculture",
        ),
        (
            user2_id,
            "Rendre les transports en commun gratuits",
            "Supprimer les frais de transport en commun dans toutes les villes de plus de 100 000 habitants, financé par une taxe sur les entreprises.",
            "transport,social",
        ),
        (
            user3_id,
            "Augmenter le budget de l'éducation de 20%",
            "Investir massivement dans l'éducation nationale pour réduire les effectifs par classe et revaloriser les salaires des enseignants.",
            "education,social",
        ),
        (
            user3_id,
            "Légaliser et réguler le cannabis",
            "Autoriser la vente contrôlée de cannabis pour les adultes, avec taxation et réglementation stricte sur la qualité et la distribution.",
            "sante,justice",
        ),
        (
            user1_id,
            "Rénovation énergétique obligatoire des bâtiments",
            "Imposer la rénovation énergétique de tous les bâtiments avant 2035, avec aides publiques pour les ménages modestes.",
            "environnement,logement",
        ),
        (
            user2_id,
            "Instaurer un référendum d'initiative citoyenne",
            "Permettre aux citoyens de proposer et voter des lois par référendum avec 500 000 signatures.",
            "democratie,politique",
        ),
        (
            user3_id,
            "Créer un service civique environnemental obligatoire",
            "Instaurer 6 mois de service civique obligatoire dédié à la transition écologique pour tous les jeunes de 18 ans.",
            "environnement,jeunesse",
        ),
        (
            user1_id,
            "Limiter les écarts de salaire à 1 pour 20",
            "Imposer un ratio maximal de 1 pour 20 entre le salaire le plus bas et le plus haut dans une même entreprise.",
            "economie,justice-sociale",
        ),
    ];

    for (user_id, title, description, tags) in proposals {
        let tags_json = serde_json::to_string(
            &tags
                .split(',')
                .map(|tag| tag.trim().to_string())
                .filter(|tag| !tag.is_empty())
                .collect::<Vec<_>>(),
        )?;
        let id = sqlx::query_scalar::<_, String>(
            r#"
            INSERT INTO proposals (author_user_id, title, summary, body_markdown, tags)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING CAST(id as TEXT)
            "#,
        )
        .bind(user_id)
        .bind(title)
        .bind(description)
        .bind(description)
        .bind(tags_json)
        .fetch_one(pool)
        .await
        .context("Failed to create proposal")?;

        ids.push(id);
    }

    Ok(ids)
}

async fn create_programs(
    pool: &Pool<Any>,
    user_id: &str,
    proposal_ids: &[String],
) -> Result<()> {
    // Create program 1: Progressive platform
    let program1_id = sqlx::query_scalar::<_, String>(
        r#"
        INSERT INTO programs (author_user_id, title, summary, body_markdown)
        VALUES ($1, $2, $3, $4)
        RETURNING CAST(id as TEXT)
        "#,
    )
    .bind(user_id)
    .bind("Programme Progressiste 2027")
    .bind("Un programme ambitieux pour une société plus juste, écologique et démocratique.")
    .bind("Un programme ambitieux pour une société plus juste, écologique et démocratique.")
    .fetch_one(pool)
    .await
    .context("Failed to create program 1")?;

    // Link first 5 proposals to program 1
    for (position, proposal_id) in proposal_ids.iter().take(5).enumerate() {
        sqlx::query(
            r#"
            INSERT INTO program_items (program_id, proposal_id, position)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(&program1_id)
        .bind(proposal_id)
        .bind(position as i32)
        .execute(pool)
        .await
        .context("Failed to link proposal to program 1")?;
    }

    // Create program 2: Ecological transition
    let program2_id = sqlx::query_scalar::<_, String>(
        r#"
        INSERT INTO programs (author_user_id, title, summary, body_markdown)
        VALUES ($1, $2, $3, $4)
        RETURNING CAST(id as TEXT)
        "#,
    )
    .bind(user_id)
    .bind("Transition Écologique Maintenant")
    .bind("Placer l'urgence climatique au cœur de l'action politique.")
    .bind("Placer l'urgence climatique au cœur de l'action politique.")
    .fetch_one(pool)
    .await
    .context("Failed to create program 2")?;

    // Link environmental proposals to program 2
    for (position, proposal_id) in [&proposal_ids[2], &proposal_ids[6], &proposal_ids[8]]
        .iter()
        .enumerate()
    {
        sqlx::query(
            r#"
            INSERT INTO program_items (program_id, proposal_id, position)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(&program2_id)
        .bind(*proposal_id)
        .bind(position as i32)
        .execute(pool)
        .await
        .context("Failed to link proposal to program 2")?;
    }

    Ok(())
}

async fn create_comments(
    pool: &Pool<Any>,
    user1_id: &str,
    user2_id: &str,
    user3_id: &str,
    proposal_ids: &[String],
) -> Result<()> {
    let comments = vec![
        (user2_id, &proposal_ids[0], None::<&str>, "Excellente idée ! Des études montrent que la productivité augmente avec moins d'heures."),
        (user3_id, &proposal_ids[0], None::<&str>, "Comment financer cela sans réduction de salaire ? Il faut plus de détails."),
        (user1_id, &proposal_ids[1], None::<&str>, "Le revenu universel pourrait éliminer la pauvreté et simplifier le système social."),
        (user2_id, &proposal_ids[2], None::<&str>, "Absolument nécessaire pour sauver les pollinisateurs !"),
        (user3_id, &proposal_ids[2], None::<&str>, "Les agriculteurs ont besoin d'alternatives viables. Il faut les accompagner."),
        (user1_id, &proposal_ids[3], None::<&str>, "La gratuité des transports réduirait aussi la pollution urbaine."),
        (user2_id, &proposal_ids[4], None::<&str>, "20% c'est bien, mais il faudrait viser 30% pour rattraper le retard."),
        (user3_id, &proposal_ids[5], None::<&str>, "La légalisation permettrait de mieux contrôler la qualité et de réduire le trafic."),
        (user1_id, &proposal_ids[5], None::<&str>, "Il faut aussi prévoir de la prévention et de l'éducation sur les risques."),
        (user2_id, &proposal_ids[6], None::<&str>, "Les aides doivent être suffisantes pour que ce ne soit pas qu'un cadeau aux riches."),
        (user3_id, &proposal_ids[7], None::<&str>, "La démocratie directe est l'avenir ! Donnons le pouvoir au peuple."),
        (user1_id, &proposal_ids[7], None::<&str>, "Attention aux dérives populistes. Il faut des garde-fous."),
        (user2_id, &proposal_ids[8], None::<&str>, "Bonne idée mais 6 mois c'est peut-être trop long. 3 mois suffiraient."),
        (user3_id, &proposal_ids[9], None::<&str>, "Enfin une mesure concrète contre les inégalités scandaleuses !"),
        (user1_id, &proposal_ids[9], None::<&str>, "Le ratio 1 pour 20 existe déjà dans certaines entreprises coopératives."),
    ];

    for (user_id, proposal_id, parent_id, content) in comments {
        sqlx::query(
            r#"
            INSERT INTO comments (author_user_id, target_type, target_id, parent_comment_id, body_markdown)
            VALUES ($1, 'proposal', $2, $3, $4)
            "#,
        )
        .bind(user_id)
        .bind(proposal_id)
        .bind(parent_id)
        .bind(content)
        .execute(pool)
        .await
        .context("Failed to create comment")?;
    }

    Ok(())
}

async fn create_votes(
    pool: &Pool<Any>,
    user1_id: &str,
    user2_id: &str,
    user3_id: &str,
    proposal_ids: &[String],
) -> Result<()> {
    // User 1 votes
    for proposal_id in &proposal_ids[0..7] {
        sqlx::query(
            r#"
            INSERT INTO votes (user_id, target_type, target_id, value)
            VALUES ($1, 'proposal', $2, 1)
            "#,
        )
        .bind(user1_id)
        .bind(proposal_id)
        .execute(pool)
        .await
        .context("Failed to create vote")?;
    }

    // User 2 votes (mostly positive, some against)
    for proposal_id in &proposal_ids[0..5] {
        sqlx::query(
            r#"
            INSERT INTO votes (user_id, target_type, target_id, value)
            VALUES ($1, 'proposal', $2, 1)
            "#,
        )
        .bind(user2_id)
        .bind(proposal_id)
        .execute(pool)
        .await
        .context("Failed to create vote")?;
    }

    sqlx::query(
        r#"
        INSERT INTO votes (user_id, target_type, target_id, value)
        VALUES ($1, 'proposal', $2, -1)
        "#,
    )
    .bind(user2_id)
    .bind(&proposal_ids[5])
    .execute(pool)
    .await
    .context("Failed to create vote")?;

    // User 3 votes (mixed)
    for proposal_id in &proposal_ids[1..4] {
        sqlx::query(
            r#"
            INSERT INTO votes (user_id, target_type, target_id, value)
            VALUES ($1, 'proposal', $2, 1)
            "#,
        )
        .bind(user3_id)
        .bind(proposal_id)
        .execute(pool)
        .await
        .context("Failed to create vote")?;
    }

    for proposal_id in &proposal_ids[7..10] {
        sqlx::query(
            r#"
            INSERT INTO votes (user_id, target_type, target_id, value)
            VALUES ($1, 'proposal', $2, 1)
            "#,
        )
        .bind(user3_id)
        .bind(proposal_id)
        .execute(pool)
        .await
        .context("Failed to create vote")?;
    }

    Ok(())
}
