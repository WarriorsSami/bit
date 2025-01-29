use std::sync::Arc;

pub mod blob_object;
pub mod git_object;
pub mod git_repository;
pub mod object_type;

type ByteArray = Arc<[u8]>;
