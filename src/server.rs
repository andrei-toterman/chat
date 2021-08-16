use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Message {
    Joined(Arc<String>),
    Said(Arc<String>, Arc<String>),
    Left(Arc<String>),
}
