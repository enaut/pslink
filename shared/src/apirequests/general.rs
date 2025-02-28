//! The more generic request datatypes
use std::ops::Deref;

use serde::{Deserialize, Serialize};
/// Filter one column according to the containing string.
#[derive(Clone, Deserialize, Serialize, Debug, Default)]
pub struct Filter {
    pub sieve: String,
}

impl Deref for Filter {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.sieve
    }
}

/// Possible order directions
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub enum Ordering {
    Ascending,
    Descending,
}

/// An operation on a column
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Operation<T, V> {
    pub column: T,
    pub value: V,
}

/// To differentiate between creating a new record and editing.
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
pub enum EditMode {
    Create,
    Edit,
}

impl Default for EditMode {
    fn default() -> Self {
        Self::Create
    }
}

/// When a message is sent between client and server (like for a dialog).
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Message {
    pub message: String,
}

/// Send a message on success and also one on error.
#[derive(Clone, Deserialize, Serialize, Debug)]
pub enum Status<T> {
    Success(T),
    Error(Message),
}
