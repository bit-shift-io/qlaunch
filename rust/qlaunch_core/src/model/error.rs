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
    #[error("item {0} has both an app and a shortcut")]
    ItemHasAppAndShortcut(Id),
    #[error("item {0} is a shortcut and cannot also have an app")]
    ItemHasShortcut(Id),
    #[error("the root group {0} cannot be a folder")]
    RootIsFolder(Id),
    #[error("folder group {0} has no tile item")]
    FolderGroupHasNoTile(Id),
    #[error("folder group {0} has more than one tile item")]
    FolderGroupHasManyTiles(Id),
    #[error("item {0} is a folder tile and must not have an app or a shortcut")]
    FolderTileHasContent(Id),
    #[error("folder tile {0} cannot live inside a folder")]
    FolderTileInFolder(Id),
    #[error("group {0} is a folder and cannot hold a role")]
    FolderHoldsRole(Id),
    #[error("group {0} is not a folder group")]
    NotAFolderGroup(Id),
    #[error("group {0} cannot contain a folder")]
    NotAFolderDestination(Id),
}
