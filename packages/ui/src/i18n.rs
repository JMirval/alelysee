use dioxus::prelude::*;

/// Supported languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Fr,
    En,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Fr => "fr",
            Lang::En => "en",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_ascii_lowercase().as_str() {
            "fr" | "fr-fr" => Some(Lang::Fr),
            "en" | "en-us" | "en-gb" => Some(Lang::En),
            _ => None,
        }
    }
}

/// Provide `Signal<Lang>` to the component tree, defaulting to French.
#[component]
pub fn I18nProvider(children: Element) -> Element {
    let mut lang = use_signal(|| Lang::Fr);
    use_context_provider(|| lang);

    // Best-effort: load from localStorage or browser language after mount.
    use_effect(move || {
        spawn(async move {
            let js = r#"
            (function(){
              try {
                const saved = localStorage.getItem("alelysee_lang");
                if(saved && typeof saved === "string" && saved.length > 0) return saved;
              } catch(e) {}
              try { return (navigator.language || "fr"); } catch(e) {}
              return "fr";
            })()
            "#;
            if let Ok(v) = document::eval(js).await {
                if let Some(code) = v.as_str() {
                    if let Some(next) = Lang::from_code(code) {
                        lang.set(next);
                    }
                }
            }
        });
    });

    rsx! { {children} }
}

pub fn use_lang() -> Signal<Lang> {
    if let Some(sig) = try_use_context::<Signal<Lang>>() {
        return sig;
    }

    // Fallback for SSR or mis-ordered providers to avoid panics in production.
    eprintln!("startup: missing I18nProvider context, using local Lang::Fr signal");
    use_signal(|| Lang::Fr)
}

pub fn set_lang(lang: Lang) {
    let mut s = use_lang();
    s.set(lang);
    spawn(async move {
        let _ = document::eval(&format!(
            r#"(function(){{ try {{ localStorage.setItem("alelysee_lang","{}"); }} catch(e) {{}} return ""; }})()"#,
            lang.code()
        ))
        .await;
    });
}

/// Translate a key for a given language. Falls back to French if missing.
pub fn t(lang: Lang, key: &str) -> String {
    match (lang, key) {
        // Nav / common
        (Lang::Fr, "app.name") => "Alelysee".to_string(),
        (Lang::En, "app.name") => "Alelysee".to_string(),
        (Lang::Fr, "nav.proposals") => "Propositions".to_string(),
        (Lang::En, "nav.proposals") => "Proposals".to_string(),
        (Lang::Fr, "nav.programs") => "Programmes".to_string(),
        (Lang::En, "nav.programs") => "Programs".to_string(),
        (Lang::Fr, "nav.profile") => "Profil".to_string(),
        (Lang::En, "nav.profile") => "Profile".to_string(),
        (Lang::Fr, "nav.signin") => "Se connecter / S'inscrire".to_string(),
        (Lang::En, "nav.signin") => "Sign in / Sign up".to_string(),
        (Lang::Fr, "nav.edit_profile") => "Modifier le profil".to_string(),
        (Lang::En, "nav.edit_profile") => "Edit profile".to_string(),
        (Lang::Fr, "nav.signout") => "Se déconnecter".to_string(),
        (Lang::En, "nav.signout") => "Sign out".to_string(),
        (Lang::Fr, "lang.label") => "Langue".to_string(),

        // Home / hero
        (Lang::Fr, "home.tagline") => "Proposer. Regrouper. Débattre. Voter.".to_string(),
        (Lang::En, "home.tagline") => "Propose. Bundle. Debate. Vote.".to_string(),
        (Lang::Fr, "home.subtitle") => "Écrivez des propositions, regroupez-les en programmes, discutez avec votes, commentaires et vidéos.".to_string(),
        (Lang::En, "home.subtitle") => "Write proposals, bundle them into programs, and discuss with votes, comments, and videos.".to_string(),
        (Lang::Fr, "home.cta.proposals") => "Explorer les propositions".to_string(),
        (Lang::En, "home.cta.proposals") => "Explore proposals".to_string(),
        (Lang::Fr, "home.cta.programs") => "Parcourir les programmes".to_string(),
        (Lang::En, "home.cta.programs") => "Browse programs".to_string(),
        (Lang::Fr, "home.tip") => "Astuce : connectez-vous pour voter, commenter et publier des vidéos.".to_string(),
        (Lang::En, "home.tip") => "Tip: sign in to vote, comment, and upload videos.".to_string(),

        // Proposals
        (Lang::Fr, "proposals.title") => "Propositions".to_string(),
        (Lang::En, "proposals.title") => "Proposals".to_string(),
        (Lang::Fr, "proposals.new") => "Nouvelle proposition".to_string(),
        (Lang::En, "proposals.new") => "New proposal".to_string(),
        (Lang::Fr, "proposals.need_signin_create") => "Vous devez vous connecter pour créer des propositions.".to_string(),
        (Lang::En, "proposals.need_signin_create") => "You need to sign in to create proposals.".to_string(),
        (Lang::Fr, "proposals.form.title") => "Titre".to_string(),
        (Lang::En, "proposals.form.title") => "Title".to_string(),
        (Lang::Fr, "proposals.form.title_ph") => "Titre de la proposition".to_string(),
        (Lang::En, "proposals.form.title_ph") => "Proposal title".to_string(),
        (Lang::Fr, "proposals.form.summary_opt") => "Résumé (facultatif)".to_string(),
        (Lang::En, "proposals.form.summary_opt") => "Summary (optional)".to_string(),
        (Lang::Fr, "proposals.form.summary_ph") => "Résumé en une phrase".to_string(),
        (Lang::En, "proposals.form.summary_ph") => "One sentence summary".to_string(),
        (Lang::Fr, "proposals.form.body") => "Contenu (Markdown)".to_string(),
        (Lang::En, "proposals.form.body") => "Body (Markdown)".to_string(),
        (Lang::Fr, "proposals.form.body_ph") => "Rédigez la proposition…".to_string(),
        (Lang::En, "proposals.form.body_ph") => "Write the proposal…".to_string(),
        (Lang::Fr, "proposals.form.tags") => "Tags (séparés par des virgules)".to_string(),
        (Lang::En, "proposals.form.tags") => "Tags (comma-separated)".to_string(),
        (Lang::Fr, "proposals.form.tags_ph") => "économie, santé, éducation".to_string(),
        (Lang::En, "proposals.form.tags_ph") => "economy, healthcare, education".to_string(),
        (Lang::Fr, "proposals.form.create") => "Créer".to_string(),
        (Lang::En, "proposals.form.create") => "Create".to_string(),
        (Lang::Fr, "proposals.created_open") => "Créé ! Ouvrir :".to_string(),
        (Lang::En, "proposals.created_open") => "Created! Open:".to_string(),
        (Lang::Fr, "proposals.bundle_into_program") => "Ajouter à un programme".to_string(),
        (Lang::En, "proposals.bundle_into_program") => "Bundle into program".to_string(),
        (Lang::Fr, "common.vote") => "Vote".to_string(),
        (Lang::En, "common.vote") => "Vote".to_string(),
        (Lang::Fr, "common.id") => "id :".to_string(),
        (Lang::En, "common.id") => "id:".to_string(),
        (Lang::Fr, "common.back") => "Retour".to_string(),
        (Lang::En, "common.back") => "Back".to_string(),
        (Lang::Fr, "common.loading") => "Chargement…".to_string(),
        (Lang::En, "common.loading") => "Loading…".to_string(),
        (Lang::Fr, "common.error_prefix") => "Erreur :".to_string(),
        (Lang::En, "common.error_prefix") => "Error:".to_string(),
        (Lang::Fr, "common.signin") => "Se connecter".to_string(),
        (Lang::En, "common.signin") => "Sign in".to_string(),
        (Lang::Fr, "common.no_proposals_yet") => "Aucune proposition pour le moment.".to_string(),
        (Lang::En, "common.no_proposals_yet") => "No proposals yet.".to_string(),
        (Lang::Fr, "common.no_programs_yet") => "Aucun programme pour le moment.".to_string(),
        (Lang::En, "common.no_programs_yet") => "No programs yet.".to_string(),
        (Lang::Fr, "common.no_videos_yet") => "Aucune vidéo pour le moment.".to_string(),
        (Lang::En, "common.no_videos_yet") => "No videos yet.".to_string(),
        (Lang::Fr, "common.no_comments_yet") => "Aucun commentaire pour le moment.".to_string(),
        (Lang::En, "common.no_comments_yet") => "No comments yet.".to_string(),
        (Lang::Fr, "common.no_activity_yet") => "Aucune activité pour le moment.".to_string(),
        (Lang::En, "common.no_activity_yet") => "No activity yet.".to_string(),
        (Lang::Fr, "common.signin_to_vote") => "Connectez-vous pour voter".to_string(),
        (Lang::En, "common.signin_to_vote") => "Sign in to vote".to_string(),
        (Lang::Fr, "common.signin_to_comment") => "Connectez-vous pour commenter.".to_string(),
        (Lang::En, "common.signin_to_comment") => "Sign in to comment.".to_string(),
        (Lang::Fr, "common.signin_to_upload_video") => "Connectez-vous pour envoyer une vidéo.".to_string(),
        (Lang::En, "common.signin_to_upload_video") => "Sign in to upload a video.".to_string(),
        (Lang::Fr, "videos.loading_player") => "Chargement du lecteur…".to_string(),
        (Lang::En, "videos.loading_player") => "Loading player…".to_string(),
        (Lang::Fr, "vote.clear") => "Effacer".to_string(),
        (Lang::En, "vote.clear") => "Clear".to_string(),
        (Lang::Fr, "vote.your_vote") => "Votre vote :".to_string(),
        (Lang::En, "vote.your_vote") => "Your vote:".to_string(),
        (Lang::Fr, "comments.title") => "Commentaires".to_string(),
        (Lang::En, "comments.title") => "Comments".to_string(),
        (Lang::Fr, "comments.placeholder") => "Écrivez un commentaire…".to_string(),
        (Lang::En, "comments.placeholder") => "Write a comment…".to_string(),
        (Lang::Fr, "comments.post") => "Publier".to_string(),
        (Lang::En, "comments.post") => "Post".to_string(),
        (Lang::Fr, "comments.empty_error") => "Le commentaire ne peut pas être vide".to_string(),
        (Lang::En, "comments.empty_error") => "Comment cannot be empty".to_string(),
        (Lang::Fr, "comments.by") => "par".to_string(),
        (Lang::En, "comments.by") => "by".to_string(),

        // Programs
        (Lang::Fr, "programs.title") => "Programmes".to_string(),
        (Lang::En, "programs.title") => "Programs".to_string(),
        (Lang::Fr, "programs.new") => "Nouveau programme".to_string(),
        (Lang::En, "programs.new") => "New program".to_string(),
        (Lang::Fr, "programs.need_signin_create") => "Vous devez vous connecter pour créer des programmes.".to_string(),
        (Lang::En, "programs.need_signin_create") => "You need to sign in to create programs.".to_string(),
        (Lang::Fr, "programs.form.title") => "Titre".to_string(),
        (Lang::En, "programs.form.title") => "Title".to_string(),
        (Lang::Fr, "programs.form.title_ph") => "Titre du programme".to_string(),
        (Lang::En, "programs.form.title_ph") => "Program title".to_string(),
        (Lang::Fr, "programs.form.summary") => "Résumé".to_string(),
        (Lang::En, "programs.form.summary") => "Summary".to_string(),
        (Lang::Fr, "programs.form.summary_ph") => "Quel est l'objectif de ce programme ?".to_string(),
        (Lang::En, "programs.form.summary_ph") => "What does this program achieve?".to_string(),
        (Lang::Fr, "programs.form.body") => "Contenu (Markdown)".to_string(),
        (Lang::En, "programs.form.body") => "Body (Markdown)".to_string(),
        (Lang::Fr, "programs.form.body_ph") => "Rédigez le programme…".to_string(),
        (Lang::En, "programs.form.body_ph") => "Write the program…".to_string(),
        (Lang::Fr, "programs.form.proposal_ids") => "IDs des propositions à inclure (séparés par des virgules)".to_string(),
        (Lang::En, "programs.form.proposal_ids") => "Proposal IDs to include (comma-separated)".to_string(),
        (Lang::Fr, "programs.form.proposal_ids_ph") => "uuid1, uuid2, uuid3".to_string(),
        (Lang::En, "programs.form.proposal_ids_ph") => "uuid1, uuid2, uuid3".to_string(),
        (Lang::Fr, "programs.form.create") => "Créer".to_string(),
        (Lang::En, "programs.form.create") => "Create".to_string(),
        (Lang::Fr, "programs.created_open") => "Créé ! Ouvrir :".to_string(),
        (Lang::En, "programs.created_open") => "Created! Open:".to_string(),
        (Lang::Fr, "programs.browse_proposals") => "Parcourir les propositions".to_string(),
        (Lang::En, "programs.browse_proposals") => "Browse proposals".to_string(),
        (Lang::Fr, "programs.bundled_proposals") => "Propositions incluses".to_string(),
        (Lang::En, "programs.bundled_proposals") => "Bundled proposals".to_string(),
        (Lang::Fr, "programs.none_bundled") => "Aucune proposition incluse pour le moment.".to_string(),
        (Lang::En, "programs.none_bundled") => "No proposals added yet.".to_string(),

        // Auth
        (Lang::Fr, "auth.signin.title") => "Connexion".to_string(),
        (Lang::En, "auth.signin.title") => "Sign in".to_string(),
        (Lang::Fr, "auth.required") => "Connexion requise".to_string(),
        (Lang::En, "auth.required") => "Sign in required".to_string(),
        (Lang::Fr, "auth.required.body") => "Vous devez vous connecter pour utiliser cette fonctionnalité.".to_string(),
        (Lang::En, "auth.required.body") => "You need to sign in to use this feature.".to_string(),
        (Lang::Fr, "auth.required.cta") => "Aller à la connexion".to_string(),
        (Lang::En, "auth.required.cta") => "Go to sign in".to_string(),

        (Lang::Fr, "auth.signin.body") => "Connectez-vous ou creez un compte via un fournisseur OAuth.".to_string(),
        (Lang::En, "auth.signin.body") => "Sign in or sign up with an OAuth provider.".to_string(),
        (Lang::Fr, "auth.signin.continue") => "Continuer vers la connexion".to_string(),
        (Lang::En, "auth.signin.continue") => "Continue to sign in".to_string(),
        (Lang::Fr, "auth.signin.hint") => "Après connexion, vous serez redirigé vers cette application.".to_string(),
        (Lang::En, "auth.signin.hint") => "After signing in, you'll be redirected back to this app.".to_string(),
        (Lang::Fr, "auth.callback.title") => "Finalisation de la connexion…".to_string(),
        (Lang::En, "auth.callback.title") => "Finishing sign in…".to_string(),
        (Lang::Fr, "auth.callback.body.prefix") => "Si cet écran ne redirige pas, allez sur ".to_string(),
        (Lang::En, "auth.callback.body.prefix") => "If this screen doesn't redirect, go to ".to_string(),
        (Lang::Fr, "auth.callback.body.suffix") => ".".to_string(),
        (Lang::En, "auth.callback.body.suffix") => ".".to_string(),
        (Lang::Fr, "me.title") => "Mon compte".to_string(),
        (Lang::En, "me.title") => "My account".to_string(),
        (Lang::Fr, "me.signed_out") => "Vous n'êtes pas connecté.".to_string(),
        (Lang::En, "me.signed_out") => "You are not signed in.".to_string(),
        (Lang::Fr, "me.signin") => "Se connecter".to_string(),
        (Lang::En, "me.signin") => "Sign in".to_string(),
        (Lang::Fr, "me.user_id") => "Identifiant :".to_string(),
        (Lang::En, "me.user_id") => "User id:".to_string(),
        (Lang::Fr, "me.signed_in_as") => "Connecté en tant que".to_string(),
        (Lang::En, "me.signed_in_as") => "Signed in as".to_string(),
        (Lang::Fr, "me.profile_complete") => "Profil complet.".to_string(),
        (Lang::En, "me.profile_complete") => "Profile complete.".to_string(),
        (Lang::Fr, "me.profile_incomplete") => "Profil incomplet : ajoutez un nom d'affichage.".to_string(),
        (Lang::En, "me.profile_incomplete") => "Profile incomplete: add a display name.".to_string(),
        (Lang::Fr, "me.complete_profile") => "Compléter le profil".to_string(),
        (Lang::En, "me.complete_profile") => "Complete profile".to_string(),
        (Lang::Fr, "auth.not_signed_in") => "Non connecté".to_string(),
        (Lang::En, "auth.not_signed_in") => "Not signed in".to_string(),
        (Lang::Fr, "auth.config_error_prefix") => "Erreur de configuration :".to_string(),
        (Lang::En, "auth.config_error_prefix") => "Config error:".to_string(),
        (Lang::Fr, "auth.auth_error_prefix") => "Erreur d'authentification :".to_string(),
        (Lang::En, "auth.auth_error_prefix") => "Auth error:".to_string(),

        // Fallback: use French string if present, else show key.
        (Lang::En, k) => t(Lang::Fr, k),
        (Lang::Fr, _) => key.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_french_strings() {
        assert_eq!(t(Lang::Fr, "nav.proposals"), "Propositions");
        assert_eq!(t(Lang::En, "nav.proposals"), "Proposals");
    }

    #[test]
    fn fallback_to_french_then_key() {
        // Has French but not English explicitly:
        assert_eq!(t(Lang::En, "lang.label"), t(Lang::Fr, "lang.label"));
        // Missing everywhere returns key:
        assert_eq!(t(Lang::En, "missing.key"), "missing.key");
    }
}
