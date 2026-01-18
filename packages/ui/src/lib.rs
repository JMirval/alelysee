//! This crate contains all shared UI for the workspace.

mod hero;
pub use hero::Hero;

mod navbar;
pub use navbar::Navbar;

mod echo;
pub use echo::Echo;

mod auth;
pub use auth::{
    AuthBootstrap, AuthCallback, AuthGate, MePage, RequestPasswordResetForm,
    ResetPasswordConfirmForm, SignIn, SignOutButton, SignUpForm, VerifyEmailPage,
};

mod proposals;
pub use proposals::{ProposalDetailPage, ProposalListPage, ProposalNewPage};

mod programs;
pub use programs::{ProgramDetailPage, ProgramListPage, ProgramNewPage};

mod vote;
pub use vote::VoteWidget;

mod comments;
pub use comments::CommentThread;

mod profile;
pub use profile::{ActivityFeed, ProfileEditPage};

mod videos;
pub use videos::VideoSection;

mod theme;
pub use theme::CivicTheme;

mod account_menu;
pub use account_menu::AccountMenu;

mod toast;
pub use toast::{use_toasts, ToastProvider};

mod i18n;
pub use i18n::{set_lang, t, use_lang, I18nProvider, Lang};
