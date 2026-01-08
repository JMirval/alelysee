use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentTargetType {
    Proposal,
    Program,
    Video,
    Comment,
}

impl ContentTargetType {
    pub fn as_db(&self) -> &'static str {
        match self {
            ContentTargetType::Proposal => "proposal",
            ContentTargetType::Program => "program",
            ContentTargetType::Video => "video",
            ContentTargetType::Comment => "comment",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityAction {
    Created,
    VotedUp,
    VotedDown,
    Commented,
}

impl ActivityAction {
    pub fn as_db(&self) -> &'static str {
        match self {
            ActivityAction::Created => "created",
            ActivityAction::VotedUp => "voted_up",
            ActivityAction::VotedDown => "voted_down",
            ActivityAction::Commented => "commented",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub user_id: Uuid,
    pub display_name: String,
    pub bio: String,
    pub avatar_url: Option<String>,
    pub location: Option<String>,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub author_user_id: Uuid,
    pub title: String,
    pub summary: String,
    pub body_markdown: String,
    pub tags: Vec<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub vote_score: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub id: Uuid,
    pub author_user_id: Uuid,
    pub title: String,
    pub summary: String,
    pub body_markdown: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub vote_score: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramItem {
    pub program_id: Uuid,
    pub proposal_id: Uuid,
    pub position: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Video {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub target_type: ContentTargetType,
    pub target_id: Uuid,
    pub s3_bucket: String,
    pub s3_key: String,
    pub content_type: String,
    pub duration_seconds: Option<i32>,
    pub created_at: OffsetDateTime,
    pub vote_score: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoteState {
    pub target_type: ContentTargetType,
    pub target_id: Uuid,
    pub score: i64,
    pub my_vote: Option<i16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub author_user_id: Uuid,
    pub target_type: ContentTargetType,
    pub target_id: Uuid,
    pub parent_comment_id: Option<Uuid>,
    pub body_markdown: String,
    pub created_at: OffsetDateTime,
    pub vote_score: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub action: ActivityAction,
    pub target_type: ContentTargetType,
    pub target_id: Uuid,
    pub created_at: OffsetDateTime,
    // Best-effort display info for the feed
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UploadIntent {
    pub presigned_put_url: String,
    pub s3_key: String,
    pub bucket: String,
}
