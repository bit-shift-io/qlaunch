use thiserror::Error;

use super::{AppRef, GroupRole, Id};

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ModelError {
    #[error("the root group is missing or does not hold the Root role")]
    MissingRoot,
    #[error("more than one group holds the {0:?} role")]
    DuplicateRole(GroupRole),
    #[error("no group holds the required {0:?} role")]
    MissingRole(GroupRole),
    #[error("unknown group {0}")]
    UnknownGroup(Id),
    #[error("unknown item {0}")]
    UnknownItem(Id),
    #[error("more than one item refers to {}/{}", .0.package, .0.activity)]
    DuplicateAppRef(AppRef),
    #[error("this role cannot be assigned or unset")]
    RoleNotAssignable,
    #[error("group {0} holds a role")]
    GroupHoldsRole(Id),
    #[error("group {0} is not a page")]
    NotAPage(Id),
    #[error("more than one group has id {0}")]
    DuplicateGroupId(Id),
    #[error("more than one item has id {0}")]
    DuplicateItemId(Id),
    #[error("the root group {0} has a parent")]
    RootHasParent(Id),
    #[error("group {0} has no parent but is not the root")]
    SecondRoot(Id),
    #[error("group {0} is not a child of the root group")]
    NotChildOfRoot(Id),
    #[error("item {0} is in the root group")]
    ItemInRoot(Id),
}
