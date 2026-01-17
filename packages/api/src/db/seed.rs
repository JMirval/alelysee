use sqlx::{Pool, Any};
use anyhow::{Result, Context};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

pub async fn seed_database(pool: &Pool<Any>) -> Result<()> {
    tracing::info!("Starting database seeding...");

    // Create users with hashed passwords
    let argon2 = Argon2::default();
    let password = "Password123";
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .context("Failed to hash password")?
        .to_string();

    // Create 3 users
    let user1_id = sqlx::query_scalar::<_, i32>(
        r#"
        INSERT INTO users (email, password_hash, username, is_verified, created_at)
        VALUES ($1, $2, $3, true, datetime('now'))
        RETURNING id
        "#,
    )
    .bind("user1@local.dev")
    .bind(&password_hash)
    .bind("Alice Dupont")
    .fetch_one(pool)
    .await
    .context("Failed to create user1")?;

    let user2_id = sqlx::query_scalar::<_, i32>(
        r#"
        INSERT INTO users (email, password_hash, username, is_verified, created_at)
        VALUES ($1, $2, $3, true, datetime('now'))
        RETURNING id
        "#,
    )
    .bind("user2@local.dev")
    .bind(&password_hash)
    .bind("Bob Martin")
    .fetch_one(pool)
    .await
    .context("Failed to create user2")?;

    let user3_id = sqlx::query_scalar::<_, i32>(
        r#"
        INSERT INTO users (email, password_hash, username, is_verified, created_at)
        VALUES ($1, $2, $3, true, datetime('now'))
        RETURNING id
        "#,
    )
    .bind("user3@local.dev")
    .bind(&password_hash)
    .bind("Claire Lefebvre")
    .fetch_one(pool)
    .await
    .context("Failed to create user3")?;

    tracing::info!("Created 3 users");

    // Create proposals
    let proposal_ids = create_proposals(pool, user1_id, user2_id, user3_id).await?;
    tracing::info!("Created {} proposals", proposal_ids.len());

    // Create programs
    create_programs(pool, user1_id, &proposal_ids).await?;
    tracing::info!("Created programs");

    // Create comments
    create_comments(pool, user1_id, user2_id, user3_id, &proposal_ids).await?;
    tracing::info!("Created comments");

    // Create votes
    create_votes(pool, user1_id, user2_id, user3_id, &proposal_ids).await?;
    tracing::info!("Created votes");

    tracing::info!("Database seeding completed successfully");
    Ok(())
}

async fn create_proposals(
    pool: &Pool<Any>,
    user1_id: i32,
    user2_id: i32,
    user3_id: i32,
) -> Result<Vec<i32>> {
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
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO proposals (user_id, title, description, tags, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'published', datetime('now'), datetime('now'))
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(title)
        .bind(description)
        .bind(tags)
        .fetch_one(pool)
        .await
        .context("Failed to create proposal")?;

        ids.push(id);
    }

    Ok(ids)
}

async fn create_programs(pool: &Pool<Any>, user_id: i32, proposal_ids: &[i32]) -> Result<()> {
    // Create program 1: Progressive platform
    let program1_id = sqlx::query_scalar::<_, i32>(
        r#"
        INSERT INTO programs (user_id, title, description, created_at, updated_at)
        VALUES ($1, $2, $3, datetime('now'), datetime('now'))
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind("Programme Progressiste 2027")
    .bind("Un programme ambitieux pour une société plus juste, écologique et démocratique.")
    .fetch_one(pool)
    .await
    .context("Failed to create program 1")?;

    // Link first 5 proposals to program 1
    for (position, &proposal_id) in proposal_ids.iter().take(5).enumerate() {
        sqlx::query(
            r#"
            INSERT INTO program_proposals (program_id, proposal_id, position)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(program1_id)
        .bind(proposal_id)
        .bind(position as i32)
        .execute(pool)
        .await
        .context("Failed to link proposal to program 1")?;
    }

    // Create program 2: Ecological transition
    let program2_id = sqlx::query_scalar::<_, i32>(
        r#"
        INSERT INTO programs (user_id, title, description, created_at, updated_at)
        VALUES ($1, $2, $3, datetime('now'), datetime('now'))
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind("Transition Écologique Maintenant")
    .bind("Placer l'urgence climatique au cœur de l'action politique.")
    .fetch_one(pool)
    .await
    .context("Failed to create program 2")?;

    // Link environmental proposals to program 2
    for (position, &proposal_id) in [proposal_ids[2], proposal_ids[6], proposal_ids[8]]
        .iter()
        .enumerate()
    {
        sqlx::query(
            r#"
            INSERT INTO program_proposals (program_id, proposal_id, position)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(program2_id)
        .bind(proposal_id)
        .bind(position as i32)
        .execute(pool)
        .await
        .context("Failed to link proposal to program 2")?;
    }

    Ok(())
}

async fn create_comments(
    pool: &Pool<Any>,
    user1_id: i32,
    user2_id: i32,
    user3_id: i32,
    proposal_ids: &[i32],
) -> Result<()> {
    let comments = vec![
        (user2_id, proposal_ids[0], None, "Excellente idée ! Des études montrent que la productivité augmente avec moins d'heures."),
        (user3_id, proposal_ids[0], None, "Comment financer cela sans réduction de salaire ? Il faut plus de détails."),
        (user1_id, proposal_ids[1], None, "Le revenu universel pourrait éliminer la pauvreté et simplifier le système social."),
        (user2_id, proposal_ids[2], None, "Absolument nécessaire pour sauver les pollinisateurs !"),
        (user3_id, proposal_ids[2], None, "Les agriculteurs ont besoin d'alternatives viables. Il faut les accompagner."),
        (user1_id, proposal_ids[3], None, "La gratuité des transports réduirait aussi la pollution urbaine."),
        (user2_id, proposal_ids[4], None, "20% c'est bien, mais il faudrait viser 30% pour rattraper le retard."),
        (user3_id, proposal_ids[5], None, "La légalisation permettrait de mieux contrôler la qualité et de réduire le trafic."),
        (user1_id, proposal_ids[5], None, "Il faut aussi prévoir de la prévention et de l'éducation sur les risques."),
        (user2_id, proposal_ids[6], None, "Les aides doivent être suffisantes pour que ce ne soit pas qu'un cadeau aux riches."),
        (user3_id, proposal_ids[7], None, "La démocratie directe est l'avenir ! Donnons le pouvoir au peuple."),
        (user1_id, proposal_ids[7], None, "Attention aux dérives populistes. Il faut des garde-fous."),
        (user2_id, proposal_ids[8], None, "Bonne idée mais 6 mois c'est peut-être trop long. 3 mois suffiraient."),
        (user3_id, proposal_ids[9], None, "Enfin une mesure concrète contre les inégalités scandaleuses !"),
        (user1_id, proposal_ids[9], None, "Le ratio 1 pour 20 existe déjà dans certaines entreprises coopératives."),
    ];

    for (user_id, proposal_id, parent_id, content) in comments {
        sqlx::query(
            r#"
            INSERT INTO comments (proposal_id, user_id, parent_id, content, created_at, updated_at)
            VALUES ($1, $2, $3, $4, datetime('now'), datetime('now'))
            "#,
        )
        .bind(proposal_id)
        .bind(user_id)
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
    user1_id: i32,
    user2_id: i32,
    user3_id: i32,
    proposal_ids: &[i32],
) -> Result<()> {
    // User 1 votes
    for &proposal_id in &proposal_ids[0..7] {
        sqlx::query(
            r#"
            INSERT INTO votes (user_id, proposal_id, vote_type, created_at)
            VALUES ($1, $2, 'for', datetime('now'))
            "#,
        )
        .bind(user1_id)
        .bind(proposal_id)
        .execute(pool)
        .await
        .context("Failed to create vote")?;
    }

    // User 2 votes (mostly positive, some against)
    for &proposal_id in &proposal_ids[0..5] {
        sqlx::query(
            r#"
            INSERT INTO votes (user_id, proposal_id, vote_type, created_at)
            VALUES ($1, $2, 'for', datetime('now'))
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
        INSERT INTO votes (user_id, proposal_id, vote_type, created_at)
        VALUES ($1, $2, 'against', datetime('now'))
        "#,
    )
    .bind(user2_id)
    .bind(proposal_ids[5])
    .execute(pool)
    .await
    .context("Failed to create vote")?;

    // User 3 votes (mixed)
    for &proposal_id in &proposal_ids[1..4] {
        sqlx::query(
            r#"
            INSERT INTO votes (user_id, proposal_id, vote_type, created_at)
            VALUES ($1, $2, 'for', datetime('now'))
            "#,
        )
        .bind(user3_id)
        .bind(proposal_id)
        .execute(pool)
        .await
        .context("Failed to create vote")?;
    }

    for &proposal_id in &proposal_ids[7..10] {
        sqlx::query(
            r#"
            INSERT INTO votes (user_id, proposal_id, vote_type, created_at)
            VALUES ($1, $2, 'for', datetime('now'))
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
