use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Message {
    Joined(Arc<String>),
    Said(Arc<String>, Arc<String>),
    Left(Arc<String>),
    Err(Error),
}

#[derive(Debug, Error, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Error {
    #[error("that username is already taken")]
    UsernameTaken,
    #[error("internal server error")]
    Internal,
    #[error("lost {0} messages due to connection issues")]
    Lost(u64),
}
