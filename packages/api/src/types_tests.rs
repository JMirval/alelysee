#![cfg(test)]

use crate::types::{ActivityAction, ContentTargetType};

#[test]
fn content_target_type_as_db() {
    assert_eq!(ContentTargetType::Proposal.as_db(), "proposal");
    assert_eq!(ContentTargetType::Program.as_db(), "program");
    assert_eq!(ContentTargetType::Video.as_db(), "video");
    assert_eq!(ContentTargetType::Comment.as_db(), "comment");
}

#[test]
fn activity_action_as_db() {
    assert_eq!(ActivityAction::Created.as_db(), "created");
    assert_eq!(ActivityAction::VotedUp.as_db(), "voted_up");
    assert_eq!(ActivityAction::VotedDown.as_db(), "voted_down");
    assert_eq!(ActivityAction::Commented.as_db(), "commented");
}


