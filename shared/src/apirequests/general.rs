use std::ops::Deref;

use serde::{Deserialize, Serialize};
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

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Debug)]
pub enum Ordering {
    Ascending,
    Descending,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Operation<T, V> {
    pub column: T,
    pub value: V,
}

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

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Message {
    pub message: String,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub enum Status {
    Success(Message),
    Error(Message),
}
